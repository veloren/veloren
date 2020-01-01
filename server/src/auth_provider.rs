use authc::{AuthClient, AuthToken, Uuid};
use common::msg::ServerError;
use hashbrown::HashMap;
use std::str::FromStr;

fn contains_value(map: &HashMap<String, String>, value: &str) -> bool {
    let mut contains = false;
    for ev in map.values() {
        if value == ev {
            contains = true;
        }
    }
    contains
}

fn derive_uuid(username: &str) -> Uuid {
    let mut state: [u8; 16] = [
        52, 17, 19, 239, 52, 17, 19, 239, 52, 17, 19, 239, 52, 17, 19, 239,
    ];
    for mix_byte_1 in username.as_bytes() {
        for i in 0..16 {
            let mix_byte_step: u8 = mix_byte_1
                .wrapping_pow(239)
                .wrapping_mul((i as u8).wrapping_pow(43));
            let mix_byte_2 = state[i + mix_byte_step as usize % 16];
            let rot_step: u8 = mix_byte_1
                .wrapping_pow(29)
                .wrapping_mul((i as u8).wrapping_pow(163));
            state[i] = (state[i] ^ mix_byte_1)
                .wrapping_mul(mix_byte_2)
                .rotate_left(rot_step as u32);
        }
    }
    Uuid::from_slice(&state).unwrap()
}

pub struct AuthProvider {
    accounts: HashMap<String, String>,
    auth_server: Option<AuthClient>,
}

impl AuthProvider {
    pub fn new(auth_addr: Option<String>) -> Self {
        let auth_server = match auth_addr {
            Some(addr) => Some(AuthClient::new(addr)),
            None => None,
        };

        AuthProvider {
            accounts: HashMap::new(),
            auth_server,
        }
    }

    pub fn query(&mut self, username_or_token: String) -> Result<bool, ServerError> {
        // Based on whether auth server is provided or not we expect an username or
        // token
        match &self.auth_server {
            // Token from auth server expected
            Some(srv) => {
                log::info!("Validating '{}' token.", &username_or_token);
                if let Ok(token) = AuthToken::from_str(&username_or_token) {
                    match srv.validate(token) {
                        Ok(id) => {
                            if contains_value(&self.accounts, &id.to_string()) {
                                return Err(ServerError::AlreadyLoggedIn);
                            }
                            let username = srv.uuid_to_username(id.clone())?;
                            self.accounts.insert(username, id.to_string());
                            Ok(true)
                        },
                        Err(e) => Err(ServerError::from(e)),
                    }
                } else {
                    Ok(false)
                }
            },
            // Username is expected
            None => {
                if !self.accounts.contains_key(&username_or_token) {
                    log::info!("New User '{}'", username_or_token);
                    let uuid = derive_uuid(&username_or_token);
                    self.accounts.insert(username_or_token, uuid.to_string());
                    Ok(true)
                } else {
                    Err(ServerError::AlreadyLoggedIn)
                }
            },
        }
    }
}
