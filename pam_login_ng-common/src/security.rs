use rsa::{pkcs1::DecodeRsaPublicKey, Error as RSAError, Pkcs1v15Encrypt, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionPreludeError {
    #[error("Error importing the pem public key")]
    PubKeyImportError,

    #[error("RSA error: {0}")]
    RSAError(#[from] RSAError),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPrelude {
    pub_pkcs1_pem: String,
    one_time_token: Vec<u8>,
}

impl SessionPrelude {
    pub fn new(pub_pkcs1_pem: String) -> Self {
        let one_time_token = vec![];

        Self {
            one_time_token,
            pub_pkcs1_pem,
        }
    }

    // Serialize the struct to a JSON string
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    // Deserialize a JSON string to the struct
    pub fn from_string(s: &str) -> Self {
        serde_json::from_str(s).unwrap()
    }

    pub fn one_time_token(&self) -> Vec<u8> {
        self.one_time_token.clone()
    }

    pub fn encrypt(&self, plain_main_password: &String) -> Result<Vec<u8>, SessionPreludeError> {
        let Ok(pubkey) = RsaPublicKey::from_pkcs1_pem(self.pub_pkcs1_pem.as_str()) else {
            return Err(SessionPreludeError::PubKeyImportError);
        };

        let mut rng = rand::thread_rng();

        Ok(pubkey
            .encrypt(&mut rng, Pkcs1v15Encrypt, plain_main_password.as_bytes())
            .map_err(SessionPreludeError::RSAError)?)
    }
}
