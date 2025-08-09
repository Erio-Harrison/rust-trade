use thiserror::Error;
use crate::data::types::DataError;
use crate::exchange::ExchangeError;

/// Service layer error types
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Exchange error: {0}")]
    Exchange(#[from] ExchangeError),
    
    #[error("Data error: {0}")]
    Data(#[from] DataError),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Task error: {0}")]
    Task(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Service shutdown")]
    Shutdown,
    
    #[error("Retry limit exceeded: {0}")]
    RetryLimitExceeded(String),
}

impl ServiceError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ServiceError::Exchange(e) => true,
            ServiceError::Data(_) => true, // Data errors are usually recoverable
            ServiceError::Task(_) => true,
            ServiceError::Shutdown => false,
            ServiceError::Config(_) => false,
            ServiceError::Validation(_) => false,
            ServiceError::RetryLimitExceeded(_) => false,
        }
    }
}