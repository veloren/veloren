use authc::{AuthClient, AuthToken};
use common::msg::ServerError;
use hashbrown::HashMap;
use std::str::FromStr;

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
                // TODO: Check if already logged in!
                log::info!("Validating '{}' token.", &username_or_token);
                match srv.validate(
                    AuthToken::from_str(&username_or_token).expect("Failed parsing token"), // TODO: POSSIBLE DOS, handle result!
                ) {
                    Ok(id) => {
                        // TODO: Get username!
                        self.accounts.insert("peter".into(), id.to_string());
                        Ok(true)
                    }
                    Err(e) => {
                        log::error!("{}", e);
                        Ok(false)
                    }
                }
            },
            // Username is expected
            None => {
                if !self.accounts.contains_key(&username_or_token) {
                    log::info!("New User '{}'", username_or_token);
                    self.accounts
                        .insert(username_or_token, "whateverUUID".into()); // TODO: generate UUID
                    Ok(true)
                } else {
                    Err(ServerError::AlreadyLoggedIn)
                }
            },
        }
    }
}
