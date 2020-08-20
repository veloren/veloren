pub struct Config {
    pub sea_level: f32,
    pub mountain_scale: f32,
    pub snow_temp: f32,
    pub temperate_temp: f32,
    pub tropical_temp: f32,
    pub desert_temp: f32,
    pub desert_hum: f32,
    pub forest_hum: f32,
    pub jungle_hum: f32,
    /// Rainfall (in meters) per m² of surface per minute.  Default is set to
    /// make it approximately 1 m rainfall / year uniformly across the whole
    /// land area, which is the average rainfall on Earth.
    pub rainfall_chunk_rate: f32,
    /// Roughness coefficient is an empirical value that controls the rate of
    /// energy loss of water in a river.  The higher it is, the more water
    /// slows down as it flows downhill, which consequently leads to lower
    /// velocities and higher river area for the same flow rate.
    ///
    /// See https://wwwrcamnl.wr.usgs.gov/sws/fieldmethods/Indirects/nvalues/index.htm.
    ///
    /// The default is set to over 0.06, which is pretty high but still within a
    /// reasonable range for rivers.  The higher this is, the quicker rivers
    /// appear, and since we often will have high slopes we want to give
    /// rivers as much of a chance as possible.  In the future we can set
    /// this dynamically.
    ///
    /// NOTE: The values in the link are in seconds / (m^(-1/3)), but we use
    /// them without conversion as though they are in minutes / (m^(-1/3)).
    /// The idea here is that our clock speed has time go by at
    /// approximately 1 minute per second, but since velocity depends on
    /// this parameter, we want flow rates to still "look" natural at the second
    /// level.  The way we are cheating is that we still allow the refill
    /// rate (via rainfall) of rivers and lakes to be specified as though
    /// minutes are *really* minutes.  This reduces the amount of water
    /// needed to form a river of a given area by 60, but hopefully this should
    /// not feel too unnatural since the refill rate is still below what
    /// people should be able to perceive.
    pub river_roughness: f32,
    /// Maximum width of rivers, in terms of a multiple of the horizontal chunk
    /// size.
    ///
    /// Currently, not known whether setting this above 1.0 will work properly.
    /// Please use with care!
    pub river_max_width: f32,
    /// Minimum height at which rivers display.
    pub river_min_height: f32,
    /// Rough desired river width-to-depth ratio (in terms of horizontal chunk
    /// width / m, for some reason).  Not exact.
    pub river_width_to_depth: f32,
}

pub const CONFIG: Config = Config {
    sea_level: 140.0,
    mountain_scale: 2048.0,
    snow_temp: -0.8,
    temperate_temp: -0.4,
    tropical_temp: 0.4,
    desert_temp: 0.8,
    desert_hum: 0.15,
    forest_hum: 0.5,
    jungle_hum: 0.85,
    rainfall_chunk_rate: 1.0 / (512.0 * 32.0 * 32.0),
    river_roughness: 0.06125,
    river_max_width: 2.0,
    river_min_height: 0.25,
    river_width_to_depth: 8.0,
};
