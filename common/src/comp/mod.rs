pub mod actor;
pub mod agent;
pub mod animation;
pub mod inputs;
pub mod phys;
pub mod player;
pub mod stats;

// Reexports
pub use actor::Actor;
pub use actor::Body;
pub use actor::HumanoidBody;
pub use actor::QuadrupedBody;
pub use agent::Agent;
pub use animation::Animation;
pub use animation::AnimationInfo;
pub use inputs::Actions;
pub use inputs::InputEvent;
pub use inputs::Inputs;
pub use player::Player;
pub use player::Respawn;
pub use stats::Dying;
pub use stats::Stats;
