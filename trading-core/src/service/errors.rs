use crate::exchange::ExchangeError;
use thiserror::Error;
use trading_common::data::types::DataError;

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
}
