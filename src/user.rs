use bytevec2::*;

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};

extern crate bcrypt;
use bcrypt::{hash, verify, DEFAULT_COST};

use thiserror::Error;

use crate::auth::*;
use crate::error::*;

#[derive(Debug, Copy, Clone, Error)]
pub enum UserAuthDataError {
    #[error("Wrong intermediate key")]
    WrongIntermediateKey,
    #[error("Main password not set")]
    MainPasswordNotSet,
    #[error("Could not authenticate")]
    CouldNotAuthenticate,
    #[error("Authentication method unsupported")]
    MatchingAuthNotProvided,
    #[error("Invalid password (probably contains invalid characters)")]
    InvalidPassword,
}

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub struct AuthDataNonce {
        a0: u8,
        a1: u8,
        a2: u8,
        a3: u8,
        a4: u8,
        a5: u8,
        a6: u8,
        a7: u8,
        a8: u8,
        a9: u8,
        a10: u8,
        a11: u8
    }
}

impl Into<[u8; 12]> for AuthDataNonce {
    fn into(self) -> [u8; 12] {
        [
            self.a0, self.a1, self.a2, self.a3, self.a4, self.a5, self.a6, self.a7, self.a8,
            self.a9, self.a10, self.a11,
        ]
    }
}

impl From<[u8; 12]> for AuthDataNonce {
    fn from(bytes: [u8; 12]) -> Self {
        Self {
            a0: bytes[0],
            a1: bytes[1],
            a2: bytes[2],
            a3: bytes[3],
            a4: bytes[4],
            a5: bytes[5],
            a6: bytes[6],
            a7: bytes[7],
            a8: bytes[8],
            a9: bytes[9],
            a10: bytes[10],
            a11: bytes[11],
        }
    }
}

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub struct AuthDataSalt {
        a0: u8,
        a1: u8,
        a2: u8,
        a3: u8,
        a4: u8,
        a5: u8,
        a6: u8,
        a7: u8,
        a8: u8,
        a9: u8,
        a10: u8,
        a11: u8,
        a12: u8,
        a13: u8,
        a14: u8,
        a15: u8,
        a16: u8,
        a17: u8,
        a18: u8,
        a19: u8,
        a20: u8,
        a21: u8,
        a22: u8,
        a23: u8,
        a24: u8,
        a25: u8,
        a26: u8,
        a27: u8,
        a28: u8,
        a29: u8,
        a30: u8,
        a31: u8
    }
}

impl Into<[u8; 32]> for AuthDataSalt {
    fn into(self) -> [u8; 32] {
        [
            self.a0, self.a1, self.a2, self.a3, self.a4, self.a5, self.a6, self.a7, self.a8,
            self.a9, self.a10, self.a11, self.a12, self.a13, self.a14, self.a15, self.a16,
            self.a17, self.a18, self.a19, self.a20, self.a21, self.a22, self.a23, self.a24,
            self.a25, self.a26, self.a27, self.a28, self.a29, self.a30, self.a31,
        ]
    }
}

impl From<[u8; 32]> for AuthDataSalt {
    fn from(bytes: [u8; 32]) -> Self {
        Self {
            a0: bytes[0],
            a1: bytes[1],
            a2: bytes[2],
            a3: bytes[3],
            a4: bytes[4],
            a5: bytes[5],
            a6: bytes[6],
            a7: bytes[7],
            a8: bytes[8],
            a9: bytes[9],
            a10: bytes[10],
            a11: bytes[11],
            a12: bytes[12],
            a13: bytes[13],
            a14: bytes[14],
            a15: bytes[15],
            a16: bytes[16],
            a17: bytes[17],
            a18: bytes[18],
            a19: bytes[19],
            a20: bytes[20],
            a21: bytes[21],
            a22: bytes[22],
            a23: bytes[23],
            a24: bytes[24],
            a25: bytes[25],
            a26: bytes[26],
            a27: bytes[27],
            a28: bytes[28],
            a29: bytes[29],
            a30: bytes[30],
            a31: bytes[31],
        }
    }
}

bytevec_decl! {
    #[derive(PartialEq, Eq, Debug, Clone)]
    pub struct MainPassword {
        main_hash: String,
        enc_main: Vec<u8>,
        enc_main_nonce: AuthDataNonce,

        intermediate_key_salt: AuthDataSalt,
        intermediate_key_hash: String
    }
}

impl MainPassword {
    pub fn new(
        main: &Vec<u8>,
        intermediate_key: &String,
        intermediate_salt: &[u8; 32],
    ) -> Result<Self, UserOperationError> {
        let main_hash =
            hash(main, DEFAULT_COST).map_err(|err| UserOperationError::HashingError(err))?;

        let intermediate_key_hash = hash(intermediate_key, DEFAULT_COST)
            .map_err(|err| UserOperationError::HashingError(err))?;

        let intermediate_derived_key =
            crate::derive_key(&intermediate_key.as_str(), intermediate_salt);

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let main_nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        let temp: [u8; 12] = main_nonce.into();

        match main_cipher.encrypt(&main_nonce, main.as_ref()) {
            Ok(enc_main) => Ok(Self {
                main_hash,
                enc_main,
                enc_main_nonce: AuthDataNonce::from(temp),
                intermediate_key_salt: AuthDataSalt::from(intermediate_salt.clone()),
                intermediate_key_hash,
            }),
            Err(err) => Err(UserOperationError::EncryptionError(err)),
        }
    }

    pub fn plain(&self, ik_or_main: &String) -> Result<Vec<u8>, UserOperationError> {
        if verify(ik_or_main, &self.main_hash.as_str())
            .map_err(|err| UserOperationError::HashingError(err))?
        {
            return Ok(crate::password_to_vec(&ik_or_main));
        }

        // provided data was not the main password itself: threat it as the intermediate key
        let intermediate_key = ik_or_main;

        if !verify(intermediate_key, self.intermediate_key_hash.as_str())
            .map_err(|err| UserOperationError::HashingError(err))?
        {
            return Err(UserOperationError::User(
                UserAuthDataError::WrongIntermediateKey,
            ));
        }

        let temp: [u8; 32] = self.intermediate_key_salt.into();
        let intermediate_derived_key =
            crate::derive_key(&intermediate_key.as_str(), temp.as_slice());

        let key = Key::<Aes256Gcm>::from_slice(&intermediate_derived_key);

        let main_cipher = Aes256Gcm::new(key);
        let temp: [u8; 12] = self.enc_main_nonce.into();
        let main_nonce = Nonce::from_slice(temp.as_slice());

        let decrypted_main = main_cipher
            .decrypt(main_nonce, self.enc_main.as_ref())
            .map_err(|err| UserOperationError::EncryptionError(err))?;

        if !verify(decrypted_main.as_slice(), &self.main_hash)
            .map_err(|err| UserOperationError::HashingError(err))?
        {
            return Err(UserOperationError::User(
                UserAuthDataError::WrongIntermediateKey,
            ));
        }

        Ok(decrypted_main)
    }
}

#[derive(Debug, Clone)]
pub struct UserAuthData {
    main: Option<MainPassword>,
    auth: Vec<SecondaryAuth>,
}

impl UserAuthData {
    pub fn new() -> Self {
        Self {
            main: None,
            auth: vec![],
        }
    }

    pub fn add_secondary_password(
        &mut self,
        name: &str,
        intermediate: &String,
        secondary_password: &String,
    ) -> Result<(), UserOperationError> {
        if !crate::is_valid_password(secondary_password) {
            return Err(UserOperationError::User(UserAuthDataError::InvalidPassword));
        }

        // this makes the check about correctness of the intermediate key
        let _ = self.main(intermediate)?;

        self.auth.push(SecondaryAuth::new_password(
            name,
            None,
            SecondaryPassword::new(intermediate, secondary_password)?,
        ));

        Ok(())
    }

    pub fn has_main(&self) -> bool {
        return self.main.is_some();
    }

    pub fn main_by_auth(
        &self,
        secondary_password: &Option<String>,
    ) -> Result<String, UserOperationError> {
        let main = self.main.as_ref().ok_or(UserOperationError::User(
            UserAuthDataError::MainPasswordNotSet,
        ))?;

        if let Some(provided_pw) = secondary_password {
            if !crate::is_valid_password(provided_pw) {
                return Err(UserOperationError::User(UserAuthDataError::InvalidPassword));
            } else if let Ok(main_pw) = main.plain(provided_pw) {
                return Ok(crate::vec_to_password(&main_pw));
            }
        }

        for sec_auth in (&self.auth).into_iter() {
            if let Ok(intermediate) = sec_auth.intermediate(secondary_password) {
                if let Ok(main_pw_as_vec) = main.plain(&intermediate) {
                    return Ok(crate::vec_to_password(&main_pw_as_vec));
                }
            }
        }

        Err(UserOperationError::User(
            UserAuthDataError::CouldNotAuthenticate,
        ))
    }

    pub fn main(&self, intermediate_key: &String) -> Result<String, UserOperationError> {
        if !crate::is_valid_password(intermediate_key) {
            return Err(UserOperationError::User(UserAuthDataError::InvalidPassword));
        }

        match &self.main {
            Some(main) => Ok(crate::vec_to_password(
                (main.plain(intermediate_key)?).as_ref(),
            )),
            None => Err(UserOperationError::User(
                UserAuthDataError::MainPasswordNotSet,
            )),
        }
    }

    pub fn set_main(
        &mut self,
        main: &String,
        intermediate_key: &String,
    ) -> Result<(), UserOperationError> {
        if !crate::is_valid_password(main) {
            return Err(UserOperationError::User(UserAuthDataError::InvalidPassword));
        }

        if !crate::is_valid_password(intermediate_key) {
            return Err(UserOperationError::User(UserAuthDataError::InvalidPassword));
        }

        match &self.main {
            Some(m) => {
                if !verify(intermediate_key, &m.intermediate_key_hash)
                    .map_err(|err| UserOperationError::HashingError(err))?
                {
                    return Err(UserOperationError::User(
                        UserAuthDataError::WrongIntermediateKey,
                    ));
                }

                let temp: [u8; 32] = m.intermediate_key_salt.into();
                let mp = MainPassword::new(&crate::password_to_vec(main), intermediate_key, &temp)?;

                self.main = Some(mp);

                Ok(())
            }
            None => match MainPassword::new(
                &crate::password_to_vec(main),
                intermediate_key,
                // generate a new random salt using the aes-gcm library (it will create a 32 bytes key)
                &<[u8; 32]>::try_from(Aes256Gcm::generate_key(&mut OsRng).to_vec().as_slice())
                    .unwrap(),
            ) {
                Ok(mp) => {
                    self.main = Some(mp);

                    Ok(())
                }
                Err(err) => Err(err),
            },
        }
    }

    pub(crate) fn main_password(&self) -> &Option<MainPassword> {
        &self.main
    }

    pub fn secondary(&self) -> std::slice::Iter<SecondaryAuth> {
        self.auth.iter()
    }

    pub(crate) fn push_main(&mut self, value: MainPassword) {
        self.main = Some(value);
    }

    pub(crate) fn push_secondary(&mut self, value: SecondaryAuth) {
        self.auth.push(value);
    }
}
