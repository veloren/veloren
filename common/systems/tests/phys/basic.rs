use crate::utils;
use approx::assert_relative_eq;
use common::{comp::Controller, resources::Time, shared_server_config::ServerConstants};
use specs::WorldExt;
use std::error::Error;
use utils::{DT, DT_F64, EPSILON};
use vek::{Vec2, Vec3, approx};

#[test]
fn simple_run() {
    let mut state = utils::setup(veloren_common_systems::add_local_systems);
    utils::create_player(&mut state);
    state.tick(
        DT,
        false,
        None,
        &ServerConstants {
            day_cycle_coefficient: 24.0,
        },
        |_, _| {},
    );
}

#[test]
fn dont_fall_outside_world() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup(utils::add_char_and_phys_systems);
    let p1 = utils::create_player(&mut state);

    {
        let mut storage = state.ecs_mut().write_storage::<common::comp::Pos>();
        storage
            .insert(p1, common::comp::Pos(Vec3::new(1000.0, 1000.0, 265.0)))
            .unwrap();
    }

    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 1000.0);
    assert_relative_eq!(pos.0.y, 1000.0);
    assert_relative_eq!(pos.0.z, 265.0);
    assert_eq!(vel.0, Vec3::zero());

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 1000.0);
    assert_relative_eq!(pos.0.y, 1000.0);
    assert_relative_eq!(pos.0.z, 265.0);
    assert_eq!(vel.0, Vec3::zero());
    Ok(())
}

#[test]
fn fall_simple() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup(utils::add_char_and_phys_systems);
    let p1 = utils::create_player(&mut state);

    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.0);
    assert_relative_eq!(pos.0.y, 16.0);
    assert_relative_eq!(pos.0.z, 265.0);
    assert_eq!(vel.0, Vec3::zero());

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.0);
    assert_relative_eq!(pos.0.y, 16.0);
    assert_relative_eq!(pos.0.z, 264.9975, epsilon = EPSILON);
    assert_relative_eq!(vel.0.z, -0.25, epsilon = EPSILON);

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.z, 264.9925, epsilon = EPSILON);
    assert_relative_eq!(vel.0.z, -0.49969065, epsilon = EPSILON);

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.z, 264.985, epsilon = EPSILON);
    assert_relative_eq!(vel.0.z, -0.7493813, epsilon = EPSILON);

    utils::tick(&mut state, DT * 7);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(state.ecs_mut().read_resource::<Time>().0, DT_F64 * 10.0);
    assert_relative_eq!(pos.0.z, 264.8102, epsilon = EPSILON);
    assert_relative_eq!(vel.0.z, -2.4969761, epsilon = EPSILON);

    Ok(())
}

#[test]
/// will fall in 20 x DT and 2 x 10*DT steps. compare the end result and make
/// log the "error" between both calculations
fn fall_dt_speed_diff() -> Result<(), Box<dyn Error>> {
    let mut sstate = utils::setup(utils::add_char_and_phys_systems);
    let mut fstate = utils::setup(utils::add_char_and_phys_systems);
    let sp1 = utils::create_player(&mut sstate);
    let fp1 = utils::create_player(&mut fstate);

    for _ in 0..10 {
        utils::tick(&mut sstate, DT);
    }
    utils::tick(&mut fstate, DT * 10);

    let (spos, svel, _) = utils::get_transform(&sstate, sp1)?;
    let (fpos, fvel, _) = utils::get_transform(&fstate, fp1)?;
    assert_relative_eq!(spos.0.x, 16.0);
    assert_relative_eq!(spos.0.y, 16.0);
    assert_relative_eq!(spos.0.z, 264.86267, epsilon = EPSILON);
    assert_relative_eq!(svel.0.z, -2.496151, epsilon = EPSILON);
    assert_relative_eq!(fpos.0.x, 16.0);
    assert_relative_eq!(fpos.0.y, 16.0);
    assert_relative_eq!(fpos.0.z, 264.75, epsilon = EPSILON);
    assert_relative_eq!(fvel.0.z, -2.5, epsilon = EPSILON);

    assert_relative_eq!((spos.0.z - fpos.0.z).abs(), 0.1126709, epsilon = EPSILON);
    assert_relative_eq!((svel.0.z - fvel.0.z).abs(), 0.0038490295, epsilon = EPSILON);

    for _ in 0..10 {
        utils::tick(&mut sstate, DT);
    }
    utils::tick(&mut fstate, DT * 10);

    let (spos, svel, _) = utils::get_transform(&sstate, sp1)?;
    let (fpos, fvel, _) = utils::get_transform(&fstate, fp1)?;
    assert_relative_eq!(spos.0.x, 16.0);
    assert_relative_eq!(spos.0.y, 16.0);
    assert_relative_eq!(spos.0.z, 264.47607, epsilon = EPSILON);
    assert_relative_eq!(svel.0.z, -4.9847627, epsilon = EPSILON);
    assert_relative_eq!(fpos.0.x, 16.0);
    assert_relative_eq!(fpos.0.y, 16.0);
    assert_relative_eq!(fpos.0.z, 264.25067, epsilon = EPSILON);
    assert_relative_eq!(fvel.0.z, -4.9930925, epsilon = EPSILON);

    // Diff after 200ms
    assert_relative_eq!((spos.0.z - fpos.0.z).abs(), 0.22540283, epsilon = EPSILON);
    assert_relative_eq!((svel.0.z - fvel.0.z).abs(), 0.008329868, epsilon = EPSILON);

    Ok(())
}

#[test]
fn walk_simple() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup(utils::add_char_and_phys_systems);
    let p1 = utils::create_player(&mut state);

    for _ in 0..100 {
        utils::tick(&mut state, DT);
    }
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.z, 257.0); // make sure it landed on ground
    assert_eq!(vel.0, Vec3::zero());

    let mut actions = Controller::default();
    actions.inputs.move_dir = Vec2::new(1.0, 0.0);
    utils::set_control(&mut state, p1, actions)?;

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.01, epsilon = EPSILON);
    assert_relative_eq!(pos.0.y, 16.0);
    assert_relative_eq!(pos.0.z, 257.0);
    assert_relative_eq!(vel.0.x, 0.90703666, epsilon = EPSILON);
    assert_relative_eq!(vel.0.y, 0.0);
    assert_relative_eq!(vel.0.z, 0.0);

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.029068, epsilon = EPSILON);
    assert_relative_eq!(vel.0.x, 1.7296565, epsilon = EPSILON);

    utils::tick(&mut state, DT);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.05636, epsilon = EPSILON);
    assert_relative_eq!(vel.0.x, 2.4756372, epsilon = EPSILON);

    for _ in 0..8 {
        utils::tick(&mut state, DT);
    }
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.492111, epsilon = EPSILON);
    assert_relative_eq!(vel.0.x, 6.411994, epsilon = EPSILON);

    Ok(())
}

#[test]
fn walk_max() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup(utils::add_char_and_phys_systems);
    for x in 2..30 {
        utils::generate_chunk(&mut state, Vec2::new(x, 0));
    }
    let p1 = utils::create_player(&mut state);

    for _ in 0..100 {
        utils::tick(&mut state, DT);
    }

    let mut actions = Controller::default();
    actions.inputs.move_dir = Vec2::new(1.0, 0.0);
    utils::set_control(&mut state, p1, actions)?;

    for _ in 0..500 {
        utils::tick(&mut state, DT);
    }
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 68.40794, epsilon = EPSILON);
    assert_relative_eq!(vel.0.x, 9.695188, epsilon = EPSILON);
    for _ in 0..100 {
        utils::tick(&mut state, DT);
    }
    let (_, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(vel.0.x, 9.695188, epsilon = EPSILON);

    Ok(())
}

#[test]
/// will run in 20 x DT and 2 x 10*DT steps. compare the end result and make
/// log the "error" between both calculations
fn walk_dt_speed_diff() -> Result<(), Box<dyn Error>> {
    let mut sstate = utils::setup(utils::add_char_and_phys_systems);
    let mut fstate = utils::setup(utils::add_char_and_phys_systems);
    let sp1 = utils::create_player(&mut sstate);
    let fp1 = utils::create_player(&mut fstate);

    for _ in 0..100 {
        utils::tick(&mut sstate, DT);
        utils::tick(&mut fstate, DT);
    }

    let mut actions = Controller::default();
    actions.inputs.move_dir = Vec2::new(1.0, 0.0);
    utils::set_control(&mut sstate, sp1, actions.clone())?;
    utils::set_control(&mut fstate, fp1, actions)?;

    for _ in 0..10 {
        utils::tick(&mut sstate, DT);
    }
    utils::tick(&mut fstate, DT * 10);

    let (spos, svel, _) = utils::get_transform(&sstate, sp1)?;
    let (fpos, fvel, _) = utils::get_transform(&fstate, fp1)?;
    assert_relative_eq!(spos.0.x, 16.421423, epsilon = EPSILON);
    assert_relative_eq!(spos.0.y, 16.0);
    assert_relative_eq!(spos.0.z, 257.0);
    assert_relative_eq!(svel.0.x, 6.071788, epsilon = EPSILON);
    assert_relative_eq!(fpos.0.x, 16.993896, epsilon = EPSILON);
    assert_relative_eq!(fpos.0.y, 16.0);
    assert_relative_eq!(fpos.0.z, 257.0);
    assert_relative_eq!(fvel.0.x, 3.7484815, epsilon = EPSILON);

    assert_relative_eq!((spos.0.x - fpos.0.x).abs(), 0.5724735, epsilon = EPSILON);
    assert_relative_eq!((svel.0.x - fvel.0.x).abs(), 2.3233063, epsilon = EPSILON);

    for _ in 0..10 {
        utils::tick(&mut sstate, DT);
    }
    utils::tick(&mut fstate, DT * 10);

    let (spos, svel, _) = utils::get_transform(&sstate, sp1)?;
    let (fpos, fvel, _) = utils::get_transform(&fstate, fp1)?;
    assert_relative_eq!(spos.0.x, 17.248621, epsilon = EPSILON);
    assert_relative_eq!(svel.0.x, 8.344364, epsilon = EPSILON);
    assert_relative_eq!(fpos.0.x, 18.357212, epsilon = EPSILON);
    assert_relative_eq!(fvel.0.x, 5.1417327, epsilon = EPSILON);

    // Diff after 200ms
    assert_relative_eq!((spos.0.x - fpos.0.x).abs(), 1.1085911, epsilon = EPSILON);
    assert_relative_eq!((svel.0.x - fvel.0.x).abs(), 3.2026315, epsilon = EPSILON);

    Ok(())
}

#[test]
fn cant_run_during_fall() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup(utils::add_char_and_phys_systems);
    let p1 = utils::create_player(&mut state);

    let mut actions = Controller::default();
    actions.inputs.move_dir = Vec2::new(1.0, 0.0);
    utils::set_control(&mut state, p1, actions)?;

    utils::tick(&mut state, DT * 2);
    let (pos, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(pos.0.x, 16.0);
    assert_relative_eq!(pos.0.y, 16.0);
    assert_relative_eq!(vel.0.x, 0.0);
    assert_relative_eq!(vel.0.y, 0.0);

    utils::tick(&mut state, DT * 2);
    let (_, vel, _) = utils::get_transform(&state, p1)?;
    assert_relative_eq!(state.ecs_mut().read_resource::<Time>().0, DT_F64 * 4.0);
    assert_relative_eq!(pos.0.x, 16.0);
    assert_relative_eq!(pos.0.y, 16.0);
    assert_relative_eq!(vel.0.x, 0.04999693, epsilon = EPSILON);
    assert_relative_eq!(vel.0.y, 0.0, epsilon = EPSILON);

    Ok(())
}

#[derive(Clone, Copy)]
struct FricParams {
    friction_co: f32,
    projected_area: f32,
    density: f32,
    mass: f32,
    max_acc: f32,
}
impl Default for FricParams {
    fn default() -> Self {
        Self {
            friction_co: 0.9_f32,
            projected_area: 0.75_f32,
            density: 1.225_f32,
            mass: 1.0_f32,
            max_acc: 9.2_f32, /* on ground the maximum speed you can get by walking is the speed
                               * of gravity */
        }
    }
}
impl FricParams {
    // https://en.wikipedia.org/wiki/Drag_(physics)
    // Todo: old impl was (mass * acc.abs() / (friction_co * projected_area * 0.5 *
    // density * mass )).sqrt();
    fn v_term(&self, acc: f32) -> f32 {
        ((2.0 * self.mass * acc.abs()) / (self.density * self.projected_area * self.friction_co))
            .sqrt()
    }
}
trait MathHelp {
    fn coth(&self) -> Self;
    fn acoth(&self) -> Self;
}
impl MathHelp for f32 {
    // coth(x) = 1/tanh(x)`
    fn coth(&self) -> Self { 1.0 / self.tanh() }

    // `acoth(x) = atanh(1/x)`
    fn acoth(&self) -> Self { (1.0 / self).atanh() }
}

fn test_run_helper(
    tps: u32,
    start_vel: Vec3<f32>,
    start_pos: Vec3<f32>,
    test_series: &Vec<(/* acc */ Vec3<f32>, /* duration */ f64)>,
) -> (Vec3<f32>, Vec3<f32>) {
    let dt = 1.0 / tps as f64;
    let params = FricParams::default();
    let v_term = params.v_term(params.max_acc);
    println!(
        "\ndt: {:0>4.4} - mass: {:0>4.4} - v_term: {:0>4.4}",
        dt, params.mass, v_term
    );
    let (mut vel, mut pos) = (start_vel, start_pos);
    let mut i = 0;
    for (move_dir, duration) in test_series {
        let count = (duration / dt).round() as usize;
        println!("move_dir: {:0>4.4}", move_dir);
        for _ in 0..count {
            (vel, pos) = acc_with_frict_tick_print(i, *move_dir, vel, pos, dt, params);
            i += 1;
        }
    }
    (vel, pos)
}

macro_rules! veriy_diffs {
    ($vel1:expr, $pos1:expr, $vel2:expr, $pos2:expr) => {
        let vel_diff = ($vel2 - $vel1).map(|xyz| xyz.abs());
        let pos_diff = ($pos2 - $pos1).map(|xyz| xyz.abs());
        println!("vel_diff: {:4.4},   pos_diff: {:4.4}", vel_diff, pos_diff);
        assert_relative_eq!(vel_diff.x, 0.0, epsilon = EPSILON);
        assert_relative_eq!(vel_diff.y, 0.0, epsilon = EPSILON);
        assert_relative_eq!(vel_diff.z, 0.0, epsilon = EPSILON);
        assert_relative_eq!(pos_diff.x, 0.0, epsilon = EPSILON);
        assert_relative_eq!(pos_diff.y, 0.0, epsilon = EPSILON);
        assert_relative_eq!(pos_diff.z, 0.0, epsilon = EPSILON);
    };
}

#[inline(always)]
fn acc_with_frict_tick_print(
    i: usize,
    move_dir: Vec3<f32>,
    vel: Vec3<f32>,
    pos: Vec3<f32>,
    dt: f64,
    params: FricParams,
) -> (Vec3<f32>, Vec3<f32>) {
    let acc = move_dir * params.max_acc;
    let vel_t = acc.map(|xyz| xyz.signum() * params.v_term(xyz));
    let revert_fak = vel / vel_t;

    let (vel, pos) = acc_with_frict_tick(move_dir, vel, pos, dt, params);
    let ending = ((i + 1) as f64 * dt * 100.0).round() as i64;
    let line = format!(
        "[{:0>2.2}]:    acc: {:0>5.4},    revert_fak: {:0>4.4},    vel: {:0>4.4},    pos: {:0>7.4}",
        (i + 1) as f64 * dt,
        acc,
        revert_fak,
        vel,
        pos
    );
    if ending % 10 != 0 {
        println!("\x1b[94m{}\x1b[0m", line)
    } else {
        println!("{}", line)
    }
    (vel, pos)
}
/// ROLLING_FRICTION_FORCE + AIR_FRICTION_FORCE + TILT_FRICT_FORCE + ACCEL_FORCE
/// = TOTAL_FORCE
///
/// TILT_FRICT_FORCE = 0.0
/// TOTAL_FORCE = depends on char = const
/// ACCEL_FORCE = TOTAL_FORCE - ROLLING_FRICTION_FORCE - AIR_FRICTION_FORCE
/// ACCEL = ACCEL_FORCE / MASS
///
/// ROLLING_FRICTION_FORCE => Indepent of vel
/// AIR_FRICTION_FORCE => propotional to vel²
///
/// https://www.energie-lexikon.info/fahrwiderstand.html
/// https://www.energie-lexikon.info/reibung.html
/// https://sciencing.com/calculate-force-friction-6454395.html
/// https://www.leifiphysik.de/mechanik/reibung-und-fortbewegung
fn acc_with_frict_tick(
    move_dir: Vec3<f32>,
    vel: Vec3<f32>,
    pos: Vec3<f32>,
    dt: f64,
    params: FricParams,
) -> (Vec3<f32>, Vec3<f32>) {
    let acc = move_dir * params.max_acc; // btw: cant accelerate faster than gravity on foot

    // controller
    // I know what you think, wtf, yep: https://math.stackexchange.com/questions/1929436/line-integral-of-force-of-air-resistanc
    // basically an integral of the air resistance formula which scales with v^2
    // transformed with an ODE.

    // terminal velocity equals the maximum velocity that can be reached by acc
    // alone
    let vel_t = acc.map(|xyz| xyz.signum() * params.v_term(xyz));

    // thanks to kilpkonn for figuring this out
    // https://en.wikipedia.org/wiki/Drag_(physics)
    //
    // upper and lower are upper and lower bound for integral
    let revert_fak = vel / vel_t;

    let (mut pos, mut vel) = (pos, vel);
    for i in 0..2 {
        let dt = dt as f32;
        let (v, p) = {
            let acc = acc[i];
            let vel = vel[i];
            let pos = pos[i];
            let vel_t = vel_t[i];
            let revert_fak = revert_fak[i];
            if acc.abs() < f32::EPSILON {
                // https://www.wolframalpha.com/input?i=m%2F%28Cx+%2B+m%2FV%29+dx+integrate
                let c = params.density * params.projected_area * params.friction_co;
                let lower = params.mass * params.mass.ln() / c;
                let upper = params.mass * (c * vel * dt + params.mass).ln() / c;
                let pos = pos + (upper - lower);
                let vel = params.mass
                    / (params.density * params.projected_area * params.friction_co * dt
                        + params.mass / vel);
                (vel, pos)
            } else if revert_fak <= 0.0 {
                // Handle passing through 0 differently as the function changes
                // https://www.wolframalpha.com/input?i=V*tan%28x*g%2FV+%2B+atan%28v%2FV%29%29+%3D+0+solve+for+x
                let dt_to_zero = vel_t * (vel / vel_t).atan() / acc.abs();
                if dt_to_zero < dt {
                    // Step with only part of dt that is left after reaching 0 vel
                    let lower = vel_t.powi(2) * (vel / vel_t).atan().cos().ln() / acc;
                    let upper = -vel_t.powi(2)
                        * (acc * dt_to_zero / vel_t + (vel / vel_t).atan()).cos().ln()
                        / acc;
                    let pos = pos + (upper - lower);
                    let dt = dt - dt_to_zero;
                    // https://www.wolframalpha.com/input?i=V+*+tanh%28xg%2FV%29+dx+integrate
                    // lower bound is 0
                    let pos = pos + vel_t.powi(2) * (acc * dt / vel_t).cosh().ln() / acc;
                    let vel = vel_t * (dt * acc / vel_t).tanh();
                    (vel, pos)
                } else {
                    // https://www.wolframalpha.com/input?i=V+*+tan%28xg%2FV+%2B+atan%28v%2FV%29%29+dx+integrate
                    let lower = -vel_t.powi(2) * (vel / vel_t).atan().cos().ln() / acc;
                    let upper =
                        -vel_t.powi(2) * (acc * dt / vel_t + (vel / vel_t).atan()).cos().ln() / acc;
                    let pos = pos + (upper - lower);
                    let vel = vel_t * (dt * acc / vel_t + (vel / vel_t).atan()).tan();
                    (vel, pos)
                }
            } else if revert_fak >= 1.0 {
                // https://www.wolframalpha.com/input?i=V+*+coth%28xg%2FV+%2B+acoth%28v%2FV%29%29+dx+integrate
                let lower = (vel_t.powi(2) * (vel / vel_t).acoth().cosh().ln()
                    + (vel / vel_t).acoth().tanh().ln())
                    / acc;
                let upper = (vel_t.powi(2)
                    * (acc * dt / vel_t + (vel / vel_t).acoth()).cosh().ln()
                    + (acc * dt / vel_t + (vel / vel_t).acoth()).tanh().ln())
                    / acc;
                let pos = pos + (upper - lower);
                let vel = vel_t * (dt * acc / vel_t + (vel / vel_t).acoth()).coth();
                (vel, pos)
            } else {
                // https://www.wolframalpha.com/input?i=V+*+tanh%28xg%2FV+%2B+atanh%28v%2FV%29%29+dx+integrate
                let lower = vel_t.powi(2) * ((vel / vel_t).atanh()).cosh().ln() / acc;
                let upper =
                    vel_t.powi(2) * (acc * dt / vel_t + (vel / vel_t).atanh()).cosh().ln() / acc;
                let pos = pos + (upper - lower);
                let vel = vel_t * (dt * acc / vel_t + (vel / vel_t).atanh()).tanh();
                (vel, pos)
            }
        };
        vel[i] = v;
        pos[i] = p;
    }
    (vel, pos)
}

#[test]
fn physics_constant_walk() {
    let series = vec![(0.5_f32 * Vec3::unit_x(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_constant_stay() {
    let series = vec![(Vec3::zero(), 1.0)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
    assert_relative_eq!(vel_05.x, 0.0, epsilon = EPSILON);
    assert_relative_eq!(vel_05.y, 0.0, epsilon = EPSILON);
    assert_relative_eq!(vel_05.z, 0.0, epsilon = EPSILON);
    assert_relative_eq!(pos_05.x, 0.0, epsilon = EPSILON);
    assert_relative_eq!(pos_05.y, 0.0, epsilon = EPSILON);
    assert_relative_eq!(pos_05.z, 0.0, epsilon = EPSILON);
}

#[test]
fn physics_walk_run_b() {
    let series = vec![(0.5_f32 * Vec3::unit_x(), 2.0), (Vec3::unit_x(), 2.0)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_walk_run_walk() {
    let series = vec![
        (0.5_f32 * Vec3::unit_x(), 2.0),
        (Vec3::unit_x(), 3.0),
        (0.5_f32 * Vec3::unit_x(), 0.5),
    ];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_run_walk_b() {
    let series = vec![(Vec3::unit_x(), 0.5), (0.5_f32 * Vec3::unit_x(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_runlong_walk_b() {
    let series = vec![(Vec3::unit_x(), 3.0), (0.5_f32 * Vec3::unit_x(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_walk_stop() {
    let series = vec![(0.5_f32 * Vec3::unit_x(), 0.5), (Vec3::zero(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_runlong_stop() {
    let series = vec![(Vec3::unit_x(), 3.0), (Vec3::zero(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_stop_walk() {
    let series = vec![(Vec3::zero(), 0.5), (0.5_f32 * Vec3::unit_x(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_run_walkback() {
    let series = vec![
        (0.5_f32 * Vec3::unit_x(), 0.5),
        (-0.5_f32 * Vec3::unit_x(), 0.5),
    ];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_runlong_walkback_stop() {
    let series = vec![
        (Vec3::unit_x(), 3.0),
        (-0.5_f32 * Vec3::unit_x(), 0.5),
        (Vec3::zero(), 0.5),
    ];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_walkback_stop() {
    let series = vec![(-0.5_f32 * Vec3::unit_x(), 0.5), (Vec3::zero(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}

#[test]
fn physics_walkback_run() {
    let series = vec![(-0.5_f32 * Vec3::unit_x(), 5.0), (Vec3::unit_x(), 0.5)];
    let (vel_05, pos_05) = test_run_helper(20, Vec3::zero(), Vec3::zero(), &series);
    let (vel_10, pos_10) = test_run_helper(10, Vec3::zero(), Vec3::zero(), &series);

    veriy_diffs!(vel_05, pos_05, vel_10, pos_10);
}
