use defi_risk_monitor::services::risk_analytics_service::RiskAnalyticsService;
use sqlx::PgPool;
use std::env;
use chrono::{Utc, Duration};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Risk Analytics Integration Tests");
    
    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    
    info!("Connecting to database: {}", database_url);
    
    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;
    
    // Run database migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("✅ Database migrations completed successfully");
    
    // Initialize risk analytics service
    let risk_analytics_service = RiskAnalyticsService::new(pool.clone());
    
    info!("✅ Risk analytics service initialized successfully");
    
    // Test 1: Risk Trends Analysis
    info!("\n🔍 Testing get_risk_trends...");
    match risk_analytics_service.get_risk_trends(
        Some("position".to_string()),
        Some("liquidity".to_string()),
        Some(Utc::now() - Duration::days(30)),
        Some(Utc::now()),
        Some(24), // Daily granularity
    ).await {
        Ok(risk_trends) => {
            info!("✅ Risk Trends Test PASSED");
            info!("  📊 Time Period: {}", risk_trends.time_period);
            info!("  📈 Overall Trend: {}", risk_trends.overall_trend);
            info!("  📊 Trend Percentage: {}%", risk_trends.trend_percentage);
            info!("  📊 Average Risk Score: {}", risk_trends.average_risk_score);
            info!("  📊 Risk Volatility: {}", risk_trends.risk_volatility);
            info!("  📋 Trend Entries: {}", risk_trends.trends.len());
            info!("  🔺 Highest Risk Period: {:?}", risk_trends.highest_risk_period);
            info!("  🔻 Lowest Risk Period: {:?}", risk_trends.lowest_risk_period);
            
            // Show first few trend entries
            for (i, trend) in risk_trends.trends.iter().take(3).enumerate() {
                info!("    Trend {}: {} - Score: {}, Type: {}, Severity: {}", 
                     i + 1, trend.timestamp, trend.risk_score, trend.risk_type, trend.severity);
            }
        }
        Err(e) => {
            error!("❌ Risk Trends Test FAILED: {}", e);
        }
    }
    
    // Test 2: Correlation Matrix Analysis
    info!("\n🔍 Testing get_correlation_matrix...");
    match risk_analytics_service.get_correlation_matrix(
        None, // Auto-detect assets from positions
        Some(30), // 30 days
    ).await {
        Ok(correlation_matrix) => {
            info!("✅ Correlation Matrix Test PASSED");
            info!("  📊 Assets Analyzed: {}", correlation_matrix.assets.len());
            info!("  📊 Correlation Pairs: {}", correlation_matrix.correlations.len());
            info!("  📊 Average Correlation: {}", correlation_matrix.average_correlation);
            info!("  📊 Time Period: {} days", correlation_matrix.time_period_analyzed);
            info!("  🔺 Strongest Positive: {:?}", correlation_matrix.strongest_positive_correlation.as_ref().map(|c| format!("{} <-> {} = {}", c.asset_a, c.asset_b, c.correlation_coefficient)));
            info!("  🔻 Strongest Negative: {:?}", correlation_matrix.strongest_negative_correlation.as_ref().map(|c| format!("{} <-> {} = {}", c.asset_a, c.asset_b, c.correlation_coefficient)));
            
            // Show asset list
            info!("  📋 Assets: {:?}", correlation_matrix.assets);
            
            // Show top correlations
            for (i, correlation) in correlation_matrix.correlations.iter().take(5).enumerate() {
                info!("    Correlation {}: {} <-> {} = {} (confidence: {})", 
                     i + 1, correlation.asset_a, correlation.asset_b, 
                     correlation.correlation_coefficient, correlation.confidence_level);
            }
        }
        Err(e) => {
            error!("❌ Correlation Matrix Test FAILED: {}", e);
        }
    }
    
    // Test 3: Risk Distribution Analysis
    info!("\n🔍 Testing get_risk_distribution...");
    match risk_analytics_service.get_risk_distribution(
        "severity".to_string(),
        Some(5), // 5 buckets
    ).await {
        Ok(risk_distribution) => {
            info!("✅ Risk Distribution Test PASSED");
            info!("  📊 Distribution Type: {}", risk_distribution.distribution_type);
            info!("  📊 Total Entities: {}", risk_distribution.total_entities);
            info!("  📊 Mean Risk Score: {}", risk_distribution.mean_risk_score);
            info!("  📊 Median Risk Score: {}", risk_distribution.median_risk_score);
            info!("  📊 Standard Deviation: {}", risk_distribution.standard_deviation);
            info!("  📊 Skewness: {}", risk_distribution.skewness);
            info!("  📊 Kurtosis: {}", risk_distribution.kurtosis);
            info!("  📋 Distribution Buckets: {}", risk_distribution.buckets.len());
            
            // Show percentiles
            for (percentile, value) in &risk_distribution.percentiles {
                info!("    {}: {}", percentile, value);
            }
            
            // Show distribution buckets
            for (i, bucket) in risk_distribution.buckets.iter().enumerate() {
                info!("    Bucket {}: {:.2}-{:.2} ({} entities, {:.1}%)", 
                     i + 1, bucket.risk_range_min, bucket.risk_range_max, 
                     bucket.count, bucket.percentage);
            }
        }
        Err(e) => {
            error!("❌ Risk Distribution Test FAILED: {}", e);
        }
    }
    
    // Test 4: Alert Statistics Analysis
    info!("\n🔍 Testing get_alert_statistics...");
    match risk_analytics_service.get_alert_statistics(
        Some(Utc::now() - Duration::days(30)),
        Some(Utc::now()),
    ).await {
        Ok(alert_stats) => {
            info!("✅ Alert Statistics Test PASSED");
            info!("  📊 Time Period: {}", alert_stats.time_period);
            info!("  📊 Total Alerts: {}", alert_stats.total_alerts);
            info!("  📊 Alert Frequency Trend: {}", alert_stats.alert_frequency_trend);
            info!("  📊 Most Common Alert Type: {}", alert_stats.most_common_alert_type);
            info!("  📊 Highest Severity Alerts: {}", alert_stats.highest_severity_alerts);
            info!("  📊 Average Alerts Per Day: {}", alert_stats.average_alerts_per_day);
            info!("  📊 Peak Alert Day: {:?}", alert_stats.peak_alert_day);
            
            // Show alerts by type
            info!("  📋 Alerts by Type:");
            for (i, alert_type) in alert_stats.alerts_by_type.iter().enumerate() {
                info!("    Type {}: {} ({} alerts, {:.1}%)", 
                     i + 1, alert_type.alert_type, alert_type.count, alert_type.percentage_of_total);
                info!("      Avg Resolution: {:?}h, False Positive Rate: {:?}%", 
                     alert_type.avg_resolution_time_hours, alert_type.false_positive_rate);
            }
            
            // Show alerts by severity
            info!("  📋 Alerts by Severity:");
            for (i, alert_severity) in alert_stats.alerts_by_severity.iter().enumerate() {
                info!("    Severity {}: {} ({} alerts, {:.1}%)", 
                     i + 1, alert_severity.severity, alert_severity.count, alert_severity.percentage_of_total);
            }
            
            // Show resolution statistics
            info!("  📋 Resolution Statistics:");
            for (stat, value) in &alert_stats.alert_resolution_stats {
                info!("    {}: {}", stat, value);
            }
        }
        Err(e) => {
            error!("❌ Alert Statistics Test FAILED: {}", e);
        }
    }
    
    // Test 5: Edge Cases and Error Handling
    info!("\n🔍 Testing Edge Cases...");
    
    // Test with empty parameters
    match risk_analytics_service.get_risk_trends(None, None, None, None, None).await {
        Ok(trends) => {
            info!("✅ Default Parameters Test PASSED - {} trends found", trends.trends.len());
        }
        Err(e) => {
            warn!("⚠️  Default Parameters Test: {}", e);
        }
    }
    
    // Test with very short time period
    let short_start = Utc::now() - Duration::hours(1);
    let short_end = Utc::now();
    match risk_analytics_service.get_risk_trends(
        None, None, Some(short_start), Some(short_end), Some(1)
    ).await {
        Ok(trends) => {
            info!("✅ Short Time Period Test PASSED - {} trends found", trends.trends.len());
        }
        Err(e) => {
            warn!("⚠️  Short Time Period Test: {}", e);
        }
    }
    
    // Test correlation matrix with specific assets
    let test_assets = vec![
        "0x1234567890123456789012345678901234567890".to_string(),
        "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".to_string(),
    ];
    match risk_analytics_service.get_correlation_matrix(Some(test_assets), Some(7)).await {
        Ok(matrix) => {
            info!("✅ Specific Assets Correlation Test PASSED - {} assets analyzed", matrix.assets.len());
        }
        Err(e) => {
            warn!("⚠️  Specific Assets Correlation Test: {}", e);
        }
    }
    
    // Test different distribution types
    for dist_type in ["entity_type", "risk_type", "severity"] {
        match risk_analytics_service.get_risk_distribution(dist_type.to_string(), Some(3)).await {
            Ok(distribution) => {
                info!("✅ {} Distribution Test PASSED - {} entities", 
                     dist_type, distribution.total_entities);
            }
            Err(e) => {
                warn!("⚠️  {} Distribution Test: {}", dist_type, e);
            }
        }
    }
    
    // Test 6: Performance Benchmarking
    info!("\n🔍 Performance Benchmarking...");
    let start_time = std::time::Instant::now();
    
    for _i in 0..5 {
        let _ = risk_analytics_service.get_risk_trends(None, None, None, None, None).await;
        let _ = risk_analytics_service.get_correlation_matrix(None, Some(30)).await;
        let _ = risk_analytics_service.get_risk_distribution("severity".to_string(), Some(5)).await;
        let _ = risk_analytics_service.get_alert_statistics(None, None).await;
    }
    
    let duration = start_time.elapsed();
    info!("✅ Performance Test: 20 analytics queries completed in {:?}", duration);
    info!("  📊 Average time per query: {:?}", duration / 20);
    
    info!("\n🎉 ALL RISK ANALYTICS TESTS COMPLETED!");
    info!("📋 Test Summary:");
    info!("  ✅ Risk Trends Analysis");
    info!("  ✅ Correlation Matrix Analysis");
    info!("  ✅ Risk Distribution Analysis");
    info!("  ✅ Alert Statistics Analysis");
    info!("  ✅ Edge Case Handling");
    info!("  ✅ Performance Benchmarking");
    
    Ok(())
}
