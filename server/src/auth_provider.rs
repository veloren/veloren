use authc::{AuthClient, AuthToken, Uuid};
use common::msg::RegisterError;
use hashbrown::HashMap;
use std::str::FromStr;

fn derive_uuid(username: &str) -> Uuid {
    let mut state: [u8; 16] = [
        52, 17, 19, 239, 52, 17, 19, 239, 52, 17, 19, 239, 52, 17, 19, 239,
    ];
    for mix_byte_1 in username.as_bytes() {
        for i in 0..16 {
            let mix_byte_step: u8 = mix_byte_1
                .wrapping_pow(239)
                .wrapping_mul((i as u8).wrapping_pow(43));
            let mix_byte_2 = state[(i + mix_byte_step as usize) % 16];
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
    accounts: HashMap<Uuid, String>,
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

    pub fn query(&mut self, username_or_token: String) -> Result<(), RegisterError> {
        // Based on whether auth server is provided or not we expect an username or
        // token
        match &self.auth_server {
            // Token from auth server expected
            Some(srv) => {
                log::info!("Validating '{}' token.", &username_or_token);
                // Parse token
                let token = AuthToken::from_str(&username_or_token)
                    .map_err(|e| RegisterError::AuthError(e.to_string()))?;
                // Validate token
                let uuid = srv.validate(token)?;
                // Check if already logged in
                if self.accounts.contains_key(&uuid) {
                    return Err(RegisterError::AlreadyLoggedIn);
                }
                // Log in
                let username = srv.uuid_to_username(uuid.clone())?;
                self.accounts.insert(uuid, username);
                Ok(())
            },
            // Username is expected
            None => {
                // Assume username was provided
                let username = username_or_token;
                let uuid = derive_uuid(&username);
                if !self.accounts.contains_key(&uuid) {
                    log::info!("New User '{}'", username);
                    self.accounts.insert(uuid, username);
                    Ok(())
                } else {
                    Err(RegisterError::AlreadyLoggedIn)
                }
            },
        }
    }
}
