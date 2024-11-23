pub mod login;
pub mod user;
pub mod auth;
pub mod error;
pub mod storage;
pub mod conversation;
pub mod cli;
pub mod pam;

#[cfg(feature = "greetd")]
pub mod greetd;

#[cfg(test)]
pub(crate) mod tests;

extern crate bytevec2;

pub const DEFAULT_CMD: &str = "/bin/sh";

pub const DEFAULT_XATTR_NAME: &str = "user.login-ng";

use std::io::BufRead;

use hkdf::*;
use sha2::Sha256;

pub use rpassword::prompt_password;

pub(crate) fn derive_key(input: &str, salt: &[u8]) -> [u8; 32] {
    // Create an HKDF instance with SHA-256 as the hash function
    let hkdf = Hkdf::<Sha256>::new(Some(salt), input.as_bytes());

    // Prepare a buffer for the derived key
    let mut okm = [0u8; 32]; // Output key material (32 bytes)

    // Extract the key material
    hkdf.expand(&[], &mut okm).expect("Failed to expand key");

    okm
}

pub(crate) fn password_to_vec(password: &String) -> Vec<u8> {
    password.as_str().into()
}

pub(crate) fn vec_to_password(vec: &Vec<u8>) -> String {
    String::from_utf8_lossy(vec.as_slice()).to_string()
}

// this MUST be implemented and used because entering invalid strings can be a security hole (see lossy_utf8)
pub(crate) fn is_valid_password(password: &String) -> bool {
    vec_to_password(password_to_vec(password).as_ref()) == password.clone()
}

pub fn prompt_stderr(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let stdin = std::io::stdin();
    let mut stdin_iter = stdin.lock().lines();
    eprint!("{}", prompt);
    Ok(stdin_iter.next().ok_or("no input")??)
}