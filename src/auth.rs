use serde::{Deserialize, Serialize};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash, verify};

use crate::error::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecondaryPassword {
    enc_intermediate_nonce: [u8; 12],
    enc_intermediate: Vec<u8>, // this is encrypted with the (password, enc_intermediate_nonce)
    
    password_salt: [u8; 32],

    password_hash: String // this is used to check the entered password
}

impl SecondaryPassword {

    // WARNING: it is the user responsibility to check that the intermediate value matches the MainPassword field,
    // therefore the user MUST verify() it beforehand
    pub fn new(
        intermediate: &String,
        password: &String
    ) -> Result<Self, UserOperationError> {
        let password_salt = <[u8; 32]>::try_from(
            Aes256Gcm::generate_key(&mut OsRng).to_vec().as_slice()
        ).unwrap();

        let password_hash = hash(password.as_str(), DEFAULT_COST).map_err(|err| UserOperationError::HashingError(err))?;

        let password_derived_key = crate::derive_key(&password.as_str(), &password_salt);

        let key = Key::<Aes256Gcm>::from_slice(&password_derived_key);

        let cipher = Aes256Gcm::new(key);
        
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let enc_intermediate = cipher.encrypt(&nonce, crate::password_to_vec(intermediate).as_ref()).map_err(|err|  UserOperationError::EncryptionError(err))?;

        Ok(
            Self {
                enc_intermediate_nonce: nonce.into(),
                enc_intermediate,
                password_salt,
                password_hash
            }
        )
    }

    // get the intermediate if the password is correct
    pub fn intermediate(
        &self,
        password: &String
    ) -> Result<String, UserOperationError> {
        if !verify(password.as_str(), &self.password_hash.as_str()).map_err(|err| UserOperationError::HashingError(err))? {
            return Err(UserOperationError::User(Error::CouldNotAuthenticate))
        }

        let password_derived_key = crate::derive_key(&password.as_str(), &self.password_salt);

        let key = Key::<Aes256Gcm>::from_slice(&password_derived_key);
        let cipher = Aes256Gcm::new(key);

        let nonce = Nonce::from_slice(&self.enc_intermediate_nonce.as_slice());

        let dec_result = cipher.decrypt(nonce, self.enc_intermediate.as_ref()).map_err(|err| UserOperationError::EncryptionError(err))?;

        Ok(crate::vec_to_password(&dec_result))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SecondaryAuth {
    Password(SecondaryPassword)
}

impl SecondaryAuth {
    pub fn intermediate(
        &self,
        secondary_password: &Option<String>
    ) -> Result<String, UserOperationError> {
        match self {
            SecondaryAuth::Password(pwd) => {
                match &secondary_password {
                    Some(provided_secondary) => pwd.intermediate(provided_secondary),
                    None => Err(UserOperationError::User(Error::MatchingAuthNotProvided))
                }
            }
        }
    }
}
