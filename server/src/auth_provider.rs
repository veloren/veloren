use log::{info, warn};
use std::collections::HashMap;

pub struct AuthProvider {
    accounts: HashMap<String, String>,
}

impl AuthProvider {
    pub fn new() -> Self {
        AuthProvider {
            accounts: HashMap::new(),
        }
    }

    pub fn query(&mut self, username: String, password: String) -> bool {
        if let Some(pass) = self.accounts.get(&username) {
            if pass != &password {
                warn!(
                    "User '{}' attempted to log in with invalid password '{}'!",
                    username, password
                );
                return false;
            }
            info!("User '{}' successfully authenticated", username);
        } else {
            info!("Registered new user '{}'", username);
            self.accounts.insert(username, password);
        }
        true
    }
}
