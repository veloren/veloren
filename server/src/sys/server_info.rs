use common::{comp::Player, util::GIT_DATE_TIMESTAMP};
use common_ecs::{Origin, Phase, System};
use lazy_static::lazy_static;
use specs::{Read, ReadStorage};
use tracing::error;
use veloren_query_server::proto::ServerInfo;

use crate::{Settings, Tick};

// Update the server stats every 60 ticks
const INFO_SEND_INTERVAL: u64 = 60;

lazy_static! {
    pub static ref GIT_HASH: u32 =
        u32::from_str_radix(&common::util::GIT_HASH[..8], 16).expect("Invalid git hash");
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Tick>,
        Read<'a, Settings>,
        Option<Read<'a, tokio::sync::watch::Sender<ServerInfo>>>,
        ReadStorage<'a, Player>,
    );

    const NAME: &'static str = "server_info";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut common_ecs::Job<Self>, (tick, settings, sender, players): Self::SystemData) {
        if let Some(sender) = sender.as_ref()
            && tick.0 % INFO_SEND_INTERVAL == 0
        {
            let count = players.count().try_into().unwrap_or(u16::MAX);
            if let Err(error) = sender.send(ServerInfo {
                git_hash: *GIT_HASH,
                git_timestamp: *GIT_DATE_TIMESTAMP,
                players_count: count,
                player_cap: settings.max_players,
                battlemode: settings.gameplay.battle_mode.into(),
            }) {
                error!(?error, "Failed to send server info to the query server");
            }
        }
    }
}
