pub mod actor;
mod agent;
mod animation;
mod controller;
mod inputs;
mod physics;
mod player;
mod stats;

// Reexports
pub use actor::{Actor, Body, HumanoidBody, QuadrupedBody, QuadrupedMediumBody};
pub use agent::Agent;
pub use animation::{Animation, AnimationInfo};
pub use controller::Controller;
pub use inputs::{Attacking, Gliding, Jumping, MoveDir, OnGround, Respawning, Rolling};
pub use physics::{Acceleration, ForceUpdate, Ori, Pos, Position, Vel, Velocity};
pub use player::Player;
pub use stats::{Dying, HealthSource, Stats};
