use thiserror::Error;

#[derive(Error, Debug)]
pub enum AztibaseError {
    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Invalid block: {0}")]
    InvalidBlock(String),

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
}

pub type AztibaseResult<T> = Result<T, AztibaseError>;
