use crate::settings::BanRecord;
use authc::{AuthClient, AuthClientError, AuthToken, Uuid};
use common_net::msg::RegisterError;
#[cfg(feature = "plugins")]
use common_state::plugin::memory_manager::EcsWorld;
#[cfg(feature = "plugins")]
use common_state::plugin::PluginMgr;
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

    Uuid::from_u128(state)
}

/// derive Uuid for "singleplayer" is a pub fn
pub fn derive_singleplayer_uuid() -> Uuid { derive_uuid("singleplayer") }

pub struct PendingLogin {
    pending_r: oneshot::Receiver<Result<(String, Uuid), RegisterError>>,
}

impl PendingLogin {
    pub(crate) fn new_success(username: String, uuid: Uuid) -> Self {
        let (pending_s, pending_r) = oneshot::channel();
        let _ = pending_s.send(Ok((username, uuid)));

        Self { pending_r }
    }
}

impl Component for PendingLogin {
    type Storage = IdvStorage<Self>;
}

pub struct LoginProvider {
    runtime: Arc<Runtime>,
    auth_server: Option<Arc<AuthClient>>,
}

impl LoginProvider {
    pub fn new(auth_addr: Option<String>, runtime: Arc<Runtime>) -> Self {
        tracing::trace!(?auth_addr, "Starting LoginProvider");

        let auth_server = auth_addr.map(|addr| {
            let (scheme, authority) = addr.split_once("://").expect("invalid auth url");

            let scheme = scheme
                .parse::<authc::Scheme>()
                .expect("invalid auth url scheme");
            let authority = authority
                .parse::<authc::Authority>()
                .expect("invalid auth url authority");

            Arc::new(AuthClient::new(scheme, authority).expect("insecure auth scheme"))
        });

        Self {
            runtime,
            auth_server,
        }
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

    pub fn login(
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
                // Hardcoded admins can always log in.
                let is_admin = admins.contains(&uuid);
                if !is_admin {
                    if let Some(ban_record) = banlist.get(&uuid) {
                        // Pull reason string out of ban record and send a copy of it
                        return Some(Err(RegisterError::Banned(ban_record.reason.clone())));
                    }

                    // non-admins can only join if the whitelist is empty (everyone can join)
                    // or his name is in the whitelist
                    if !whitelist.is_empty() && !whitelist.contains(&uuid) {
                        return Some(Err(RegisterError::NotOnWhitelist));
                    }
                }

                #[cfg(feature = "plugins")]
                {
                    // Plugin player join hooks execute for all players, but are only allowed to
                    // filter non-admins.
                    match plugin_manager.execute_event(&world, &PlayerJoinEvent {
                        player_name: username.clone(),
                        player_id: *uuid.as_bytes(),
                    }) {
                        Ok(e) => {
                            if !is_admin {
                                for i in e.into_iter() {
                                    if let PlayerJoinResult::Kick(a) = i {
                                        return Some(Err(RegisterError::Kicked(a)));
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            error!("Error occured while executing `on_join`: {:?}", e);
                        },
                    };
                }

                info!(?username, "New User");
                Some(Ok((username, uuid)))
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
        match async {
            let uuid = srv.validate(token).await?;
            let username = srv.uuid_to_username(uuid).await?;
            let r: Result<_, authc::AuthClientError> = Ok((username, uuid));
            r
        }
        .await
        {
            Err(e) => Err(RegisterError::AuthError(e.to_string())),
            Ok((username, uuid)) => Ok((username, uuid)),
        }
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
