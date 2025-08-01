use std::collections::HashMap;
use tracing::{warn, error, info, debug};
use crate::error::{AppError, classification::{classify_error, ErrorCategory}};
use serde::{Serialize, Deserialize};

/// Types of constraint violations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintViolationType {
    UniqueConstraint,
    ForeignKeyConstraint,
    CheckConstraint,
    NotNullConstraint,
    PrimaryKeyConstraint,
    ExclusionConstraint,
    Unknown,
}

/// Detailed constraint violation information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolationInfo {
    pub violation_type: ConstraintViolationType,
    pub constraint_name: Option<String>,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
    pub conflicting_value: Option<String>,
    pub suggested_resolution: String,
    pub is_recoverable: bool,
    pub recovery_strategy: Option<String>,
}

/// Enhanced constraint violation handler
pub struct ConstraintViolationHandler {
    /// Known constraint patterns and their resolutions
    constraint_patterns: HashMap<String, ConstraintViolationInfo>,
}

impl Default for ConstraintViolationHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintViolationHandler {
    /// Create a new constraint violation handler
    pub fn new() -> Self {
        let mut handler = Self {
            constraint_patterns: HashMap::new(),
        };
        
        handler.initialize_patterns();
        handler
    }
    
    /// Initialize known constraint violation patterns
    fn initialize_patterns(&mut self) {
        // Unique constraint violations
        let unique_patterns = vec![
            ("unique constraint", "duplicate key value"),
            ("duplicate key", "violates unique constraint"),
            ("already exists", "unique"),
        ];
        
        for (pattern, _) in unique_patterns {
            self.constraint_patterns.insert(pattern.to_string(), ConstraintViolationInfo {
                violation_type: ConstraintViolationType::UniqueConstraint,
                constraint_name: None,
                table_name: None,
                column_name: None,
                conflicting_value: None,
                suggested_resolution: "Check for existing records with the same unique key value. Consider updating instead of inserting, or use UPSERT operations.".to_string(),
                is_recoverable: true,
                recovery_strategy: Some("Use INSERT ... ON CONFLICT DO UPDATE or check existence before insert".to_string()),
            });
        }
        
        // Foreign key constraint violations
        let fk_patterns = vec![
            ("foreign key constraint", "violates foreign key"),
            ("fk_", "foreign key"),
            ("references", "does not exist"),
        ];
        
        for (pattern, _) in fk_patterns {
            self.constraint_patterns.insert(pattern.to_string(), ConstraintViolationInfo {
                violation_type: ConstraintViolationType::ForeignKeyConstraint,
                constraint_name: None,
                table_name: None,
                column_name: None,
                conflicting_value: None,
                suggested_resolution: "Ensure the referenced record exists in the parent table before creating the foreign key relationship.".to_string(),
                is_recoverable: true,
                recovery_strategy: Some("Create parent record first or use deferred constraint checking".to_string()),
            });
        }
        
        // Check constraint violations
        let check_patterns = vec![
            ("check constraint", "violates check"),
            ("ck_", "check"),
            ("constraint", "check"),
        ];
        
        for (pattern, _) in check_patterns {
            self.constraint_patterns.insert(pattern.to_string(), ConstraintViolationInfo {
                violation_type: ConstraintViolationType::CheckConstraint,
                constraint_name: None,
                table_name: None,
                column_name: None,
                conflicting_value: None,
                suggested_resolution: "Verify that the data meets the check constraint conditions (e.g., value ranges, format requirements).".to_string(),
                is_recoverable: true,
                recovery_strategy: Some("Validate and transform data before insertion".to_string()),
            });
        }
        
        // Not null constraint violations
        let not_null_patterns = vec![
            ("not null constraint", "null value"),
            ("not-null constraint", "violates not-null"),
            ("column", "cannot be null"),
        ];
        
        for (pattern, _) in not_null_patterns {
            self.constraint_patterns.insert(pattern.to_string(), ConstraintViolationInfo {
                violation_type: ConstraintViolationType::NotNullConstraint,
                constraint_name: None,
                table_name: None,
                column_name: None,
                conflicting_value: None,
                suggested_resolution: "Provide a non-null value for the required column or set a default value.".to_string(),
                is_recoverable: true,
                recovery_strategy: Some("Set default values or validate input data completeness".to_string()),
            });
        }
        
        // Primary key constraint violations
        let pk_patterns = vec![
            ("primary key constraint", "duplicate"),
            ("pk_", "primary key"),
            ("primary key", "already exists"),
        ];
        
        for (pattern, _) in pk_patterns {
            self.constraint_patterns.insert(pattern.to_string(), ConstraintViolationInfo {
                violation_type: ConstraintViolationType::PrimaryKeyConstraint,
                constraint_name: None,
                table_name: None,
                column_name: None,
                conflicting_value: None,
                suggested_resolution: "Use a unique primary key value. Consider using auto-generated keys or check for existing records.".to_string(),
                is_recoverable: true,
                recovery_strategy: Some("Use auto-increment keys or generate unique identifiers".to_string()),
            });
        }
    }
    
    /// Analyze a constraint violation error and provide detailed information
    pub fn analyze_constraint_violation(&self, error: &AppError) -> Option<ConstraintViolationInfo> {
        let error_msg = match error {
            AppError::DatabaseError(msg) => msg,
            _ => return None,
        };
        
        // First, check if this is actually a constraint violation
        let classification = classify_error(error);
        if classification.category != ErrorCategory::ConstraintViolation {
            return None;
        }
        
        let error_lower = error_msg.to_lowercase();
        
        // Try to match against known patterns in order of specificity (longer patterns first)
        let mut patterns: Vec<_> = self.constraint_patterns.iter().collect();
        patterns.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // Sort by pattern length descending
        
        for (pattern, base_info) in patterns {
            if error_lower.contains(pattern) {
                let mut info = base_info.clone();
                
                // Try to extract additional details from the error message
                self.extract_constraint_details(&error_lower, &mut info);
                
                debug!("Constraint violation analyzed: {:?}", info);
                return Some(info);
            }
        }
        
        // If no specific pattern matched, return generic constraint violation info
        Some(ConstraintViolationInfo {
            violation_type: ConstraintViolationType::Unknown,
            constraint_name: None,
            table_name: None,
            column_name: None,
            conflicting_value: None,
            suggested_resolution: "Review the database constraint that was violated and adjust the data accordingly.".to_string(),
            is_recoverable: false,
            recovery_strategy: None,
        })
    }
    
    /// Extract detailed information from constraint violation error message
    fn extract_constraint_details(&self, error_msg: &str, info: &mut ConstraintViolationInfo) {
        // Try to extract constraint name
        if let Some(constraint_name) = self.extract_constraint_name(error_msg) {
            info.constraint_name = Some(constraint_name);
        }
        
        // Try to extract table name
        if let Some(table_name) = self.extract_table_name(error_msg) {
            info.table_name = Some(table_name);
        }
        
        // Try to extract column name
        if let Some(column_name) = self.extract_column_name(error_msg) {
            info.column_name = Some(column_name);
        }
        
        // Try to extract conflicting value
        if let Some(value) = self.extract_conflicting_value(error_msg) {
            info.conflicting_value = Some(value);
        }
    }
    
    /// Extract constraint name from error message
    fn extract_constraint_name(&self, error_msg: &str) -> Option<String> {
        // Common patterns for constraint names in PostgreSQL error messages
        let patterns = vec![
            r#"constraint "([^"]+)""#,
            r#"constraint '([^']+)'"#,
            r#"constraint ([a-zA-Z_][a-zA-Z0-9_]*)"#,
        ];
        
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(error_msg) {
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str().to_string());
                    }
                }
            }
        }
        
        None
    }
    
    /// Extract table name from error message
    fn extract_table_name(&self, error_msg: &str) -> Option<String> {
        let patterns = vec![
            r#"table "([^"]+)""#,
            r#"table '([^']+)'"#,
            r#"on table ([a-zA-Z_][a-zA-Z0-9_]*)"#,
            r#"in table ([a-zA-Z_][a-zA-Z0-9_]*)"#,
        ];
        
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(error_msg) {
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str().to_string());
                    }
                }
            }
        }
        
        None
    }
    
    /// Extract column name from error message
    fn extract_column_name(&self, error_msg: &str) -> Option<String> {
        let patterns = vec![
            r#"column "([^"]+)""#,
            r#"column '([^']+)'"#,
            r#"column ([a-zA-Z_][a-zA-Z0-9_]*)"#,
        ];
        
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(error_msg) {
                    if let Some(name) = captures.get(1) {
                        return Some(name.as_str().to_string());
                    }
                }
            }
        }
        
        None
    }
    
    /// Extract conflicting value from error message
    fn extract_conflicting_value(&self, error_msg: &str) -> Option<String> {
        let patterns = vec![
            r#"value "([^"]+)""#,
            r#"value '([^']+)'"#,
            r#"key \(([^)]+)\)"#,
            r#"=\(([^)]+)\)"#,
        ];
        
        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(error_msg) {
                    if let Some(value) = captures.get(1) {
                        return Some(value.as_str().to_string());
                    }
                }
            }
        }
        
        None
    }
    
    /// Generate a user-friendly error message for constraint violations
    pub fn generate_user_friendly_message(&self, info: &ConstraintViolationInfo) -> String {
        match info.violation_type {
            ConstraintViolationType::UniqueConstraint => {
                format!(
                    "A record with this value already exists{}. {}",
                    if let Some(column) = &info.column_name {
                        format!(" in column '{}'", column)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::ForeignKeyConstraint => {
                format!(
                    "The referenced record does not exist{}. {}",
                    if let Some(table) = &info.table_name {
                        format!(" in table '{}'", table)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::NotNullConstraint => {
                format!(
                    "A required field is missing{}. {}",
                    if let Some(column) = &info.column_name {
                        format!(" (column '{}')", column)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::CheckConstraint => {
                format!(
                    "The data does not meet the required conditions{}. {}",
                    if let Some(constraint) = &info.constraint_name {
                        format!(" (constraint '{}')", constraint)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::PrimaryKeyConstraint => {
                format!(
                    "A record with this primary key already exists{}. {}",
                    if let Some(value) = &info.conflicting_value {
                        format!(" (value: {})", value)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::ExclusionConstraint => {
                format!(
                    "The data conflicts with an exclusion constraint{}. {}",
                    if let Some(constraint) = &info.constraint_name {
                        format!(" (constraint '{}')", constraint)
                    } else {
                        String::new()
                    },
                    info.suggested_resolution
                )
            },
            ConstraintViolationType::Unknown => {
                format!("A database constraint was violated. {}", info.suggested_resolution)
            },
        }
    }
    
    /// Check if a constraint violation is recoverable
    pub fn is_recoverable(&self, error: &AppError) -> bool {
        if let Some(info) = self.analyze_constraint_violation(error) {
            info.is_recoverable
        } else {
            false
        }
    }
    
    /// Get recovery strategy for a constraint violation
    pub fn get_recovery_strategy(&self, error: &AppError) -> Option<String> {
        if let Some(info) = self.analyze_constraint_violation(error) {
            info.recovery_strategy
        } else {
            None
        }
    }
}

/// Global constraint violation handler instance
static mut CONSTRAINT_HANDLER: Option<ConstraintViolationHandler> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Get the global constraint violation handler instance
pub fn get_constraint_handler() -> &'static ConstraintViolationHandler {
    unsafe {
        INIT.call_once(|| {
            CONSTRAINT_HANDLER = Some(ConstraintViolationHandler::new());
        });
        CONSTRAINT_HANDLER.as_ref().unwrap()
    }
}

/// Analyze constraint violation and return detailed information
pub fn analyze_constraint_violation(error: &AppError) -> Option<ConstraintViolationInfo> {
    get_constraint_handler().analyze_constraint_violation(error)
}

/// Generate user-friendly message for constraint violation
pub fn generate_constraint_error_message(error: &AppError) -> Option<String> {
    if let Some(info) = analyze_constraint_violation(error) {
        Some(get_constraint_handler().generate_user_friendly_message(&info))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique_constraint_analysis() {
        let handler = ConstraintViolationHandler::new();
        
        let error = AppError::DatabaseError(
            "duplicate key value violates unique constraint \"users_email_key\"".to_string()
        );
        
        let info = handler.analyze_constraint_violation(&error).unwrap();
        assert_eq!(info.violation_type, ConstraintViolationType::UniqueConstraint);
        assert!(info.is_recoverable);
    }
    
    #[test]
    fn test_foreign_key_constraint_analysis() {
        let handler = ConstraintViolationHandler::new();
        
        let error = AppError::DatabaseError(
            "insert or update on table \"orders\" violates foreign key constraint \"fk_user_id\"".to_string()
        );
        
        let info = handler.analyze_constraint_violation(&error).unwrap();
        assert_eq!(info.violation_type, ConstraintViolationType::ForeignKeyConstraint);
        assert!(info.is_recoverable);
    }
    
    #[test]
    fn test_not_null_constraint_analysis() {
        let handler = ConstraintViolationHandler::new();
        
        let error = AppError::DatabaseError(
            "null value in column \"name\" violates not-null constraint".to_string()
        );
        
        let info = handler.analyze_constraint_violation(&error).unwrap();
        assert_eq!(info.violation_type, ConstraintViolationType::NotNullConstraint);
        assert!(info.is_recoverable);
    }
    
    #[test]
    fn test_user_friendly_message_generation() {
        let handler = ConstraintViolationHandler::new();
        
        let info = ConstraintViolationInfo {
            violation_type: ConstraintViolationType::UniqueConstraint,
            constraint_name: Some("users_email_key".to_string()),
            table_name: Some("users".to_string()),
            column_name: Some("email".to_string()),
            conflicting_value: Some("test@example.com".to_string()),
            suggested_resolution: "Use a different email address".to_string(),
            is_recoverable: true,
            recovery_strategy: Some("Check existing emails first".to_string()),
        };
        
        let message = handler.generate_user_friendly_message(&info);
        assert!(message.contains("already exists"));
        assert!(message.contains("email"));
    }
}
