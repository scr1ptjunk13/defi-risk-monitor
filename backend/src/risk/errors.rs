// Risk calculation error types
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RiskError {
    #[error("Protocol not supported: {protocol}")]
    UnsupportedProtocol { protocol: String },
    
    #[error("Invalid position data: {message}")]
    InvalidPosition { message: String },
    
    #[error("Risk calculation failed: {message}")]
    CalculationError { message: String },
    
    #[error("Missing required data: {field}")]
    MissingData { field: String },
    
    #[error("Protocol risk calculator not found: {protocol}")]
    CalculatorNotFound { protocol: String },
    
    #[error("Position validation failed: {reason}")]
    ValidationError { reason: String },
    
    #[error("External service error: {service} - {message}")]
    ExternalServiceError { service: String, message: String },
    
    #[error("Configuration error: {message}")]
    ConfigurationError { message: String },
    
    #[error("Timeout error: {operation}")]
    TimeoutError { operation: String },
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("BigDecimal conversion error: {0}")]
    BigDecimalError(#[from] bigdecimal::ParseBigDecimalError),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

impl From<String> for RiskError {
    fn from(message: String) -> Self {
        RiskError::Generic(message)
    }
}

impl From<&str> for RiskError {
    fn from(message: &str) -> Self {
        RiskError::Generic(message.to_string())
    }
}
