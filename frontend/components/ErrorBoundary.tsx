import React, { Component, ErrorInfo, ReactNode } from 'react';
import { Button, Result, Typography, Collapse, Space, Tag } from 'antd';
import { ReloadOutlined, BugOutlined, DownloadOutlined } from '@ant-design/icons';
import { errorHandler, ErrorSeverity, ErrorCategory } from '../lib/error-handling';

const { Text, Paragraph } = Typography;
const { Panel } = Collapse;

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
  onError?: (error: Error, errorInfo: ErrorInfo) => void;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  errorId: string | null;
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null
    };
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    return {
      hasError: true,
      error
    };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    // Log error using our enhanced error handling system
    const enhancedError = errorHandler.handleError(error, {
      severity: ErrorSeverity.CRITICAL,
      category: ErrorCategory.UI,
      component: 'ErrorBoundary',
      action: 'Component Render',
      showToast: false, // Don't show toast for boundary errors
      additionalData: {
        componentStack: errorInfo.componentStack,
        errorBoundary: true
      }
    });

    this.setState({
      errorInfo,
      errorId: enhancedError.id
    });

    // Call custom error handler if provided
    if (this.props.onError) {
      this.props.onError(error, errorInfo);
    }
  }

  handleReload = () => {
    window.location.reload();
  };

  handleReset = () => {
    this.setState({
      hasError: false,
      error: null,
      errorInfo: null,
      errorId: null
    });
  };

  downloadErrorReport = () => {
    if (!this.state.error || !this.state.errorId) return;

    const errorReport = {
      errorId: this.state.errorId,
      timestamp: new Date().toISOString(),
      error: {
        message: this.state.error.message,
        stack: this.state.error.stack,
        name: this.state.error.name
      },
      errorInfo: this.state.errorInfo,
      userAgent: navigator.userAgent,
      url: window.location.href,
      viewport: {
        width: window.innerWidth,
        height: window.innerHeight
      }
    };

    const blob = new Blob([JSON.stringify(errorReport, null, 2)], {
      type: 'application/json'
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `error-report-${this.state.errorId}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  render() {
    if (this.state.hasError) {
      // Custom fallback UI if provided
      if (this.props.fallback) {
        return this.props.fallback;
      }

      // Default error UI
      return (
        <div style={{ padding: '20px', minHeight: '400px' }}>
          <Result
            status="error"
            title="Something went wrong"
            subTitle="An unexpected error occurred while rendering this component."
            extra={[
              <Button type="primary" icon={<ReloadOutlined />} onClick={this.handleReload} key="reload">
                Reload Page
              </Button>,
              <Button icon={<BugOutlined />} onClick={this.handleReset} key="reset">
                Try Again
              </Button>
            ]}
          >
            <div style={{ textAlign: 'left', maxWidth: '600px', margin: '0 auto' }}>
              {this.state.errorId && (
                <div style={{ marginBottom: '16px' }}>
                  <Text strong>Error ID: </Text>
                  <Tag color="red">{this.state.errorId}</Tag>
                  <Button
                    size="small"
                    icon={<DownloadOutlined />}
                    onClick={this.downloadErrorReport}
                    style={{ marginLeft: '8px' }}
                  >
                    Download Report
                  </Button>
                </div>
              )}

              <Collapse ghost>
                <Panel header="Error Details" key="details">
                  <Space direction="vertical" style={{ width: '100%' }}>
                    <div>
                      <Text strong>Error Message:</Text>
                      <Paragraph code copyable style={{ marginTop: '4px' }}>
                        {this.state.error?.message}
                      </Paragraph>
                    </div>

                    {this.state.error?.stack && (
                      <div>
                        <Text strong>Stack Trace:</Text>
                        <Paragraph
                          code
                          copyable
                          style={{
                            marginTop: '4px',
                            maxHeight: '200px',
                            overflow: 'auto',
                            fontSize: '11px'
                          }}
                        >
                          {this.state.error.stack}
                        </Paragraph>
                      </div>
                    )}

                    {this.state.errorInfo?.componentStack && (
                      <div>
                        <Text strong>Component Stack:</Text>
                        <Paragraph
                          code
                          copyable
                          style={{
                            marginTop: '4px',
                            maxHeight: '200px',
                            overflow: 'auto',
                            fontSize: '11px'
                          }}
                        >
                          {this.state.errorInfo.componentStack}
                        </Paragraph>
                      </div>
                    )}
                  </Space>
                </Panel>
              </Collapse>

              <div style={{ marginTop: '16px', padding: '12px', backgroundColor: '#f6f8fa', borderRadius: '6px' }}>
                <Text type="secondary">
                  <strong>What can you do?</strong>
                  <ul style={{ marginTop: '8px', marginBottom: '0' }}>
                    <li>Try reloading the page</li>
                    <li>Clear your browser cache and cookies</li>
                    <li>Try using an incognito/private browsing window</li>
                    <li>If the problem persists, please contact support with the error ID above</li>
                  </ul>
                </Text>
              </div>
            </div>
          </Result>
        </div>
      );
    }

    return this.props.children;
  }
}

// Higher-order component for wrapping components with error boundary
export function withErrorBoundary<P extends object>(
  Component: React.ComponentType<P>,
  fallback?: ReactNode,
  onError?: (error: Error, errorInfo: ErrorInfo) => void
) {
  const WrappedComponent = (props: P) => (
    <ErrorBoundary fallback={fallback} onError={onError}>
      <Component {...props} />
    </ErrorBoundary>
  );

  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;
  return WrappedComponent;
}

// Specialized error boundaries for different parts of the app
export const DashboardErrorBoundary: React.FC<{ children: ReactNode }> = ({ children }) => (
  <ErrorBoundary
    onError={(error, errorInfo) => {
      errorHandler.handleError(error, {
        severity: ErrorSeverity.HIGH,
        category: ErrorCategory.UI,
        component: 'Dashboard',
        action: 'Render',
        additionalData: { componentStack: errorInfo.componentStack }
      });
    }}
  >
    {children}
  </ErrorBoundary>
);

export const PositionErrorBoundary: React.FC<{ children: ReactNode }> = ({ children }) => (
  <ErrorBoundary
    onError={(error, errorInfo) => {
      errorHandler.handleError(error, {
        severity: ErrorSeverity.HIGH,
        category: ErrorCategory.UI,
        component: 'Position',
        action: 'Render',
        additionalData: { componentStack: errorInfo.componentStack }
      });
    }}
  >
    {children}
  </ErrorBoundary>
);

export const ChartErrorBoundary: React.FC<{ children: ReactNode }> = ({ children }) => (
  <ErrorBoundary
    fallback={
      <div style={{ padding: '20px', textAlign: 'center', border: '1px dashed #d9d9d9', borderRadius: '6px' }}>
        <BugOutlined style={{ fontSize: '24px', color: '#999', marginBottom: '8px' }} />
        <div>Chart failed to load</div>
        <Button size="small" onClick={() => window.location.reload()} style={{ marginTop: '8px' }}>
          Reload
        </Button>
      </div>
    }
    onError={(error, errorInfo) => {
      errorHandler.handleError(error, {
        severity: ErrorSeverity.MEDIUM,
        category: ErrorCategory.UI,
        component: 'Chart',
        action: 'Render',
        additionalData: { componentStack: errorInfo.componentStack }
      });
    }}
  >
    {children}
  </ErrorBoundary>
);
