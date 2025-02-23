use common::{
    comp::Pos,
    resources::TimeOfDay,
    terrain::{CoordinateConversions, NEIGHBOR_DELTA, SiteKindMeta, TerrainGrid},
    weather::WeatherGrid,
};
use common_base::{self, prof_span};
use itertools::Itertools;
use vek::*;

/// Simulates winds based on weather and terrain data for specific position
// TODO: Consider exporting it if one wants to build nice visuals
pub(super) fn simulated_wind_vel(
    pos: &Pos,
    weather: &WeatherGrid,
    terrain: &TerrainGrid,
    time_of_day: &TimeOfDay,
) -> Result<Vec3<f32>, ()> {
    prof_span!(guard, "Apply Weather INIT");

    let pos_2d = pos.0.as_().xy();
    let chunk_pos: Vec2<i32> = pos_2d.wpos_to_cpos();
    let Some(current_chunk) = terrain.get_key(chunk_pos) else {
        return Err(());
    };

    let meta = current_chunk.meta();

    let interp_weather = weather.get_interpolated(pos.0.xy());
    // Weather sim wind
    let interp_alt = terrain
        .get_interpolated(pos_2d, |c| c.meta().alt())
        .unwrap_or(0.);
    let interp_tree_density = terrain
        .get_interpolated(pos_2d, |c| c.meta().tree_density())
        .unwrap_or(0.);
    let interp_town = terrain
        .get_interpolated(pos_2d, |c| match c.meta().site() {
            Some(SiteKindMeta::Settlement(_)) => 2.7,
            _ => 1.0,
        })
        .unwrap_or(0.);
    let normal = terrain
        .get_interpolated(pos_2d, |c| {
            c.meta()
                .approx_chunk_terrain_normal()
                .unwrap_or(Vec3::unit_z())
        })
        .unwrap_or(Vec3::unit_z());
    let above_ground = pos.0.z - interp_alt;
    let wind_velocity = interp_weather.wind_vel();

    let surrounding_chunks_metas = NEIGHBOR_DELTA
        .iter()
        .map(move |&(x, y)| chunk_pos + Vec2::new(x, y))
        .filter_map(|cpos| terrain.get_key(cpos).map(|c| c.meta()))
        .collect::<Vec<_>>();

    drop(guard);

    prof_span!(guard, "thermals");

    // === THERMALS ===

    // Sun angle of incidence.
    //
    // 0.0..1.0, 0.25 morning, 0.45 midday, 0.66 evening, 0.79 night, 0.0/1.0
    // midnight
    let sun_dir = time_of_day.get_sun_dir().normalized();
    let mut lift = ((sun_dir - normal.normalized()).magnitude() - 0.5).max(0.2) * 2.3;

    // TODO: potential source of harsh edges in wind speed.
    let temperatures = surrounding_chunks_metas.iter().map(|m| m.temp()).minmax();

    // More thermals if hot chunks border cold chunks
    lift *= match temperatures {
        itertools::MinMaxResult::NoElements | itertools::MinMaxResult::OneElement(_) => 1.0,
        itertools::MinMaxResult::MinMax(a, b) => 0.8 + ((a - b).abs() * 1.1),
    }
    .min(2.0);

    // TODO: potential source of harsh edges in wind speed.
    //
    // Way more thermals in strong rain as its often caused by strong thermals.
    // Less in weak rain or cloudy ..
    lift *= if interp_weather.rain.is_between(0.5, 1.0) && interp_weather.cloud.is_between(0.6, 1.0)
    {
        1.5
    } else if interp_weather.rain.is_between(0.2, 0.5) && interp_weather.cloud.is_between(0.3, 0.6)
    {
        0.8
    } else {
        1.0
    };

    // The first 15 blocks are weaker. Starting from the ground should be difficult.
    lift *= (above_ground / 15.).min(1.);
    lift *= (220. - above_ground / 20.).clamp(0.0, 1.0);

    // TODO: Smooth this, and increase height some more (500 isnt that much higher
    // than the spires)
    if interp_alt > 500.0 {
        lift *= 0.8;
    }

    // More thermals above towns, the materials tend to heat up more.
    lift *= interp_town;

    // Bodies of water cool the air, causing less thermals.
    lift *= terrain
        .get_interpolated(pos_2d, |c| 1. - c.meta().near_water() as i32 as f32)
        .unwrap_or(1.);

    drop(guard);

    // === Ridge/Wave lift ===

    let mut ridge_lift = {
        const RIDGE_LIFT_COEFF: f32 = 1.0;

        let steepness = normal.angle_between(Vec3::unit_z());

        // angle between normal and wind
        let mut angle = wind_velocity.angle_between(normal.xy()); // 1.4 radians of zero

        // a deadzone of +-1.5 radians if wind is blowing away from
        // the mountainside.
        angle = (angle - 1.3).max(0.0);

        // the ridge lift is based on the angle and the velocity of the wind
        angle * steepness * wind_velocity.magnitude() * RIDGE_LIFT_COEFF
    };

    // Cliffs mean more lift
    // 44 seems to be max, according to a lerp in WorldSim::generate_cliffs
    ridge_lift *= 0.9 + (meta.cliff_height() / 44.0) * 1.2;

    // Height based fall-off (https://www.desmos.com/calculator/jijqfunchg)
    ridge_lift *= 1. / (1. + (1.3f32.powf(0.1 * above_ground - 15.)));

    // More flat wind above ground (https://www.desmos.com/calculator/jryiyqsdnx)
    let wind_factor = 1. / (0.25 + (0.96f32.powf(0.1 * above_ground - 15.)));

    let mut wind_vel = (wind_velocity * wind_factor).with_z(lift + ridge_lift);

    // probably 0. to 1. src: SiteKind::is_suitable_loc comparisons
    wind_vel *= (1.0 - interp_tree_density).max(0.7);

    // Clamp magnitude, we never want to throw players around way too fast.
    let magn = wind_vel.magnitude_squared().max(0.0001);

    // 600 here is compared to squared ~ 25. this limits the magnitude of the wind.
    wind_vel *= magn.min(600.) / magn;

    Ok(wind_vel)
}
