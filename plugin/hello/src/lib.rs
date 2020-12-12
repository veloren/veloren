extern crate plugin_rt;

use plugin_rt::*;
use plugin_rt::api::{Action, events::*};

#[event_handler]
pub fn on_load(load: PluginLoadEvent) -> () {
    send_actions(vec![Action::Print("This is a test".to_owned())]);
    println!("Hello world");
}

#[event_handler]
pub fn on_command_testplugin(command: ChatCommandEvent) -> Result<Vec<String>, String> {
    Ok(vec![format!("Player of id {:?} sended command with args {:?}",command.player,command.command_args)])
}

#[event_handler]
pub fn on_player_join(input: PlayerJoinEvent) -> PlayerJoinResult {
    send_actions(vec![Action::PlayerSendMessage(input.player_id,format!("Welcome {} on our server",input.player_name))]);
    if input.player_name == "Cheater123" {
        PlayerJoinResult::CloseConnection
    } else {
        PlayerJoinResult::None
    }
}
