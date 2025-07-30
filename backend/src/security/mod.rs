pub mod input_validation;
pub mod sql_injection_prevention;
pub mod secrets_management;
pub mod static_analysis;
pub mod audit_trail;

// Explicitly re-export only what's needed from each module
// This prevents ambiguous glob re-exports

// From input_validation
pub use input_validation::{
    InputValidator,
    ValidationResult,
};

// From sql_injection_prevention
pub use sql_injection_prevention::{
    SafeQueryBuilder,
    PreparedStatementCache,
    SqlSafetyChecker,
};

// From secrets_management
pub use secrets_management::{
    SecretsManager,
    SecretValue,
    SecretType,
    SecretAccess,
    EnvSecurityScanner,
    SecurityIssue,
    // Note: SecuritySeverity is re-exported from audit_trail to avoid conflict
};

// From static_analysis
pub use static_analysis::{
    AnalysisResult,
    QualityIssue,
    Vulnerability,
    VulnerabilityCategory,
    VulnerabilitySeverity,
    QualityImpact,
    ProjectAnalysisReport,
    SecurityReport,
    StaticAnalyzer,
};

// From audit_trail
pub use audit_trail::{
    SecurityAuditEvent,
    SecurityEventType,
    SecuritySeverity,
    SecurityAuditService,
    SecurityStatistics,
    ComplianceReport,
};
