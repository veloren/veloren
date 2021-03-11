use crate::settings::BanRecord;
use authc::{AuthClient, AuthClientError, AuthToken, Uuid};
use common_net::msg::RegisterError;
#[cfg(feature = "plugins")]
use common_sys::plugin::memory_manager::EcsWorld;
#[cfg(feature = "plugins")]
use common_sys::plugin::PluginMgr;
use hashbrown::{HashMap, HashSet};
use plugin_api::event::{PlayerJoinEvent, PlayerJoinResult};
use specs::Component;
use specs_idvs::IdvStorage;
use std::{str::FromStr, sync::Arc};
use tokio::{runtime::Runtime, sync::oneshot};
use tracing::{error, info};

fn derive_uuid(username: &str) -> Uuid {
    let mut state = 144066263297769815596495629667062367629;

    for byte in username.as_bytes() {
        state ^= *byte as u128;
        state = state.wrapping_mul(309485009821345068724781371);
    }

    Uuid::from_slice(&state.to_be_bytes()).unwrap()
}

/// derive Uuid for "singleplayer" is a pub fn
pub fn derive_singleplayer_uuid() -> Uuid { derive_uuid("singleplayer") }

pub struct PendingLogin {
    pending_r: oneshot::Receiver<Result<(String, Uuid), RegisterError>>,
}

impl Component for PendingLogin {
    type Storage = IdvStorage<Self>;
}

pub struct LoginProvider {
    runtime: Arc<Runtime>,
    accounts: HashMap<Uuid, String>,
    auth_server: Option<Arc<AuthClient>>,
}

impl LoginProvider {
    pub fn new(auth_addr: Option<String>, runtime: Arc<Runtime>) -> Self {
        tracing::trace!(?auth_addr, "Starting LoginProvider");
        let auth_server = auth_addr
            .map(|addr| Arc::new(AuthClient::new(authc::Authority::from_str(&addr).unwrap())));

        Self {
            runtime,
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

    pub fn verify(&self, username_or_token: &str) -> PendingLogin {
        let (pending_s, pending_r) = oneshot::channel();

        match &self.auth_server {
            // Token from auth server expected
            Some(srv) => {
                let srv = Arc::clone(srv);
                let username_or_token = username_or_token.to_string();
                self.runtime.spawn(async move {
                    let _ = pending_s.send(Self::query(srv, &username_or_token).await);
                });
            },
            // Username is expected
            None => {
                let username = username_or_token;
                let uuid = derive_uuid(username);
                let _ = pending_s.send(Ok((username.to_string(), uuid)));
            },
        }

        PendingLogin { pending_r }
    }

    pub fn try_login(
        &mut self,
        pending: &mut PendingLogin,
        #[cfg(feature = "plugins")] world: &EcsWorld,
        #[cfg(feature = "plugins")] plugin_manager: &PluginMgr,
        admins: &HashSet<Uuid>,
        whitelist: &HashSet<Uuid>,
        banlist: &HashMap<Uuid, BanRecord>,
    ) -> Option<Result<(String, Uuid), RegisterError>> {
        match pending.pending_r.try_recv() {
            Ok(Err(e)) => Some(Err(e)),
            Ok(Ok((username, uuid))) => {
                if let Some(ban_record) = banlist.get(&uuid) {
                    // Pull reason string out of ban record and send a copy of it
                    return Some(Err(RegisterError::Banned(ban_record.reason.clone())));
                }

                // user can only join if he is admin, the whitelist is empty (everyone can join)
                // or his name is in the whitelist
                if !whitelist.is_empty() && !whitelist.contains(&uuid) && !admins.contains(&uuid) {
                    return Some(Err(RegisterError::NotOnWhitelist));
                }
                #[cfg(feature = "plugins")]
                {
                    match plugin_manager.execute_event(&world, &PlayerJoinEvent {
                        player_name: username.clone(),
                        player_id: *uuid.as_bytes(),
                    }) {
                        Ok(e) => {
                            for i in e.into_iter() {
                                if let PlayerJoinResult::Kick(a) = i {
                                    return Some(Err(RegisterError::Kicked(a)));
                                }
                            }
                        },
                        Err(e) => {
                            error!("Error occured while executing `on_join`: {:?}", e);
                        },
                    };
                }

                // add the user to self.accounts
                match self.login(uuid, username.clone()) {
                    Ok(()) => Some(Ok((username, uuid))),
                    Err(e) => Some(Err(e)),
                }
            },
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                error!("channel got closed to early, this shouldn't happen");
                Some(Err(RegisterError::AuthError(
                    "Internal Error verifying".to_string(),
                )))
            },
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => None,
        }
    }

    async fn query(
        srv: Arc<AuthClient>,
        username_or_token: &str,
    ) -> Result<(String, Uuid), RegisterError> {
        info!(?username_or_token, "Validating token");
        // Parse token
        let token = AuthToken::from_str(username_or_token)
            .map_err(|e| RegisterError::AuthError(e.to_string()))?;
        // Validate token
        let uuid = srv.validate(token).await?;
        let username = srv.uuid_to_username(uuid).await?;
        Ok((username, uuid))
    }

    pub fn username_to_uuid(&self, username: &str) -> Result<Uuid, AuthClientError> {
        match &self.auth_server {
            Some(srv) => {
                //TODO: optimize
                self.runtime.block_on(srv.username_to_uuid(&username))
            },
            None => Ok(derive_uuid(username)),
        }
    }
}
