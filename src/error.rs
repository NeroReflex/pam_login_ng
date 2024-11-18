use serde_json::Error as SerdeError;
use std::error::Error as StdError;
use std::io::Error as IoError;
use aes_gcm::Error as AesError;

use thiserror::Error;

#[derive(Debug)]
pub struct ValidationError {
    field: String,
    message: String
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ValidationError: {} - {}", self.field, self.message)
    }
}

impl ValidationError {
    pub fn new(
        field: String,
        message: String
    ) -> Self {
        Self {
            field,
            message
        }
    }
}

impl StdError for ValidationError {}

#[derive(Debug, Copy, Clone)]
pub enum Error {
    WrongIntermediateKey,
    MainPasswordNotSet,
    CouldNotAuthenticate,
    MatchingAuthNotProvided,
    InvalidPassword,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongIntermediateKey => write!(f, "Wrong intermediate key"),
            Self::MainPasswordNotSet => write!(f, "Main password not set"),
            Self::CouldNotAuthenticate => write!(f, "Could not authenticate"),
            Self::MatchingAuthNotProvided => write!(f, "Authentication method unsupported"),
            Self::InvalidPassword => write!(f, "Invalid password (probably contains invalid characters)")
        }
    }
}

impl StdError for Error {}

#[derive(Debug, Error)]
pub enum UserOperationError {
    #[error("File I/O error: {0}")]
    Io(#[from] IoError),
    #[error("JSON deserialization error: {0}")]
    Serde(#[from] SerdeError),
    #[error("Encryption error: {0}")]
    EncryptionError(/*#[from]*/ AesError),
    #[error("Hashing error: {0}")]
    HashingError(#[from] bcrypt::BcryptError),
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    #[error("login-ng error: {0}")]
    User(#[from] Error),
}
