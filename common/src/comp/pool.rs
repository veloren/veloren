use crate::{
    combat::Attack,
    resources::{Secs, Time},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::Component;


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoolProperties {
    pub attack: Attack,
    /// Radius of the AOE
    pub radius: f32,
    /// How often the pool applies its attack
    pub tick_dur: Secs,
    /// Total lifespan of the pool before it despawns
    pub duration: Secs,
}

/// A lingering area-of-effect hazard.  Placed at impact point by
/// projectiles with Effect::BecomePool  
//  Each tick it performs a line-of-sight check against entities in range
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pool {
    pub properties: PoolProperties,
    pub start_time: Time,
    pub last_tick: Time,
    pub owner: Option<Uid>,
}

impl Component for Pool {
    type Storage = specs::DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}
