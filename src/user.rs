use serde::{Deserialize, Serialize};

use std::io::{BufWriter, BufReader};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};

extern crate bcrypt;
use bcrypt::{DEFAULT_COST, hash, verify};

use std::path::Path;
use std::fs::File;

use crate::error::*;
use crate::auth::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MainPassword {
    main_hash: String,
    enc_main: Vec<u8>,
    enc_main_nonce: [u8; 12],
    
    intermediate_key_salt: [u8; 32],
    intermediate_key_hash: String,
}

impl MainPassword {
    pub fn new(
        main: &Vec<u8>,
        intermediate_key: &String,
        intermediate_salt: &[u8; 32]
    ) -> Result<Self, UserOperationError> {
        let main_hash = hash(main, DEFAULT_COST).map_err(|err| UserOperationError::HashingError(err))?;

        let intermediate_key_hash = hash(intermediate_key, DEFAULT_COST).map_err(|err| UserOperationError::HashingError(err))?;

        let intermediate_derived_key = crate::derive_key(&intermediate_key.as_str(), intermediate_salt);

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let main_nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        match main_cipher.encrypt(&main_nonce, main.as_ref()) {
            Ok(enc_main) => Ok(Self{
                main_hash,
                enc_main,
                enc_main_nonce: main_nonce.into(),
                intermediate_key_salt: intermediate_salt.clone(),
                intermediate_key_hash,
            }),
            Err(err) => Err(UserOperationError::EncryptionError(err))
        }
    }

    pub fn plain(&self, ik_or_main: &String) -> Result<Vec<u8>, UserOperationError> {
        if verify(ik_or_main, &self.main_hash.as_str()).map_err(|err| UserOperationError::HashingError(err))? {
            return Ok(crate::password_to_vec(&ik_or_main))
        }

        // provided data was not the main password itself: threat it as the intermediate key
        let intermediate_key = ik_or_main;

        if !verify(intermediate_key, self.intermediate_key_hash.as_str()).map_err(|err| UserOperationError::HashingError(err))? {
            return Err(UserOperationError::User(Error::WrongIntermediateKey))
        }
        
        let intermediate_derived_key = crate::derive_key(&intermediate_key.as_str(), &self.intermediate_key_salt);

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let main_nonce = Nonce::from_slice(self.enc_main_nonce.as_slice());

        let decrypted_main = main_cipher.decrypt(main_nonce, self.enc_main.as_ref()).map_err(|err| UserOperationError::EncryptionError(err))?;

        if !verify(decrypted_main.as_slice(), &self.main_hash).map_err(|err| UserOperationError::HashingError(err))? {
            return Err(UserOperationError::User(Error::WrongIntermediateKey))
        }

       Ok(decrypted_main)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    main: Option<MainPassword>,
    auth: Vec<SecondaryAuth>
}

impl User {
    pub fn new() -> Self {
        Self {
            main: None,
            auth: vec![],
        }
    }

    pub fn add_secondary_password(
        &mut self,
        intermediate: &String,
        secondary_password: &String
    ) -> Result<(), UserOperationError> {
        if !crate::is_valid_password(secondary_password) {
            return Err(UserOperationError::User(Error::InvalidPassword))
        }

        // this makes the check about correctness of the intermediate key
        let _ = self.main(intermediate)?;

        let secondary_auth_method = SecondaryPassword::new(intermediate, secondary_password)?;

        self.auth.push(SecondaryAuth::Password(secondary_auth_method));

        Ok(())
    }

    pub fn has_main(&self) -> bool {
        return self.main.is_some()
    }

    pub fn main_by_auth(
        &self,
        secondary_password: &Option<String>
    ) -> Result<String, UserOperationError> {
        let main = self.main.as_ref().ok_or(UserOperationError::User(Error::MainPasswordNotSet))?;
        
        if let Some(provided_pw) = secondary_password {
            if !crate::is_valid_password(provided_pw) {
                return Err(UserOperationError::User(Error::InvalidPassword))
            } else if let Ok(main_pw) = main.plain(provided_pw) {
                return Ok(crate::vec_to_password(&main_pw))
            }
        }

        for sec_auth in (&self.auth).into_iter() {
            if let Ok(intermediate) = sec_auth.intermediate(secondary_password) {
                if let Ok(main_pw_as_vec) = main.plain(&intermediate) {
                    return Ok(crate::vec_to_password(&main_pw_as_vec))
                }
            }
        }

        Err(UserOperationError::User(Error::CouldNotAuthenticate))
    }

    pub fn main(
        &self,    
        intermediate_key: &String
    ) -> Result<String, UserOperationError> {
        if !crate::is_valid_password(intermediate_key) {
            return Err(UserOperationError::User(Error::InvalidPassword))
        }

        match &self.main {
            Some(main) => Ok(crate::vec_to_password((main.plain(intermediate_key)?).as_ref())),
            None => Err(UserOperationError::User(Error::MainPasswordNotSet))
        }
    }

    pub fn set_main(
        &mut self,
        main: &String,
        intermediate_key: &String
    ) -> Result<(), UserOperationError> {
        if !crate::is_valid_password(main) {
            return Err(UserOperationError::User(Error::InvalidPassword))
        }

        if !crate::is_valid_password(intermediate_key) {
            return Err(UserOperationError::User(Error::InvalidPassword))
        }

        match &self.main {
            Some(m) => {
                if !verify(intermediate_key, &m.intermediate_key_hash).map_err(|err| UserOperationError::HashingError(err))? {
                    return Err(UserOperationError::User(Error::WrongIntermediateKey))
                }

                let mp = MainPassword::new(
                    &crate::password_to_vec(main),
                    intermediate_key,
                    &m.intermediate_key_salt
                )?;

                self.main = Some(mp);
    
                Ok(())
            }
            None => match MainPassword::new(
                &crate::password_to_vec(main),
                intermediate_key,
                // generate a new random salt using the aes-gcm library (it will create a 32 bytes key)
                &<[u8; 32]>::try_from(
                    Aes256Gcm::generate_key(&mut OsRng).to_vec().as_slice()
                ).unwrap()
            ) {
                Ok(mp) => {
                    self.main = Some(mp);

                    Ok(())
                }
                Err(err) => Err(err)
            }
        }
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, UserOperationError> {
        let file = File::open(path).map_err(|err| UserOperationError::Io(err))?;

        let reader = std::io::BufReader::new(file);

        Ok(serde_json::from_reader::<BufReader<File>, User>(reader).map_err(|err| UserOperationError::Serde(err))?)
    }

    pub fn store_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), UserOperationError> {
        // Open the file for writing
        let file = File::create(path).map_err(|err| UserOperationError::Io(err))?;
        
        // Create a buffered writer
        let writer = BufWriter::new(file);
        
        // Serialize the User struct to JSON and write it to the file
        Ok(serde_json::to_writer_pretty(writer, self).map_err(|err| UserOperationError::Serde(err))?)
    }
}
