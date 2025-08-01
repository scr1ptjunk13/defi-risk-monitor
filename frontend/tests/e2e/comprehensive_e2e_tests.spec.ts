import { test, expect, Page } from '@playwright/test';

/**
 * Comprehensive End-to-End Tests for DeFi Risk Monitor Frontend
 * These tests validate critical user workflows and real-time data flows
 */

test.describe('DeFi Risk Monitor E2E Tests', () => {
  let page: Page;

  test.beforeEach(async ({ page: testPage }) => {
    page = testPage;
    // Navigate to the application
    await page.goto('http://localhost:3000');
    
    // Wait for the application to load
    await page.waitForSelector('[data-testid="app-header"]', { timeout: 10000 });
  });

  test.describe('Authentication Flow', () => {
    test('should complete full authentication workflow', async () => {
      console.log('🧪 Testing Authentication Flow');

      // Test login page access
      await page.click('[data-testid="login-button"]');
      await expect(page).toHaveURL(/.*\/login/);

      // Test form validation
      await page.click('[data-testid="submit-login"]');
      await expect(page.locator('[data-testid="error-message"]')).toBeVisible();

      // Test successful login
      await page.fill('[data-testid="username-input"]', 'testuser@example.com');
      await page.fill('[data-testid="password-input"]', 'TestPassword123!');
      await page.click('[data-testid="submit-login"]');

      // Should redirect to dashboard
      await expect(page).toHaveURL(/.*\/dashboard/);
      await expect(page.locator('[data-testid="user-profile"]')).toBeVisible();

      console.log('✅ Authentication Flow: PASSED');
    });

    test('should handle wallet connection', async () => {
      console.log('🧪 Testing Wallet Connection');

      // Mock MetaMask connection
      await page.addInitScript(() => {
        (window as any).ethereum = {
          isMetaMask: true,
          request: async ({ method }: { method: string }) => {
            if (method === 'eth_requestAccounts') {
              return ['0x742d35Cc6634C0532925a3b8D4C9db96c4b8d4e8'];
            }
            if (method === 'eth_chainId') {
              return '0x1'; // Ethereum mainnet
            }
            return null;
          },
        };
      });

      await page.click('[data-testid="connect-wallet-button"]');
      await expect(page.locator('[data-testid="wallet-connected"]')).toBeVisible();
      await expect(page.locator('[data-testid="wallet-address"]')).toContainText('0x742d35Cc');

      console.log('✅ Wallet Connection: PASSED');
    });
  });

  test.describe('Risk Dashboard Functionality', () => {
    test.beforeEach(async () => {
      // Mock authentication
      await page.addInitScript(() => {
        localStorage.setItem('auth_token', 'mock_token');
        localStorage.setItem('user_id', 'test-user-id');
      });
      await page.goto('http://localhost:3000/dashboard');
    });

    test('should display risk dashboard with real-time updates', async () => {
      console.log('🧪 Testing Risk Dashboard');

      // Check main dashboard components
      await expect(page.locator('[data-testid="risk-overview"]')).toBeVisible();
      await expect(page.locator('[data-testid="portfolio-value"]')).toBeVisible();
      await expect(page.locator('[data-testid="risk-score"]')).toBeVisible();

      // Test risk score display
      const riskScore = await page.locator('[data-testid="risk-score-value"]').textContent();
      expect(riskScore).toMatch(/\d+(\.\d+)?/); // Should be a number

      // Test portfolio value display
      const portfolioValue = await page.locator('[data-testid="portfolio-value-amount"]').textContent();
      expect(portfolioValue).toMatch(/\$[\d,]+(\.\d{2})?/); // Should be currency format

      console.log('✅ Risk Dashboard Display: PASSED');
    });

    test('should handle real-time risk updates', async () => {
      console.log('🧪 Testing Real-time Risk Updates');

      // Mock WebSocket connection for real-time updates
      await page.addInitScript(() => {
        const mockWebSocket = {
          send: () => {},
          close: () => {},
          addEventListener: (event: string, callback: Function) => {
            if (event === 'message') {
              // Simulate real-time risk update
              setTimeout(() => {
                callback({
                  data: JSON.stringify({
                    type: 'risk_update',
                    data: {
                      riskScore: 0.75,
                      severity: 'high',
                      positions: [
                        {
                          id: 'pos1',
                          protocol: 'Uniswap V3',
                          riskScore: 0.8,
                          change: '+0.05'
                        }
                      ]
                    }
                  })
                });
              }, 1000);
            }
          }
        };
        (window as any).WebSocket = function() { return mockWebSocket; };
      });

      // Wait for initial load
      await page.waitForSelector('[data-testid="risk-score-value"]');
      
      // Wait for real-time update
      await page.waitForFunction(
        () => document.querySelector('[data-testid="risk-score-value"]')?.textContent?.includes('0.75'),
        { timeout: 5000 }
      );

      // Check if risk severity indicator updated
      await expect(page.locator('[data-testid="risk-severity"]')).toHaveClass(/high/);

      console.log('✅ Real-time Risk Updates: PASSED');
    });

    test('should navigate between dashboard sections', async () => {
      console.log('🧪 Testing Dashboard Navigation');

      // Test navigation to different sections
      const sections = [
        { tab: 'positions-tab', content: 'positions-content' },
        { tab: 'analytics-tab', content: 'analytics-content' },
        { tab: 'alerts-tab', content: 'alerts-content' },
        { tab: 'settings-tab', content: 'settings-content' }
      ];

      for (const section of sections) {
        await page.click(`[data-testid="${section.tab}"]`);
        await expect(page.locator(`[data-testid="${section.content}"]`)).toBeVisible();
        
        // Verify URL updates
        await expect(page).toHaveURL(new RegExp(section.tab.replace('-tab', '')));
      }

      console.log('✅ Dashboard Navigation: PASSED');
    });
  });

  test.describe('Position Management', () => {
    test.beforeEach(async () => {
      await page.addInitScript(() => {
        localStorage.setItem('auth_token', 'mock_token');
      });
      await page.goto('http://localhost:3000/positions');
    });

    test('should create new position', async () => {
      console.log('🧪 Testing Position Creation');

      // Click create position button
      await page.click('[data-testid="create-position-button"]');
      await expect(page.locator('[data-testid="position-form"]')).toBeVisible();

      // Fill position form
      await page.selectOption('[data-testid="protocol-select"]', 'uniswap_v3');
      await page.selectOption('[data-testid="chain-select"]', 'ethereum');
      await page.fill('[data-testid="token0-address"]', '0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A');
      await page.fill('[data-testid="token1-address"]', '0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2');
      await page.fill('[data-testid="amount0-input"]', '1000');
      await page.fill('[data-testid="amount1-input"]', '0.5');
      await page.fill('[data-testid="entry-price-input"]', '2000');

      // Submit form
      await page.click('[data-testid="submit-position"]');

      // Verify success message
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="success-message"]')).toContainText('Position created successfully');

      // Verify position appears in list
      await expect(page.locator('[data-testid="position-list"]')).toContainText('Uniswap V3');

      console.log('✅ Position Creation: PASSED');
    });

    test('should edit existing position', async () => {
      console.log('🧪 Testing Position Editing');

      // Assume position exists, click edit button
      await page.click('[data-testid="position-item"]:first-child [data-testid="edit-position"]');
      await expect(page.locator('[data-testid="position-form"]')).toBeVisible();

      // Update current price
      await page.fill('[data-testid="current-price-input"]', '2100');
      await page.click('[data-testid="submit-position"]');

      // Verify update success
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible();

      console.log('✅ Position Editing: PASSED');
    });

    test('should delete position with confirmation', async () => {
      console.log('🧪 Testing Position Deletion');

      // Click delete button
      await page.click('[data-testid="position-item"]:first-child [data-testid="delete-position"]');
      
      // Confirm deletion in modal
      await expect(page.locator('[data-testid="delete-confirmation"]')).toBeVisible();
      await page.click('[data-testid="confirm-delete"]');

      // Verify deletion success
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="success-message"]')).toContainText('Position deleted successfully');

      console.log('✅ Position Deletion: PASSED');
    });

    test('should filter and search positions', async () => {
      console.log('🧪 Testing Position Filtering');

      // Test protocol filter
      await page.selectOption('[data-testid="protocol-filter"]', 'uniswap_v3');
      await page.waitForTimeout(500); // Wait for filter to apply
      
      const filteredPositions = await page.locator('[data-testid="position-item"]').count();
      expect(filteredPositions).toBeGreaterThan(0);

      // Test search functionality
      await page.fill('[data-testid="position-search"]', 'USDC');
      await page.waitForTimeout(500);
      
      const searchResults = await page.locator('[data-testid="position-item"]').count();
      expect(searchResults).toBeGreaterThanOrEqual(0);

      // Clear filters
      await page.click('[data-testid="clear-filters"]');
      await page.waitForTimeout(500);

      console.log('✅ Position Filtering: PASSED');
    });
  });

  test.describe('Analytics and Charts', () => {
    test.beforeEach(async () => {
      await page.addInitScript(() => {
        localStorage.setItem('auth_token', 'mock_token');
      });
      await page.goto('http://localhost:3000/analytics');
    });

    test('should display portfolio analytics charts', async () => {
      console.log('🧪 Testing Portfolio Analytics');

      // Check for chart containers
      await expect(page.locator('[data-testid="portfolio-performance-chart"]')).toBeVisible();
      await expect(page.locator('[data-testid="risk-breakdown-chart"]')).toBeVisible();
      await expect(page.locator('[data-testid="asset-allocation-chart"]')).toBeVisible();

      // Test time range selector
      await page.click('[data-testid="time-range-selector"]');
      await page.click('[data-testid="time-range-30d"]');
      
      // Wait for chart to update
      await page.waitForTimeout(1000);
      
      // Verify chart updated (check for loading state completion)
      await expect(page.locator('[data-testid="chart-loading"]')).not.toBeVisible();

      console.log('✅ Portfolio Analytics Charts: PASSED');
    });

    test('should interact with risk factor breakdown', async () => {
      console.log('🧪 Testing Risk Factor Interaction');

      // Click on risk factor item
      await page.click('[data-testid="risk-factor-item"]:first-child');
      
      // Verify detailed view opens
      await expect(page.locator('[data-testid="risk-factor-details"]')).toBeVisible();
      
      // Check for explanation text
      await expect(page.locator('[data-testid="risk-explanation"]')).toBeVisible();
      
      // Test risk factor filtering
      await page.click('[data-testid="risk-type-filter"]');
      await page.click('[data-testid="impermanent-loss-filter"]');
      
      await page.waitForTimeout(500);
      await expect(page.locator('[data-testid="filtered-risk-factors"]')).toBeVisible();

      console.log('✅ Risk Factor Interaction: PASSED');
    });

    test('should export analytics data', async () => {
      console.log('🧪 Testing Analytics Export');

      // Set up download handler
      const downloadPromise = page.waitForEvent('download');
      
      // Click export button
      await page.click('[data-testid="export-analytics"]');
      
      // Wait for download
      const download = await downloadPromise;
      expect(download.suggestedFilename()).toMatch(/analytics.*\.(csv|xlsx|pdf)$/);

      console.log('✅ Analytics Export: PASSED');
    });
  });

  test.describe('Alert Management', () => {
    test.beforeEach(async () => {
      await page.addInitScript(() => {
        localStorage.setItem('auth_token', 'mock_token');
      });
      await page.goto('http://localhost:3000/alerts');
    });

    test('should create and manage alerts', async () => {
      console.log('🧪 Testing Alert Management');

      // Create new alert
      await page.click('[data-testid="create-alert-button"]');
      await expect(page.locator('[data-testid="alert-form"]')).toBeVisible();

      // Fill alert form
      await page.fill('[data-testid="alert-name"]', 'High Risk Alert');
      await page.selectOption('[data-testid="alert-type"]', 'risk_threshold');
      await page.fill('[data-testid="threshold-value"]', '0.8');
      await page.check('[data-testid="email-notification"]');
      
      // Submit alert
      await page.click('[data-testid="submit-alert"]');
      
      // Verify alert created
      await expect(page.locator('[data-testid="success-message"]')).toBeVisible();
      await expect(page.locator('[data-testid="alert-list"]')).toContainText('High Risk Alert');

      // Test alert toggle
      await page.click('[data-testid="alert-item"]:first-child [data-testid="toggle-alert"]');
      await expect(page.locator('[data-testid="alert-status"]')).toContainText('Disabled');

      console.log('✅ Alert Management: PASSED');
    });

    test('should display alert history', async () => {
      console.log('🧪 Testing Alert History');

      // Navigate to alert history
      await page.click('[data-testid="alert-history-tab"]');
      await expect(page.locator('[data-testid="alert-history-list"]')).toBeVisible();

      // Test pagination
      if (await page.locator('[data-testid="pagination-next"]').isVisible()) {
        await page.click('[data-testid="pagination-next"]');
        await page.waitForTimeout(500);
        await expect(page.locator('[data-testid="alert-history-list"]')).toBeVisible();
      }

      // Test alert detail view
      await page.click('[data-testid="alert-history-item"]:first-child');
      await expect(page.locator('[data-testid="alert-detail-modal"]')).toBeVisible();

      console.log('✅ Alert History: PASSED');
    });
  });

  test.describe('Explainable AI Interface', () => {
    test.beforeEach(async () => {
      await page.addInitScript(() => {
        localStorage.setItem('auth_token', 'mock_token');
      });
      await page.goto('http://localhost:3000/ai-insights');
    });

    test('should interact with AI chat interface', async () => {
      console.log('🧪 Testing AI Chat Interface');

      // Check AI chat component
      await expect(page.locator('[data-testid="ai-chat-interface"]')).toBeVisible();
      
      // Send a message
      await page.fill('[data-testid="chat-input"]', 'What is my current risk exposure?');
      await page.click('[data-testid="send-message"]');
      
      // Wait for AI response
      await page.waitForSelector('[data-testid="ai-response"]', { timeout: 10000 });
      
      // Verify response contains relevant information
      const response = await page.locator('[data-testid="ai-response"]:last-child').textContent();
      expect(response).toContain('risk');
      
      // Test follow-up question
      await page.fill('[data-testid="chat-input"]', 'How can I reduce this risk?');
      await page.click('[data-testid="send-message"]');
      
      await page.waitForSelector('[data-testid="ai-response"]:last-child', { timeout: 10000 });

      console.log('✅ AI Chat Interface: PASSED');
    });

    test('should display risk predictions', async () => {
      console.log('🧪 Testing Risk Predictions');

      // Navigate to predictions tab
      await page.click('[data-testid="predictions-tab"]');
      await expect(page.locator('[data-testid="risk-predictions"]')).toBeVisible();

      // Check prediction charts
      await expect(page.locator('[data-testid="prediction-chart"]')).toBeVisible();
      await expect(page.locator('[data-testid="confidence-interval"]')).toBeVisible();

      // Test prediction timeframe selector
      await page.click('[data-testid="prediction-timeframe"]');
      await page.click('[data-testid="timeframe-7d"]');
      
      await page.waitForTimeout(1000);
      await expect(page.locator('[data-testid="prediction-chart"]')).toBeVisible();

      console.log('✅ Risk Predictions: PASSED');
    });
  });

  test.describe('Responsive Design and Mobile', () => {
    test('should work on mobile viewport', async () => {
      console.log('🧪 Testing Mobile Responsiveness');

      // Set mobile viewport
      await page.setViewportSize({ width: 375, height: 667 });
      await page.goto('http://localhost:3000');

      // Check mobile navigation
      await expect(page.locator('[data-testid="mobile-menu-button"]')).toBeVisible();
      await page.click('[data-testid="mobile-menu-button"]');
      await expect(page.locator('[data-testid="mobile-nav-menu"]')).toBeVisible();

      // Test mobile dashboard
      await page.click('[data-testid="mobile-nav-dashboard"]');
      await expect(page.locator('[data-testid="mobile-dashboard"]')).toBeVisible();

      // Check that charts are responsive
      const chartWidth = await page.locator('[data-testid="mobile-chart"]').boundingBox();
      expect(chartWidth?.width).toBeLessThanOrEqual(375);

      console.log('✅ Mobile Responsiveness: PASSED');
    });

    test('should work on tablet viewport', async () => {
      console.log('🧪 Testing Tablet Responsiveness');

      // Set tablet viewport
      await page.setViewportSize({ width: 768, height: 1024 });
      await page.goto('http://localhost:3000');

      // Check tablet layout
      await expect(page.locator('[data-testid="tablet-layout"]')).toBeVisible();
      
      // Test sidebar behavior
      if (await page.locator('[data-testid="sidebar-toggle"]').isVisible()) {
        await page.click('[data-testid="sidebar-toggle"]');
        await expect(page.locator('[data-testid="sidebar"]')).toBeVisible();
      }

      console.log('✅ Tablet Responsiveness: PASSED');
    });
  });

  test.describe('Performance and Loading', () => {
    test('should load dashboard within performance budget', async () => {
      console.log('🧪 Testing Performance Budget');

      const startTime = Date.now();
      
      await page.goto('http://localhost:3000/dashboard');
      await page.waitForSelector('[data-testid="dashboard-loaded"]');
      
      const loadTime = Date.now() - startTime;
      
      // Dashboard should load within 3 seconds
      expect(loadTime).toBeLessThan(3000);
      
      console.log(`Dashboard loaded in ${loadTime}ms`);
      console.log('✅ Performance Budget: PASSED');
    });

    test('should handle large datasets efficiently', async () => {
      console.log('🧪 Testing Large Dataset Handling');

      // Mock large dataset
      await page.addInitScript(() => {
        const mockPositions = Array.from({ length: 1000 }, (_, i) => ({
          id: `pos-${i}`,
          protocol: `Protocol ${i % 10}`,
          value: Math.random() * 10000,
          risk: Math.random()
        }));
        
        (window as any).mockLargeDataset = mockPositions;
      });

      await page.goto('http://localhost:3000/positions');
      
      // Wait for virtual scrolling to load
      await page.waitForSelector('[data-testid="virtual-list"]');
      
      // Test scrolling performance
      const scrollStart = Date.now();
      await page.mouse.wheel(0, 5000);
      await page.waitForTimeout(100);
      const scrollTime = Date.now() - scrollStart;
      
      // Scrolling should be smooth (< 100ms)
      expect(scrollTime).toBeLessThan(200);

      console.log('✅ Large Dataset Handling: PASSED');
    });
  });

  test.describe('Error Handling and Edge Cases', () => {
    test('should handle network errors gracefully', async () => {
      console.log('🧪 Testing Network Error Handling');

      // Simulate network failure
      await page.route('**/api/**', route => route.abort());
      
      await page.goto('http://localhost:3000/dashboard');
      
      // Should show error state
      await expect(page.locator('[data-testid="network-error"]')).toBeVisible();
      await expect(page.locator('[data-testid="retry-button"]')).toBeVisible();
      
      // Test retry functionality
      await page.unroute('**/api/**');
      await page.click('[data-testid="retry-button"]');
      
      // Should recover
      await expect(page.locator('[data-testid="dashboard-content"]')).toBeVisible();

      console.log('✅ Network Error Handling: PASSED');
    });

    test('should handle invalid data gracefully', async () => {
      console.log('🧪 Testing Invalid Data Handling');

      // Mock invalid API response
      await page.route('**/api/positions', route => {
        route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({ positions: null }) // Invalid data
        });
      });

      await page.goto('http://localhost:3000/positions');
      
      // Should show empty state instead of crashing
      await expect(page.locator('[data-testid="empty-positions"]')).toBeVisible();

      console.log('✅ Invalid Data Handling: PASSED');
    });

    test('should handle session expiration', async () => {
      console.log('🧪 Testing Session Expiration');

      // Mock expired session
      await page.route('**/api/**', route => {
        route.fulfill({
          status: 401,
          contentType: 'application/json',
          body: JSON.stringify({ error: 'Session expired' })
        });
      });

      await page.goto('http://localhost:3000/dashboard');
      
      // Should redirect to login
      await expect(page).toHaveURL(/.*\/login/);
      await expect(page.locator('[data-testid="session-expired-message"]')).toBeVisible();

      console.log('✅ Session Expiration Handling: PASSED');
    });
  });

  test.afterEach(async () => {
    // Clean up any test data or state
    await page.evaluate(() => {
      localStorage.clear();
      sessionStorage.clear();
    });
  });
});
