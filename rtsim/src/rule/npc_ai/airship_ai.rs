use crate::{
    ai::{finish, just, now, predicate::timeout, Action, NpcCtx, State},
    data::{npc::SimulationMode, Npc},
};
use common::{
    comp::{
        Content,
        agent::{BrakingMode, FlightMode},
        compass::Direction,
    },
    rtsim::NpcId,
    store::Id,
    util::Dir,
};
use num_traits::cast::FromPrimitive;
use rand::prelude::*;
use std::{
    cmp::Ordering, collections::VecDeque, sync::Mutex, time::Duration
};
use vek::*;
use world::{
    civ::airship_travel::{AirshipDockingApproach, Airships},
    site::Site,
    util::{DHashMap, CARDINALS},
};

#[cfg(debug_assertions)] use tracing::debug;

const AIRSHIP_APPROACH_FINAL_HEIGHT_DELTA: f32 = 100.0;
const AIRSHIP_DOCK_TRANSITION_HEIGHT_DELTA: f32 = 150.0;
const CLOSE_TO_DOCKING_SITE_DISTANCE: f32 = 225.0f32 * 225.0f32;
const SPEED_ADJUST_CLOSE_AIRSHIP_DISTANCE: f32 = 2500.0f32 * 2500.0f32;
const SOMEWHAT_CLOSE_AIRSHIP_DISTANCE: f32 = 1000.0f32 * 1000.0f32;
const PASSING_CLOSE_AIRSHIP_DISTANCE: f32 = 700.0f32 * 700.0f32;
const VERY_CLOSE_AIRSHIP_DISTANCE: f32 = 400.0f32 * 400.0f32;
const SLOW_DOWN_SPEED_MULTIPLIER: f32 = 0.3;
const CRUISE_CHECKPOINT_DISTANCE: f32 = 800.0;
const NEXT_PILOT_CRUISE_SPEED_TOLERANCE: f32 = 2.0;
const MOVING_AVERAGE_SCALE_FACTOR: f64 = 10000.0;
const NEXT_PILOT_MOVING_AVERAGE_CAPACITY: usize = 5;
const NEXT_PILOT_MOVING_AVERAGE_MIN_SIZE: usize = 3;

/// Airships can slow down or hold position to avoid collisions with other
/// airships.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
enum AirshipAvoidanceMode {
    #[default]
    None,
    Hold(Vec3<f32>, Dir),
    SlowDown,
    Stuck(Vec3<f32>),
}

// Context data for piloting the airship.
// This is the context data for the pilot_airship action.
#[derive(Debug, Clone)]
struct AirshipRouteContext {
    // The route index (index into the outer vec of airships.routes)
    route_index: usize,
    // The length of the route in legs. (size of the inner vec of airships.routes[route_index])
    route_length: usize,
    // The expected airship speed when simulated.
    simulated_airship_speed: f32,
    // The next route leg index.
    next_leg: usize,
    // The NpcId of the captain ahead of this one.
    next_pilot: NpcId,
    // next_pilot: Option<NpcId>,
    // The current approach.
    current_leg_approach: Option<AirshipDockingApproach>,
    // The next approach.
    next_leg_approach: Option<AirshipDockingApproach>,
    // A point short of the approach transition point where the frequency of avoidance checks increases.
    next_leg_cruise_checkpoint_pos: Vec3<f32>,
    
    // Avoidance Data

    // For tracking the airship's position history to determine if the airship is stuck.
    my_pos_tracker: Option<PositionTracker>,
    // For tracking airship velocity towards the approach transition point.
    my_rate_tracker: Option<RateTracker>,
    // For tracking the next pilot's velocity towards the approach transition point.
    next_pilot_rate_tracker_: Option<RateTracker>,
    // Timer for checking the airship trackers.
    avoidance_timer: Duration,
    // Timer used when holding, either on approach or at the dock.
    hold_timer: f32,
    // Whether the initial hold message has been sent to the client.
    hold_announced: bool, 
    // The original speed factor passed to the fly_airship action.
    speed_factor: f32,
    // The moving average of the next pilot's velocity during cruise phase.
    next_pilot_average_velocity: MovingAverage<i64>,
    // The current avoidance mode for the airship.
    avoid_mode: AirshipAvoidanceMode,
    // Whether the airship had to hold during the last flight.
    did_hold: bool,
    // number of times the airship had to slow down during the last flight.
    slow_count: u32,
    // The extra docking time due to holding.
    extra_hold_dock_time: f32,
    // The extra docking time due to holding.
    extra_slowdown_dock_time: f32,
}

impl Default for AirshipRouteContext {
    fn default() -> Self {
        Self {
            route_index: usize::MAX,
            route_length: 0,
            simulated_airship_speed: 0.0,
            next_leg: 0,
            next_pilot: NpcId::default(),
            current_leg_approach: None,
            next_leg_approach: None,
            next_leg_cruise_checkpoint_pos: Vec3::default(),
            my_pos_tracker: None,
            my_rate_tracker: None,
            next_pilot_rate_tracker_: None,
            avoidance_timer: Duration::default(),
            hold_timer: 0.0,
            hold_announced: false,
            speed_factor: 0.0,
            next_pilot_average_velocity: MovingAverage::new(NEXT_PILOT_MOVING_AVERAGE_CAPACITY),
            avoid_mode: AirshipAvoidanceMode::default(),
            did_hold: false,
            slow_count: 0,
            extra_hold_dock_time: 0.0,
            extra_slowdown_dock_time: 0.0,
        }
    }
}

// Context data for the legacy airship route.
// This is the context data for the pilot_airship_legacy action.
#[derive(Debug, Default, Clone)]
struct AirshipRouteContextLegacy {
    // The current approach index, 0 1 or none
    current_approach_index: Option<usize>,
    // The route's site ids.
    site_ids: Option<[Id<Site>; 2]>,

    // For fly_airship action
    // For determining the velocity and direction of this and other airships on the route.
    trackers: DHashMap<NpcId, ZoneDistanceTracker>,
    // For tracking the airship's position history to determine if the airship is stuck.
    pos_trackers: DHashMap<NpcId, PositionTracker>,
    // Timer for checking the airship trackers.
    avoidance_timer: Duration,
    // Timer used when holding, either on approach or at the dock.
    hold_timer: f32,
    // Whether the initial hold message has been sent to the client.
    hold_announced: bool,
    // The original speed factor passed to the fly_airship action.
    speed_factor: f32,
    // The current avoidance mode for the airship.
    avoid_mode: AirshipAvoidanceMode,
    // Whether the airship had to hold during the last flight.
    did_hold: bool,
    // number of times the airship had to slow down during the last flight.
    slow_count: u32,
    // The extra docking time due to holding.
    extra_hold_dock_time: f32,
    // The extra docking time due to holding.
    extra_slowdown_dock_time: f32,
}

/// Tracks the airship position history.
/// Used for determining if an airship is stuck.
#[derive(Debug, Default, Clone)]
struct PositionTracker {
    // The airship's position history. Used for determining if the airship is stuck in one place.
    pos_history: Vec<Vec3<f32>>,
    // The route to follow for backing out of a stuck position.
    backout_route: Vec<Vec3<f32>>,
}

impl PositionTracker {
    const BACKOUT_TARGET_DIST: f32 = 50.0;
    const MAX_POS_HISTORY_SIZE: usize = 5;

    // Add a new position to the position history, maintaining a fixed size.
    fn add_position(&mut self, new_pos: Vec3<f32>) {
        if self.pos_history.len() >= PositionTracker::MAX_POS_HISTORY_SIZE {
            self.pos_history.remove(0);
        }
        self.pos_history.push(new_pos);
    }

    // Check if the airship is stuck in one place.
    fn is_stuck(&mut self, ctx: &mut NpcCtx, target_pos: &Vec2<f32>) -> bool {
        // The position history must be full to determine if the airship is stuck.
        if self.pos_history.len() == PositionTracker::MAX_POS_HISTORY_SIZE
            && self.backout_route.is_empty()
        {
            if let Some(last_pos) = self.pos_history.last() {
                // If all the positions in the history are within 10 of the last position,
                if self
                    .pos_history
                    .iter()
                    .all(|pos| pos.distance_squared(*last_pos) < 10.0)
                {
                    // Airship is stuck on some obstacle.
                    // If current position is near the ground, backout while gaining altitude
                    // else backout at the current height and then ascend to clear the obstacle.
                    // This function is only called in cruise phase, so the direction to backout
                    // will be the opposite of the direction from the current pos to the approach
                    // initial pos.
                    if let Some(backout_dir) = (ctx.npc.wpos.xy() - target_pos)
                        .with_z(0.0)
                        .try_normalized()
                    {
                        let ground = ctx
                            .world
                            .sim()
                            .get_surface_alt_approx(last_pos.xy().map(|e| e as i32));
                        let mut backout_pos = ctx.npc.wpos + backout_dir * 100.0;
                        if (ctx.npc.wpos.z - ground).abs() < 10.0 {
                            backout_pos.z += 50.0;
                        }
                        self.backout_route =
                            vec![backout_pos, backout_pos + Vec3::unit_z() * 200.0];
                        // The airship is stuck.
                        #[cfg(debug_assertions)]
                        debug!(
                            "Airship {} Stuck! at {:?}, backout_dir:{:?}, backout_pos:{:?}",
                            format!("{:?}", ctx.npc_id),
                            ctx.npc.wpos,
                            backout_dir,
                            backout_pos
                        );
                    }
                }
            }
        }
        !self.backout_route.is_empty()
    }

    fn next_backout_pos(&mut self, ctx: &mut NpcCtx) -> Option<Vec3<f32>> {
        if let Some(pos) = self.backout_route.first().cloned() {
            if ctx.npc.wpos.distance_squared(pos) < PositionTracker::BACKOUT_TARGET_DIST.powi(2) {
                self.backout_route.remove(0);
            }
            Some(pos)
        } else {
            self.pos_history.clear();
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
struct RateTracker {
    // ref_pos: Vec2<f32>,
    last_pos: Option<Vec2<f32>>,
    last_time: f32,
}

impl RateTracker {
    fn update(&mut self, current_pos: Vec2<f32>, time: f32) -> f32 {
        let rate = if let Some(last_pos) = self.last_pos {
            let dt = time - self.last_time;
            if dt > 0.0 { 
                current_pos.distance(last_pos) / dt // blocks per second
            } else {
                0.0
            }
        } else {
            0.0
        };
        self.last_pos = Some(current_pos);
        self.last_time = time;
        rate
    }
}


#[derive(Debug, PartialEq)]
enum DistanceTrend {
    ApproachingDock,
    DepartingDock,
    Docked,
}

#[derive(Debug, PartialEq)]
enum DistanceZone {
    InsideReference,
    InsideMyDistance,
    OutsideMyDistance,
}

/// Tracks airship distance trend from a fixed position. Used for airship
/// traffic control.
#[derive(Debug, Default, Clone)]
struct ZoneDistanceTracker {
    fixed_pos: Vec2<f32>,
    stable_tolerance: f32,
    ref_dist: Option<f32>,
    prev_dist: Option<f32>,
    avg_dist: Option<f32>,
}

impl ZoneDistanceTracker {
    fn update(
        &mut self,
        mypos: Vec2<f32>,
        otherpos: Vec2<f32>,
    ) -> (Option<DistanceTrend>, Option<DistanceZone>) {
        const SMOOTHING_FACTOR: f32 = 0.3; // Damp out some fluctuations so that we know if the trend is stable.
        // There is no delta time here because the measurements are taken at larger time
        // intervals, on the order of seconds.
        let current_dist = otherpos.distance_squared(self.fixed_pos);
        let trend = if let Some(prev_dist) = self.prev_dist {
            let my_dist = mypos.distance_squared(self.fixed_pos);
            let avg_dist = self
                .avg_dist
                .map(|prev_avg_dist| Lerp::lerp(prev_avg_dist, current_dist, SMOOTHING_FACTOR))
                .unwrap_or(current_dist);
            self.avg_dist = Some(avg_dist);
            let zone = if current_dist < my_dist {
                if let Some(ref_dist) = self.ref_dist {
                    if current_dist < ref_dist {
                        DistanceZone::InsideReference
                    } else {
                        DistanceZone::InsideMyDistance
                    }
                } else {
                    DistanceZone::InsideMyDistance
                }
            } else {
                DistanceZone::OutsideMyDistance
            };
            if avg_dist.abs() < self.stable_tolerance {
                (Some(DistanceTrend::Docked), Some(zone))
            } else if current_dist < prev_dist {
                (Some(DistanceTrend::ApproachingDock), Some(zone))
            } else {
                (Some(DistanceTrend::DepartingDock), Some(zone))
            }
        } else {
            self.avg_dist = Some(current_dist);
            (None, None)
        };
        self.prev_dist = Some(current_dist);
        trend
    }
}

#[derive(Clone, Debug)]
struct MovingAverage<T>
where
    T: Default + FromPrimitive + std::ops::AddAssign + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + Copy,
{
    values: VecDeque<T>,
    size: usize,
    sum: T,
}

impl<T> MovingAverage<T>
where
    T: Default + FromPrimitive + std::ops::AddAssign + std::ops::Sub<Output = T> + std::ops::Div<Output = T> + Copy,
{
    fn new(size: usize) -> Self {
        Self {
            values: VecDeque::with_capacity(size),
            size,
            sum: T::from_u32(0).unwrap(),
        }
    }

    fn add(&mut self, value: T) {
        if self.values.len() == self.size {
            if let Some(old_value) = self.values.pop_front() {
                self.sum = self.sum - old_value;
            }
        }
        self.values.push_back(value);
        self.sum += value;
    }

    fn average(&self) -> T {
        if self.values.len() < NEXT_PILOT_MOVING_AVERAGE_MIN_SIZE {
            T::from_u32(0).unwrap()
        } else {
            self.sum / T::from_u32(self.values.len() as u32).unwrap()
        }
    }

}


/// The flight phases of an airship.
/// When the airship is loaded from RTSim data there is no "current approach
/// index", so the 'Reset' phases are used to get the airship back on the route.
/// The non-'Reset' phases are used for the normal flight loop.
#[derive(Debug, Copy, Clone, PartialEq, Default)]
enum AirshipFlightPhase {
    Ascent,
    Cruise,
    ApproachFinal,
    Transition,
    // Docking,
    #[default]
    Docked,
}

/// Wrapper for the fly_airship action, so the route context fields can be
/// reset.
fn fly_airship(
    phase: AirshipFlightPhase,
    wpos: Vec3<f32>,
    goal_dist: f32,
    initial_speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<AirshipRouteContext> {
    now(move |ctx, airship_context: &mut AirshipRouteContext| {
        let debug_npc = {
            let npc_id_str = format!("{:?}", ctx.npc_id);
            npc_id_str == "NpcId(1584v1)"
        };
        if debug_npc {
            if phase == AirshipFlightPhase::Cruise {
                debug!("My pilot: cruise phase, reset speed factor")
            }
        }
        airship_context.speed_factor = initial_speed_factor;
        airship_context.avoidance_timer = radar_interval;
        airship_context.avoid_mode = AirshipAvoidanceMode::None;
        if phase == AirshipFlightPhase::Cruise && matches!(ctx.npc.mode, SimulationMode::Loaded) {
            airship_context.my_pos_tracker = Some(PositionTracker::default());
            airship_context.my_rate_tracker = Some(RateTracker::default());
            airship_context.next_pilot_rate_tracker_ = Some(RateTracker::default());
        } else {
            airship_context.my_pos_tracker = None;
            airship_context.my_rate_tracker = None;
            airship_context.next_pilot_rate_tracker_ = None;
        }
        fly_airship_inner(
            phase,
            wpos,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            direction_override,
            flight_mode,
            with_collision_avoidance,
            radar_interval,
        )        
    })
}

fn fly_airship_inner(
    phase: AirshipFlightPhase,
    wpos: Vec3<f32>,
    goal_dist: f32,
    initial_speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<AirshipRouteContext> {
    just(
        move |ctx, airship_context: &mut AirshipRouteContext| {

            let debug_npc = {
                let npc_id_str = format!("{:?}", ctx.npc_id);
                npc_id_str == "NpcId(1584v1)"
            };

            let remaining = airship_context
                .avoidance_timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            if remaining.is_none() {
                airship_context.avoidance_timer = radar_interval;
                // The collision avoidance checks are not done every tick (no dt required), only
                // every 2-4 seconds.
                if with_collision_avoidance {
                    // The cruise phase is after docking, so the current approach is the next_leg_approach in the airship context.
                    if let Some(current_approach) = airship_context.next_leg_approach
                    {
                        let mypos = ctx.npc.wpos;

                        if phase == AirshipFlightPhase::Cruise {
                            if let Some(position_tracker) = &mut airship_context.my_pos_tracker {
                                position_tracker.add_position(mypos);
                                // Check if the airship is stuck in one place.
                                if position_tracker.is_stuck(ctx, &current_approach.approach_transition_pos.xy())
                                    && let Some(backout_pos) = position_tracker.next_backout_pos(ctx)
                                {
                                    airship_context.avoid_mode =
                                        AirshipAvoidanceMode::Stuck(backout_pos);
                                } else {
                                    // If the mode was stuck, but the airship is no longer stuck, clear
                                    // the mode.
                                    if matches!(
                                        airship_context.avoid_mode,
                                        AirshipAvoidanceMode::Stuck(..)
                                    ) {
                                        airship_context.avoid_mode = AirshipAvoidanceMode::None;
                                    }
                                }
                            }
                            
                            if let Some(my_rate_tracker) = &mut airship_context.my_rate_tracker
                            {
                                let my_rate = my_rate_tracker.update(
                                    mypos.xy(),
                                    ctx.time.0 as f32,
                                );

                                if let Some(next_pilot_rate_tracker) = &mut airship_context.next_pilot_rate_tracker_
                                    && let Some(next_pilot) = ctx.state.data().npcs.get(airship_context.next_pilot)
                                {
                                    let next_pilot_rate = next_pilot_rate_tracker.update(
                                        next_pilot.wpos.xy(),
                                        ctx.time.0 as f32,
                                    );
                                    // The next pilot could be Loaded or Simulated. In either case, track the moving average of
                                    // the next pilot's velocity if the velocity is greater than the expected simulated
                                    // cruise speed minus a small tolerance (i.e., when it can be expected to be cruising).
                                    if next_pilot_rate > airship_context.simulated_airship_speed - NEXT_PILOT_CRUISE_SPEED_TOLERANCE {
                                        // Sccale up the velocity so that the moving average can be done as an integer.
                                        airship_context.next_pilot_average_velocity.add((next_pilot_rate as f64 * MOVING_AVERAGE_SCALE_FACTOR) as i64);
                                    }
                                    
                                    if airship_context.avoid_mode == AirshipAvoidanceMode::None {
                                        if my_rate > 0.0 {
                                            use world::civ::airship_travel::{airship_globals};
                                            let airship_globals = airship_globals().expect("AIRSHIP_GLOBAL_DATA lock poisoned");
                                            if airship_globals.speed_factor_override > 0.0 {                                                
                                                airship_context.speed_factor = airship_globals.speed_factor_override;
                                            } else {
                                                // If there's enough data to estimate the next pilot's
                                                // cruising speed, then adjust my speed factor to match
                                                // if this pilot is close enough to the next pilot that changing speed would help
                                                // match the next pilot's cuising speed.
                                                 
                                                // scale the next pilot's average velocity down to a float
                                                let next_pilot_avg_velocity_i64 = airship_context.next_pilot_average_velocity.average();
                                                if airship_context.next_pilot_average_velocity.average() > 0
                                                    && ctx.npc.wpos.xy().distance_squared(next_pilot.wpos.xy()) < SPEED_ADJUST_CLOSE_AIRSHIP_DISTANCE 
                                                {
                                                    let next_pilot_avg_velocity = (next_pilot_avg_velocity_i64 as f64 / MOVING_AVERAGE_SCALE_FACTOR) as f32;
                                                    let new_speed_factor = -2.0173430f32 + 
                                                        0.3885298f32 * next_pilot_avg_velocity +
                                                        -0.0221485f32 * next_pilot_avg_velocity.powi(2) +
                                                        0.0004694f32 * next_pilot_avg_velocity.powi(3);
                                                    if new_speed_factor > 0.0 {
                                                        airship_context.speed_factor = new_speed_factor;
                                                        if debug_npc {
                                                            debug!(
                                                                "My pilot: Adjusting speed factor to {}, next pilot avg velocity: {}",
                                                                airship_context.speed_factor,
                                                                next_pilot_avg_velocity
                                                            );
                                                        }
                                                    } else {
                                                        // If the new speed factor is negative, don't change it.
                                                        if debug_npc {
                                                            debug!(
                                                                "My pilot: Not adjusting speed factor, new speed factor is negative: {}",
                                                                new_speed_factor
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            if debug_npc {
                                                debug!("My pilot velocity/next pilot velocity {}, {}, speed_factor: {}", my_rate, next_pilot_rate, airship_context.speed_factor);
                                            }
                                        } else {
                                            airship_context.speed_factor = initial_speed_factor;
                                            if debug_npc {
                                                debug!("My pilot velocity/next pilot velocity {}, {}, speed_factor: {}", my_rate, next_pilot_rate, airship_context.speed_factor);
                                            }
                                        }
                                    } else {
                                        if debug_npc {
                                            debug!("My pilot velocity/next pilot velocity {}, {}, speed_factor: {}", my_rate, next_pilot_rate, airship_context.speed_factor);
                                        }
                                    }
                                } else {
                                    if debug_npc {
                                        debug!("My pilot velocity: {}", my_rate);
                                    }
                                }
                            }
                        }

                        if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Stuck(..))
                            && airship_context.next_pilot != NpcId::default()
                        {                            
                            // Get the avoidance mode for airship ahead on the route.
                            let avoidance = 
                                if let Some(pilot) = ctx.state.data().npcs.get(airship_context.next_pilot) {
                                    let pilot_wpos = pilot.wpos;
                                    /*
                                        The basic logic:
                                        If the other airship is near my docking position
                                            If I'm really close to the other airship
                                                hold position
                                            else if I'm close to the other airship
                                                slow down
                                            else
                                                do nothing
                                            end
                                        else if I'm close to the other airship
                                            slow down
                                        else
                                            do nothing
                                        end

                                        d1 = pilot ahead distance from the current approach docking position
                                        d2 = distance between my position and the position of the pilot ahead

                                        If d1 < CLOSE_TO_DOCKING_SITE_DISTANCE
                                            The other airship is near the dock
                                            If d2 < VERY_CLOSE_AIRSHIP_DISTANCE
                                                hold position
                                            else if d2 < SOMEWHAT_CLOSE_AIRSHIP_DISTANCE
                                                slow down
                                            else
                                                do nothing
                                            end
                                        else if d2 < PASSING_CLOSE_AIRSHIP_DISTANCE
                                            slow down
                                        else
                                            do nothing
                                        end                                            
                                    */
                                    let d1 = pilot_wpos.xy().distance_squared(current_approach.airship_pos.xy());
                                    let d2 = mypos.xy().distance_squared(pilot_wpos.xy());
                                    if debug_npc {
                                        debug!("My Pilot d1:{}, d2:{}", d1.sqrt(), d2.sqrt());
                                    }
                                    // Once holding, move the hold criteria outwards so that the airship
                                    // doesn't stop holding mode just due to osillations in the hold position.
                                    let close_dist_adjustment = if matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Hold(..)) {
                                        50.0f32.powi(2)
                                    } else {
                                        0.0
                                    };

                                    if d1 < CLOSE_TO_DOCKING_SITE_DISTANCE {
                                        if d2 < VERY_CLOSE_AIRSHIP_DISTANCE + close_dist_adjustment {
                                            AirshipAvoidanceMode::Hold(
                                                mypos,
                                                Dir::from_unnormalized((current_approach.dock_center - mypos.xy()).with_z(0.0))
                                                    .unwrap_or_default(),
                                            )
                                        } else if d2 < SOMEWHAT_CLOSE_AIRSHIP_DISTANCE {
                                            AirshipAvoidanceMode::SlowDown
                                        } else {
                                            AirshipAvoidanceMode::None
                                        }
                                    } else if d2 < PASSING_CLOSE_AIRSHIP_DISTANCE {
                                        AirshipAvoidanceMode::SlowDown
                                    } else {
                                        AirshipAvoidanceMode::None
                                    }
                                } else {
                                    AirshipAvoidanceMode::None
                                };
                            if matches!(avoidance, AirshipAvoidanceMode::Hold(..)){
                                // Don't reenter hold mode
                                if !matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::Hold(..)
                                ) {
                                    airship_context.did_hold = true;
                                    airship_context.avoid_mode = avoidance;
                                    airship_context.hold_timer = ctx.rng.gen_range(4.0..7.0);
                                    airship_context.hold_announced = false;
                                    if debug_npc {
                                        debug!(
                                            "My pilot: Hold position at {:?}, hold timer: {}",
                                            mypos,
                                            airship_context.hold_timer
                                        );
                                    }
                                }
                            } else if matches!(avoidance, AirshipAvoidanceMode::SlowDown) {
                                // Don't reenter slow down mode
                                if !matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::SlowDown
                                ) {
                                    airship_context.slow_count += 1;
                                    airship_context.avoid_mode = AirshipAvoidanceMode::SlowDown;
                                    airship_context.speed_factor = initial_speed_factor * SLOW_DOWN_SPEED_MULTIPLIER;
                                    if debug_npc {
                                        debug!(
                                            "My pilot: Slow down mode, count: {}, speed factor: {}",
                                            airship_context.slow_count,
                                            airship_context.speed_factor
                                        );
                                    }
                                }
                            } else {
                                // Clear the avoidance mode if it was set to hold or slow down.
                                if debug_npc && !matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::None
                                ) {
                                    debug!(
                                        "My pilot: No avoidance mode, clearing from {:?}",
                                        airship_context.avoid_mode
                                    );
                                }
                                airship_context.avoid_mode = AirshipAvoidanceMode::None;
                            }
                        }
                    }
                } else {
                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                    airship_context.speed_factor = initial_speed_factor;
                }
            } else {
                airship_context.avoidance_timer = remaining.unwrap_or(radar_interval);
            }

            if let AirshipAvoidanceMode::Stuck(unstick_target) = airship_context.avoid_mode {
                // Unstick the airship
                ctx.controller.do_goto_with_height_and_dir(
                    unstick_target,
                    1.5,
                    None,
                    None,
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) =
                airship_context.avoid_mode
            {
                airship_context.hold_timer -= ctx.dt;
                if airship_context.hold_timer <= 0.0 {
                    if !airship_context.hold_announced {
                        airship_context.hold_announced = true;
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-announce_hold"));
                    } else {
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-continue_hold"));
                    }
                    airship_context.hold_timer = ctx.rng.gen_range(10.0..20.0);
                }
                // Hold position (same idea as holding station at the dock except allow
                // oscillations)
                let hold_pos = if matches!(ctx.npc.mode, SimulationMode::Simulated) {
                    hold_pos
                } else {
                    // Airship is loaded, add some randomness to the hold position
                    // so that the airship doesn't look like it's stuck in one place.
                    // This also keeps the propellers spinning slowly and somewhat randomly.
                    hold_pos
                        + (Vec3::new(0.7, 0.8, 0.9).map(|e| (e * ctx.time.0).sin())
                            * Vec3::new(5.0, 5.0, 10.0))
                        .map(|e| e as f32)
                };

                // Holding position
                ctx.controller.do_goto_with_height_and_dir(
                    hold_pos,
                    0.25,
                    None,
                    Some(hold_dir),
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else {
                // Not holding or stuck.
                // Use terrain height offset if specified.
                let height_offset_opt = if with_terrain_following {
                    Some(height_offset)
                } else {
                    None
                };
                // Move the airship
                ctx.controller.do_goto_with_height_and_dir(
                    wpos,
                    airship_context.speed_factor,
                    height_offset_opt,
                    direction_override,
                    flight_mode,
                );
            }
        },
    )
    .repeat()
    .boxed()
    .stop_if(move |ctx: &mut NpcCtx| {
        if flight_mode == FlightMode::FlyThrough {
            // we only care about the xy distance (just get close to the target position)
            ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
        } else {
            // Braking mode means the PID controller will be controlling all three axes
            ctx.npc.wpos.distance_squared(wpos) < goal_dist.powi(2)
        }
    })
    .debug(move || {
        format!(
            "fly airship, phase:{:?}, tgt pos:({}, {}, {}), goal dist:{}, speed:{}, height:{}, \
             terrain following:{}, FlightMode:{:?}, collision avoidance:{}, radar interval:{}",
            phase,
            wpos.x,
            wpos.y,
            wpos.z,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            flight_mode,
            with_collision_avoidance,
            radar_interval.as_secs_f32()
        )
    })
    .map(|_, _| ())
}

/// Wrapper for the fly_airship action, so the route context fields can be
/// reset.
fn fly_airship_legacy(
    phase: AirshipFlightPhase,
    wpos: Vec3<f32>,
    goal_dist: f32,
    initial_speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<AirshipRouteContextLegacy> {
    now(move |_, airship_context: &mut AirshipRouteContextLegacy| {
        airship_context.speed_factor = initial_speed_factor;
        airship_context.avoidance_timer = radar_interval;
        airship_context.avoid_mode = AirshipAvoidanceMode::None;
        airship_context.trackers.clear();
        fly_airship_inner_legacy(
            phase,
            wpos,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            direction_override,
            flight_mode,
            with_collision_avoidance,
            radar_interval,
        )
    })
}

/// Called from pilot_airship_legacy to move the airship along phases of the
/// route and for initial routing after server startup. The bulk of this action
/// is collision-avoidance monitoring. The frequency of the collision-avoidance
/// tests is controlled by the radar_interval parameter.
/// The collision-avoidance logic has the ability to change the airship's speed
/// and to hold position.
///
/// # Avoidance Logic
/// All airships on the same route follow what is essentially a race track.
/// All collision issues are caused by airships catching up to each other on the
/// route. To avoid collisions, the postion and movement of other airships on
/// the route is monitored at a interval of between 2 and 4 seconds. If another
/// airship is moving towards the same docking position, it may be ahead or
/// behind the monitoring airship. If the other airship is ahead, and the
/// monitoring airship is approaching the docking position, the monitoring
/// airship will slow down or stop to avoid a potential conflict.
///
/// # Parameters
///
/// - `route_context`: The AirshipRouteContextLegacy owned by the
///   pilot_airship_legacy action.
/// - `phase`: One of the phases of the airship flight loop or reset stages.
/// - `wpos`: The fly-to target position for the airship.
/// - `goal_dist`: The distance to the target position at which this action
///   (this phase) will stop.
/// - `initial_speed_factor`: The initial speed factor for the airship. Can be
///   modified by collision-avoidance.
/// - `height_offset`: The height offset for the airship (height above terrain).
///   This is only used if with_terrain_following is true.
/// - `with_terrain_following`: Whether to follow the terrain. If true, the
///   airship will fly at a height above the terrain. If false, the airship
///   flies directly to the target position.
/// - `direction_override`: An optional direction override for the airship. If
///   Some, the airship will be oriented (pointed) in this direction.
/// - `flight_mode`: Influences the positioning of the airship. When approaching
///   or at the target position, the airship either slows (brakes) and holds or
///   flys through, expecting to continue on the next flight phase.
/// - `with_collision_avoidance`: Whether to perform collision avoidance. It's
///   not needed for docking or initial ascent because other airships must give
///   way to this airship.
/// - `radar_interval`: The interval at which to check on the positions and
///   movements of other airships on the same route.
///
/// # Returns
///
/// An Action
fn fly_airship_inner_legacy(
    phase: AirshipFlightPhase,
    wpos: Vec3<f32>,
    goal_dist: f32,
    initial_speed_factor: f32,
    height_offset: f32,
    with_terrain_following: bool,
    direction_override: Option<Dir>,
    flight_mode: FlightMode,
    with_collision_avoidance: bool,
    radar_interval: Duration,
) -> impl Action<AirshipRouteContextLegacy> {
    just(
        move |ctx, airship_context: &mut AirshipRouteContextLegacy| {
            let remaining = airship_context
                .avoidance_timer
                .checked_sub(Duration::from_secs_f32(ctx.dt));
            if remaining.is_none() {
                airship_context.avoidance_timer = radar_interval;
                // The collision avoidance checks are not done every tick (no dt required), only
                // every 2-4 seconds.
                if with_collision_avoidance {
                    if let Some(approach_index) = airship_context.current_approach_index
                        && let Some(route_id) = ctx
                            .state
                            .data()
                            .airship_sim_legacy
                            .assigned_routes
                            .get(&ctx.npc_id)
                        && let Some(route) = ctx.world.civs().airships.legacy_routes.get(route_id)
                        && let Some(approach) = route.approaches.get(approach_index)
                        && let Some(pilots) = ctx
                            .state
                            .data()
                            .airship_sim_legacy
                            .route_pilots
                            .get(route_id)
                    {
                        let mypos = ctx.npc.wpos;

                        // The intermediate reference distance is either the approach initial point
                        // (when cruising) or final point (when on
                        // approach).
                        let tracker_ref_dist = match phase {
                            AirshipFlightPhase::Cruise => Some(
                                approach
                                    .approach_initial_pos
                                    .distance_squared(approach.airship_pos.xy()),
                            ),
                            AirshipFlightPhase::ApproachFinal => Some(
                                approach
                                    .approach_final_pos
                                    .distance_squared(approach.airship_pos.xy()),
                            ),
                            _ => None,
                        };

                        if phase == AirshipFlightPhase::Cruise {
                            let position_tracker =
                                airship_context.pos_trackers.entry(ctx.npc_id).or_default();
                            position_tracker.add_position(mypos);
                            // Check if the airship is stuck in one place.
                            if position_tracker.is_stuck(ctx, &approach.approach_initial_pos)
                                && let Some(backout_pos) = position_tracker.next_backout_pos(ctx)
                            {
                                airship_context.avoid_mode =
                                    AirshipAvoidanceMode::Stuck(backout_pos);
                            } else {
                                // If the mode was stuck, but the airship is no longer stuck, clear
                                // the mode.
                                if matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::Stuck(..)
                                ) {
                                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                                }
                            }
                        }

                        if !matches!(airship_context.avoid_mode, AirshipAvoidanceMode::Stuck(..)) {
                            // Collect the avoidance modes for other airships on the route.
                            let avoidance: Vec<AirshipAvoidanceMode> = pilots
                                .iter()
                                .filter(|pilot_id| **pilot_id != ctx.npc_id)
                                .filter_map(|pilot_id| {
                                    ctx.state.data().npcs.get(*pilot_id).and_then(|pilot| {
                                    let pilot_wpos = pilot.wpos;
                                    let other_tracker = airship_context
                                        .trackers
                                        .entry(*pilot_id)
                                        .or_insert_with(|| ZoneDistanceTracker {
                                            fixed_pos: approach.airship_pos.xy(),
                                            stable_tolerance: 20.0,
                                            ref_dist: tracker_ref_dist,
                                            ..Default::default()
                                        });
                                    // the tracker determines if the other airship is moving towards
                                    // or away from my docking position
                                    // and if towards, whether it is inside of my position (ahead of
                                    // me) and if ahead, whether it is
                                    // inside the reference distance. If ahead of me but no inside
                                    // the reference distance, I should slow down.
                                    // If ahead of me and inside the reference distance, I should
                                    // slow down or hold depending on which
                                    // phase I'm in.
                                    let (trend, zone) =
                                        other_tracker.update(mypos.xy(), pilot_wpos.xy());
                                    match trend {
                                        Some(DistanceTrend::ApproachingDock) => {
                                            // other airship is moving towards the (my) docking
                                            // position
                                            match zone {
                                                Some(DistanceZone::InsideMyDistance) => {
                                                    // other airship is ahead, on the same route,
                                                    // but outside
                                                    // the reference distance (either the approach
                                                    // initial point or final point)
                                                    match phase {
                                                        // If I'm currently cruising, slow down if
                                                        // the other airship is
                                                        // within 2000 blocks
                                                        AirshipFlightPhase::Cruise => {
                                                            let dist2 = mypos
                                                                .xy()
                                                                .distance_squared(pilot_wpos.xy());
                                                            if dist2 < 2000.0f32.powi(2) {
                                                                Some(AirshipAvoidanceMode::SlowDown)
                                                            } else {
                                                                None
                                                            }
                                                        },
                                                        // If I'm currently on approach, stop and
                                                        // hold
                                                        AirshipFlightPhase::ApproachFinal => {
                                                            Some(AirshipAvoidanceMode::Hold(
                                                                mypos,
                                                                Dir::from_unnormalized(
                                                                    (approach.approach_final_pos
                                                                        - mypos.xy())
                                                                    .with_z(0.0),
                                                                )
                                                                .unwrap_or_default(),
                                                            ))
                                                        },
                                                        // If I'm currently transitioning to above
                                                        // the dock, hold position
                                                        AirshipFlightPhase::Transition => {
                                                            Some(AirshipAvoidanceMode::Hold(
                                                                mypos,
                                                                approach.airship_direction,
                                                            ))
                                                        },
                                                        _ => None,
                                                    }
                                                },
                                                Some(DistanceZone::InsideReference) => {
                                                    // other airship is ahead, on the same route,
                                                    // and inside
                                                    // the reference distance (either the approach
                                                    // initial point or final point)
                                                    match phase {
                                                        // If I'm currently on approach, slow down,
                                                        // to eventually
                                                        // hold near the dock.
                                                        AirshipFlightPhase::ApproachFinal => {
                                                            Some(AirshipAvoidanceMode::SlowDown)
                                                        },
                                                        // else I'm cruising, the other airship is
                                                        // well ahead, and
                                                        // I might eventually have to hold nearer
                                                        // the dock.
                                                        // There is no reference distance if on
                                                        // final.
                                                        _ => None,
                                                    }
                                                },
                                                // else other airship is behind me, ignore
                                                _ => None,
                                            }
                                        },
                                        // other airship is at the dock (or desending to the dock)
                                        Some(DistanceTrend::Docked) => {
                                            // other airship is ahead, on the same route, near the
                                            // dock.
                                            match phase {
                                                // If I'm currently on approach, slow down, to
                                                // eventually probably hold near the dock.
                                                AirshipFlightPhase::ApproachFinal => {
                                                    Some(AirshipAvoidanceMode::SlowDown)
                                                },
                                                // If I'm currently transitioning to above the dock,
                                                // hold position
                                                AirshipFlightPhase::Transition => {
                                                    Some(AirshipAvoidanceMode::Hold(
                                                        mypos,
                                                        approach.airship_direction,
                                                    ))
                                                },
                                                // otherwise continue until some other condition is
                                                // met
                                                _ => None,
                                            }
                                        },
                                        // else other airship is moving away from my dock or there
                                        // isn't enough data to determine the trend.
                                        // Do nothing.
                                        _ => None,
                                    }
                                })
                                })
                                .collect();

                            if let Some((hold_pos, hold_dir)) = avoidance.iter().find_map(|mode| {
                                if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) = mode {
                                    Some((hold_pos, hold_dir))
                                } else {
                                    None
                                }
                            }) {
                                if !matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::Hold(..)
                                ) {
                                    airship_context.did_hold = true;
                                    airship_context.avoid_mode =
                                        AirshipAvoidanceMode::Hold(*hold_pos, *hold_dir);
                                    airship_context.hold_timer = ctx.rng.gen_range(4.0..7.0);
                                    airship_context.hold_announced = false;
                                }
                            } else if avoidance
                                .iter()
                                .any(|mode| matches!(mode, AirshipAvoidanceMode::SlowDown))
                            {
                                if !matches!(
                                    airship_context.avoid_mode,
                                    AirshipAvoidanceMode::SlowDown
                                ) {
                                    airship_context.slow_count += 1;
                                    airship_context.avoid_mode = AirshipAvoidanceMode::SlowDown;
                                    airship_context.speed_factor = initial_speed_factor * 0.5;
                                }
                            } else {
                                airship_context.avoid_mode = AirshipAvoidanceMode::None;
                                airship_context.speed_factor = initial_speed_factor;
                            }
                        }
                    }
                } else {
                    airship_context.avoid_mode = AirshipAvoidanceMode::None;
                    airship_context.speed_factor = initial_speed_factor;
                }
            } else {
                airship_context.avoidance_timer = remaining.unwrap_or(radar_interval);
            }

            if let AirshipAvoidanceMode::Stuck(unstick_target) = airship_context.avoid_mode {
                // Unstick the airship
                ctx.controller.do_goto_with_height_and_dir(
                    unstick_target,
                    1.5,
                    None,
                    None,
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else if let AirshipAvoidanceMode::Hold(hold_pos, hold_dir) =
                airship_context.avoid_mode
            {
                airship_context.hold_timer -= ctx.dt;
                if airship_context.hold_timer <= 0.0 {
                    if !airship_context.hold_announced {
                        airship_context.hold_announced = true;
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-announce_hold"));
                    } else {
                        ctx.controller
                            .say(None, Content::localized("npc-speech-pilot-continue_hold"));
                    }
                    airship_context.hold_timer = ctx.rng.gen_range(10.0..20.0);
                }
                // Hold position (same idea as holding station at the dock except allow
                // oscillations)
                let hold_pos = if matches!(ctx.npc.mode, SimulationMode::Simulated) {
                    hold_pos
                } else {
                    // Airship is loaded, add some randomness to the hold position
                    // so that the airship doesn't look like it's stuck in one place.
                    // This also keeps the propellers spinning slowly and somewhat randomly.
                    hold_pos
                        + (Vec3::new(0.7, 0.8, 0.9).map(|e| (e * ctx.time.0).sin())
                            * Vec3::new(5.0, 5.0, 10.0))
                        .map(|e| e as f32)
                };

                // Holding position
                ctx.controller.do_goto_with_height_and_dir(
                    hold_pos,
                    0.25,
                    None,
                    Some(hold_dir),
                    FlightMode::Braking(BrakingMode::Normal),
                );
            } else {
                // use terrain height offset if specified
                let height_offset_opt = if with_terrain_following {
                    Some(height_offset)
                } else {
                    None
                };
                // Move the airship
                ctx.controller.do_goto_with_height_and_dir(
                    wpos,
                    airship_context.speed_factor,
                    height_offset_opt,
                    direction_override,
                    flight_mode,
                );
            }
        },
    )
    .repeat()
    .boxed()
    .stop_if(move |ctx: &mut NpcCtx| {
        if flight_mode == FlightMode::FlyThrough {
            // we only care about the xy distance (just get close to the target position)
            ctx.npc.wpos.xy().distance_squared(wpos.xy()) < goal_dist.powi(2)
        } else {
            // Braking mode means the PID controller will be controlling all three axes
            ctx.npc.wpos.distance_squared(wpos) < goal_dist.powi(2)
        }
    })
    .debug(move || {
        format!(
            "fly airship, phase:{:?}, tgt pos:({}, {}, {}), goal dist:{}, speed:{}, height:{}, \
             terrain following:{}, FlightMode:{:?}, collision avoidance:{}, radar interval:{}",
            phase,
            wpos.x,
            wpos.y,
            wpos.z,
            goal_dist,
            initial_speed_factor,
            height_offset,
            with_terrain_following,
            flight_mode,
            with_collision_avoidance,
            radar_interval.as_secs_f32()
        )
    })
    .map(|_, _| ())
}

/// Get the target position for airship movement given the target position, the
/// default height above terrain, and the height above terrain for the airship
/// route cruise phase. This samples terrain points aound the target pos to get
/// the maximum terrain altitude in a 200 block radius of the target position
/// (only checking 4 cardinal directions). and returns the input approach_pos
/// with z equal to the maximum terrain alt + height or the default_alt
/// whichever is greater.
fn approach_target_pos(
    ctx: &mut NpcCtx,
    approach_pos: Vec2<f32>,
    default_alt: f32,
    height: f32,
) -> Vec3<f32> {
    // sample 4 terrain altitudes around the final approach point and take the max.
    let max_alt = CARDINALS
        .iter()
        .map(|rpos| {
            ctx.world
                .sim()
                .get_surface_alt_approx(approach_pos.as_() + rpos * 200)
        })
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
        .unwrap_or(default_alt);
    approach_pos.with_z(max_alt + height)
}

/// Calculates how to resume a route. Called when the server starts up.
/// The airship will be at the approach final point, and this function
/// just needs to figure out to which approach index that correlates.
/// Returns the index of the approach to resume on.
fn resume_route(airships: &Airships, route_id: &u32, ctx: &mut NpcCtx) -> usize {
    if let Some(route) = airships.legacy_routes.get(route_id) {
        route
            .approaches
            .iter()
            .enumerate()
            .min_by_key(|(_, approach)| {
                approach
                    .approach_final_pos
                    .distance_squared(ctx.npc.wpos.xy()) as i32
            })
            .map(|(index, _)| index)
            .unwrap_or(0)
    } else {
        0
    }
}

/// The NPC is the airship captain. This action defines the flight loop for the
/// airship. The captain NPC is autonomous and will fly the airship along the
/// assigned route. The routes are established and assigned to the captain NPCs
/// when the world is generated.
pub fn pilot_airship<S: State>() -> impl Action<S> {
    now(move |ctx, route_context: &mut AirshipRouteContext| {
        // get the assigned route and start leg indexes
        if let Some((route_index, start_leg_index)) = ctx.state.data().airship_sim.assigned_routes.get(&ctx.npc_id)
        {    
            // At this point in the code, the airship is at the end of current route leg, about to descend and dock.
            // If route_context.route_index is the default value (usize::MAX) it means the server has just started.

            let mut docking_delay = 0.02;

            if route_context.route_index == usize::MAX {
                // The server has just started, so we need to set up the route context fixed values.
                route_context.route_index = *route_index;
                route_context.next_leg = *start_leg_index;
                if let Some(next_pilot_entry) = ctx.state.data().airship_sim.next_pilots.get(&ctx.npc_id)
                    && let Some(next_pilot_id) = next_pilot_entry
                {
                    route_context.next_pilot = *next_pilot_id;
                }
                route_context.simulated_airship_speed =
                    ctx.state.data().npcs.mounts.get_mount_link(ctx.npc_id)
                        .map(|mount_link|
                            ctx.state.data().npcs.get(mount_link.mount)
                                .map(|airship| {
                                    airship.body.max_speed_approx()
                                })
                                .unwrap_or_default()
                        ).or_else(|| {
                            // If the mount link is not found, use a default speed.
                            Some(0.0)
                        })
                        .unwrap_or(0.0);



                docking_delay = ctx.rng.gen_range(1..5) as f64 * 10.0;
                debug!(
                    "Server startup, airship pilot {:?} starting on route {} at end of leg {}, following pilot {:?}",
                    ctx.npc_id,
                    route_context.route_index,
                    route_context.next_leg,
                    route_context.next_pilot
                );
                if route_context.next_pilot == NpcId::default() {
                    tracing::error!("Pilot {:?} has no next pilot to follow.", ctx.npc_id);
                }
            }
            // The airship is continuing the flight loop.
            
            // set the approach data for the current leg
            // Needed: docking position and direction.
            route_context.current_leg_approach = Some(ctx.world.civs().airships.approach_for_route_and_leg(
                route_context.route_index,
                route_context.next_leg,
            ));
            // Increment the leg index with wrap arounc
            route_context.next_leg = ctx.world.civs().airships.increment_route_leg(
                route_context.route_index,
                route_context.next_leg,
            );
            // set the approach data for the next leg
            // Needed: transition position for next leg.
            route_context.next_leg_approach = Some(ctx.world.civs().airships.approach_for_route_and_leg(
                route_context.route_index,
                route_context.next_leg,
            ));

            if route_context.current_leg_approach.is_none() ||
                route_context.next_leg_approach.is_none()
            {
                tracing::error!(
                    "Airship pilot {:?} approaches not found for route {} leg {}, stopping pilot_airship loop.",
                    ctx.npc_id,
                    route_context.route_index,
                    route_context.next_leg
                );
                return finish().map(|_, _| ()).boxed();
            }

            let current_approach = route_context.current_leg_approach.unwrap();
            let next_approach = route_context.next_leg_approach.unwrap();

            let next_leg_cruise_dir = (next_approach.approach_transition_pos.xy() - current_approach.airship_pos.xy()).normalized();
            route_context.next_leg_cruise_checkpoint_pos = (next_approach.approach_transition_pos - next_leg_cruise_dir * CRUISE_CHECKPOINT_DISTANCE).with_z(next_approach.approach_transition_pos.z);

            let speed_factor_fn = |sim_mode: SimulationMode, speed_factor: f32| {
                // The speed factor for simulated airships is always 1.0
                if matches!(sim_mode, SimulationMode::Simulated) {
                    1.0
                } else {
                    speed_factor
                }
            };

            // Hover at the approach transition point before docking.
            // If the server has just started, delay for a random time so that not all
            // airships land at the same time.
            just(move |ctx, _| {
                ctx.controller
                .do_goto_with_height_and_dir(
                    current_approach.approach_transition_pos,
                    speed_factor_fn(ctx.npc.mode, 0.4),
                    Some(current_approach.height),
                    Some(current_approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Precise),
                );
            })
            .repeat()
            .stop_if(timeout(docking_delay))

            // Regular Flight Loop
            // Fly 3D to directly above the docking position, full PID control

            .then(
                fly_airship(
                    AirshipFlightPhase::Transition,
                    current_approach.airship_pos + Vec3::unit_z() * current_approach.height,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.4),
                    current_approach.height,
                    true,
                    Some(current_approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(2.0),
            ))
            // Descend and Dock
            //    Docking
            //      Drop to 125 blocks above the dock with slightly looser controller settings
            //      then to the docking position with full precision control.
            //      This helps to ensure that the airship docks vertically and avoids collisions
            //      with other airships and the dock. The speed_factor is high to
            //      give a strong response to the PID controller. The speed_factor is reduced
            //      once docked to stop the airship propellers from rotating.
            //      Vary the timeout to get variation in the docking sequence.
            .then(
                // descend to 125 blocks above the dock
                just(move |ctx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos + Vec3::unit_z() * 125.0,
                            speed_factor_fn(ctx.npc.mode, 0.8),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Normal),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.gen_range(10.0..14.0))))
            // .then(
            //     // descend to 35 blocks above the dock
            //     just(move |ctx, _| {
            //         ctx.controller
            //             .do_goto_with_height_and_dir(
            //                 current_approach.airship_pos + Vec3::unit_z() * 35.0,
            //                 0.7, None,
            //                 Some(current_approach.airship_direction),
            //                 FlightMode::Braking(BrakingMode::Normal),
            //             );
            //     })
            //     .repeat()
            //     .stop_if(timeout(ctx.rng.gen_range(7.0..9.5))))
            .then(
                // descend to docking position
                just(move |ctx: &mut NpcCtx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos,
                            speed_factor_fn(ctx.npc.mode, 0.7),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                })
                .repeat()
                // .stop_if(timeout(ctx.rng.gen_range(6.0..8.0))))
                .stop_if(timeout(ctx.rng.gen_range(12.0..16.0))))
            // Announce arrival
            .then(just(|ctx: &mut NpcCtx, _| {
                ctx.controller
                    .say(None, Content::localized("npc-speech-pilot-landed"));
            }))

            // Docked - Wait at Dock
            .then(
                now(move |ctx, route_context:&mut AirshipRouteContext| {
                    // The extra time to hold at the dock is a route_context variable.
                    // Adjust the extra time based on the airship's behavior during the previous
                    // flight. If the airship had to hold position, add 30 seconds to the dock time.
                    // If the airship did not hold position, subtract 45 seconds from the dock time
                    // because we want to reverse the extra penalty faster than building it up in case
                    // the airship transitions to simulation mode.
                    // If the airship had to slow down add 15 and if not subtract 20.
                    if route_context.did_hold {
                        route_context.extra_hold_dock_time += 30.0;
                    } else if route_context.extra_hold_dock_time > 45.0 {
                        route_context.extra_hold_dock_time -= 45.0;
                    } else {
                        route_context.extra_hold_dock_time = 0.0;
                    }
                    if route_context.slow_count > 0 {
                        route_context.extra_slowdown_dock_time += 15.0;
                    } else if route_context.extra_slowdown_dock_time > 20.0 {
                        route_context.extra_slowdown_dock_time -= 20.0;
                    } else {
                        route_context.extra_slowdown_dock_time = 0.0;
                    }

                    let docking_time = route_context.extra_hold_dock_time + route_context.extra_slowdown_dock_time + Airships::docking_duration();
                    #[cfg(debug_assertions)]
                    {
                        let docked_site_name = ctx.index.sites.get(current_approach.site_id).name().to_string();
                        debug!("{}, Docked at {}, did_hold:{}, slow_count:{}, extra_hold_dock_time:{}, extra_slowdown_dock_time:{}, docking_time:{}", format!("{:?}", ctx.npc_id), docked_site_name, route_context.did_hold, route_context.slow_count, route_context.extra_hold_dock_time, route_context.extra_slowdown_dock_time, docking_time);
                    }
                    route_context.did_hold = false;
                    route_context.slow_count = 0;

                    just(move |ctx, _| {
                        ctx.controller
                        .do_goto_with_height_and_dir(
                            current_approach.airship_pos,
                            speed_factor_fn(ctx.npc.mode, 0.75),
                            None,
                            Some(current_approach.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                    })
                    .repeat()
                    .stop_if(timeout(ctx.rng.gen_range(10.0..16.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, route_context:&mut AirshipRouteContext| {
                            // get the name of the site the airship is going to next.
                            // The route_context.current_approach_index has not been switched yet,
                            // so the index is the opposite of the current approach index.
                            let next_site_name = ctx.index.sites.get(next_approach.site_id).name().to_string();
                            ctx.controller.say(
                                None,
                                Content::localized_with_args("npc-speech-pilot-announce_next", [
                                (
                                    "dir",
                                    Direction::from_dir((next_approach.approach_transition_pos - ctx.npc.wpos).xy()).localize_npc(),
                                ),
                                ("dst", Content::Plain(next_site_name.to_string())),
                                ]),
                            );
                        })
                    )
                    .repeat()
                    .stop_if(timeout(docking_time as f64))
                })
            ).then(
                // announce takeoff
                just(move |ctx, route_context:&mut AirshipRouteContext| {
                    ctx.controller.say(
                    None,
                        Content::localized_with_args("npc-speech-pilot-takeoff", [
                            ("src", Content::Plain(ctx.index.sites.get(current_approach.site_id).name().to_string())),
                            ("dst", Content::Plain(ctx.index.sites.get(next_approach.site_id).name().to_string())),
                        ]),
                    );
                })
            ).then(
                // Take off, full PID control
                fly_airship(
                    AirshipFlightPhase::Ascent,
                    current_approach.airship_pos + Vec3::unit_z() * 100.0,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.2),
                    0.0,
                    false,
                    Some(current_approach.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
            ).then(
                // Ascend to Cruise Alt, full PID control
                fly_airship(
                    AirshipFlightPhase::Ascent,
                    current_approach.airship_pos + Vec3::unit_z() * current_approach.height,
                    20.0,
                    speed_factor_fn(ctx.npc.mode, 0.4),
                    0.0,
                    false,
                    Some(Dir::from_unnormalized((next_approach.approach_transition_pos - ctx.npc.wpos).xy().with_z(0.0)).unwrap_or_default()),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
            ).then(
                // Fly 2D to Destination Transition Point
                fly_airship(
                    AirshipFlightPhase::Cruise,
                    route_context.next_leg_cruise_checkpoint_pos,
                    50.0,
                    speed_factor_fn(ctx.npc.mode, 1.0),
                    next_approach.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                    true,
                    Duration::from_secs_f32(5.0),
                )
            ).then(
                // Fly 2D to Destination Transition Point
                fly_airship(
                    AirshipFlightPhase::Cruise,
                    next_approach.approach_transition_pos,
                    50.0,
                    speed_factor_fn(ctx.npc.mode, 1.0),
                    next_approach.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                    true,
                    Duration::from_secs_f32(1.0),
                )
            )
            .map(|_, _| ()).boxed()
        } else {
            //  There are no routes assigned.
            //  This is unexpected and never happens in testing, just do nothing so the compiler doesn't complain.
            finish().map(|_, _| ()).boxed()
        }
    })
    .repeat()
    .with_state(AirshipRouteContext::default())
    .map(|_, _| ())
}

/// The NPC is the airship captain. This action defines the flight loop for the
/// airship. The captain NPC is autonomous and will fly the airship along the
/// assigned route. The routes are established and assigned to the captain NPCs
/// when the world is generated.
pub fn pilot_airship_legacy<S: State>() -> impl Action<S> {
    /*
        Phases of the airship flight:
        1. Docked at a docking position. Hovering in place. Mode 3d with direction vector.
        2. Ascending staight up, to flight transition point. Mode 3d with direction vector.
        3. Cruising to destination initial point. Mode 2d with variable height offset (from terrain).
        4. Flying to final point. Mode 3d with direction vector.
        5. Flying to docking transition point. Mode 3d with direction vector.
        6. Descending and docking. Mode 3d with direction vector.

                                                                                +
                                                                                +...+
                                                                                +  <------------ Docking Position
                                                                                |
                                                            Aligned with        |
                                                            Docking Direction   |
                                                                                |
                                                                                |
                                                                                |
                                                                Turn Towards    | <----------- Final Point
                                                                Docking Pos    /-
        Docked Position (start)                                              /-
            |                                              Turn Towards    /--
            |                                              Final Point  /-
            -> -------------------Cruise------------------------------  <-------------------- Initial Point
            +
            +...+
            +

        Algorithm for piloting the airship:

        Phase  State    Parameters                      Completion Conditions   Collision Avoidance
        1      Docked   3D Position, Dir                Docking Timeout         No
        2      Ascent   3D Position, Dir                Altitude reached        No
        3      Cruise   2D Position, Height             2D Position reached     Yes
        4      Approach 3D Position, Height, Dir        2D Position reached     Yes
        5      Final    3D Position, Dir                3D Position reached     Yes
        6      Landing  3D Position, Dir                3D Position reached     No
    */

    now(move |ctx, route_context: &mut AirshipRouteContextLegacy| {

        // get the assigned route
        if let Some(route_id) = ctx.state.data().airship_sim_legacy.assigned_routes.get(&ctx.npc_id)
            && let Some(route) = ctx.world.civs().airships.legacy_routes.get(route_id) {

            route_context.site_ids = Some([route.approaches[0].site_id, route.approaches[1].site_id]);

            // pilot_airship_legacy action is called the first time, there will be no current approach index
            // but the airship will be sitting at the approach final point.
            let (current_approach_index, resuming) = if let Some(current_approach_index) = route_context.current_approach_index {
                (current_approach_index, false)
            } else {
                let resume_approach_index = resume_route(&ctx.world.civs().airships, route_id, ctx);
                route_context.current_approach_index = Some(resume_approach_index);
                (resume_approach_index, true)
            };

            // when current_approach_index exists, it means we're repeating the flight loop
            // if approach index is 0, then the airship is fly from site 0 to site 1, and vice versa

            // the approach flips in the middle of this loop after waiting at the dock.
            // The route_context.current_approach_index is used to determine the current approach at the
            // top of the function but any changes to route_context are not seen until the next iteration.
            // For this loop, these are the two approaches. We use #2 after docking is completed.

            let approach1 = route.approaches[current_approach_index].clone();
            let approach2 = route.approaches[(current_approach_index + 1) % 2].clone();

            // Startup delay
            // If the server has just started (resuming is true)
            // then the airship is at the approach final point.
            // It should hover there for a random time so that not all airships land at the same time.
            let start_delay = if resuming {
                ctx.rng.gen_range(1..5) as f64 * 10.0
            } else {
                0.02
            };
            just(move |ctx, _| {
                ctx.controller
                .do_goto_with_height_and_dir(
                    approach1.approach_final_pos.with_z(approach1.airship_pos.z + approach1.height - AIRSHIP_APPROACH_FINAL_HEIGHT_DELTA),
                    0.4, None,
                    Some(approach1.airship_direction),
                    FlightMode::Braking(BrakingMode::Precise),
                );
            })
            .repeat()
            .stop_if(timeout(start_delay))

            // Regular Flight Loop
            // Fly 3D to Docking Transition Point, full PID control

            .then(
                fly_airship_legacy(
                    AirshipFlightPhase::Transition,
                    approach1.airship_pos + Vec3::unit_z() * (approach1.height - AIRSHIP_DOCK_TRANSITION_HEIGHT_DELTA),
                    50.0,
                    0.4,
                    approach1.height - AIRSHIP_DOCK_TRANSITION_HEIGHT_DELTA,
                    true,
                    Some(approach1.airship_direction),
                    FlightMode::Braking(BrakingMode::Normal),
                    true,
                    Duration::from_secs_f32(2.0),
            ))
            // Descend and Dock
            //    Docking
            //      Stop in increments and settle at 150 blocks and then 50 blocks from the dock.
            //      This helps to ensure that the airship docks vertically and avoids collisions
            //      with other airships and the dock. The speed_factor is high to
            //      give a strong response to the PID controller for the first
            //      three docking phases. The speed_factor is reduced for the final docking phase
            //      to give the impression that the airship propellers are not rotating.
            //      Vary the timeout to get variation in the docking sequence.
            .then(
                // descend to 125 blocks above the dock
                just(move |ctx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos + Vec3::unit_z() * 125.0,
                            0.8, None,
                            Some(approach1.airship_direction),
                            FlightMode::Braking(BrakingMode::Normal),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.gen_range(10.0..14.0))))
            .then(
                // descend to 35 blocks above the dock
                just(move |ctx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos + Vec3::unit_z() * 35.0,
                            0.7, None,
                            Some(approach1.airship_direction),
                            FlightMode::Braking(BrakingMode::Normal),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.gen_range(7.0..9.5))))
            .then(
                // descend to docking position
                just(move |ctx: &mut NpcCtx, _| {
                    ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos,
                            0.7, None,
                            Some(approach1.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                })
                .repeat()
                .stop_if(timeout(ctx.rng.gen_range(6.0..8.0))))
            // Announce arrival
            .then(just(|ctx: &mut NpcCtx, _| {
                ctx.controller
                    .say(None, Content::localized("npc-speech-pilot-landed"));
            }))

            // Docked - Wait at Dock
            .then(
                now(move |ctx, route_context:&mut AirshipRouteContextLegacy| {
                    // The extra time to hold at the dock is a route_context variable.
                    // Adjust the extra time based on the airship's behavior during the previous
                    // flight. If the airship had to hold position, add 30 seconds to the dock time.
                    // If the airship did not hold position, subtract 45 seconds from the dock time
                    // because we want to reverse the extra penalty faster than building it up in case
                    // the airship transitions to simulation mode.
                    // If the airship had to slow down add 15 and if not subtract 20.
                    if route_context.did_hold {
                        route_context.extra_hold_dock_time += 30.0;
                    } else if route_context.extra_hold_dock_time > 45.0 {
                        route_context.extra_hold_dock_time -= 45.0;
                    } else {
                        route_context.extra_hold_dock_time = 0.0;
                    }
                    if route_context.slow_count > 0 {
                        route_context.extra_slowdown_dock_time += 15.0;
                    } else if route_context.extra_slowdown_dock_time > 20.0 {
                        route_context.extra_slowdown_dock_time -= 20.0;
                    } else {
                        route_context.extra_slowdown_dock_time = 0.0;
                    }

                    let docking_time = route_context.extra_hold_dock_time + route_context.extra_slowdown_dock_time + Airships::docking_duration();
                    #[cfg(debug_assertions)]
                    {
                        if let Some(site_ids) = route_context.site_ids
                        && let Some(approach_index) = route_context.current_approach_index {
                            let docked_site_id = site_ids[approach_index];
                            let docked_site_name = ctx.index.sites.get(docked_site_id).name().to_string();
                            debug!("{}, Docked at {}, did_hold:{}, slow_count:{}, extra_hold_dock_time:{}, extra_slowdown_dock_time:{}, docking_time:{}", format!("{:?}", ctx.npc_id), docked_site_name, route_context.did_hold, route_context.slow_count, route_context.extra_hold_dock_time, route_context.extra_slowdown_dock_time, docking_time);
                        }
                    }
                    route_context.did_hold = false;
                    route_context.slow_count = 0;

                    just(move |ctx, _| {
                        ctx.controller
                        .do_goto_with_height_and_dir(
                            approach1.airship_pos,
                            0.75, None,
                            Some(approach1.airship_direction),
                            FlightMode::Braking(BrakingMode::Precise),
                        );
                    })
                    .repeat()
                    .stop_if(timeout(ctx.rng.gen_range(10.0..16.0)))
                    // While waiting, every now and then announce where the airship is going next.
                    .then(
                        just(move |ctx, route_context:&mut AirshipRouteContextLegacy| {
                            // get the name of the site the airship is going to next.
                            // The route_context.current_approach_index has not been switched yet,
                            // so the index is the opposite of the current approach index.
                            if let Some(site_ids) = route_context.site_ids {
                                let next_site_id = site_ids[(route_context.current_approach_index.unwrap_or(0) + 1) % 2];
                                let next_site_name = ctx.index.sites.get(next_site_id).name().to_string();
                                ctx.controller.say(
                                    None,
                                    Content::localized_with_args("npc-speech-pilot-announce_next", [
                                    (
                                        "dir",
                                        Direction::from_dir(approach2.approach_initial_pos - ctx.npc.wpos.xy()).localize_npc(),
                                    ),
                                    ("dst", Content::Plain(next_site_name.to_string())),
                                    ]),
                                );
                            }
                        })
                    )
                    .repeat()
                    .stop_if(timeout(docking_time as f64))
                })
            ).then(
                // rotate the approach to the next approach index. Note the approach2 is already known,
                // this is just changing the approach index in the context data for the next loop.
                just(move |ctx, route_context:&mut AirshipRouteContextLegacy| {
                    let from_index = route_context.current_approach_index.unwrap_or(0);
                    let next_approach_index = (from_index + 1) % 2;
                    route_context.current_approach_index = Some(next_approach_index);
                    if let Some(site_ids) = route_context.site_ids {
                        ctx.controller.say(
                        None,
                            Content::localized_with_args("npc-speech-pilot-takeoff", [
                                ("src", Content::Plain(ctx.index.sites.get(site_ids[from_index]).name().to_string())),
                                ("dst", Content::Plain(ctx.index.sites.get(site_ids[next_approach_index]).name().to_string())),
                            ]),
                        );
                    }
                })
            ).then(
                // Ascend to Cruise Alt, full PID control
                fly_airship_legacy(
                    AirshipFlightPhase::Ascent,
                    approach1.airship_pos + Vec3::unit_z() * Airships::takeoff_ascent_height(),
                    50.0,
                    0.2,
                    0.0,
                    false,
                    Some(Dir::from_unnormalized((approach2.approach_initial_pos - ctx.npc.wpos.xy()).with_z(0.0)).unwrap_or_default()),
                    FlightMode::Braking(BrakingMode::Normal),
                    false,
                    Duration::from_secs_f32(120.0),
                )
            ).then(
                // Fly 2D to Destination Initial Point
                fly_airship_legacy(
                    AirshipFlightPhase::Cruise,
                    approach_target_pos(ctx, approach2.approach_initial_pos, approach2.airship_pos.z + approach2.height, approach2.height),
                    250.0,
                    1.0,
                    approach2.height,
                    true,
                    None,
                    FlightMode::FlyThrough,
                    true,
                    Duration::from_secs_f32(4.0),
                )
            ).then(
                // Fly 3D to Destination Final Point, z PID control
                fly_airship_legacy(
                    AirshipFlightPhase::ApproachFinal,
                    approach_target_pos(ctx, approach2.approach_final_pos, approach2.airship_pos.z + approach2.height - AIRSHIP_APPROACH_FINAL_HEIGHT_DELTA, approach2.height - AIRSHIP_APPROACH_FINAL_HEIGHT_DELTA),
                    250.0,
                    0.5,
                    approach2.height - AIRSHIP_APPROACH_FINAL_HEIGHT_DELTA,
                    true,
                    Some(approach2.airship_direction),
                    FlightMode::FlyThrough,
                    true,
                    Duration::from_secs_f32(2.0),
                )
            ).map(|_, _| ()).boxed()
        } else {
            //  There are no routes assigned.
            //  This is unexpected and never happens in testing, just do nothing so the compiler doesn't complain.
            finish().map(|_, _| ()).boxed()
        }
    })
    .repeat()
    .with_state(AirshipRouteContextLegacy::default())
    .map(|_, _| ())
}

#[cfg(test)]
mod tests {
    use super::{DistanceTrend, DistanceZone, ZoneDistanceTracker, MovingAverage};
    use vek::{Vec2, Vec3};

    #[test]
    fn transition_zone_other_approaching_test() {
        let dock_pos = Vec2::new(0.0, 0.0);
        let my_pos: Vec2<f32> = Vec2::new(0.0, -100.0);
        let mut other_pos = Vec2::new(0.0, -50.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ..Default::default()
        };
        for _ in 0..50 {
            other_pos.y += 1.0;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::DepartingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn transition_zone_other_docked_test() {
        let dock_pos = Vec3::new(1050.0, 8654.33, 874.2);
        let my_pos: Vec3<f32> = Vec3::new(1000.0, 8454.33, 574.2);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos.xy(),
            stable_tolerance: 20.0,
            ..Default::default()
        };
        let time_0 = 27334.98f64;
        for i in 0..100 {
            let other_pos = dock_pos
                + (Vec3::new(0.7, 0.8, 0.9).map(|e| e * (time_0 + i as f64 * 1.37).sin())
                    * Vec3::new(5.0, 5.0, 10.0))
                .map(|e| e as f32)
                .xy();
            let (trend, zone) = tracker.update(my_pos.xy(), other_pos.xy());
            assert!(matches!(trend, Some(DistanceTrend::Docked) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn transition_zone_other_departing_test() {
        let dock_pos = Vec2::new(0.0, 0.0);
        let my_pos: Vec2<f32> = Vec2::new(-100.0, -100.0);
        let mut other_pos = Vec2::new(0.0, -1.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ..Default::default()
        };
        for _ in 0..50 {
            other_pos.y -= 1.0;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::ApproachingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_behind_test() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 4501.8);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (my_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::OutsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_in_zone2() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 5407.3);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (afp_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_departing_in_zone2() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9965.0, 5518.3);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_y = (my_pos.y - other_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::DepartingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideMyDistance) | None));
        }
    }

    #[test]
    fn approach_other_approaching_in_zone1() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(10655.0, 5518.7);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = (dock_pos.x - other_pos.x) / 50.0;
        let step_y = (dock_pos.y - other_pos.y) / 50.0;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::ApproachingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_docked() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let time_0 = 354334.98f64;
        for i in 0..50 {
            let other_pos = dock_pos
                + (Vec3::new(0.7, 0.8, 0.9).map(|e| e * (time_0 + i as f64 * 1.37).sin())
                    * Vec3::new(5.0, 5.0, 10.0))
                .map(|e| e as f32)
                .xy();
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::Docked) | None));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_departing_in_zone1() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(10987.0, 5634.0);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = (afp_pos.x - dock_pos.x) / 51.0;
        let step_y = (afp_pos.y - dock_pos.y) / 51.0;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(!(matches!(trend, Some(DistanceTrend::ApproachingDock))));
            assert!(matches!(zone, Some(DistanceZone::InsideReference) | None));
        }
    }

    #[test]
    fn approach_other_departing_behind() {
        let dock_pos = Vec2::new(10987.0, 5634.0);
        let afp_pos = Vec2::new(10642.0, 5518.5);
        let my_pos: Vec2<f32> = Vec2::new(9965.12, 5407.23);
        let mut other_pos = Vec2::new(9964.8, 5406.55);
        let mut tracker = ZoneDistanceTracker {
            fixed_pos: dock_pos,
            stable_tolerance: 20.0,
            ref_dist: Some(afp_pos.distance_squared(dock_pos)),
            ..Default::default()
        };
        let step_x = -11.37;
        let step_y = -23.87;
        for _ in 0..50 {
            other_pos.x += step_x;
            other_pos.y += step_y;
            let (trend, zone) = tracker.update(my_pos, other_pos);
            assert!(matches!(trend, Some(DistanceTrend::DepartingDock) | None));
            assert!(matches!(zone, Some(DistanceZone::OutsideMyDistance) | None));
        }
    }

    #[test]
    fn moving_average_test() {
        let mut ma: MovingAverage<f32> = MovingAverage::new(5);
        ma.add(1.0);
        ma.add(2.0);
        ma.add(3.0);
        ma.add(4.0);
        ma.add(5.0);
        assert_eq!(ma.average(), 3.0);

        ma.add(6.0); // This will remove the first value (1.0)
        assert_eq!(ma.average(), 4.0);

        ma.add(7.0); // This will remove the second value (2.0)
        assert_eq!(ma.average(), 5.0);

        ma.add(8.0); // This will remove the third value (3.0)
        assert_eq!(ma.average(), 6.0);

        ma.add(9.0); // This will remove the fourth value (4.0)
        assert_eq!(ma.average(), 7.0);

        ma.add(10.0); // This will remove the fifth value (5.0)
        assert_eq!(ma.average(), 8.0);

        let mut ma2: MovingAverage<i64> = MovingAverage::new(5);
        ma2.add((1000.0f32/1000.0) as i64);
        ma2.add((2000.0f32/1000.0) as i64);
        ma2.add((3000.0f32/1000.0) as i64);
        ma2.add((4000.0f32/1000.0) as i64);
        ma2.add((5000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 3);

        ma2.add((6000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 4);

        ma2.add((7000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 5);

        ma2.add((8000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 6);

        ma2.add((9000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 7);

        ma2.add((10000.0f32/1000.0) as i64);
        assert_eq!(ma2.average(), 8);

        let mut ma3: MovingAverage<i64> = MovingAverage::new(5);
        ma3.add((20.99467f32*10000.0) as i64);
        ma3.add((20.987871f32*10000.0) as i64);
        ma3.add((20.69861f32*10000.0) as i64);
        ma3.add((20.268217f32*10000.0) as i64);
        ma3.add((20.230164f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.6358).abs() < 0.0001);

        ma3.add((20.48151f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.5332).abs() < 0.0001);

        ma3.add((20.568598f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.4493).abs() < 0.0001);

        ma3.add((20.909971f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.4916).abs() < 0.0001);

        ma3.add((21.014437f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.6408).abs() < 0.0001);

        ma3.add((20.62308f32*10000.0) as i64);
        assert!((ma3.average() as f64 / 10000.0 - 20.7194).abs() < 0.0001);


    }
}
