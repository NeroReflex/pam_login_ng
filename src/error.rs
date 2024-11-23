use aes_gcm::Error as AesError;
use std::io::Error as IoError;

use thiserror::Error;

use crate::user::UserAuthDataError;

#[derive(Debug, Error)]
pub enum UserOperationError {
    #[error("File I/O error: {0}")]
    Io(#[from] IoError),
    #[error("Encryption error: {0}")]
    EncryptionError(/*#[from]*/ AesError),
    #[error("Hashing error: {0}")]
    HashingError(#[from] bcrypt::BcryptError),
    #[error("login-ng error: {0}")]
    User(#[from] UserAuthDataError),
}
