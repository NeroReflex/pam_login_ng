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
        let intermediate_derived_key = crate::derive_key(&intermediate_key.as_str(), intermediate_salt);

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let main_nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        match main_cipher.encrypt(&main_nonce, main.as_ref()) {
            Ok(main_enc) =>  match hash(intermediate_derived_key, DEFAULT_COST) {
                Ok(hash_res) => Ok(
                    Self{
                        enc_main: main_enc,
                        enc_main_nonce: main_nonce.into(),
                        intermediate_key_salt: intermediate_salt.clone(),
                        intermediate_key_hash: hash_res,
                    }
                ),
                Err(err) => Err(UserOperationError::HashingError(err))
            },
            Err(err) => Err(UserOperationError::EncryptionError(err))
        }
    }

    pub fn plain(&self, intermediate_key: &String) -> Result<Vec<u8>, UserOperationError> {
        let intermediate_derived_key = crate::derive_key(&intermediate_key.as_str(), &self.intermediate_key_salt);

        if !verify(intermediate_derived_key, self.intermediate_key_hash.as_str()).map_err(|err| UserOperationError::HashingError(err))? {
            return Err(UserOperationError::User(Error::WrongIntermediateKey))
        }

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let main_nonce = Nonce::from_slice(self.enc_main_nonce.as_slice());

        Ok(main_cipher.decrypt(main_nonce, self.enc_main.as_ref()).map_err(|err| UserOperationError::EncryptionError(err))?)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    command: String,
    arguments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    main: Option<MainPassword>,
    auth: Vec<SecondaryAuth>,
    cmd: Option<Command>
}

impl User {
    pub fn new() -> Self {
        Self {
            main: None,
            auth: vec![],
            cmd: None
        }
    }

    pub fn add_secondary_password(
        &mut self,
        intermediate: &String,
        secondary_password: &String
    ) -> Result<(), UserOperationError> {
        // this makes the check about correctness of the intermediate password
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
    
        let res = serde_json::from_reader::<BufReader<File>, User>(reader).map_err(|err| UserOperationError::Serde(err))?;
        
        if let Some(m) = &res.main {
            let nonce_len = m.enc_main_nonce.len();
            if nonce_len != 12 {
                return Err(UserOperationError::Validation(
                    ValidationError::new(
                        String::from("main_nonce"),
                        format!("Invalid length (expected 96, got {})", nonce_len)
                    )
                ));
            }
        }

        Ok(res)
    }

    pub fn store_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), UserOperationError> {
        // Open the file for writing
        let file = File::create(path).map_err(|err| UserOperationError::Io(err))?;
        
        // Create a buffered writer
        let writer = BufWriter::new(file);
        
        // Serialize the User struct to JSON and write it to the file
        serde_json::to_writer(writer, self).map_err(|err| UserOperationError::Serde(err))?;
        
        Ok(())
    }

    pub fn set_cmd(&mut self, cmd: Command) -> Result<(), UserOperationError> {
        self.cmd = Some(cmd);

        Ok(())
    }

    pub fn cmd(&self) -> Result<Option<Command>, UserOperationError> {
        Ok(self.cmd.clone())
    }

    pub fn cmd_or_default(&self, default: Command) -> Result<Command, UserOperationError> {
        match self.cmd.clone() {
            Some(command) => Ok(command),
            None => Ok(default)
        }
    }
}
