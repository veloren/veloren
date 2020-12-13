use serde::{Deserialize, Serialize};

/// A resource that stores the time of day.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeOfDay(pub f64);

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Time(pub f64);

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

/// A resource that indicates what mode the local game is being played in.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameMode {
    /// The game is being played in server mode (i.e: the code is running
    /// server-side)
    Server,
    /// The game is being played in client mode (i.e: the code is running
    /// client-side)
    Client,
    /// The game is being played in singleplayer mode (i.e: both client and
    /// server at once)
    // To be used later when we no longer start up an entirely new server for singleplayer
    Singleplayer,
}
