use std::sync::Arc;

use rsa::{
    pkcs1::DecodeRsaPublicKey, Error as RSAError, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use serde_json;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionPreludeError {
    #[error("Error importing the pem public key")]
    PubKeyImportError,

    #[error("RSA error: {0}")]
    RSAError(#[from] RSAError),

    #[error("Invalid ciphertext")]
    InvalidCiphertext,

    #[error("Plaintext too long")]
    PlaintextTooLong,

    #[error("Invalid OTP")]
    InvalidOTP,

    #[error("Internal Error")]
    InternalError,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionPrelude {
    pub_pkcs1_pem: String,
    one_time_token: Vec<u8>,
}

fn string_to_vec_u8(input: String) -> Vec<u8> {
    // Convert the String to Vec<u8>
    let vec = input.into_bytes();

    // Create a new Vec<u8> of length 255, initialized with 0u8
    let mut result = vec![0u8; 255];

    // Copy the contents of the original Vec<u8> into the new vector
    let len = vec.len().min(255); // Ensure we don't exceed the length of 255
    result[..len].copy_from_slice(&vec[..len]);

    result
}

fn combine(otp: Vec<u8>, data: Vec<u8>) -> Vec<u8> {
    let mut combined = Vec::new();

    for i in 0..data.len() {
        combined.push(data[i]);
        combined.push(otp[i % otp.len()]);
    }

    combined
}

fn split(combined: Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let mut otp = Vec::new();
    let mut data = Vec::new();

    for (i, &value) in combined.iter().enumerate() {
        if i % 2 == 0 {
            data.push(value);
        } else {
            otp.push(value);
        }
    }

    (otp, data)
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

    pub fn encrypt(&self, plaintext: String) -> Result<Vec<u8>, SessionPreludeError> {
        if plaintext.len() > 255 {
            return Err(SessionPreludeError::PlaintextTooLong);
        }

        if self.one_time_token.len() != 255 {
            return Err(SessionPreludeError::InvalidOTP);
        }

        let Ok(pubkey) = RsaPublicKey::from_pkcs1_pem(self.pub_pkcs1_pem.as_str()) else {
            return Err(SessionPreludeError::PubKeyImportError);
        };

        let plain_vec = string_to_vec_u8(plaintext);
        if plain_vec.len() != 255 {
            return Err(SessionPreludeError::InternalError);
        }

        let mut rng = rand::thread_rng();

        pubkey
            .encrypt(
                &mut rng,
                Pkcs1v15Encrypt,
                combine(self.one_time_token.clone(), plain_vec).as_slice(),
            )
            .map_err(SessionPreludeError::RSAError)
    }

    pub fn decrypt(
        priv_key: Arc<RsaPrivateKey>,
        ciphertext: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<u8>), SessionPreludeError> {
        let plaintext_mixed = priv_key
            .decrypt(Pkcs1v15Encrypt, ciphertext.as_slice())
            .map_err(SessionPreludeError::RSAError)?;

        if plaintext_mixed.len() != 510 {
            return Err(SessionPreludeError::InvalidCiphertext);
        }

        let (otp, plaintext_long) = split(plaintext_mixed);

        if otp.len() != 255 {
            return Err(SessionPreludeError::InvalidCiphertext);
        }

        let plaintext: Vec<u8> = plaintext_long
            .iter()
            .filter_map(|ch| match ch {
                0u8 => None,
                ch => Some(*ch),
            })
            .collect();

        Ok((otp, plaintext))
    }
}
