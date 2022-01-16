// The limit on distance between the entity and a collectible (squared)
pub const MAX_PICKUP_RANGE: f32 = 5.0;
pub const MAX_MOUNT_RANGE: f32 = 5.0;
pub const MAX_TRADE_RANGE: f32 = 20.0;

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

// Stat increase per level (multiplied by 10 compared to what you'll see in UI)
pub const ENERGY_PER_LEVEL: u16 = 5;
pub const HP_PER_LEVEL: u16 = 5;
