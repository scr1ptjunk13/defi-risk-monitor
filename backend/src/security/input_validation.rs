use std::collections::HashSet;
use regex::Regex;
use bigdecimal::{BigDecimal, Zero};
use serde::{Deserialize, Serialize};

/// Comprehensive input validation for DeFi risk monitoring
#[derive(Debug, Clone)]
pub struct InputValidator {
    ethereum_address_regex: Regex,
    max_position_size: BigDecimal,
    max_slippage_percent: f64,
    max_string_length: usize,
    allowed_protocols: HashSet<String>,
    allowed_chain_ids: HashSet<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub sanitized_value: Option<String>,
}

impl Default for InputValidator {
    fn default() -> Self {
        let mut allowed_protocols = HashSet::new();
        allowed_protocols.insert("uniswap_v3".to_string());
        allowed_protocols.insert("uniswap_v2".to_string());
        allowed_protocols.insert("sushiswap".to_string());
        allowed_protocols.insert("curve".to_string());
        allowed_protocols.insert("balancer".to_string());
        allowed_protocols.insert("aave".to_string());


        let mut allowed_chain_ids = HashSet::new();
        allowed_chain_ids.insert(1);    // Ethereum
        allowed_chain_ids.insert(137);  // Polygon
        allowed_chain_ids.insert(42161); // Arbitrum
        allowed_chain_ids.insert(10);   // Optimism
        allowed_chain_ids.insert(56);   // BSC

        Self {
            ethereum_address_regex: Regex::new(r"^0x[a-fA-F0-9]{40}$").unwrap(),
            max_position_size: BigDecimal::from(1_000_000_000u64),
            max_slippage_percent: 50.0,
            max_string_length: 1000,
            allowed_protocols,
            allowed_chain_ids,
        }
    }
}

impl InputValidator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate Ethereum address format
    pub fn validate_address(&self, address: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let warnings = Vec::new();

        if !self.ethereum_address_regex.is_match(address) {
            errors.push("Invalid Ethereum address format".to_string());
        }

        if address == "0x0000000000000000000000000000000000000000" {
            errors.push("Zero address not allowed".to_string());
        }

        let sanitized = address.to_lowercase();

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            sanitized_value: Some(sanitized),
        }
    }

    /// Validate BigDecimal amounts
    pub fn validate_amount(&self, amount: &BigDecimal, field_name: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if amount < &BigDecimal::zero() {
            errors.push(format!("{} cannot be negative", field_name));
        }

        if amount.is_zero() && (field_name.contains("price") || field_name.contains("liquidity")) {
            warnings.push(format!("{} is zero, which may indicate missing data", field_name));
        }

        if amount > &self.max_position_size {
            errors.push(format!("{} exceeds maximum allowed value", field_name));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            sanitized_value: None,
        }
    }

    /// Validate percentage values
    pub fn validate_percentage(&self, percentage: f64, field_name: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if percentage < 0.0 {
            errors.push(format!("{} cannot be negative", field_name));
        }

        if percentage > 100.0 {
            errors.push(format!("{} cannot exceed 100%", field_name));
        }

        if percentage > self.max_slippage_percent && field_name.contains("slippage") {
            errors.push(format!("Slippage {} exceeds maximum allowed {}%", percentage, self.max_slippage_percent));
        }

        if percentage > 50.0 && !field_name.contains("slippage") {
            warnings.push(format!("{} is unusually high ({}%)", field_name, percentage));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            sanitized_value: None,
        }
    }

    /// Validate string inputs for SQL injection and XSS
    pub fn validate_string(&self, input: &str, field_name: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if input.len() > self.max_string_length {
            errors.push(format!("{} exceeds maximum length of {}", field_name, self.max_string_length));
        }

        // Check for dangerous patterns
        let dangerous_patterns = [
            "drop table", "delete from", "insert into", "update set",
            "union select", "script", "javascript", "onload", "onerror"
        ];

        let input_lower = input.to_lowercase();
        for pattern in &dangerous_patterns {
            if input_lower.contains(pattern) {
                errors.push(format!("{} contains potentially dangerous content", field_name));
                break;
            }
        }

        // Sanitize input
        let sanitized = input
            .chars()
            .filter(|c| c.is_alphanumeric() || " ._-@".contains(*c))
            .collect::<String>();

        if sanitized != input {
            warnings.push(format!("{} contains special characters that were removed", field_name));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            sanitized_value: Some(sanitized),
        }
    }

    /// Validate protocol name
    pub fn validate_protocol(&self, protocol: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let protocol_lower = protocol.to_lowercase();

        if !self.allowed_protocols.contains(&protocol_lower) {
            errors.push(format!("Protocol {} is not supported", protocol));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
            sanitized_value: Some(protocol_lower),
        }
    }

    /// Validate chain ID
    pub fn validate_chain_id(&self, chain_id: i32) -> ValidationResult {
        let mut errors = Vec::new();

        if !self.allowed_chain_ids.contains(&chain_id) {
            errors.push(format!("Chain ID {} is not supported", chain_id));
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
            sanitized_value: None,
        }
    }

    /// Comprehensive position validation
    pub fn validate_position_data(
        &self,
        token0_address: &str,
        token1_address: &str,
        pool_address: &str,
        liquidity: &BigDecimal,
        amount0: &BigDecimal,
        amount1: &BigDecimal,
        chain_id: i32,
    ) -> ValidationResult {
        let mut all_errors = Vec::new();
        let mut all_warnings = Vec::new();

        // Validate addresses
        for (addr, _name) in [
            (token0_address, "token0_address"),
            (token1_address, "token1_address"),
            (pool_address, "pool_address"),
        ] {
            let result = self.validate_address(addr);
            all_errors.extend(result.errors);
            all_warnings.extend(result.warnings);
        }

        // Validate amounts
        for (amount, name) in [
            (liquidity, "liquidity"),
            (amount0, "amount0"),
            (amount1, "amount1"),
        ] {
            let result = self.validate_amount(amount, name);
            all_errors.extend(result.errors);
            all_warnings.extend(result.warnings);
        }

        // Validate chain ID
        let chain_result = self.validate_chain_id(chain_id);
        all_errors.extend(chain_result.errors);
        all_warnings.extend(chain_result.warnings);

        // Check for duplicate token addresses
        if token0_address.to_lowercase() == token1_address.to_lowercase() {
            all_errors.push("Token0 and Token1 cannot be the same".to_string());
        }

        ValidationResult {
            is_valid: all_errors.is_empty(),
            errors: all_errors,
            warnings: all_warnings,
            sanitized_value: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_validation() {
        let validator = InputValidator::new();
        
        // Valid address
        let result = validator.validate_address("0x1234567890123456789012345678901234567890");
        assert!(result.is_valid);
        
        // Invalid format
        let result = validator.validate_address("invalid_address");
        assert!(!result.is_valid);
        
        // Zero address
        let result = validator.validate_address("0x0000000000000000000000000000000000000000");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_amount_validation() {
        let validator = InputValidator::new();
        
        // Valid amount
        let amount = BigDecimal::from(1000);
        let result = validator.validate_amount(&amount, "test_amount");
        assert!(result.is_valid);
        
        // Negative amount
        let amount = BigDecimal::from(-100);
        let result = validator.validate_amount(&amount, "test_amount");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_string_validation() {
        let validator = InputValidator::new();
        
        // Valid string
        let result = validator.validate_string("valid_protocol_name", "protocol");
        assert!(result.is_valid);
        
        // Dangerous content
        let result = validator.validate_string("drop table users", "protocol");
        assert!(!result.is_valid);
    }

    #[test]
    fn test_protocol_validation() {
        let validator = InputValidator::new();
        
        // Valid protocol
        let result = validator.validate_protocol("uniswap_v3");
        assert!(result.is_valid);
        
        // Invalid protocol
        let result = validator.validate_protocol("unknown_protocol");
        assert!(!result.is_valid);
    }
}
