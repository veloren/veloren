#[cfg(feature = "airship_log")]
use crate::rule::npc_ai::airship_logger::airship_logger;

use crate::{
    ai::{Action, NpcCtx, State, finish, just, now, seq},
    data::npc::SimulationMode,
};
use common::{
    comp::{
        Content,
        agent::{BrakingMode, FlightMode},
        compass::Direction,
    },
    util::Dir,
};
use rand::prelude::*;
use std::{cmp::Ordering, collections::VecDeque, time::Duration};
use vek::*;
use world::civ::airship_travel::{AirshipDockingApproach, AirshipFlightPhase};

#[cfg(debug_assertions)]
macro_rules! debug_airships {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            0 => tracing::error!($($arg)*),
            1 => tracing::warn!($($arg)*),
            2 => tracing::info!($($arg)*),
            3 => tracing::debug!($($arg)*),
            4 => tracing::trace!($($arg)*),
            _ => tracing::trace!($($arg)*),
        }
    }
}

#[cfg(not(debug_assertions))]
macro_rules! debug_airships {
    ($($arg:tt)*) => {};
}

const AIRSHIP_PROGRESS_UPDATE_INTERVAL: f32 = 5.0; // seconds

/// The context data for the pilot_airship action.
#[derive(Debug, Clone)]
struct AirshipRouteContext {
    /// The route index (index into the outer vec of airships.routes)
    route_index: usize,
    /// The next route leg index.
    current_leg: usize,
    /// True for the first leg (initial startup), false otherwise.
    first_leg: bool,
    /// The current approach.
    current_leg_approach: Option<AirshipDockingApproach>,
    /// The direction override for departure and approach phases.
    cruise_direction: Option<Dir>,
    /// The next route leg approach.
    next_leg_approach: Option<AirshipDockingApproach>,

    // Timing
    /// The context time at the start of the route. All route leg segment
    /// times are measured relative to this time.
    route_time_zero: f64,
    /// Timer used for various periodic countdowns.
    route_timer: Duration,
    /// The context time at the beginning of the current route leg.
    leg_ctx_time_begin: f64,

    // Docking phase
    /// The times at which announcements are made during docking.
    announcements: VecDeque<f32>,

    /// For tracking the airship's position history to determine if the airship
    /// is stuck.
    my_stuck_tracker: Option<StuckAirshipTracker>,
    /// Timer for checking the airship trackers.
    stuck_timer: Duration,
    /// Timer used when holding, either on approach or at the dock.
    stuck_backout_pos: Option<Vec3<f32>>,
}

impl Default for AirshipRouteContext {
    fn default() -> Self {
        Self {
            route_index: usize::MAX,
            current_leg: 0,
            first_leg: true,
            current_leg_approach: None,
            cruise_direction: None,
            next_leg_approach: None,
            route_time_zero: 0.0,
            route_timer: Duration::default(),
            leg_ctx_time_begin: 0.0,
            announcements: VecDeque::new(),
            my_stuck_tracker: None,
            stuck_timer: Duration::default(),
            stuck_backout_pos: None,
        }
    }
}

/// Tracks the airship position history.
/// Used for determining if an airship is stuck.
#[derive(Debug, Default, Clone)]
struct StuckAirshipTracker {
    /// The airship's position history. Used for determining if the airship is
    /// stuck in one place.
    pos_history: Vec<Vec3<f32>>,
    /// The route to follow for backing out of a stuck position.
    backout_route: Vec<Vec3<f32>>,
}

impl StuckAirshipTracker {
    /// The distance to back out from the stuck position.
    const BACKOUT_DIST: f32 = 100.0;
    /// The tolerance for determining if the airship has reached a backout
    /// position.
    const BACKOUT_TARGET_DIST: f64 = 50.0;
    /// The number of positions to track in the position history.
    const MAX_POS_HISTORY_SIZE: usize = 5;
    /// The height for testing if the airship is near the ground.
    const NEAR_GROUND_HEIGHT: f32 = 10.0;

    /// Add a new position to the position history, maintaining a fixed size.
    fn add_position(&mut self, new_pos: Vec3<f32>) {
        if self.pos_history.len() >= StuckAirshipTracker::MAX_POS_HISTORY_SIZE {
            self.pos_history.remove(0);
        }
        self.pos_history.push(new_pos);
    }

    /// Get the current backout position.
    /// If the backout route is not empty, return the first position in the
    /// route. As a side effect, if the airship position is within the
    /// target distance of the first backout position, remove the backout
    /// position. If there are no more backout postions, the position
    /// history is cleared (because it will be stale data), and return None.
    fn current_backout_pos(&mut self, ctx: &mut NpcCtx) -> Option<Vec3<f32>> {
        if !self.backout_route.is_empty()
            && let Some(pos) = self.backout_route.first().cloned()
        {
            if ctx.npc.wpos.as_::<f64>().distance_squared(pos.as_::<f64>())
                < StuckAirshipTracker::BACKOUT_TARGET_DIST.powi(2)
            {
                self.backout_route.remove(0);
            }
            Some(pos)
        } else {
            self.pos_history.clear();
            None
        }
    }

    /// Check if the airship is stuck in one place. This check is done only in
    /// cruise flight when the PID controller is affecting the Z axis
    /// movement only. When the airship gets stuck, it will stop moving. The
    /// only recourse is reverse direction, back up, and then ascend to
    /// hopefully fly over the top of the obstacle. This may be repeated if the
    /// airship gets stuck again. When the determination is made that the
    /// airship is stuck, two positions are generated for the backout
    /// procedure: the first is in the reverse of the direction the airship
    /// was recently moving, and the second is straight up from the first
    /// position. If the airship was near the ground when it got stuck, the
    /// initial backout is done while climbing slightly to avoid any other
    /// near-ground objects. If the airship was not near the ground, the
    /// initial backout position is at the same height as the current position,
    fn is_stuck(
        &mut self,
        ctx: &mut NpcCtx,
        current_pos: &Vec3<f32>,
        target_pos: &Vec2<f32>,
    ) -> bool {
        self.add_position(*current_pos);
        // The position history must be full to determine if the airship is stuck.
        if self.pos_history.len() == StuckAirshipTracker::MAX_POS_HISTORY_SIZE
            && self.backout_route.is_empty()
            && let Some(last_pos) = self.pos_history.last()
        {
            // If all the positions in the history are within 10 of the last position,
            if self
                .pos_history
                .iter()
                .all(|pos| pos.as_::<f64>().distance_squared(last_pos.as_::<f64>()) < 10.0)
            {
                // Airship is stuck on some obstacle.

                // The direction to backout is opposite to the direction from the airship
                // to where it was going before it got stuck.
                if let Some(backout_dir) = (ctx.npc.wpos.xy() - target_pos)
                    .with_z(0.0)
                    .try_normalized()
                {
                    let ground = ctx
                        .world
                        .sim()
                        .get_surface_alt_approx(last_pos.xy().map(|e| e as i32));
                    // The position to backout to is the current position + a distance in the
                    // backout direction.
                    let mut backout_pos =
                        ctx.npc.wpos + backout_dir * StuckAirshipTracker::BACKOUT_DIST;
                    // Add a z offset to the backout pos if the airship is near the ground.
                    if (ctx.npc.wpos.z - ground).abs() < StuckAirshipTracker::NEAR_GROUND_HEIGHT {
                        backout_pos.z += 50.0;
                    }
                    self.backout_route = vec![backout_pos, backout_pos + Vec3::unit_z() * 200.0];
                    // The airship is stuck.
                    #[cfg(debug_assertions)]
                    debug_airships!(
                        2,
                        "Airship {} Stuck! at {} {} {}, backout_dir:{:?}, backout_pos:{:?}",
                        format!("{:?}", ctx.npc_id),
                        ctx.npc.wpos.x,
                        ctx.npc.wpos.y,
                        ctx.npc.wpos.z,
                        backout_dir,
                        backout_pos
                    );
                    self.backout_route = vec![backout_pos, backout_pos + Vec3::unit_z() * 200.0];
                }
            }
        }
        !self.backout_route.is_empty()
    }
}

#[cfg(debug_assertions)]
fn check_phase_completion_time(
    ctx: &mut NpcCtx,
    airship_context: &mut AirshipRouteContext,
    leg_index: usize,
    phase: AirshipFlightPhase,
) {
    let route_leg = &ctx.world.civs().airships.routes[airship_context.route_index].legs
        [airship_context.current_leg];
    // route time = context time - route time zero
    let completion_route_time = ctx.time.0 - airship_context.route_time_zero;
    let time_delta = completion_route_time - route_leg.segments[leg_index].route_time;
    if time_delta > 10.0 {
        debug_airships!(
            4,
            "Airship {} route {} leg {} completed phase {:?} late by {:.1} seconds, crt {}, \
             scheduled end time {}",
            format!("{:?}", ctx.npc_id),
            airship_context.route_index,
            airship_context.current_leg,
            phase,
            time_delta,
            completion_route_time,
            route_leg.segments[leg_index].route_time
        );
    } else if time_delta < -10.0 {
        debug_airships!(
            4,
            "Airship {} route {} leg {} completed phase {:?} early by {:.1} seconds, crt {}, \
             scheduled end time {}",
            format!("{:?}", ctx.npc_id),
            airship_context.route_index,
            airship_context.current_leg,
            phase,
            time_delta,
            completion_route_time,
            route_leg.segments[leg_index].route_time
        );
    } else {
        debug_airships!(
            4,
            "Airship {} route {} leg {} completed phase {:?} on time, crt {}, scheduled end time \
             {}",
            format!("{:?}", ctx.npc_id),
            airship_context.route_index,
            airship_context.current_leg,
            phase,
            completion_route_time,
            route_leg.segments[leg_index].route_time
        );
    }
}

fn fly_airship(
    phase: AirshipFlightPhase,
    approach: AirshipDockingApproach,
) -> impl Action<AirshipRouteContext> {
    now(move |ctx, airship_context: &mut AirshipRouteContext| {
        airship_context.stuck_timer = Duration::from_secs_f32(5.0);
        airship_context.stuck_backout_pos = None;
        let route_leg = &ctx.world.civs().airships.routes[airship_context.route_index].legs
            [airship_context.current_leg];

        ctx.controller.current_airship_pilot_leg = Some((airship_context.current_leg, phase));

        let nominal_speed = ctx.world.civs().airships.nominal_speed;
        let leg_segment = &route_leg.segments[phase as usize];
        // The actual leg start time was recorded when the previous leg ended.
        let leg_ctx_time_begin = airship_context.leg_ctx_time_begin;

        /*
            Duration:
            - starting leg: leg segment route time - spawning position route time
            - all other legs: leg segment duration adjusted for early/late start
            Distance to fly:
            - Cruise phases: from the current pos to the leg segment target pos
            - Descent: two steps, targeting above the dock then to the dock,
              with random variation.
            - Ascent: from the current pos to cruise height above the dock
            - Docked: zero distance
        */

        let fly_duration = if airship_context.first_leg {
            /*
               if this is the very first leg for this airship,
               then the leg duration is the leg segment (end) route time minus the
               spawn route time.

               airship_context.route_time_zero =
                   ctx.time.0 - my_spawn_loc.spawn_route_time;
               Spawn route time = ctx.time - airship_context.route_time_zero
               dur = leg_segment.route_time - spawn_route_time
                   = leg_segment.route_time - (ctx.time.0 - airship_context.route_time_zero)
            */
            debug_airships!(
                4,
                "Airship {} route {} leg {} first leg phase {} from {},{} dur {:.1}s",
                format!("{:?}", ctx.npc_id),
                airship_context.route_index,
                airship_context.current_leg,
                phase,
                ctx.npc.wpos.x,
                ctx.npc.wpos.y,
                leg_segment.route_time - (ctx.time.0 - airship_context.route_time_zero)
            );
            ((leg_segment.route_time - (ctx.time.0 - airship_context.route_time_zero)) as f32)
                .max(1.0)
        } else {
            // Duration is the leg segment duration adjusted for
            // early/late starts caused by time errors in the
            // previous leg.
            let leg_start_route_time =
                airship_context.leg_ctx_time_begin - airship_context.route_time_zero;
            // Adjust the leg duration according to actual route start time vs expected
            // route start time.
            let expected_leg_route_start_time =
                leg_segment.route_time - leg_segment.duration as f64;
            let route_time_err = (leg_start_route_time - expected_leg_route_start_time) as f32;
            if route_time_err > 10.0 {
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} starting {} late by {:.1}s, rt {:.2}, expected rt \
                     {:.2}, leg dur {:.1}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    phase,
                    route_time_err,
                    leg_start_route_time,
                    expected_leg_route_start_time,
                    leg_segment.duration - route_time_err
                );
                leg_segment.duration - route_time_err
            } else if route_time_err < -10.0 {
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} starting {} early by {:.1}s, rt {:.2}, expected \
                     rt {:.2}, leg dur {:.1}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    phase,
                    -route_time_err,
                    leg_start_route_time,
                    expected_leg_route_start_time,
                    leg_segment.duration - route_time_err
                );
                leg_segment.duration - route_time_err
            } else {
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} starting {} on time, leg dur {:.1}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    phase,
                    leg_segment.duration
                );
                leg_segment.duration
            }
            .max(1.0)
        };

        let fly_distance = match phase {
            AirshipFlightPhase::DepartureCruise
            | AirshipFlightPhase::ApproachCruise
            | AirshipFlightPhase::Transition => {
                ctx.npc.wpos.xy().distance(leg_segment.to_world_pos)
            },
            AirshipFlightPhase::Descent => ctx.npc.wpos.z - approach.airship_pos.z,
            AirshipFlightPhase::Ascent => approach.airship_pos.z + approach.height - ctx.npc.wpos.z,
            AirshipFlightPhase::Docked => 0.0,
        };

        let context_end_time = ctx.time.0 + fly_duration as f64;

        match phase {
            AirshipFlightPhase::DepartureCruise => {
                airship_context.my_stuck_tracker = Some(StuckAirshipTracker::default());
                airship_context.route_timer = Duration::from_secs_f32(1.0);
                airship_context.cruise_direction =
                    Dir::from_unnormalized((approach.midpoint - ctx.npc.wpos.xy()).with_z(0.0));
                // v = d/t
                // speed factor = v/nominal_speed = (d/t)/nominal_speed
                let speed = (fly_distance / fly_duration) / nominal_speed;
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} DepartureCruise, fly_distance {:.1}, fly_duration \
                     {:.1}s, speed factor {:.3}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    fly_distance,
                    fly_duration,
                    speed
                );
                // Fly 2D to approach midpoint
                fly_airship_inner(
                    AirshipFlightPhase::DepartureCruise,
                    ctx.npc.wpos,
                    leg_segment.to_world_pos.with_z(0.0),
                    50.0,
                    leg_ctx_time_begin,
                    fly_duration,
                    context_end_time,
                    speed,
                    approach.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                )
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(
                        ctx,
                        airship_context,
                        0,
                        AirshipFlightPhase::DepartureCruise,
                    );
                }))
                .map(|_, _| ())
                .boxed()
            },
            AirshipFlightPhase::ApproachCruise => {
                airship_context.route_timer = Duration::from_secs_f32(1.0);
                airship_context.cruise_direction = Dir::from_unnormalized(
                    (approach.approach_transition_pos - approach.midpoint).with_z(0.0),
                );
                // v = d/t
                // speed factor = v/nominal_speed = (d/t)/nominal_speed
                let speed = (fly_distance / fly_duration) / nominal_speed;
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} ApproachCruise, fly_distance {:.1}, fly_duration \
                     {:.1}s, speed factor {:.3}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    fly_distance,
                    fly_duration,
                    speed
                );
                // Fly 2D to transition point
                fly_airship_inner(
                    AirshipFlightPhase::ApproachCruise,
                    ctx.npc.wpos,
                    leg_segment.to_world_pos.with_z(0.0),
                    50.0,
                    leg_ctx_time_begin,
                    fly_duration,
                    context_end_time,
                    speed,
                    approach.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                )
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(
                        ctx,
                        airship_context,
                        1,
                        AirshipFlightPhase::ApproachCruise,
                    );
                }))
                .map(|_, _| ())
                .boxed()
            },
            AirshipFlightPhase::Transition => {
                // let phase_duration = get_phase_duration(ctx, airship_context, route_leg, 2,
                // phase); let context_end_time = ctx.time.0 + phase_duration;
                airship_context.route_timer = Duration::from_secs_f32(1.0);
                // v = d/t
                // speed factor = v/nominal_speed = (d/t)/nominal_speed
                let speed = (fly_distance / fly_duration) / nominal_speed;
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} Transition, fly_distance {:.1}, fly_duration \
                     {:.1}s, speed factor {:.3}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    fly_distance,
                    fly_duration,
                    speed
                );
                // fly 3D to descent point
                fly_airship_inner(
                    AirshipFlightPhase::Transition,
                    ctx.npc.wpos,
                    leg_segment
                        .to_world_pos
                        .with_z(approach.airship_pos.z + approach.height),
                    20.0,
                    leg_ctx_time_begin,
                    fly_duration,
                    context_end_time,
                    speed,
                    approach.height,
                    true,
                    Some(approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                )
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(
                        ctx,
                        airship_context,
                        2,
                        AirshipFlightPhase::Transition,
                    );
                }))
                .map(|_, _| ())
                .boxed()
            },
            AirshipFlightPhase::Descent => {
                // Descend and Dock
                airship_context.route_timer = Duration::from_secs_f32(1.0);
                // v = d/t
                // speed factor = v/nominal_speed = (d/t)/nominal_speed
                /*
                   Divide the descent into two steps with the 2nd step moving slowly
                   (so more time and less distance) to give more variation.
                   The total descent distance is fly_distance and the total duration is
                   fly_duration. However, we want to stop the descent a bit above the dock
                   so that any overshoot does not carry the airship much below the dock altitude.
                   Account for the case that fly_distance <= desired_offset.
                */
                let desired_offset = ctx.rng.random_range(4.0..6.0);
                let descent_offset = if fly_distance > desired_offset {
                    desired_offset
                } else {
                    0.0
                };
                let descent_dist = fly_distance - descent_offset;
                // step 1 dist is 60-80% of the descent distance
                let step1_dist = descent_dist * ctx.rng.random_range(0.7..0.85);
                let step1_dur = fly_duration * ctx.rng.random_range(0.4..0.55);
                // Make loaded airships descend slightly faster
                let speed_mult = if matches!(ctx.npc.mode, SimulationMode::Loaded) {
                    ctx.rng.random_range(1.1..1.25)
                } else {
                    1.0
                };
                let speed1 = (step1_dist / step1_dur) / nominal_speed * speed_mult;
                let speed2 = ((descent_dist - step1_dist) / (fly_duration - step1_dur))
                    / nominal_speed
                    * speed_mult;
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} Descent, fly_distance {:.1}, fly_duration {:.1}s, \
                     desired_offset {:.1}, descent_offset {:.1}, descent_dist {:.1}, step1_dist \
                     {:.1}, step1_dur {:.3}s, speedmult {:.1}, speed1 {:.3}, speed2 {:.3}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    fly_distance,
                    fly_duration,
                    desired_offset,
                    descent_offset,
                    descent_dist,
                    step1_dist,
                    step1_dur,
                    speed_mult,
                    speed1,
                    speed2
                );

                // fly 3D to the intermediate descent point
                fly_airship_inner(
                    AirshipFlightPhase::Descent,
                    ctx.npc.wpos,
                    ctx.npc.wpos - Vec3::unit_z() * step1_dist,
                    20.0,
                    leg_ctx_time_begin,
                    step1_dur,
                    context_end_time - step1_dur as f64,
                    speed1,
                    0.0,
                    false,
                    Some(approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                )
                .then (fly_airship_inner(
                    AirshipFlightPhase::Descent,
                    ctx.npc.wpos - Vec3::unit_z() * step1_dist,
                    approach.airship_pos + Vec3::unit_z() * descent_offset,
                    20.0,
                    leg_ctx_time_begin + step1_dur as f64,
                    fly_duration - step1_dur,
                    context_end_time,
                    speed2,
                    0.0,
                    false,
                    Some(approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Precise),
                ))
                // Announce arrival
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(ctx, airship_context, 3, AirshipFlightPhase::Descent);
                    log_airship_position(ctx, airship_context.route_index, &AirshipFlightPhase::Docked);
                    ctx.controller
                        .say(None, Content::localized("npc-speech-pilot-landed"));
                }))
                .map(|_, _| ()).boxed()
            },
            AirshipFlightPhase::Docked => {
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} Docked ctx time {:.1}, \
                     airship_context.leg_ctx_time_begin {:.1}, docking duration {:.1}s",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    leg_ctx_time_begin,
                    airship_context.leg_ctx_time_begin,
                    fly_duration,
                );
                /*
                    Divide up the docking time into intervals of 10 to 16 seconds,
                    and at each interval make an announcement. Make the last announcement
                    approximately 10-12 seconds before the end of the docking time.
                    Make the first announcement 5-8 seconds after starting the docking time.
                    The minimum announcement time after the start is 5 seconds, and the
                    minimum time before departure announcement is 10 seconds.
                    The minimum interval between announcements is 10 seconds.

                    Docked      Announce        Announce           Announce       Depart
                    |-----------|---------------|----------------------|--------------|
                    0         5-8s             ...               duration-10-12s
                        min 5s        min 10           min 10               min 10s

                    If docking duration is less than 10 seconds, no announcements.
                */
                let announcement_times = {
                    let mut times = Vec::new();
                    if fly_duration > 10.0 {
                        let first_time = ctx.rng.random_range(5.0..8.0);
                        let last_time = fly_duration - ctx.rng.random_range(10.0..12.0);
                        if first_time + 10.0 > last_time {
                            // Can't do two, try one.
                            let mid_time = fly_duration / 2.0;
                            if mid_time > 5.0 && (fly_duration - mid_time) > 10.0 {
                                times.push(mid_time);
                            }
                        } else {
                            // use first, then fill forward with random 10..16s intervals
                            times.push(first_time);
                            let mut last_time = first_time;
                            let mut t = first_time + ctx.rng.random_range(10.0..16.0);
                            while t < fly_duration - 10.0 {
                                times.push(t);
                                last_time = t;
                                t += ctx.rng.random_range(10.0..16.0);
                            }
                            if last_time < fly_duration - 22.0 {
                                // add one last announcement before the final 10s
                                times.push(last_time + 12.0);
                            }
                            times.sort_by(|a, b| {
                                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                    }
                    times
                };
                airship_context.announcements = announcement_times.into_iter().collect();
                now(move |ctx, airship_context: &mut AirshipRouteContext| {
                    // get next announcement time or the docking end time.
                    // Don't consume the announcement time yet.
                    let dock_ctx_end_time = if let Some(at) = airship_context.announcements.front()
                    {
                        leg_ctx_time_begin + *at as f64
                    } else {
                        context_end_time
                    };
                    fly_airship_inner(
                        AirshipFlightPhase::Docked,
                        ctx.npc.wpos,
                        approach.airship_pos,
                        5.0,
                        leg_ctx_time_begin,
                        fly_duration,
                        dock_ctx_end_time,
                        0.75,
                        0.0,
                        false,
                        Some(approach.airship_direction),
                        FlightMode::Braking(BrakingMode::Precise),
                    )
                    .then(just(
                        |ctx, airship_context: &mut AirshipRouteContext| {
                            // Now consume the announcement time. If there was one, announce the
                            // next site. If not, we're at the end and
                            // will be exiting this repeat loop.
                            if airship_context.announcements.pop_front().is_some() {
                                // make announcement and log position
                                let (dst_site_name, dst_site_dir) = if let Some(next_leg_approach) =
                                    airship_context.next_leg_approach
                                {
                                    (
                                        ctx.index
                                            .sites
                                            .get(next_leg_approach.site_id)
                                            .name()
                                            .unwrap_or("Unknown Site")
                                            .to_string(),
                                        Direction::from_dir(
                                            next_leg_approach.approach_transition_pos
                                                - ctx.npc.wpos.xy(),
                                        )
                                        .localize_npc(),
                                    )
                                } else {
                                    ("Unknown Site".to_string(), Direction::North.localize_npc())
                                };
                                ctx.controller.say(
                                    None,
                                    Content::localized("npc-speech-pilot-announce_next")
                                        .with_arg("dir", dst_site_dir)
                                        .with_arg("dst", dst_site_name),
                                );
                                log_airship_position(
                                    ctx,
                                    airship_context.route_index,
                                    &AirshipFlightPhase::Docked,
                                );
                            }
                        },
                    ))
                })
                .repeat()
                .stop_if(move |ctx: &mut NpcCtx| ctx.time.0 >= context_end_time)
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(
                        ctx,
                        airship_context,
                        4,
                        AirshipFlightPhase::Docked,
                    );
                    log_airship_position(
                        ctx,
                        airship_context.route_index,
                        &AirshipFlightPhase::Docked,
                    );
                }))
                .map(|_, _| ())
                .boxed()
            },
            AirshipFlightPhase::Ascent => {
                log_airship_position(
                    ctx,
                    airship_context.route_index,
                    &AirshipFlightPhase::Ascent,
                );
                airship_context.route_timer = Duration::from_secs_f32(0.5);
                // v = d/t
                let speed = (fly_distance / fly_duration) / nominal_speed;
                debug_airships!(
                    4,
                    "Airship {} route {} leg {} Ascent, fly_distance {:.1}, fly_duration {:.1}s, \
                     speed factor {:.3}",
                    format!("{:?}", ctx.npc_id),
                    airship_context.route_index,
                    airship_context.current_leg,
                    fly_distance,
                    fly_duration,
                    speed
                );
                let src_site_name = ctx
                    .index
                    .sites
                    .get(approach.site_id)
                    .name()
                    .unwrap_or("Unknown Site")
                    .to_string();
                let dst_site_name =
                    if let Some(next_leg_approach) = airship_context.next_leg_approach {
                        ctx.index
                            .sites
                            .get(next_leg_approach.site_id)
                            .name()
                            .unwrap_or("Unknown Site")
                            .to_string()
                    } else {
                        "Unknown Site".to_string()
                    };
                ctx.controller.say(
                    None,
                    Content::localized("npc-speech-pilot-takeoff")
                        .with_arg("src", src_site_name)
                        .with_arg("dst", dst_site_name),
                );
                fly_airship_inner(
                    AirshipFlightPhase::Ascent,
                    ctx.npc.wpos,
                    approach.airship_pos + Vec3::unit_z() * approach.height,
                    20.0,
                    leg_ctx_time_begin,
                    fly_duration,
                    context_end_time,
                    speed,
                    0.0,
                    false,
                    Some(approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                )
                .then(just(|ctx, airship_context: &mut AirshipRouteContext| {
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = false;
                    #[cfg(debug_assertions)]
                    check_phase_completion_time(
                        ctx,
                        airship_context,
                        5,
                        AirshipFlightPhase::Ascent,
                    );
                }))
                .map(|_, _| ())
                .boxed()
            },
        }
    })
}

/// The action that moves the airship.
fn fly_airship_inner(
    phase: AirshipFlightPhase,
    from: Vec3<f32>,
    to: Vec3<f32>,
    goal_dist: f32,
    leg_ctx_time_begin: f64,
    leg_duration: f32,
    context_tgt_end_time: f64,
    speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
) -> impl Action<AirshipRouteContext> {
    just(move |ctx, airship_context: &mut AirshipRouteContext| {
        // The target position is used for determining the
        // reverse direction for 'unsticking' the airship if it gets stuck in
        // one place.
        let stuck_tracker_target_loc = to.xy();

        // Determine where the airship should be.
        let nominal_pos = match phase {
            AirshipFlightPhase::DepartureCruise
            | AirshipFlightPhase::ApproachCruise
            | AirshipFlightPhase::Transition => {
                // Flying 2d, compute nominal x,y pos.
                let route_interpolation_ratio =
                    ((ctx.time.0 - leg_ctx_time_begin) as f32 / leg_duration).clamp(0.0, 1.0);
                (from.xy() + (to.xy() - from.xy()) * route_interpolation_ratio).with_z(to.z)
            },
            AirshipFlightPhase::Descent | AirshipFlightPhase::Ascent => {
                // Only Z movement, compute z pos.
                // Starting altitude is terrain altitude + height offset
                // Ending altitude is to.z
                let route_interpolation_ratio =
                    ((ctx.time.0 - leg_ctx_time_begin) as f32 / leg_duration).clamp(0.0, 1.0);
                let nominal_z = from.z + (to.z - from.z) * route_interpolation_ratio;
                to.xy().with_z(nominal_z)
            },
            _ => {
                // docking phase has no movement
                to
            },
        };

        // Periodically check if the airship is stuck.
        let timer = airship_context
            .route_timer
            .checked_sub(Duration::from_secs_f32(ctx.dt));
        // keep or reset the timer
        airship_context.route_timer =
            timer.unwrap_or(Duration::from_secs_f32(AIRSHIP_PROGRESS_UPDATE_INTERVAL));
        if timer.is_none() {
            // Timer expired.
            // log my position
            #[cfg(feature = "airship_log")]
            {
                // Check position error
                let distance_to_nominal = match phase {
                    AirshipFlightPhase::DepartureCruise
                    | AirshipFlightPhase::ApproachCruise
                    | AirshipFlightPhase::Transition => {
                        // Flying 2d, compute nominal x,y pos.
                        ctx.npc
                            .wpos
                            .xy()
                            .as_::<f64>()
                            .distance(nominal_pos.xy().as_())
                    },
                    _ => ctx.npc.wpos.as_::<f64>().distance(nominal_pos.as_()),
                };
                log_airship_position_plus(
                    ctx,
                    airship_context.route_index,
                    &phase,
                    distance_to_nominal,
                    0.0,
                );
            }

            // If in cruise phase, check if the airship is stuck and reset the cruise
            // direction.
            if matches!(
                phase,
                AirshipFlightPhase::DepartureCruise | AirshipFlightPhase::ApproachCruise
            ) {
                // Check if we're stuck
                if let Some(stuck_tracker) = &mut airship_context.my_stuck_tracker
                    && stuck_tracker.is_stuck(ctx, &ctx.npc.wpos, &stuck_tracker_target_loc)
                    && let Some(backout_pos) = stuck_tracker.current_backout_pos(ctx)
                {
                    airship_context.stuck_backout_pos = Some(backout_pos);
                } else if airship_context.stuck_backout_pos.is_some() {
                    #[cfg(debug_assertions)]
                    debug_airships!(
                        2,
                        "{:?} unstuck at pos: {} {}",
                        ctx.npc_id,
                        ctx.npc.wpos.x as i32,
                        ctx.npc.wpos.y as i32,
                    );
                    airship_context.stuck_backout_pos = None;
                };
                // Reset cruise direction
                airship_context.cruise_direction =
                    Dir::from_unnormalized((to.xy() - ctx.npc.wpos.xy()).with_z(0.0));
            }
        }
        // move the airship
        if let Some(backout_pos) = airship_context.stuck_backout_pos {
            // Unstick the airship
            ctx.controller.do_goto_with_height_and_dir(
                backout_pos,
                1.5,
                None,
                None,
                FlightMode::Braking(BrakingMode::Normal),
            );
        } else {
            // Normal movement, not stuck.
            let height_offset_opt = if with_terrain_following {
                Some(height_offset)
            } else {
                None
            };
            // In the long cruise phases, the airship should face the target position.
            // When the airship is loaded, the movement vector can change dramatically from
            // velocity differences due to climbing or descending (terrain following), and
            // due to wind effects. Use a fixed cruise direction that is updated
            // periodically instead of relying on the action_nodes code that
            // tries to align the airship direction with the instantaneous
            // movement vector.
            let dir_opt = if direction_override.is_some() {
                direction_override
            } else if matches!(
                phase,
                AirshipFlightPhase::DepartureCruise | AirshipFlightPhase::ApproachCruise
            ) {
                airship_context.cruise_direction
            } else {
                None
            };
            ctx.controller.do_goto_with_height_and_dir(
                nominal_pos,
                speed_factor,
                height_offset_opt,
                dir_opt,
                flight_mode,
            );
        }
    })
    .repeat()
    .boxed()
    .stop_if(move |ctx: &mut NpcCtx| {
        match phase {
            AirshipFlightPhase::Descent | AirshipFlightPhase::Ascent => {
                ctx.time.0 >= context_tgt_end_time
                    || ctx.npc.wpos.as_::<f64>().distance_squared(to.as_())
                        < (goal_dist as f64).powi(2)
            },
            AirshipFlightPhase::Docked => {
                // docking phase has no movement, just wait for the duration
                if ctx.time.0 >= context_tgt_end_time {
                    debug_airships!(
                        4,
                        "Airship {} docking phase complete time now {:.1} >= context_tgt_end_time \
                         {:.1}",
                        format!("{:?}", ctx.npc_id),
                        ctx.time.0,
                        context_tgt_end_time,
                    );
                }
                ctx.time.0 >= context_tgt_end_time
            },
            _ => {
                if flight_mode == FlightMode::FlyThrough {
                    // we only care about the xy distance (just get close to the target position)
                    ctx.npc
                        .wpos
                        .xy()
                        .as_::<f64>()
                        .distance_squared(to.xy().as_())
                        < (goal_dist as f64).powi(2)
                } else {
                    // Braking mode means the PID controller will be controlling all three axes
                    ctx.npc.wpos.as_::<f64>().distance_squared(to.as_())
                        < (goal_dist as f64).powi(2)
                }
            },
        }
    })
    .debug(move || {
        format!(
            "fly airship, phase:{:?}, tgt pos:({}, {}, {}), goal dist:{}, leg dur: {}, initial \
             speed:{}, height:{}, terrain following:{}, FlightMode:{:?}",
            phase,
            to.x,
            to.y,
            to.z,
            goal_dist,
            leg_duration,
            speed_factor,
            height_offset,
            with_terrain_following,
            flight_mode,
        )
    })
    .map(|_, _| ())
}

/// The NPC is the airship captain. This action defines the flight loop for the
/// airship. The captain NPC is autonomous and will fly the airship along the
/// assigned route. The routes are established and assigned to the captain NPCs
/// when the world is generated.
pub fn pilot_airship<S: State>() -> impl Action<S> {
    now(move |ctx, airship_context: &mut AirshipRouteContext| {
        // get the assigned route and start leg indexes
        if let Some((route_index, start_leg_index)) =
            ctx.data.airship_sim.assigned_routes.get(&ctx.npc_id)
        {
            // If airship_context.route_index is the default value (usize::MAX) it means the
            // server has just started.
            let is_initial_startup = airship_context.route_index == usize::MAX;
            if is_initial_startup {
                setup_airship_route_context(ctx, airship_context, route_index, start_leg_index);
            } else {
                // Increment the leg index with wrap around
                airship_context.current_leg = ctx
                    .world
                    .civs()
                    .airships
                    .increment_route_leg(airship_context.route_index, airship_context.current_leg);
                if airship_context.current_leg == 0 {
                    // We have wrapped around to the start of the route, add the route duration
                    // to route_time_zero.
                    airship_context.route_time_zero +=
                        ctx.world.civs().airships.routes[airship_context.route_index].total_time;
                    debug_airships!(
                        4,
                        "Airship {} route {} completed full route, route time zero now {:.1}, \
                         ctx.time.0 - route_time_zero = {:.3}",
                        format!("{:?}", ctx.npc_id),
                        airship_context.route_index,
                        airship_context.route_time_zero,
                        ctx.time.0 - airship_context.route_time_zero
                    );
                } else {
                    debug_airships!(
                        4,
                        "Airship {} route {} starting next leg {}, current route time {:.1}",
                        format!("{:?}", ctx.npc_id),
                        airship_context.route_index,
                        airship_context.current_leg,
                        ctx.time.0 - airship_context.route_time_zero
                    );
                }
            }

            // set the approach data for the current leg
            // Needed: docking position and direction.
            airship_context.current_leg_approach =
                Some(ctx.world.civs().airships.approach_for_route_and_leg(
                    airship_context.route_index,
                    airship_context.current_leg,
                    &ctx.world.sim().map_size_lg(),
                ));

            if airship_context.current_leg_approach.is_none() {
                tracing::error!(
                    "Airship pilot {:?} approach not found for route {} leg {}, stopping \
                     pilot_airship loop.",
                    ctx.npc_id,
                    airship_context.route_index,
                    airship_context.current_leg
                );
                return finish().map(|_, _| ()).boxed();
            }

            // Get the next leg index.
            // The destination of the next leg is needed for announcements while docked.
            let next_leg_index = ctx
                .world
                .civs()
                .airships
                .increment_route_leg(airship_context.route_index, airship_context.current_leg);
            airship_context.next_leg_approach =
                Some(ctx.world.civs().airships.approach_for_route_and_leg(
                    airship_context.route_index,
                    next_leg_index,
                    &ctx.world.sim().map_size_lg(),
                ));
            if airship_context.next_leg_approach.is_none() {
                tracing::warn!(
                    "Airship pilot {:?} approach not found for next route {} leg {}",
                    ctx.npc_id,
                    airship_context.route_index,
                    next_leg_index
                );
            }

            // The initial flight sequence is used when the server first starts up.
            if is_initial_startup {
                // Figure out what flight phase to start with.
                // Search the route's spawning locations for the one that is
                // closest to the airship's current position.
                let my_route = &ctx.world.civs().airships.routes[airship_context.route_index];
                if let Some(my_spawn_loc) = my_route.spawning_locations.iter().min_by(|a, b| {
                    let dist_a = ctx.npc.wpos.xy().as_::<f64>().distance_squared(a.pos.as_());
                    let dist_b = ctx.npc.wpos.xy().as_::<f64>().distance_squared(b.pos.as_());
                    dist_a.partial_cmp(&dist_b).unwrap_or(Ordering::Equal)
                }) {
                    // The airship starts somewhere along the route.
                    // Adjust the route_zero_time according to the route time in the spawn location
                    // data. At initialization, the airship is at the spawn
                    // location and the route time is whatever is in the spawn location data.
                    airship_context.route_time_zero = ctx.time.0 - my_spawn_loc.spawn_route_time;
                    airship_context.leg_ctx_time_begin = ctx.time.0;
                    airship_context.first_leg = true;
                    debug_airships!(
                        4,
                        "Airship {} route {} leg {}, initial start up on phase {:?}, setting \
                         route_time_zero to {:.1} (ctx.time.0 {} - my_spawn_loc.spawn_route_time \
                         {})",
                        format!("{:?}", ctx.npc_id),
                        airship_context.route_index,
                        airship_context.current_leg,
                        my_spawn_loc.flight_phase,
                        airship_context.route_time_zero,
                        ctx.time.0,
                        my_spawn_loc.spawn_route_time,
                    );
                    initial_flight_sequence(my_spawn_loc.flight_phase)
                        .map(|_, _| ())
                        .boxed()
                } else {
                    // No spawning location, should not happen
                    tracing::error!(
                        "Airship pilot {:?} spawning location not found for route {} leg {}",
                        ctx.npc_id,
                        airship_context.route_index,
                        next_leg_index
                    );
                    finish().map(|_, _| ()).boxed()
                }
            } else {
                nominal_flight_sequence().map(|_, _| ()).boxed()
            }
        } else {
            //  There are no routes assigned.
            //  This is unexpected and never happens in testing, just do nothing so the
            // compiler doesn't complain.
            finish().map(|_, _| ()).boxed()
        }
    })
    .repeat()
    .with_state(AirshipRouteContext::default())
    .map(|_, _| ())
}

fn setup_airship_route_context(
    _ctx: &mut NpcCtx,
    route_context: &mut AirshipRouteContext,
    route_index: &usize,
    leg_index: &usize,
) {
    route_context.route_index = *route_index;
    route_context.current_leg = *leg_index;

    #[cfg(debug_assertions)]
    {
        let current_approach = _ctx.world.civs().airships.approach_for_route_and_leg(
            route_context.route_index,
            route_context.current_leg,
            &_ctx.world.sim().map_size_lg(),
        );
        debug_airships!(
            4,
            "Server startup, airship pilot {:?} starting on route {} leg {}, target dock: {} {}",
            _ctx.npc_id,
            route_context.route_index,
            route_context.current_leg,
            current_approach.airship_pos.x as i32,
            current_approach.airship_pos.y as i32,
        );
    }
}

fn initial_flight_sequence(start_phase: AirshipFlightPhase) -> impl Action<AirshipRouteContext> {
    now(move |_, airship_context: &mut AirshipRouteContext| {
        let approach = airship_context.current_leg_approach.unwrap();
        let phases = match start_phase {
            AirshipFlightPhase::DepartureCruise => vec![
                (AirshipFlightPhase::DepartureCruise, approach),
                (AirshipFlightPhase::ApproachCruise, approach),
                (AirshipFlightPhase::Transition, approach),
                (AirshipFlightPhase::Descent, approach),
                (AirshipFlightPhase::Docked, approach),
                (AirshipFlightPhase::Ascent, approach),
            ],
            AirshipFlightPhase::ApproachCruise => vec![
                (AirshipFlightPhase::ApproachCruise, approach),
                (AirshipFlightPhase::Transition, approach),
                (AirshipFlightPhase::Descent, approach),
                (AirshipFlightPhase::Docked, approach),
                (AirshipFlightPhase::Ascent, approach),
            ],
            AirshipFlightPhase::Transition => vec![
                (AirshipFlightPhase::Transition, approach),
                (AirshipFlightPhase::Descent, approach),
                (AirshipFlightPhase::Docked, approach),
                (AirshipFlightPhase::Ascent, approach),
            ],
            AirshipFlightPhase::Descent => vec![
                (AirshipFlightPhase::Descent, approach),
                (AirshipFlightPhase::Docked, approach),
                (AirshipFlightPhase::Ascent, approach),
            ],
            AirshipFlightPhase::Docked => {
                // Adjust the initial docking time.
                vec![
                    (AirshipFlightPhase::Docked, approach),
                    (AirshipFlightPhase::Ascent, approach),
                ]
            },
            AirshipFlightPhase::Ascent => vec![(AirshipFlightPhase::Ascent, approach)],
        };
        seq(phases
            .into_iter()
            .map(|(phase, current_approach)| fly_airship(phase, current_approach)))
    })
}

fn nominal_flight_sequence() -> impl Action<AirshipRouteContext> {
    now(move |_, airship_context: &mut AirshipRouteContext| {
        let approach = airship_context.current_leg_approach.unwrap();
        let phases = vec![
            (AirshipFlightPhase::DepartureCruise, approach),
            (AirshipFlightPhase::ApproachCruise, approach),
            (AirshipFlightPhase::Transition, approach),
            (AirshipFlightPhase::Descent, approach),
            (AirshipFlightPhase::Docked, approach),
            (AirshipFlightPhase::Ascent, approach),
        ];
        seq(phases
            .into_iter()
            .map(|(phase, current_approach)| fly_airship(phase, current_approach)))
    })
}

#[cfg(feature = "airship_log")]
/// Get access to the global airship logger and log an airship position.
fn log_airship_position(ctx: &NpcCtx, route_index: usize, phase: &AirshipFlightPhase) {
    log_airship_position_plus(ctx, route_index, phase, 0.0, 0.0);
}

#[cfg(feature = "airship_log")]
fn log_airship_position_plus(
    ctx: &NpcCtx,
    route_index: usize,
    phase: &AirshipFlightPhase,
    value1: f64,
    value2: f64,
) {
    if let Ok(mut logger) = airship_logger() {
        logger.log_position(
            ctx.npc_id,
            ctx.index.seed,
            route_index,
            phase,
            ctx.time.0,
            ctx.npc.wpos,
            matches!(ctx.npc.mode, SimulationMode::Loaded),
            value1,
            value2,
        );
    } else {
        tracing::warn!("Failed to log airship position for {:?}", ctx.npc_id);
    }
}

#[cfg(not(feature = "airship_log"))]
/// When the logging feature is not enabled, this should become a no-op.
fn log_airship_position(_: &NpcCtx, _: usize, _: &AirshipFlightPhase) {}
