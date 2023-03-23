use crate::utils;
use approx::assert_relative_eq;
use common::{comp::Controller, resources::Time, shared_server_config::ServerConstants};
use specs::WorldExt;
use std::error::Error;
use utils::{DT, DT_F64, EPSILON};
use vek::{approx, Vec2, Vec3};
use veloren_common_systems::add_local_systems;

#[test]
fn simple_run() {
    let mut state = utils::setup();
    utils::create_player(&mut state);
    state.tick(
        DT,
        |dispatcher_builder| {
            add_local_systems(dispatcher_builder);
        },
        false,
        None,
        &ServerConstants::default(),
    );
}

#[test]
fn dont_fall_outside_world() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup();
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
    let mut state = utils::setup();
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
    let mut sstate = utils::setup();
    let mut fstate = utils::setup();
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
    assert_relative_eq!(fpos.0.z, 264.25073, epsilon = EPSILON);
    assert_relative_eq!(fvel.0.z, -4.9930925, epsilon = EPSILON);

    // Diff after 200ms
    assert_relative_eq!((spos.0.z - fpos.0.z).abs(), 0.2253418, epsilon = EPSILON);
    assert_relative_eq!((svel.0.z - fvel.0.z).abs(), 0.008329868, epsilon = EPSILON);

    Ok(())
}

#[test]
fn walk_simple() -> Result<(), Box<dyn Error>> {
    let mut state = utils::setup();
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
    let mut state = utils::setup();
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
    let mut sstate = utils::setup();
    let mut fstate = utils::setup();
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
    let mut state = utils::setup();
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
