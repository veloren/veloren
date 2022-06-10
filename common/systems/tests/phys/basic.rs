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

// The problem with old_vec is that we cant start with 0.0 0.0 0.0 as it will
// make the first tick different on all examples

#[test]
fn physics_theory() -> Result<(), Box<dyn Error>> {
    let tick = |i: usize, move_dir: f64, vel: f64, pos: f64, dt: f64| {
        /*
           ROLLING_FRICTION_FORCE + AIR_FRICTION_FORCE + TILT_FRICT_FORCE + ACCEL_FORCE = TOTAL_FORCE

           TILT_FRICT_FORCE = 0.0
           TOTAL_FORCE = depends on char = const
           ACCEL_FORCE = TOTAL_FORCE - ROLLING_FRICTION_FORCE - AIR_FRICTION_FORCE
           ACCEL = ACCEL_FORCE / MASS

           ROLLING_FRICTION_FORCE => Indepent of vel
           AIR_FRICTION_FORCE => propotional to vel²
           https://www.energie-lexikon.info/fahrwiderstand.html

           https://www.energie-lexikon.info/reibung.html

           https://sciencing.com/calculate-force-friction-6454395.html

           https://www.leifiphysik.de/mechanik/reibung-und-fortbewegung


        */

        let mass = 1.0;

        let air_friction_co = 0.9_f64;
        let air_friction_area = 0.75_f64;
        let air_density = 1.225_f64;
        let c = air_friction_co * air_friction_area * 0.5 * air_density * mass;
        //let acc = accel_force / mass * move_dir;

        let acc = 9.2_f64; // btw: cant accelerate faster than gravity on foot
        let old_vel = vel;

        // controller
        // I know what you think, wtf, yep: https://math.stackexchange.com/questions/1929436/line-integral-of-force-of-air-resistanc

        // basically an integral of the air resistance formula which scales with v^2
        // transformed with an ODE.
        let vel = (mass * acc / c).sqrt()
            * (((c / acc / mass).sqrt() * old_vel).atanh() + (c * acc / mass).sqrt() * dt).tanh();

        //physics
        //let acc2 = (vel - old_vel) / dt;
        //let distance =  0.5 * vel * dt - 0.5 * acc2 * dt * dt;

        let distance_last = mass / c * (((old_vel * (c / acc / mass).sqrt()).atanh()).cosh()).ln();
        let distance = mass / c
            * (((old_vel * (c / acc / mass).sqrt()).atanh() + dt * (c * acc / mass).sqrt()).cosh())
                .ln();
        let diff = distance - distance_last;

        let pos = pos + diff;

        //if ((i+1) as f64 *dt * 10.0).round() as i64 % 2 == 0 {
        println!(
            "[{:0>2.1}]:    move_dir: {:0>1.1},    acc: {:0>4.4},    vel: {:0>4.4},    dist: \
             {:0>7.4},    dist: {:0>7.4},    pos: {:0>7.4},    c: {:0>4.4}",
            (i + 1) as f64 * dt,
            move_dir,
            acc,
            vel,
            distance_last,
            distance,
            pos,
            c
        );
        //}
        (acc, vel, pos)
    };

    let (vel_final_01, pos_final_01) = {
        println!("");
        const DT: f64 = 0.1;
        println!("dt: {}", DT);
        let (_acc, mut vel, mut pos) = (0.0, 0.0, 0.0);
        for i in 0..30 {
            (_, vel, pos) = tick(i, 1.0, vel, pos, DT);
        }
        (vel, pos)
    };

    let (vel_final_02, pos_final_02) = {
        println!("");
        const DT: f64 = 0.2;
        println!("dt: {}", DT);
        let (_acc, mut vel, mut pos) = (0.0, 0.0, 0.0);
        for i in 0..15 {
            (_, vel, pos) = tick(i, 1.0, vel, pos, DT);
        }
        (vel, pos)
    };

    let (vel_final_10, pos_final_10) = {
        println!("");
        const DT: f64 = 1.0;
        println!("dt: {}", DT);
        let (_acc, mut vel, mut pos) = (0.0, 0.0, 0.0);
        for i in 0..3 {
            (_, vel, pos) = tick(i, 1.0, vel, pos, DT);
        }
        (vel, pos)
    };

    let vel_diff = (vel_final_02 - vel_final_01).abs();
    let pos_diff = (pos_final_02 - pos_final_01).abs();
    println!(
        "[ #1 ] vel_diff: {:4.4},   pos_diff: {:4.4}",
        vel_diff, pos_diff
    );

    let vel_diff = (vel_final_10 - vel_final_01).abs();
    let pos_diff = (pos_final_10 - pos_final_01).abs();
    println!(
        "[ #2 ] vel_diff: {:4.4},   pos_diff: {:4.4}",
        vel_diff, pos_diff
    );

    Ok(())
}
