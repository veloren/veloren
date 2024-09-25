mod bindings;

use bindings::{
    exports::veloren::plugin::events::Guest,
    veloren::plugin::{
        actions,
        information::Entity,
        types::{self, GameMode, Health, JoinResult, PlayerId, Uid},
    },
};
use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
struct Component {}

static COUNTER: AtomicBool = AtomicBool::new(false);

impl Guest for Component {
    fn load(mode: GameMode) {
        actions::register_command("test");
        match mode {
            GameMode::Server => println!("Hello, server!"),
            GameMode::Client => println!("Hello, client!"),
            GameMode::SinglePlayer => println!("Hello, singleplayer!"),
        }
    }
}

impl bindings::exports::veloren::plugin::server_events::Guest for Component {
    fn join(player_name: String, player_id: PlayerId) -> JoinResult {
        if COUNTER.fetch_not(Ordering::SeqCst) {
            JoinResult::Kick(format!("Rejected user {player_name}, id {player_id:?}"))
        } else {
            JoinResult::None
        }
    }

    fn command(
        command: String,
        command_args: Vec<String>,
        player: Uid,
    ) -> Result<Vec<String>, String> {
        let entity: Result<Entity, types::Error> = Entity::find_entity(player);
        let health = entity
            .as_ref()
            .map_err(|err| *err)
            .and_then(|entity| entity.health())
            .unwrap_or(Health {
                base_max: 0.0,
                maximum: 0.0,
                current: 0.0,
            });
        Ok(vec![format!(
            "Player id {player:?} name {} with {health:?} command {command} args {command_args:?}",
            entity.and_then(|e| e.name()).unwrap_or_default(),
        )])
    }
}

bindings::export!(Component with_types_in bindings);
