extern crate plugin_rt;

use plugin_rt::*;
use plugin_rt::api::{Action, event::*};

#[event_handler]
pub fn on_load(load: PluginLoadEvent) -> () {
    emit_action(Action::Print("This is a test".to_owned()));
    println!("Hello world");
}

#[event_handler]
pub fn on_command_testplugin(command: ChatCommandEvent) -> Result<Vec<String>, String> {
    Ok(vec![format!("Player of id {:?} sended command with args {:?}",command.player,command.command_args)])
}

#[event_handler]
pub fn on_player_join(input: PlayerJoinEvent) -> PlayerJoinResult {
    emit_action(Action::PlayerSendMessage(input.player_id,format!("Welcome {} on our server",input.player_name)));
    if input.player_name == "Cheater123" {
        PlayerJoinResult::CloseConnection
    } else {
        PlayerJoinResult::None
    }
}
