use discord_game_sdk::{Activity, Discord};
use std::ffi::CString;
use std::sync::mpsc::Sender;
use std::sync::{mpsc, Mutex, MutexGuard};
use std::thread;
use std::thread::JoinHandle;

use crate::DEFAULT_PUBLIC_SERVER;
use chrono::Utc;

/// Connects to the discord application where Images and more resides
/// can be viewed at https://discordapp.com/developers/applications/583662036194689035/rich-presence/assets
/// but requires an invitation.
const DISCORD_APPLICATION_ID: i64 = 583662036194689035;

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
    let (tx, rx) = mpsc::channel();

    Mutex::new(DiscordState {
        tx,
        thread: Some(thread::spawn(move || {
            let mut discord =
                Discord::new(DISCORD_APPLICATION_ID).expect("failed to initiate discord_game_sdk");

            //Set initial Status
            let mut current_activity = Activity::empty();
            current_activity.with_details(CString::new("Menu").expect("failed to create CString"));
            current_activity.with_state(CString::new("Idling").expect("failed to create CString"));
            current_activity.with_small_image_key(
                CString::new("veloren_logo_squareicon_2000").expect("failed to create CString"),
            );
            current_activity.with_small_image_tooltip(
                CString::new("Veloren").expect("failed to create CString"),
            );
            current_activity
                .with_large_image_key(CString::new("bg_main").expect("failed to create CString"));
            current_activity.with_large_image_tooltip(
                CString::new("veloren.net").expect("failed to create CString"),
            );

            current_activity.with_start_time(Utc::now());

            discord.update_activity(&current_activity, |_, _| {});

            'outer: loop {
                discord.empty_event_receivers();
                discord.run_callbacks();

                let msg = rx.try_recv();
                match msg {
                    Ok(update) => {
                        match update {
                            DiscordUpdate::Details(x) => current_activity
                                .with_details(CString::new(x).expect("failed to create CString")),
                            DiscordUpdate::State(x) => current_activity
                                .with_state(CString::new(x).expect("failed to create CString")),
                            DiscordUpdate::SmallImg(x) => current_activity.with_small_image_key(
                                CString::new(x).expect("failed to create CString"),
                            ),
                            DiscordUpdate::SmallImgDesc(x) => current_activity
                                .with_small_image_tooltip(
                                    CString::new(x).expect("failed to create CString"),
                                ),
                            DiscordUpdate::LargeImg(x) => current_activity.with_large_image_key(
                                CString::new(x).expect("failed to create CString"),
                            ),
                            DiscordUpdate::LargeImgDesc(x) => current_activity
                                .with_large_image_tooltip(
                                    CString::new(x).expect("failed to create CString"),
                                ),
                            DiscordUpdate::Shutdown => break 'outer,
                        };
                        discord.update_activity(&current_activity, |_, _| {});
                    }
                    Err(_) => {}
                }
            }
        })),
    })
}

/* Some helpers */
pub fn send_menu(disc: &mut MutexGuard<DiscordState>) {
    disc.tx.send(DiscordUpdate::Details("Menu".into()));
    disc.tx.send(DiscordUpdate::State("Idling".into()));
    disc.tx.send(DiscordUpdate::LargeImg("bg_main".into()));
}
pub fn send_singleplayer(disc: &mut MutexGuard<DiscordState>) {
    disc.tx.send(DiscordUpdate::Details("Singleplayer".into()));
    disc.tx.send(DiscordUpdate::State("Playing...".into()));
}