use common::{
    comp::Player,
    util::{GIT_HASH, GIT_TIMESTAMP},
};
use common_ecs::{Origin, Phase, System};
use specs::{Join, Read, ReadStorage};
use tracing::warn;
use veloren_query_server::proto::ServerInfo;

use crate::{Settings, Tick, client::Client};

// Update the server stats every 60 ticks
const INFO_SEND_INTERVAL: u64 = 60;

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Tick>,
        Read<'a, Settings>,
        Option<Read<'a, tokio::sync::watch::Sender<ServerInfo>>>,
        ReadStorage<'a, Player>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "server_info";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut common_ecs::Job<Self>,
        (tick, settings, sender, players, clients): Self::SystemData,
    ) {
        if let Some(sender) = sender.as_ref()
            && tick.0 % INFO_SEND_INTERVAL == 0
        {
            let count = (&players, &clients)
                .join()
                // Hide silent spectators from the player count
                .filter(|(_, client)| client.client_type.emit_login_events())
                .count()
                .try_into()
                .unwrap_or(u16::MAX);
            if let Err(e) = sender.send(ServerInfo {
                git_hash: *GIT_HASH,
                git_timestamp: *GIT_TIMESTAMP,
                players_count: count,
                player_cap: settings.max_players,
                battlemode: settings.gameplay.battle_mode.into(),
            }) {
                warn!(?e, "Failed to send server info to the query server");
            }
        }
    }
}
