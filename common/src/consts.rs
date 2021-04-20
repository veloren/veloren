// The limit on distance between the entity and a collectible (squared)
pub const MAX_PICKUP_RANGE: f32 = 8.0;
pub const MAX_MOUNT_RANGE: f32 = 14.0;

pub const GRAVITY: f32 = 25.0;
pub const FRIC_GROUND: f32 = 0.15;

// Values for air taken from http://www-mdp.eng.cam.ac.uk/web/library/enginfo/aerothermal_dvd_only/aero/atmos/atmos.html
// Values below are for dry air at 15°C, sea level, 1 standard atmosphere

// pub const ATMOSPHERE: f32 = 101325.0; // Pa

// kg/m³
pub const AIR_DENSITY: f32 = 1.225;
pub const WATER_DENSITY: f32 = 999.1026;
pub const IRON_DENSITY: f32 = 7870.0;
// pub const HUMAN_DENSITY: f32 = 1010.0; // real value
pub const HUMAN_DENSITY: f32 = 990.0; // value we use to make humanoids gently float
