// The limit on distance between the entity and a collectible (squared)
pub const MAX_PICKUP_RANGE: f32 = 5.0;
pub const MAX_MOUNT_RANGE: f32 = 5.0;
pub const MAX_SPRITE_MOUNT_RANGE: f32 = 2.0;
pub const MAX_TRADE_RANGE: f32 = 5.0;
pub const MAX_NPCINTERACT_RANGE: f32 = 8.0;
pub const MAX_INTERACT_RANGE: f32 = 5.0;
pub const MAX_WAYPOINT_RANGE: f32 = 4.0;
// Player-imperceptible offset to ensure campfire healing is always
// within waypoint range (may not be necessary if floating point handling is
// reliable)
pub const MAX_CAMPFIRE_RANGE: f32 = MAX_WAYPOINT_RANGE - 0.001;

pub const GRAVITY: f32 = 25.0;
pub const FRIC_GROUND: f32 = 0.15;

// Values for air taken from http://www-mdp.eng.cam.ac.uk/web/library/enginfo/aerothermal_dvd_only/aero/atmos/atmos.html
// Values below are for dry air at 15°C, sea level, 1 standard atmosphere

// pub const ATMOSPHERE: f32 = 101325.0; // Pa

// kg/m³
pub const AIR_DENSITY: f32 = 1.225;
pub const WATER_DENSITY: f32 = 999.1026;
// LAVA_DENSITY is unsourced, estimated as "roughly three times higher" than
// water
pub const LAVA_DENSITY: f32 = 3000.0;
pub const IRON_DENSITY: f32 = 7870.0;
// pub const HUMAN_DENSITY: f32 = 1010.0; // real value
pub const HUMAN_DENSITY: f32 = 990.0; // value we use to make humanoids gently float
// 1 thread might be used for long-running cpu intensive tasks, like chunk
// generation. having at least 2 helps not blocking in the main tick here

pub const MIN_RECOMMENDED_RAYON_THREADS: usize = 2;
pub const MIN_RECOMMENDED_TOKIO_THREADS: usize = 2;

pub const SOUND_TRAVEL_DIST_PER_VOLUME: f32 = 3.0;

pub const TELEPORTER_RADIUS: f32 = 3.;

// Map settings
pub const DAY_LENGTH_DEFAULT: f64 = 30.0;
