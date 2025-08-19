use crate::risk::traits::RiskExplanation;
// Removed unused BigDecimal import
// Removed unused serde imports
use std::collections::HashMap;

/// Yearn Finance specific risk calculator
#[derive(Debug, Clone)]
pub struct YearnFinanceRiskCalculator;

/// Yearn position data for risk calculation
#[derive(Debug, Clone)]
pub struct YearnRiskData {
    pub vault_version: String,
    pub vault_type: String,
    pub category: String,
    pub net_apy: f64,
    pub gross_apr: f64,
    pub strategy_count: usize,
    pub strategy_types: Vec<String>,
    pub underlying_protocols: Vec<String>,
    pub performance_fee: f64,
    pub management_fee: f64,
    pub withdrawal_fee: f64,
    pub chain_id: u64,
    pub tvl_usd: f64,
    pub is_migrable: bool,
    pub harvest_frequency_days: u32,
    pub withdrawal_liquidity_usd: f64,
    pub is_v3: bool,
}

impl YearnFinanceRiskCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate comprehensive risk score for Yearn positions
    pub fn calculate_risk_score(&self, data: &YearnRiskData) -> (f64, f64, RiskExplanation) {
        let mut risk_factors = HashMap::new();
        let _explanations: Vec<String> = Vec::new();
        
        // 1. Smart Contract Risk (15-25 points)
        let smart_contract_risk = self.calculate_smart_contract_risk(data);
        risk_factors.insert("smart_contract_risk".to_string(), smart_contract_risk);
        
        // 2. Liquidity Risk (5-15 points)
        let liquidity_risk = self.calculate_liquidity_risk(data);
        risk_factors.insert("liquidity_risk".to_string(), liquidity_risk);
        
        // 3. Protocol Governance Risk (8-15 points)
        let governance_risk = self.calculate_governance_risk(data);
        risk_factors.insert("protocol_governance_risk".to_string(), governance_risk);
        
        // 4. Yield Strategy Risk (10-20 points)
        let strategy_risk = self.calculate_yield_strategy_risk(data);
        risk_factors.insert("yield_strategy_risk".to_string(), strategy_risk);
        
        // 5. External Protocol Dependency Risk (8-18 points)
        let dependency_risk = self.calculate_external_protocol_risk(data);
        risk_factors.insert("external_protocol_dependency_risk".to_string(), dependency_risk);
        
        // 6. Multi-Strategy Dependency Risk (V3 only, 10-20 points)
        if data.is_v3 {
            let multi_strategy_risk = self.calculate_multi_strategy_risk(data);
            risk_factors.insert("multi_strategy_dependency_risk".to_string(), multi_strategy_risk);
        }
        
        // Calculate weighted average
        let total_risk = self.calculate_weighted_risk(&risk_factors, data.is_v3);
        let confidence = self.calculate_confidence_score(data);
        
        // Generate explanations
        let explanation = self.generate_risk_explanation(data, &risk_factors, total_risk);
        
        (total_risk, confidence, explanation)
    }
    
    fn calculate_smart_contract_risk(&self, data: &YearnRiskData) -> f64 {
        let mut risk: f64 = 15.0; // Base smart contract risk
        
        // Version risk adjustment
        match data.vault_version.as_str() {
            v if v.starts_with("0.4") => risk -= 3.0, // Latest V3 versions
            v if v.starts_with("0.3") => risk -= 2.0, // V3 versions
            v if v.starts_with("0.2") => risk += 0.0, // V2 stable
            _ => risk += 8.0, // Older or unknown versions
        }
        
        // Vault type risk
        match data.vault_type.to_lowercase().as_str() {
            "automated" => risk -= 2.0,
            "experimental" => risk += 10.0,
            _ => {}
        }
        
        // V3 has higher complexity
        if data.is_v3 {
            risk += 2.0;
        }
        
        risk.max(5.0).min(25.0)
    }
    
    fn calculate_liquidity_risk(&self, data: &YearnRiskData) -> f64 {
        let mut risk: f64 = 8.0; // Base liquidity risk
        
        // TVL-based liquidity assessment
        if data.tvl_usd > 100_000_000.0 {
            risk -= 3.0; // Very high TVL
        } else if data.tvl_usd > 10_000_000.0 {
            risk -= 1.0; // High TVL
        } else if data.tvl_usd < 1_000_000.0 {
            risk += 5.0; // Low TVL
        }
        
        // Withdrawal liquidity
        let liquidity_ratio = data.withdrawal_liquidity_usd / data.tvl_usd.max(1.0);
        if liquidity_ratio > 0.5 {
            risk -= 2.0; // High withdrawal liquidity
        } else if liquidity_ratio < 0.1 {
            risk += 3.0; // Low withdrawal liquidity
        }
        
        // Category-based liquidity
        match data.category.to_lowercase().as_str() {
            "stablecoin" => risk -= 3.0,
            "volatile" => risk += 2.0,
            "curve" => risk -= 1.0,
            _ => {}
        }
        
        risk.max(3.0).min(15.0)
    }
    
    fn calculate_governance_risk(&self, data: &YearnRiskData) -> f64 {
        let mut risk: f64 = 12.0; // Base governance risk
        
        // Chain-based governance risk
        match data.chain_id {
            1 => risk -= 2.0,    // Ethereum mainnet
            250 => risk += 2.0,  // Fantom
            42161 => risk += 1.0, // Arbitrum
            10 => risk += 1.0,   // Optimism
            137 => risk += 1.0,  // Polygon
            _ => risk += 4.0,    // Unknown chains
        }
        
        // Migration risk
        if data.is_migrable {
            risk += 3.0; // Deprecated vaults
        }
        
        risk.max(8.0).min(15.0)
    }
    
    fn calculate_yield_strategy_risk(&self, data: &YearnRiskData) -> f64 {
        let mut risk: f64 = 14.0; // Base strategy risk
        
        // APY risk assessment
        if data.net_apy > 100.0 {
            risk += 6.0; // Extremely high APY
        } else if data.net_apy > 50.0 {
            risk += 4.0; // Very high APY
        } else if data.net_apy > 25.0 {
            risk += 2.0; // High APY
        } else if data.net_apy < 2.0 {
            risk += 3.0; // Suspiciously low APY
        }
        
        // Strategy diversification
        if data.strategy_count > 3 {
            risk -= 2.0; // Well diversified
        } else if data.strategy_count > 1 {
            risk -= 1.0; // Some diversification
        } else if data.strategy_count == 0 {
            risk += 4.0; // No strategy info
        }
        
        // Fee structure risk
        if data.performance_fee > 30.0 {
            risk += 3.0;
        } else if data.performance_fee > 20.0 {
            risk += 1.0;
        }
        
        if data.management_fee > 5.0 {
            risk += 2.0;
        }
        
        // Harvest frequency (more frequent = lower risk)
        if data.harvest_frequency_days > 7 {
            risk += 1.0;
        } else if data.harvest_frequency_days <= 1 {
            risk -= 1.0;
        }
        
        risk.max(8.0).min(20.0)
    }
    
    fn calculate_external_protocol_risk(&self, data: &YearnRiskData) -> f64 {
        let mut risk: f64 = 10.0; // Base external protocol risk
        
        // Number of external protocols
        let protocol_count = data.underlying_protocols.len();
        if protocol_count > 3 {
            risk += 4.0; // Many dependencies
        } else if protocol_count > 1 {
            risk += 2.0; // Some dependencies
        } else if protocol_count == 0 {
            risk -= 2.0; // Self-contained
        }
        
        // Known protocol risk assessment
        for protocol in &data.underlying_protocols {
            match protocol.to_lowercase().as_str() {
                "curve" => risk -= 1.0,    // Well-established
                "aave" => risk -= 1.0,     // Well-established
                "compound" => risk -= 1.0, // Well-established
                "balancer" => risk += 0.0, // Moderate risk
                "convex" => risk += 1.0,   // Additional complexity
                _ => risk += 2.0,          // Unknown protocols
            }
        }
        
        risk.max(5.0).min(18.0)
    }
    
    fn calculate_multi_strategy_risk(&self, data: &YearnRiskData) -> f64 {
        if !data.is_v3 {
            return 0.0;
        }
        
        let mut risk: f64 = 15.0; // Base multi-strategy risk for V3
        
        // Strategy complexity
        let strategy_types = &data.strategy_types;
        if strategy_types.len() > 4 {
            risk += 3.0; // Very complex
        } else if strategy_types.len() > 2 {
            risk += 1.0; // Moderately complex
        }
        
        // Strategy type risk
        for strategy_type in strategy_types {
            match strategy_type.to_lowercase().as_str() {
                "leverage" => risk += 2.0,
                "curve lp" => risk -= 1.0,
                "aave lending" => risk -= 1.0,
                "stable swap" => risk -= 1.0,
                _ => {}
            }
        }
        
        risk.max(10.0).min(20.0)
    }
    
    fn calculate_weighted_risk(&self, risk_factors: &HashMap<String, f64>, is_v3: bool) -> f64 {
        let weights = if is_v3 {
            // V3 weights (6 factors)
            HashMap::from([
                ("smart_contract_risk".to_string(), 0.20),
                ("liquidity_risk".to_string(), 0.15),
                ("protocol_governance_risk".to_string(), 0.15),
                ("yield_strategy_risk".to_string(), 0.20),
                ("external_protocol_dependency_risk".to_string(), 0.15),
                ("multi_strategy_dependency_risk".to_string(), 0.15),
            ])
        } else {
            // V2 weights (5 factors)
            HashMap::from([
                ("smart_contract_risk".to_string(), 0.25),
                ("liquidity_risk".to_string(), 0.20),
                ("protocol_governance_risk".to_string(), 0.20),
                ("yield_strategy_risk".to_string(), 0.25),
                ("external_protocol_dependency_risk".to_string(), 0.10),
            ])
        };
        
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        
        for (factor, &weight) in &weights {
            if let Some(&risk_value) = risk_factors.get(factor) {
                weighted_sum += risk_value * weight;
                total_weight += weight;
            }
        }
        
        if total_weight > 0.0 {
            (weighted_sum / total_weight).min(95.0)
        } else {
            50.0 // Fallback
        }
    }
    
    fn calculate_confidence_score(&self, data: &YearnRiskData) -> f64 {
        let mut confidence: f64 = 0.85; // Base confidence
        
        // Data completeness
        if data.strategy_count > 0 {
            confidence += 0.05;
        }
        
        if !data.underlying_protocols.is_empty() {
            confidence += 0.05;
        }
        
        if data.tvl_usd > 1_000_000.0 {
            confidence += 0.03; // More data available for larger vaults
        }
        
        // Version maturity
        if data.vault_version.starts_with("0.2") || data.vault_version.starts_with("0.3") {
            confidence += 0.02; // Mature versions
        }
        
        confidence.min(0.95)
    }
    
    fn generate_risk_explanation(&self, data: &YearnRiskData, risk_factors: &HashMap<String, f64>, total_risk: f64) -> RiskExplanation {
        let _risk_level = match total_risk {
            r if r < 10.0 => "low",
            r if r < 20.0 => "medium",
            r if r < 30.0 => "high",
            _ => "very_high",
        };
        
        let mut explanation = if data.is_v3 {
            format!(
                "Yearn V3 vault risk assessment considers vault complexity, strategy diversification, and protocol dependencies. "
            )
        } else {
            format!(
                "Yearn V2 vault risk assessment considers vault maturity, strategy composition, and underlying protocol risks. "
            )
        };
        
        // Add specific risk insights
        if let Some(&smart_contract_risk) = risk_factors.get("smart_contract_risk") {
            if smart_contract_risk > 18.0 {
                explanation.push_str("High smart contract risk due to experimental features or older versions. ");
            } else if smart_contract_risk < 12.0 {
                explanation.push_str("Low smart contract risk with mature, well-tested vault implementation. ");
            }
        }
        
        if let Some(&liquidity_risk) = risk_factors.get("liquidity_risk") {
            if liquidity_risk > 12.0 {
                explanation.push_str("Liquidity concerns due to low TVL or limited withdrawal capacity. ");
            } else if liquidity_risk < 8.0 {
                explanation.push_str("Strong liquidity profile with high TVL and withdrawal capacity. ");
            }
        }
        
        if data.net_apy > 25.0 {
            explanation.push_str("High APY indicates elevated risk but potential for strong returns. ");
        }
        
        if data.strategy_count > 3 {
            explanation.push_str("Well-diversified strategy portfolio reduces single-point-of-failure risk. ");
        }
        
        if data.is_v3 && data.underlying_protocols.len() > 2 {
            explanation.push_str("V3 multi-protocol integration increases complexity but enables advanced yield optimization. ");
        }
        
        let confidence_score = self.calculate_confidence_score(data);
        
        RiskExplanation {
            overall_risk_score: total_risk,
            risk_level: if total_risk < 30.0 { "Low".to_string() } 
                       else if total_risk < 60.0 { "Medium".to_string() } 
                       else if total_risk < 80.0 { "High".to_string() } 
                       else { "Critical".to_string() },
            primary_risk_factors: self.identify_key_risks(risk_factors),
            explanation,
            methodology: "Yearn Finance risk assessment based on vault version, strategy complexity, TVL, governance, and external protocol dependencies".to_string(),
            confidence_score,
            data_quality: if confidence_score > 0.8 { "High".to_string() } 
                         else if confidence_score > 0.6 { "Medium".to_string() } 
                         else { "Low".to_string() }
        }
    }
    
    fn identify_key_risks(&self, risk_factors: &HashMap<String, f64>) -> Vec<String> {
        let mut risks = Vec::new();
        
        for (factor, &value) in risk_factors {
            if value > 15.0 {
                let risk_desc = match factor.as_str() {
                    "smart_contract_risk" => "Smart contract complexity and potential vulnerabilities",
                    "liquidity_risk" => "Limited liquidity or withdrawal capacity constraints",
                    "protocol_governance_risk" => "Governance decisions and protocol upgrade risks",
                    "yield_strategy_risk" => "Strategy performance and sustainability concerns",
                    "external_protocol_dependency_risk" => "Dependency on external protocol stability",
                    "multi_strategy_dependency_risk" => "Complex multi-strategy coordination risks",
                    _ => "Unknown risk factor",
                };
                risks.push(risk_desc.to_string());
            }
        }
        
        if risks.is_empty() {
            risks.push("Standard DeFi yield farming risks apply".to_string());
        }
        
        risks
    }
    
    #[allow(dead_code)]
    fn generate_recommendations(&self, data: &YearnRiskData, risk_factors: &HashMap<String, f64>) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if let Some(&smart_contract_risk) = risk_factors.get("smart_contract_risk") {
            if smart_contract_risk > 18.0 {
                recommendations.push("Consider waiting for vault maturity or audit completion".to_string());
            }
        }
        
        if let Some(&liquidity_risk) = risk_factors.get("liquidity_risk") {
            if liquidity_risk > 12.0 {
                recommendations.push("Monitor withdrawal liquidity and consider position sizing".to_string());
            }
        }
        
        if data.net_apy > 50.0 {
            recommendations.push("Extremely high APY warrants careful due diligence".to_string());
        }
        
        if data.is_migrable {
            recommendations.push("Consider migrating to newer vault version when available".to_string());
        }
        
        if data.is_v3 && data.strategy_count > 4 {
            recommendations.push("Monitor strategy performance and rebalancing frequency".to_string());
        }
        
        recommendations.push("Diversify across multiple vaults and protocols".to_string());
        recommendations.push("Monitor harvest frequency and gas optimization".to_string());
        
        recommendations
    }
}

impl Default for YearnFinanceRiskCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_v2_data() -> YearnRiskData {
        YearnRiskData {
            vault_version: "0.2.15".to_string(),
            vault_type: "automated".to_string(),
            category: "stablecoin".to_string(),
            net_apy: 7.25,
            gross_apr: 8.0,
            strategy_count: 3,
            strategy_types: vec!["Curve LP".to_string(), "Convex Boost".to_string(), "Stable Swap".to_string()],
            underlying_protocols: vec!["Curve".to_string()],
            performance_fee: 20.0,
            management_fee: 2.0,
            withdrawal_fee: 0.0,
            chain_id: 1,
            tvl_usd: 175_000_000.0,
            is_migrable: false,
            harvest_frequency_days: 2,
            withdrawal_liquidity_usd: 12_500_000.0,
            is_v3: false,
        }
    }

    fn create_test_v3_data() -> YearnRiskData {
        YearnRiskData {
            vault_version: "0.4.2".to_string(),
            vault_type: "automated".to_string(),
            category: "volatile".to_string(),
            net_apy: 8.4,
            gross_apr: 9.2,
            strategy_count: 5,
            strategy_types: vec![
                "Curve LP".to_string(),
                "Aave Lending".to_string(),
                "Balancer Boost".to_string(),
                "Stable Swap".to_string(),
                "Leverage".to_string(),
            ],
            underlying_protocols: vec!["Curve".to_string(), "Aave".to_string(), "Balancer".to_string()],
            performance_fee: 20.0,
            management_fee: 2.0,
            withdrawal_fee: 0.0,
            chain_id: 1,
            tvl_usd: 230_000_000.0,
            is_migrable: false,
            harvest_frequency_days: 1,
            withdrawal_liquidity_usd: 25_000_000.0,
            is_v3: true,
        }
    }

    #[test]
    fn test_v2_risk_calculation() {
        let calculator = YearnFinanceRiskCalculator::new();
        let data = create_test_v2_data();
        
        let (risk_score, confidence, explanation) = calculator.calculate_risk_score(&data);
        
        assert!(risk_score >= 5.0 && risk_score <= 25.0, "V2 risk score should be reasonable: {}", risk_score);
        assert!(confidence >= 0.8 && confidence <= 1.0, "Confidence should be high: {}", confidence);
        assert!(!explanation.summary.is_empty(), "Should have risk explanation");
        assert!(!explanation.key_risks.is_empty(), "Should identify key risks");
    }

    #[test]
    fn test_v3_risk_calculation() {
        let calculator = YearnFinanceRiskCalculator::new();
        let data = create_test_v3_data();
        
        let (risk_score, confidence, explanation) = calculator.calculate_risk_score(&data);
        
        assert!(risk_score >= 10.0 && risk_score <= 30.0, "V3 risk score should be higher: {}", risk_score);
        assert!(confidence >= 0.8 && confidence <= 1.0, "Confidence should be high: {}", confidence);
        assert!(explanation.summary.contains("V3"), "Should mention V3 in explanation");
        assert!(!explanation.recommendations.is_empty(), "Should provide recommendations");
    }

    #[test]
    fn test_high_risk_experimental_vault() {
        let calculator = YearnFinanceRiskCalculator::new();
        let mut data = create_test_v2_data();
        data.vault_type = "experimental".to_string();
        data.net_apy = 150.0; // Extremely high APY
        data.tvl_usd = 500_000.0; // Low TVL
        data.is_migrable = true;
        
        let (risk_score, _confidence, explanation) = calculator.calculate_risk_score(&data);
        
        assert!(risk_score > 30.0, "Experimental vault should have high risk: {}", risk_score);
        assert!(explanation.key_risks.len() > 2, "Should identify multiple risks");
    }

    #[test]
    fn test_multi_strategy_risk_v3_only() {
        let calculator = YearnFinanceRiskCalculator::new();
        let v2_data = create_test_v2_data();
        let v3_data = create_test_v3_data();
        
        let (v2_risk, _, _) = calculator.calculate_risk_score(&v2_data);
        let (v3_risk, _, _) = calculator.calculate_risk_score(&v3_data);
        
        // V3 should generally have higher risk due to complexity
        assert!(v3_risk >= v2_risk, "V3 should have higher or equal risk due to complexity");
    }

    #[test]
    fn test_risk_factor_bounds() {
        let calculator = YearnFinanceRiskCalculator::new();
        let data = create_test_v2_data();
        
        let smart_contract_risk = calculator.calculate_smart_contract_risk(&data);
        let liquidity_risk = calculator.calculate_liquidity_risk(&data);
        let governance_risk = calculator.calculate_governance_risk(&data);
        let strategy_risk = calculator.calculate_yield_strategy_risk(&data);
        let dependency_risk = calculator.calculate_external_protocol_risk(&data);
        
        assert!(smart_contract_risk >= 5.0 && smart_contract_risk <= 25.0);
        assert!(liquidity_risk >= 3.0 && liquidity_risk <= 15.0);
        assert!(governance_risk >= 8.0 && governance_risk <= 15.0);
        assert!(strategy_risk >= 8.0 && strategy_risk <= 20.0);
        assert!(dependency_risk >= 5.0 && dependency_risk <= 18.0);
    }
}
