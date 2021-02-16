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
        "Player of id {:?} named {} sended command with args {:?}",
        command.player, command.player.get_entity_name(), command.command_args
    )])
}

#[event_handler]
pub fn on_player_join(input: PlayerJoinEvent) -> PlayerJoinResult {
    emit_action(Action::PlayerSendMessage(
        input.player_id,
        format!("Welcome {} on our server", input.player_name),
    ));
    if input.player_name == "Cheater123" {
        PlayerJoinResult::CloseConnection
    } else {
        PlayerJoinResult::None
    }
}
