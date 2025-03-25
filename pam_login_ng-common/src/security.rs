use serde::{Serialize, Deserialize};
use serde_json;

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
}
