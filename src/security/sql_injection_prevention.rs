use sqlx::PgPool;
use std::collections::HashMap;
use regex::Regex;
use crate::error::AppError;

/// SQL injection prevention utilities and safe query patterns
#[derive(Debug, Clone)]
pub struct SqlSafetyChecker {
    dangerous_patterns: Vec<Regex>,
    allowed_table_names: Vec<String>,
    allowed_column_names: Vec<String>,
}

impl Default for SqlSafetyChecker {
    fn default() -> Self {
        let dangerous_patterns = vec![
            Regex::new(r"(?i)(union\s+select)").unwrap(),
            Regex::new(r"(?i)(drop\s+table)").unwrap(),
            Regex::new(r"(?i)(delete\s+from)").unwrap(),
            Regex::new(r"(?i)(insert\s+into)").unwrap(),
            Regex::new(r"(?i)(update\s+\w+\s+set)").unwrap(),
            Regex::new(r"(?i)(exec\s*\()").unwrap(),
            Regex::new(r"(?i)(execute\s*\()").unwrap(),
            Regex::new(r"(?i)(sp_\w+)").unwrap(),
            Regex::new(r"(?i)(xp_\w+)").unwrap(),
            Regex::new(r"[';]--").unwrap(),
            Regex::new(r"/\*.*?\*/").unwrap(),
            Regex::new(r"(?i)(or\s+1\s*=\s*1)").unwrap(),
            Regex::new(r"(?i)(and\s+1\s*=\s*1)").unwrap(),
            Regex::new(r"(?i)(having\s+1\s*=\s*1)").unwrap(),
        ];

        let allowed_table_names = vec![
            "positions".to_string(),
            "pool_states".to_string(),
            "alerts".to_string(),
            "alert_thresholds".to_string(),
            "user_risk_configs".to_string(),
            "protocol_risks".to_string(),
            "mev_risks".to_string(),
            "cross_chain_risks".to_string(),
            "audit_logs".to_string(),
            "users".to_string(),
            "price_history".to_string(),
        ];

        let allowed_column_names = vec![
            "id".to_string(),
            "user_address".to_string(),
            "pool_address".to_string(),
            "token0_address".to_string(),
            "token1_address".to_string(),
            "chain_id".to_string(),
            "protocol".to_string(),
            "liquidity".to_string(),
            "amount0".to_string(),
            "amount1".to_string(),
            "tick_lower".to_string(),
            "tick_upper".to_string(),
            "fee_tier".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
            "entry_timestamp".to_string(),
            "entry_token0_price_usd".to_string(),
            "entry_token1_price_usd".to_string(),
        ];

        Self {
            dangerous_patterns,
            allowed_table_names,
            allowed_column_names,
        }
    }
}

impl SqlSafetyChecker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a string contains SQL injection patterns
    pub fn contains_sql_injection(&self, input: &str) -> bool {
        for pattern in &self.dangerous_patterns {
            if pattern.is_match(input) {
                return true;
            }
        }
        false
    }

    /// Validate table name against whitelist
    pub fn is_table_allowed(&self, table_name: &str) -> bool {
        self.allowed_table_names.contains(&table_name.to_lowercase())
    }

    /// Validate column name against whitelist
    pub fn is_column_allowed(&self, column_name: &str) -> bool {
        self.allowed_column_names.contains(&column_name.to_lowercase())
    }

    /// Sanitize string input for SQL queries
    pub fn sanitize_string(&self, input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || " ._-@".contains(*c))
            .collect()
    }

    /// Validate and sanitize user input for SQL queries
    pub fn validate_sql_input(&self, input: &str, field_name: &str) -> Result<String, AppError> {
        if self.contains_sql_injection(input) {
            return Err(AppError::SecurityError(
                format!("Potential SQL injection detected in field: {}", field_name)
            ));
        }

        Ok(self.sanitize_string(input))
    }
}

/// Safe query builder for dynamic SQL construction
#[derive(Debug)]
pub struct SafeQueryBuilder {
    query: String,
    parameters: Vec<String>,
    safety_checker: SqlSafetyChecker,
}

impl SafeQueryBuilder {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            parameters: Vec::new(),
            safety_checker: SqlSafetyChecker::new(),
        }
    }

    /// Add SELECT clause with column validation
    pub fn select(&mut self, columns: &[&str]) -> Result<&mut Self, AppError> {
        for column in columns {
            if !self.safety_checker.is_column_allowed(column) {
                return Err(AppError::SecurityError(
                    format!("Column {} is not allowed", column)
                ));
            }
        }

        self.query = format!("SELECT {}", columns.join(", "));
        Ok(self)
    }

    /// Add FROM clause with table validation
    pub fn from(&mut self, table: &str) -> Result<&mut Self, AppError> {
        if !self.safety_checker.is_table_allowed(table) {
            return Err(AppError::SecurityError(
                format!("Table {} is not allowed", table)
            ));
        }

        self.query.push_str(&format!(" FROM {}", table));
        Ok(self)
    }

    /// Add WHERE clause with parameter binding
    pub fn where_clause(&mut self, condition: &str, value: &str) -> Result<&mut Self, AppError> {
        let sanitized_value = self.safety_checker.validate_sql_input(value, "where_condition")?;
        
        self.parameters.push(sanitized_value);
        let param_index = self.parameters.len();
        
        if self.query.contains("WHERE") {
            self.query.push_str(&format!(" AND {} = ${}", condition, param_index));
        } else {
            self.query.push_str(&format!(" WHERE {} = ${}", condition, param_index));
        }
        
        Ok(self)
    }

    /// Add ORDER BY clause with column validation
    pub fn order_by(&mut self, column: &str, direction: &str) -> Result<&mut Self, AppError> {
        if !self.safety_checker.is_column_allowed(column) {
            return Err(AppError::SecurityError(
                format!("Column {} is not allowed in ORDER BY", column)
            ));
        }

        let direction = match direction.to_uppercase().as_str() {
            "ASC" | "DESC" => direction.to_uppercase(),
            _ => return Err(AppError::SecurityError(
                "Invalid ORDER BY direction, must be ASC or DESC".to_string()
            )),
        };

        self.query.push_str(&format!(" ORDER BY {} {}", column, direction));
        Ok(self)
    }

    /// Add LIMIT clause with validation
    pub fn limit(&mut self, limit: u32) -> Result<&mut Self, AppError> {
        if limit > 10000 {
            return Err(AppError::SecurityError(
                "LIMIT cannot exceed 10000 rows".to_string()
            ));
        }

        self.query.push_str(&format!(" LIMIT {}", limit));
        Ok(self)
    }

    /// Build the final query
    pub fn build(&self) -> (String, Vec<String>) {
        (self.query.clone(), self.parameters.clone())
    }
}

/// Prepared statement cache for performance and security
#[derive(Debug)]
pub struct PreparedStatementCache {
    cache: HashMap<String, String>,
    max_cache_size: usize,
}

impl PreparedStatementCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_cache_size: max_size,
        }
    }

    /// Get or create a prepared statement
    pub fn get_or_prepare(&mut self, key: &str, query: &str) -> Result<String, AppError> {
        if let Some(cached_query) = self.cache.get(key) {
            return Ok(cached_query.clone());
        }

        // Validate query before caching
        let safety_checker = SqlSafetyChecker::new();
        if safety_checker.contains_sql_injection(query) {
            return Err(AppError::SecurityError(
                "Query contains potential SQL injection patterns".to_string()
            ));
        }

        // Check cache size limit
        if self.cache.len() >= self.max_cache_size {
            // Remove oldest entry (simple LRU approximation)
            if let Some(oldest_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&oldest_key);
            }
        }

        self.cache.insert(key.to_string(), query.to_string());
        Ok(query.to_string())
    }
}

/// Safe database operations with automatic SQL injection prevention
pub struct SafeDbOperations {
    pool: PgPool,
    safety_checker: SqlSafetyChecker,
    statement_cache: PreparedStatementCache,
}

impl SafeDbOperations {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            safety_checker: SqlSafetyChecker::new(),
            statement_cache: PreparedStatementCache::new(100),
        }
    }

    /// Execute a safe SELECT query with parameter binding
    pub async fn safe_select(
        &mut self,
        table: &str,
        columns: &[&str],
        where_conditions: &[(&str, &str)],
        limit: Option<u32>,
    ) -> Result<Vec<sqlx::postgres::PgRow>, AppError> {
        let mut builder = SafeQueryBuilder::new();
        
        builder.select(columns)?
               .from(table)?;

        for (column, value) in where_conditions {
            builder.where_clause(column, value)?;
        }

        if let Some(limit_val) = limit {
            builder.limit(limit_val)?;
        }

        let (query, parameters) = builder.build();
        
        // Cache the prepared statement
        let cache_key = format!("{}_{}", table, columns.join("_"));
        let prepared_query = self.statement_cache.get_or_prepare(&cache_key, &query)?;

        // Execute with parameter binding
        let mut query_builder = sqlx::query(&prepared_query);
        for param in parameters {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Safe query execution failed: {}", e)))?;

        Ok(rows)
    }

    /// Validate user input before database operations
    pub fn validate_user_input(&self, inputs: &[(&str, &str)]) -> Result<Vec<String>, AppError> {
        let mut sanitized_inputs = Vec::new();
        
        for (field_name, value) in inputs {
            let sanitized = self.safety_checker.validate_sql_input(value, field_name)?;
            sanitized_inputs.push(sanitized);
        }
        
        Ok(sanitized_inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_injection_detection() {
        let checker = SqlSafetyChecker::new();
        
        // Test dangerous patterns
        assert!(checker.contains_sql_injection("DROP TABLE users"));
        assert!(checker.contains_sql_injection("1 OR 1=1"));
        assert!(checker.contains_sql_injection("UNION SELECT * FROM passwords"));
        assert!(checker.contains_sql_injection("exec sp_configure"));
        
        // Test safe inputs
        assert!(!checker.contains_sql_injection("normal_user_input"));
        assert!(!checker.contains_sql_injection("0x1234567890abcdef"));
        assert!(!checker.contains_sql_injection("user@example.com"));
    }

    #[test]
    fn test_table_whitelist() {
        let checker = SqlSafetyChecker::new();
        
        // Allowed tables
        assert!(checker.is_table_allowed("positions"));
        assert!(checker.is_table_allowed("alerts"));
        assert!(checker.is_table_allowed("users"));
        
        // Disallowed tables
        assert!(!checker.is_table_allowed("information_schema"));
        assert!(!checker.is_table_allowed("pg_tables"));
        assert!(!checker.is_table_allowed("unknown_table"));
    }

    #[test]
    fn test_safe_query_builder() {
        let mut builder = SafeQueryBuilder::new();
        
        let result = builder
            .select(&["id", "user_address"])
            .and_then(|b| b.from("positions"))
            .and_then(|b| b.where_clause("chain_id", "1"))
            .and_then(|b| b.order_by("created_at", "DESC"))
            .and_then(|b| b.limit(100));
        
        assert!(result.is_ok());
        
        let (query, params) = builder.build();
        assert!(query.contains("SELECT id, user_address"));
        assert!(query.contains("FROM positions"));
        assert!(query.contains("WHERE chain_id = $1"));
        assert!(query.contains("ORDER BY created_at DESC"));
        assert!(query.contains("LIMIT 100"));
        assert_eq!(params.len(), 1);
        assert_eq!(params[0], "1");
    }

    #[test]
    fn test_query_builder_security() {
        let mut builder = SafeQueryBuilder::new();
        
        // Test invalid table
        let result = builder.select(&["id"]).and_then(|b| b.from("malicious_table"));
        assert!(result.is_err());
        
        // Test invalid column
        let mut builder = SafeQueryBuilder::new();
        let result = builder.select(&["malicious_column"]);
        assert!(result.is_err());
        
        // Test invalid ORDER BY direction
        let mut builder = SafeQueryBuilder::new();
        let result = builder
            .select(&["id"])
            .and_then(|b| b.from("positions"))
            .and_then(|b| b.order_by("id", "INVALID"));
        assert!(result.is_err());
        
        // Test excessive LIMIT
        let mut builder = SafeQueryBuilder::new();
        let result = builder
            .select(&["id"])
            .and_then(|b| b.from("positions"))
            .and_then(|b| b.limit(20000));
        assert!(result.is_err());
    }

    #[test]
    fn test_string_sanitization() {
        let checker = SqlSafetyChecker::new();
        
        let malicious_input = "user DROP TABLE users";
        let sanitized = checker.sanitize_string(malicious_input);
        
        // Should preserve alphanumeric and safe characters
        let safe_input = "user_123@example.com";
        let sanitized = checker.sanitize_string(safe_input);
        assert!(sanitized.contains("user"));
        assert!(sanitized.contains("123"));
        assert!(sanitized.contains("@"));
        assert!(sanitized.contains("."));
    }

    #[test]
    fn test_prepared_statement_cache() {
        let mut cache = PreparedStatementCache::new(2);
        
        // Add first query
        let result = cache.get_or_prepare("query1", "SELECT * FROM positions");
        assert!(result.is_ok());
        
        // Add second query
        let result = cache.get_or_prepare("query2", "SELECT * FROM alerts");
        assert!(result.is_ok());
        
        // Add third query (should evict first due to size limit)
        let result = cache.get_or_prepare("query3", "SELECT * FROM users");
        assert!(result.is_ok());
        
        // Cache should have 2 items
        assert_eq!(cache.cache.len(), 2);
        
        // Test malicious query rejection
        let result = cache.get_or_prepare("malicious", "DROP TABLE users");
        assert!(result.is_err());
    }
}
