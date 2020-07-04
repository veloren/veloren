use authc::{AuthClient, AuthToken, Uuid};
use common::msg::RegisterError;
use hashbrown::HashMap;
use std::str::FromStr;
use tracing::{error, info};

fn derive_uuid(username: &str) -> Uuid {
    let mut state = 144066263297769815596495629667062367629;

    for byte in username.as_bytes() {
        state ^= *byte as u128;
        state = state.wrapping_mul(309485009821345068724781371);
    }

    Uuid::from_slice(&state.to_be_bytes()).unwrap()
}

pub struct AuthProvider {
    accounts: HashMap<Uuid, String>,
    auth_server: Option<AuthClient>,
    whitelist: Vec<String>,
}

impl AuthProvider {
    pub fn new(auth_addr: Option<String>, whitelist: Vec<String>) -> Self {
        let auth_server = match auth_addr {
            Some(addr) => Some(AuthClient::new(addr)),
            None => None,
        };

        AuthProvider {
            accounts: HashMap::new(),
            auth_server,
            whitelist,
        }
    }

    pub fn logout(&mut self, uuid: Uuid) {
        if self.accounts.remove(&uuid).is_none() {
            error!(?uuid, "Attempted to logout user that is not logged in.");
        };
    }

    pub fn query(&mut self, username_or_token: String) -> Result<(String, Uuid), RegisterError> {
        // Based on whether auth server is provided or not we expect an username or
        // token
        match &self.auth_server {
            // Token from auth server expected
            Some(srv) => {
                info!(?username_or_token, "Validating token");
                // Parse token
                let token = AuthToken::from_str(&username_or_token)
                    .map_err(|e| RegisterError::AuthError(e.to_string()))?;
                // Validate token
                let uuid = srv.validate(token)?;
                // Check if already logged in
                if self.accounts.contains_key(&uuid) {
                    return Err(RegisterError::AlreadyLoggedIn);
                }
                let username = srv.uuid_to_username(uuid)?;
                // Check if player is in whitelist
                if self.whitelist.len() > 0 && !self.whitelist.contains(&username) {
                    return Err(RegisterError::NotOnWhitelist);
                }

                // Log in
                self.accounts.insert(uuid, username.clone());
                Ok((username, uuid))
            },
            // Username is expected
            None => {
                // Assume username was provided
                let username = username_or_token;
                let uuid = derive_uuid(&username);
                if !self.accounts.contains_key(&uuid) {
                    info!(?username, "New User");
                    self.accounts.insert(uuid, username.clone());
                    Ok((username, uuid))
                } else {
                    Err(RegisterError::AlreadyLoggedIn)
                }
            },
        }
    }
}
