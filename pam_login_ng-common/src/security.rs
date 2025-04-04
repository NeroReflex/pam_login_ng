use std::sync::Arc;

use rsa::{
    pkcs1::DecodeRsaPublicKey, Error as RSAError, Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};

use thiserror::Error;

use login_ng::aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};

#[derive(Debug, Error)]
pub enum SessionPreludeError {
    #[error("Error importing the pem public key")]
    PubKeyImportError,

    #[error("RSA error: {0}")]
    RSAError(#[from] RSAError),

    #[error("AES error")]
    AESError,

    #[error("Invalid ciphertext")]
    InvalidCiphertext,

    #[error("Nonce too long")]
    NonceTooLong,

    #[error("Key too long")]
    KeyTooLong,

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
        let mut one_time_token = vec![];

        for _ in 0..255 {
            one_time_token.push(crate::rand::random())
        }

        Self {
            one_time_token,
            pub_pkcs1_pem,
        }
    }

    pub fn one_time_token(&self) -> Vec<u8> {
        self.one_time_token.clone()
    }

    pub fn encrypt(&self, plaintext: String) -> Result<Vec<u8>, SessionPreludeError> {
        let key = Aes256Gcm::generate_key(&mut OsRng);
        let serialized_key = <[u8; 32]>::try_from(key.as_slice()).unwrap();
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let cipher = Aes256Gcm::new(&key);

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

        let encrypted_message = cipher
            .encrypt(
                &nonce,
                combine(self.one_time_token.clone(), plain_vec).as_slice(),
            )
            .unwrap();

        let mut rng = rand::thread_rng();
        let rsa_encrypted_key = pubkey
            .encrypt(&mut rng, Pkcs1v15Encrypt, serialized_key.as_slice())
            .map_err(SessionPreludeError::RSAError)?;

        let nonce_slice = nonce.as_slice();

        if nonce_slice.len() > u8::MAX as usize {
            return Err(SessionPreludeError::NonceTooLong);
        }

        if rsa_encrypted_key.len() > u8::MAX as usize {
            return Err(SessionPreludeError::KeyTooLong);
        }

        let mut result = vec![nonce_slice.len() as u8, rsa_encrypted_key.len() as u8];
        result.extend_from_slice(nonce_slice);
        result.extend(rsa_encrypted_key);
        result.extend(encrypted_message);

        Ok(result)
    }

    pub fn decrypt(
        priv_key: Arc<RsaPrivateKey>,
        ciphertext: Vec<u8>,
    ) -> Result<(Vec<u8>, Vec<u8>), SessionPreludeError> {
        if ciphertext.len() < 3 {
            return Err(SessionPreludeError::InvalidCiphertext);
        }

        // Ensure the encrypted data is long enough to contain the nonce, encrypted key, and message
        let nonce_len = ciphertext[0] as usize;
        let rsa_encrypted_key_len = ciphertext[1] as usize;
        if ciphertext.len() < nonce_len + rsa_encrypted_key_len + 510 {
            return Err(SessionPreludeError::InvalidCiphertext);
        }

        // Extract the nonce (first 12 bytes for AES-GCM)
        let nonce = Nonce::from_slice(&ciphertext[2..(2 + nonce_len)]);

        // Extract the RSA-encrypted key (next 256 bytes)
        let rsa_encrypted_key =
            &ciphertext[(2 + nonce_len)..(2 + nonce_len + rsa_encrypted_key_len)];

        // Extract the encrypted message (remaining bytes)
        let encrypted_message = &ciphertext[(2 + nonce_len + rsa_encrypted_key_len)..];

        let serialized_key = priv_key
            .decrypt(Pkcs1v15Encrypt, rsa_encrypted_key)
            .map_err(SessionPreludeError::RSAError)?;

        let key = Key::<Aes256Gcm>::from_slice(&serialized_key);

        let cipher = Aes256Gcm::new(key);

        let plaintext_mixed = cipher
            .decrypt(nonce, encrypted_message)
            .map_err(|_| SessionPreludeError::AESError)?;

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
