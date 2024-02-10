#![feature(atomic_bool_fetch_not)]

mod bindings;

use bindings::{
    exports::veloren::plugin::events::Guest,
    veloren::plugin::{
        actions,
        information::Entity,
        types::{GameMode, Health, JoinResult, PlayerId, Uid},
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

    fn join(player_name: wit_bindgen::rt::string::String, player_id: PlayerId) -> JoinResult {
        if COUNTER.fetch_not(Ordering::SeqCst) {
            JoinResult::Kick(format!("Rejected user {player_name}, id {player_id:?}"))
        } else {
            JoinResult::None
        }
    }

    fn command(
        command: wit_bindgen::rt::string::String,
        command_args: wit_bindgen::rt::vec::Vec<wit_bindgen::rt::string::String>,
        player: Uid,
    ) -> Result<Vec<String>, String> {
        let entity: Result<Entity, ()> = Entity::find_entity(player);
        let health = entity.as_ref().map(|e| e.health()).unwrap_or(Health {
            base_max: 0.0,
            maximum: 0.0,
            current: 0.0,
        });
        Ok(vec![format!(
            "Player id {player:?} name {} with {health:?} command {command} args {command_args:?}",
            entity.map(|e| e.name()).unwrap_or_default(),
        )])
    }
}
