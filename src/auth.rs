use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash, verify};

use crate::{error::*, user::{AuthDataNonce, AuthDataSalt, UserAuthDataError}};

#[derive(Debug, Clone)]
pub struct SecondaryPassword {
    enc_intermediate_nonce: AuthDataNonce,
    enc_intermediate: Vec<u8>, // this is encrypted with the (password, enc_intermediate_nonce)
    
    password_salt: AuthDataSalt,

    password_hash: String // this is used to check the entered password
}

impl SecondaryPassword {

    // WARNING: it is the user responsibility to check that the intermediate value matches the MainPassword field,
    // therefore the user MUST verify() it beforehand
    pub fn new(
        intermediate: &String,
        password: &String
    ) -> Result<Self, UserOperationError> {
        let password_salt_arr = <[u8; 32]>::try_from(
            Aes256Gcm::generate_key(&mut OsRng).to_vec().as_slice()
        ).unwrap();

        let password_hash = hash(password.as_str(), DEFAULT_COST).map_err(|err| UserOperationError::HashingError(err))?;

        let password_derived_key = crate::derive_key(&password.as_str(), &password_salt_arr);

        let key = Key::<Aes256Gcm>::from_slice(&password_derived_key);

        let cipher = Aes256Gcm::new(key);
        
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let enc_intermediate = cipher.encrypt(&nonce, crate::password_to_vec(intermediate).as_ref()).map_err(|err|  UserOperationError::EncryptionError(err))?;

        let temp: [u8; 32] = password_salt_arr.into();
        let password_salt = AuthDataSalt::from(temp);
        let temp: [u8; 12] = nonce.into();
        let enc_intermediate_nonce = AuthDataNonce::from(temp);
        Ok(
            Self {
                enc_intermediate_nonce,
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
            return Err(UserOperationError::User(UserAuthDataError::CouldNotAuthenticate))
        }

        let temp: [u8; 32] = self.password_salt.into();
        let password_derived_key = crate::derive_key(&password.as_str(), temp.as_slice());

        let key = Key::<Aes256Gcm>::from_slice(&password_derived_key);
        let cipher = Aes256Gcm::new(key);

        let temp: [u8; 12] = self.enc_intermediate_nonce.into();
        let nonce = Nonce::from_slice(temp.as_slice());

        let dec_result = cipher.decrypt(nonce, self.enc_intermediate.as_ref()).map_err(|err| UserOperationError::EncryptionError(err))?;

        Ok(crate::vec_to_password(&dec_result))
    }
}

#[derive(Debug, Clone)]
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
                    None => Err(UserOperationError::User(UserAuthDataError::MatchingAuthNotProvided))
                }
            }
        }
    }
}
