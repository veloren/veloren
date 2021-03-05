use crate::settings::BanRecord;
use authc::{AuthClient, AuthClientError, AuthToken, Uuid};
use common_net::msg::RegisterError;
#[cfg(feature = "plugins")]
use common_sys::plugin::PluginMgr;
use hashbrown::{HashMap, HashSet};
use plugin_api::event::{PlayerJoinEvent, PlayerJoinResult};
use specs::World;
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

pub struct LoginProvider {
    accounts: HashMap<Uuid, String>,
    auth_server: Option<AuthClient>,
}

impl LoginProvider {
    pub fn new(auth_addr: Option<String>) -> Self {
        let auth_server = match auth_addr {
            Some(addr) => Some(AuthClient::new(&addr).unwrap()),
            None => None,
        };

        Self {
            accounts: HashMap::new(),
            auth_server,
        }
    }

    fn login(&mut self, uuid: Uuid, username: String) -> Result<(), RegisterError> {
        // make sure that the user is not logged in already
        if self.accounts.contains_key(&uuid) {
            return Err(RegisterError::AlreadyLoggedIn);
        }
        info!(?username, "New User");
        self.accounts.insert(uuid, username);
        Ok(())
    }

    pub fn logout(&mut self, uuid: Uuid) {
        if self.accounts.remove(&uuid).is_none() {
            error!(?uuid, "Attempted to logout user that is not logged in.");
        };
    }

    pub fn try_login(
        &mut self,
        username_or_token: &str,
        world: &World,
        #[cfg(feature = "plugins")] plugin_manager: &PluginMgr,
        admins: &HashSet<Uuid>,
        whitelist: &HashSet<Uuid>,
        banlist: &HashMap<Uuid, BanRecord>,
    ) -> Result<(String, Uuid), RegisterError> {
        self
            // resolve user information
            .query(username_or_token)
            // if found, check name against whitelist or if user is admin
            .and_then(|(username, uuid)| {
                // user cannot join if they are listed on the banlist
                if let Some(ban_record) = banlist.get(&uuid) {
                    // Pull reason string out of ban record and send a copy of it
                    return Err(RegisterError::Banned(ban_record.reason.clone()));
                }

                // user can only join if he is admin, the whitelist is empty (everyone can join)
                // or his name is in the whitelist
                if !whitelist.is_empty() && !whitelist.contains(&uuid) && !admins.contains(&uuid) {
                    return Err(RegisterError::NotOnWhitelist);
                }
                #[cfg(feature = "plugins")]
                match plugin_manager.execute_event(&world, &PlayerJoinEvent {
                    player_name: username.clone(),
                    player_id: *uuid.as_bytes(),
                }) {
                    Ok(e) => {
                        for i in e.into_iter() {
                            if let PlayerJoinResult::Kick(a) = i {
                                return Err(RegisterError::Kicked(a));
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error occured while executing `on_join`: {:?}",e);
                    },
                };

                // add the user to self.accounts
                self.login(uuid, username.clone())?;

                Ok((username, uuid))
            })
    }

    pub fn query(&mut self, username_or_token: &str) -> Result<(String, Uuid), RegisterError> {
        // Based on whether auth server is provided or not we expect an username or
        // token
        match &self.auth_server {
            // Token from auth server expected
            Some(srv) => {
                info!(?username_or_token, "Validating token");
                // Parse token
                let token = AuthToken::from_str(username_or_token)
                    .map_err(|e| RegisterError::AuthError(e.to_string()))?;
                // Validate token
                let uuid = srv.validate(token)?;
                let username = srv.uuid_to_username(uuid)?;
                Ok((username, uuid))
            },
            // Username is expected
            None => {
                // Assume username was provided
                let username = username_or_token;
                let uuid = derive_uuid(username);
                Ok((username.to_string(), uuid))
            },
        }
    }

    pub fn username_to_uuid(&self, username: &str) -> Result<Uuid, AuthClientError> {
        self.auth_server.as_ref().map_or_else(
            || Ok(derive_uuid(username)),
            |auth| auth.username_to_uuid(&username),
        )
    }
}
