use std::fs::{self, create_dir, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::pkcs8::LineEnding;

use crate::service::ServiceError;

pub async fn create_directory(dir_path_str: &str) -> Result<(), ServiceError> {
    let dir_path = Path::new(dir_path_str);

    if !dir_path.exists() {
        match create_dir(dir_path) {
            Ok(_) => {
                println!("📁 Directory {dir_path_str} created");

                let mut permissions = fs::metadata(dir_path)?.permissions();
                permissions.set_mode(0o700);

                fs::set_permissions(dir_path, permissions)?;
            }
            Err(err) => {
                eprintln!("❌ Could not create directory {dir_path_str}: {err}");

                return Err(ServiceError::IOError(err))
            }
        }
    }

    Ok(())
}

pub async fn read_file_or_create_default(dir_path_str: &str, file_name_str: &str) -> Result<String, ServiceError> {
    let dir_path = Path::new(dir_path_str);

    let file_path = dir_path.join(file_name_str);

    let contents = match file_path.exists() {
        true => {
            let mut contents = String::new();

            let mut file = File::open(file_path)?;
            let read = file.read_to_string(&mut contents)?;
            println!("📖 Read private key file of {read} bytes");

            contents
        }
        false => {
            eprintln!(
                "🖊️ File {dir_path_str}/{file_name_str} not found: a new one will be generated..."
            );

            let mut rng = crate::rand::thread_rng();
            let priv_key = crate::rsa::RsaPrivateKey::new(&mut rng, 4096)
                .expect("failed to generate a key");

            let contents = priv_key.to_pkcs1_pem(LineEnding::CRLF)?.to_string();

            match File::create(&file_path) {
                Ok(mut file) => {
                    let metadata = file.metadata()?;
                    let mut perm = metadata.permissions();
                    perm.set_mode(0o700);

                    fs::set_permissions(file_path, perm)?;
                    match file.write_all(contents.to_string().as_bytes()) {
                        Ok(_) => {
                            println!(
                                "✅ Generated key has been saved to {dir_path_str}/{file_name_str}"
                            )
                        }
                        Err(err) => {
                            eprintln!("❌ Failed to write the generated key to {dir_path_str}/{file_name_str}: {err}")
                        }
                    };
                }
                Err(err) => {
                    eprintln!("Failed to create the file {dir_path_str}/{file_name_str}: {err}")
                }
            };

            contents
        }
    };

    Ok(contents)
}
