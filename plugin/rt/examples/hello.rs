use veloren_plugin_rt::{
    api::{event::*, Action, GameMode},
    *,
};

#[event_handler]
pub fn on_load(load: PluginLoadEvent) {
    match load.game_mode {
        GameMode::Server => emit_action(Action::Print("Hello, server!".to_owned())),
        GameMode::Client => emit_action(Action::Print("Hello, client!".to_owned())),
        GameMode::Singleplayer => emit_action(Action::Print("Hello, singleplayer!".to_owned())),
    }
}

#[event_handler]
pub fn on_command_testplugin(command: ChatCommandEvent) -> Result<Vec<String>, String> {
    Ok(vec![format!(
        "Player of id {:?} named {} with {:?} sended command with args {:?}",
        command.player.id,
        command
            .player
            .get_player_name()
            .expect("Can't get player name"),
        command
            .player
            .get_entity_health()
            .expect("Can't get player health"),
        command.command_args
    )])
}

#[global_state]
#[derive(Default)]
struct State {
    counter: bool,
}

#[event_handler]
pub fn on_join(input: PlayerJoinEvent, state: &mut State) -> PlayerJoinResult {
    state.counter = !state.counter;
    if !state.counter {
        PlayerJoinResult::Kick(format!("You are a cheater {:?}", input))
    } else {
        PlayerJoinResult::None
    }
}
