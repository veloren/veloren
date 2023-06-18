use crate::comp::Pos;
use serde::{Deserialize, Serialize};
use specs::Entity;
use std::ops::{Mul, MulAssign};
use vek::Vec3;

/// A resource that stores the time of day.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Default)]
pub struct TimeOfDay(pub f64);
impl TimeOfDay {
    pub fn new(t: f64) -> Self { TimeOfDay(t) }

    fn get_angle_rad(self) -> f32 {
        const TIME_FACTOR: f64 = (std::f64::consts::PI * 2.0) / (3600.0 * 24.0);
        ((self.0 as f32 * TIME_FACTOR) % (std::f64::consts::PI * 2.0)) as f32
    }

    /// Computes the direction of light from the sun based on the time of day.
    pub fn get_sun_dir(self) -> Vec3<f32> {
        let angle_rad = self.get_angle_rad();
        Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos())
    }

    /// Computes the direction of light from the moon based on the time of day.
    pub fn get_moon_dir(self) -> Vec3<f32> {
        let angle_rad = self.get_angle_rad();
        -Vec3::new(-angle_rad.sin(), 0.0, angle_rad.cos() - 0.5).normalized()
    }
}

impl TimeOfDay {
    pub fn day(&self) -> f64 { self.0.rem_euclid(24.0 * 3600.0) }
}

/// A resource that stores the tick (i.e: physics) time.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Time(pub f64);

/// A resource that stores the real tick, local to the server/client.
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ProgramTime(pub f64);

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct TimeScale(pub f64);

impl Default for TimeScale {
    fn default() -> Self { Self(1.0) }
}

/// A resource that stores the time since the previous tick.
#[derive(Default)]
pub struct DeltaTime(pub f32);

/// A resource used to indicate a duration of time, in seconds
#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Secs(pub f64);

impl Mul<f64> for Secs {
    type Output = Self;

    fn mul(self, mult: f64) -> Self { Self(self.0 * mult) }
}
impl MulAssign<f64> for Secs {
    fn mul_assign(&mut self, mult: f64) { *self = *self * mult; }
}

#[derive(Default)]
pub struct EntitiesDiedLastTick(pub Vec<(Entity, Pos)>);

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

/// A resource that stores the player's entity (on the client), and None on the
/// server
#[derive(Copy, Clone, Default, Debug)]
pub struct PlayerEntity(pub Option<Entity>);

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct PlayerPhysicsSetting {
    /// true if the client wants server-authoratative physics (e.g. to use
    /// airships properly)
    pub client_optin: bool,
    /// true if the server is forcing server-authoratative physics (e.g. as
    /// punishment for wallhacking)
    pub server_force: bool,
}

impl PlayerPhysicsSetting {
    pub fn server_authoritative(&self) -> bool { self.client_optin || self.server_force }

    pub fn client_authoritative(&self) -> bool { !self.server_authoritative() }
}

/// Describe how the map should be generated.
#[derive(Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, enum_map::Enum)]
pub enum MapKind {
    /// The normal square map, with oceans beyond the map edge
    Square,
    /// A more circular map, might have more islands
    Circle,
}

impl std::fmt::Display for MapKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapKind::Square => f.write_str("Square"),
            MapKind::Circle => f.write_str("Circle"),
        }
    }
}

/// List of which players are using client-authoratative vs server-authoratative
/// physics, as a stop-gap until we can use server-authoratative physics for
/// everyone
#[derive(Clone, Default, Debug)]
pub struct PlayerPhysicsSettings {
    pub settings: hashbrown::HashMap<uuid::Uuid, PlayerPhysicsSetting>,
}

/// Describe how players interact with other players.
///
/// May be removed when we will discover better way
/// to handle duels and murders
#[derive(PartialEq, Eq, Copy, Clone, Debug, Deserialize, Serialize)]
pub enum BattleMode {
    PvP,
    PvE,
}
