use crate::comp;

/// An effect that may be applied to an entity
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    Health(i32, comp::HealthSource),
    Xp(i64),
}
