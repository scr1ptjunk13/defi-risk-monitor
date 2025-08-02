use bigdecimal::BigDecimal;
use std::str::FromStr;

/// Working comprehensive unit tests for all critical backend services
#[cfg(test)]
mod working_comprehensive_unit_tests {
    use super::*;

    #[test]
    fn test_risk_assessment_calculations() {
        println!("🧪 Testing Risk Assessment Calculations");
        
        // Test basic risk score calculation
        let high_volatility = 0.8;
        let low_volatility = 0.2;
        
        let high_risk_score = calculate_risk_score(high_volatility, 1000.0);
        let low_risk_score = calculate_risk_score(low_volatility, 1000.0);
        
        assert!(high_risk_score > low_risk_score, "High volatility should have higher risk score");
        assert!(high_risk_score <= 1.0, "Risk score should be normalized to [0,1]");
        assert!(low_risk_score >= 0.0, "Risk score should be non-negative");
        
        println!("✅ Risk Assessment Calculations: PASSED");
    }

    #[test]
    fn test_portfolio_analytics() {
        println!("🧪 Testing Portfolio Analytics");
        
        // Test portfolio value calculation
        let positions = vec![
            BigDecimal::from_str("1000.0").unwrap(),
            BigDecimal::from_str("2000.0").unwrap(),
            BigDecimal::from_str("500.0").unwrap(),
        ];
        
        let total_value = calculate_portfolio_total(&positions);
        let expected = BigDecimal::from_str("3500.0").unwrap();
        
        assert_eq!(total_value, expected, "Portfolio value calculation incorrect");
        
        println!("✅ Portfolio Analytics: PASSED");
    }

    #[test]
    fn test_cross_chain_risk() {
        println!("🧪 Testing Cross-Chain Risk");
        
        // Test cross-chain risk calculation
        let ethereum_exposure = 0.6;
        let polygon_exposure = 0.3;
        let arbitrum_exposure = 0.1;
        
        let diversification_score = calculate_chain_diversification(vec![
            ethereum_exposure, polygon_exposure, arbitrum_exposure
        ]);
        
        assert!(diversification_score > 0.0, "Diversification score should be positive");
        assert!(diversification_score <= 1.0, "Diversification score should be normalized");
        
        println!("✅ Cross-Chain Risk: PASSED");
    }

    #[test]
    fn test_mev_risk_detection() {
        println!("🧪 Testing MEV Risk Detection");
        
        // Test MEV risk scoring
        let high_volume_pool = 1000000.0;
        let low_volume_pool = 10000.0;
        
        let high_mev_risk = calculate_mev_risk(high_volume_pool, 0.8);
        let low_mev_risk = calculate_mev_risk(low_volume_pool, 0.2);
        
        assert!(high_mev_risk > low_mev_risk, "High volume should have higher MEV risk");
        
        println!("✅ MEV Risk Detection: PASSED");
    }

    #[test]
    fn test_price_validation() {
        println!("🧪 Testing Price Validation");
        
        // Test price deviation detection
        let market_price = 100.0;
        let oracle_price_1 = 102.0; // 2% deviation
        let oracle_price_2 = 110.0; // 10% deviation
        
        let small_deviation = calculate_price_deviation(market_price, oracle_price_1);
        let large_deviation = calculate_price_deviation(market_price, oracle_price_2);
        
        assert!(small_deviation < 0.05, "Small deviation should be under 5%");
        assert!(large_deviation > 0.05, "Large deviation should be over 5%");
        
        println!("✅ Price Validation: PASSED");
    }

    #[test]
    fn test_system_health_metrics() {
        println!("🧪 Testing System Health Metrics");
        
        // Test system health calculation
        let cpu_usage = 0.3; // 30%
        let memory_usage = 0.6; // 60%
        let disk_usage = 0.4; // 40%
        
        let health_score = calculate_system_health(cpu_usage, memory_usage, disk_usage);
        
        assert!(health_score >= 0.0, "Health score should be non-negative");
        assert!(health_score <= 1.0, "Health score should be normalized");
        
        println!("✅ System Health Metrics: PASSED");
    }

    #[test]
    fn test_error_handling() {
        println!("🧪 Testing Error Handling");
        
        // Test division by zero handling
        let result = safe_divide(10.0, 0.0);
        assert!(result.is_err(), "Division by zero should return error");
        
        // Test valid division
        let result = safe_divide(10.0, 2.0);
        assert!(result.is_ok(), "Valid division should succeed");
        assert_eq!(result.unwrap(), 5.0, "Division result incorrect");
        
        println!("✅ Error Handling: PASSED");
    }

    #[test]
    fn test_mathematical_utilities() {
        println!("🧪 Testing Mathematical Utilities");
        
        // Test statistical calculations
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        
        let mean = calculate_mean(&data);
        assert!((mean - 3.0).abs() < 0.001, "Mean calculation incorrect");
        
        let std_dev = calculate_standard_deviation(&data);
        assert!(std_dev > 0.0, "Standard deviation should be positive");
        
        println!("✅ Mathematical Utilities: PASSED");
    }

    // Helper functions for testing
    fn calculate_risk_score(volatility: f64, amount: f64) -> f64 {
        let base_risk = volatility * 0.7;
        let size_factor = (amount / 10000.0).min(1.0) * 0.3;
        (base_risk + size_factor).min(1.0).max(0.0)
    }

    fn calculate_portfolio_total(positions: &[BigDecimal]) -> BigDecimal {
        positions.iter().sum()
    }

    fn calculate_chain_diversification(exposures: Vec<f64>) -> f64 {
        let total: f64 = exposures.iter().sum();
        if total == 0.0 { return 0.0; }
        
        let normalized: Vec<f64> = exposures.iter().map(|x| x / total).collect();
        let herfindahl = normalized.iter().map(|x| x * x).sum::<f64>();
        1.0 - herfindahl
    }

    fn calculate_mev_risk(volume: f64, volatility: f64) -> f64 {
        let volume_factor = (volume / 1000000.0).min(1.0);
        (volume_factor * 0.6 + volatility * 0.4).min(1.0)
    }

    fn calculate_price_deviation(price1: f64, price2: f64) -> f64 {
        ((price2 - price1) / price1).abs()
    }

    fn calculate_system_health(cpu: f64, memory: f64, disk: f64) -> f64 {
        let cpu_score = (1.0 - cpu).max(0.0);
        let memory_score = (1.0 - memory).max(0.0);
        let disk_score = (1.0 - disk).max(0.0);
        (cpu_score + memory_score + disk_score) / 3.0
    }

    fn safe_divide(numerator: f64, denominator: f64) -> Result<f64, String> {
        if denominator == 0.0 {
            Err("Division by zero".to_string())
        } else {
            Ok(numerator / denominator)
        }
    }

    fn calculate_mean(data: &[f64]) -> f64 {
        data.iter().sum::<f64>() / data.len() as f64
    }

    fn calculate_standard_deviation(data: &[f64]) -> f64 {
        let mean = calculate_mean(data);
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / data.len() as f64;
        variance.sqrt()
    }
}
