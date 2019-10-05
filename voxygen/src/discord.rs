use crate::DISCORD_INSTANCE;
use crossbeam::channel::{unbounded, Sender};
use discord_rpc_sdk::{DiscordUser, EventHandlers, RichPresence, RPC};
//use parking_lot::Mutex;
use std::sync::Mutex;
use std::thread;
use std::thread::JoinHandle;
use std::time::SystemTime;

/// Connects to the discord application where Images and more resides
/// can be viewed at https://discordapp.com/developers/applications/583662036194689035/rich-presence/assets
/// but requires an invitation.
const DISCORD_APPLICATION_ID: &str = "583662036194689035";

/// Represents an update of the game which should be reflected in Discord
pub enum DiscordUpdate {
    Details(String),
    State(String),
    SmallImg(String),
    SmallImgDesc(String),
    LargeImg(String),
    LargeImgDesc(String),
    Shutdown,
}

pub struct DiscordState {
    pub tx: Sender<DiscordUpdate>,
    pub thread: Option<JoinHandle<()>>,
}

pub fn run() -> Mutex<DiscordState> {
    let (tx, rx) = unbounded();

    Mutex::new(DiscordState {
        tx,
        thread: Some(thread::spawn(move || {
            let rpc: RPC = match RPC::init::<Handlers>(DISCORD_APPLICATION_ID, true, None) {
                Ok(rpc) => rpc,
                Err(e) => {
                    log::error!("failed to initiate discord_game_sdk: {}", e);
                    return;
                }
            };

            //Set initial Status
            let mut current_presence = RichPresence {
                details: Some("In Menu".to_string()),
                state: Some("Idling".to_string()),
                start_time: Some(SystemTime::now()),
                //end_time: Some(SystemTime::now().checked_add(Duration::from_secs(360)).unwrap()),
                large_image_key: Some("snow_background".to_string()),
                large_image_text: Some("Veloren.net".to_string()),
                small_image_key: Some("veloren_logo_1024".to_string()),
                small_image_text: Some("*insert joke here*".to_string()),
                //party_id: Some("randompartyid".to_owned()),
                //party_size: Some(4),
                //party_max: Some(13),
                //spectate_secret: Some("randomhash".to_string()),
                //join_secret: Some("anotherrandomhash".to_string()),
                ..Default::default()
            };

            match rpc.update_presence(current_presence.clone()) {
                Ok(_) => {}
                Err(e) => log::error!("Failed to update discord presence: {}", e),
            }

            'outer: loop {
                rpc.run_callbacks();

                let msg = rx.try_recv();
                match msg {
                    Ok(update) => {
                        match update {
                            DiscordUpdate::Details(x) => current_presence.details = Some(x),
                            DiscordUpdate::State(x) => current_presence.state = Some(x),
                            DiscordUpdate::SmallImg(x) => {
                                current_presence.small_image_key = Some(x)
                            }
                            DiscordUpdate::SmallImgDesc(x) => {
                                current_presence.small_image_text = Some(x)
                            }
                            DiscordUpdate::LargeImg(x) => {
                                current_presence.large_image_key = Some(x)
                            }
                            DiscordUpdate::LargeImgDesc(x) => {
                                current_presence.large_image_text = Some(x)
                            }
                            DiscordUpdate::Shutdown => break 'outer,
                        };

                        match rpc.update_presence(current_presence.clone()) {
                            Ok(_) => {}
                            Err(e) => log::error!("Failed to update discord presence: {}", e),
                        }
                    }
                    Err(_) => {}
                }
            }

            rpc.clear_presence();
        })),
    })
}

struct Handlers;

impl EventHandlers for Handlers {
    fn ready(user: DiscordUser) {
        log::debug!("We're ready! {:?}", user);
    }

    fn errored(errcode: i32, message: &str) {
        log::warn!("Error {}: {}", errcode, message);
    }

    fn disconnected(errcode: i32, message: &str) {
        log::debug!("Disconnected {}: {}", errcode, message);
    }

    fn join_game(secret: &str) {
        log::debug!("Joining {}", secret);
    }

    fn spectate_game(secret: &str) {
        log::debug!("Spectating {}", secret);
    }

    fn join_request(from: DiscordUser) {
        log::debug!("Join request from {:?}", from);
    }
}

pub fn send_all(updates: Vec<DiscordUpdate>) {
    match DISCORD_INSTANCE.lock() {
        Ok(disc) => {
            for update in updates {
                let _ = disc.tx.send(update);
            }
        }
        Err(e) => log::error!("couldn't send Update to discord: {}", e),
    }
}
