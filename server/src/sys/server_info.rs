use common_ecs::{Origin, Phase, System};
use lazy_static::lazy_static;
use specs::{Read, ReadStorage};
use veloren_query_server::proto::ServerInfo;

use crate::{client::Client, Settings, Tick};

// Update the server stats every 60 ticks
const INFO_SEND_INTERVAL: u64 = 60;

lazy_static! {
    pub static ref GIT_HASH: [char; 10] = common::util::GIT_HASH[..10]
        .chars()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap_or_default();
}

#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, Tick>,
        Read<'a, Settings>,
        Read<'a, Option<tokio::sync::watch::Sender<ServerInfo>>>,
        ReadStorage<'a, Client>,
    );

    const NAME: &'static str = "server_info";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(_job: &mut common_ecs::Job<Self>, (tick, settings, sender, clients): Self::SystemData) {
        if let Some(sender) = sender.as_ref()
            && tick.0 % INFO_SEND_INTERVAL == 0
        {
            let count = clients.count().try_into().unwrap_or(u16::MAX);
            _ = sender.send(ServerInfo {
                git_hash: *GIT_HASH,
                players_count: count,
                player_cap: settings.max_players,
                battlemode: settings.gameplay.battle_mode.into(),
            });
        }
    }
}
