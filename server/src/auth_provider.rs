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
        let pwd = password.clone();
        if self.accounts.entry(username.clone()).or_insert_with(|| {
            info!("Registered new user '{}'", &username);
            pwd
        }) == &password
        {
            info!("User '{}' successfully authenticated", username);
            true
        } else {
            warn!(
                "User '{}' attempted to log in with invalid password '{}'!",
                username, password
            );
            false
        }
    }
}
