use super::*;
use common::{
    comp::{
        bird_small, humanoid, quadruped_medium, quadruped_small, Body, CharacterState, PhysicsState,
    },
    event::SfxEvent,
    states,
};
use std::time::{Duration, Instant};

#[test]
fn no_item_config_no_emit() {
    let previous_state = PreviousEntityState::default();
    let result = MovementEventMapper::should_emit(&previous_state, None);

    assert_eq!(result, false);
}

#[test]
fn config_but_played_since_threshold_no_emit() {
    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 1.0,
    };

    // Triggered a 'Run' 0 seconds ago
    let previous_state = PreviousEntityState {
        event: SfxEvent::Run,
        time: Instant::now(),
        on_ground: true,
    };

    let result =
        MovementEventMapper::should_emit(&previous_state, Some((&SfxEvent::Run, &trigger_item)));

    assert_eq!(result, false);
}

#[test]
fn config_and_not_played_since_threshold_emits() {
    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let previous_state = PreviousEntityState {
        event: SfxEvent::Idle,
        time: Instant::now().checked_add(Duration::from_secs(1)).unwrap(),
        on_ground: true,
    };

    let result =
        MovementEventMapper::should_emit(&previous_state, Some((&SfxEvent::Run, &trigger_item)));

    assert_eq!(result, true);
}

#[test]
fn same_previous_event_elapsed_emits() {
    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let previous_state = PreviousEntityState {
        event: SfxEvent::Run,
        time: Instant::now()
            .checked_sub(Duration::from_millis(500))
            .unwrap(),
        on_ground: true,
    };

    let result =
        MovementEventMapper::should_emit(&previous_state, Some((&SfxEvent::Run, &trigger_item)));

    assert_eq!(result, true);
}

#[test]
fn maps_idle() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_run_with_sufficient_velocity() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
        },
        Vec3::new(0.5, 0.8, 0.0),
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn does_not_map_run_with_insufficient_velocity() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
        },
        Vec3::new(0.02, 0.0001, 0.0),
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn does_not_map_run_with_sufficient_velocity_but_not_on_ground() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: false,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::new(0.5, 0.8, 0.0),
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_roll() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Roll(states::roll::Data {
            remaining_duration: Duration::from_millis(300),
            was_wielded: true,
        }),
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Run,
            time: Instant::now(),
            on_ground: true,
        },
        Vec3::new(0.5, 0.5, 0.0),
    );

    assert_eq!(result, SfxEvent::Roll);
}

#[test]
fn maps_land_on_ground_to_run() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn maps_glider_open() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Glide {},
        &PhysicsState {
            on_ground: false,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Jump,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::GliderOpen);
}

#[test]
fn maps_glide() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Glide {},
        &PhysicsState {
            on_ground: false,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::Glide);
}

#[test]
fn maps_glider_close_when_closing_mid_flight() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: false,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::GliderClose);
}

#[test]
#[ignore]
fn maps_glider_close_when_landing() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            on_ground: false,
        },
        Vec3::zero(),
    );

    assert_eq!(result, SfxEvent::GliderClose);
}

#[test]
fn maps_quadrupeds_running() {
    let result = MovementEventMapper::map_non_humanoid_movement_event(
        &PhysicsState {
            on_ground: true,
            on_ceiling: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        Vec3::new(0.5, 0.8, 0.0),
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn determines_relative_volumes() {
    let human =
        MovementEventMapper::get_volume_for_body_type(&Body::Humanoid(humanoid::Body::random()));

    let quadruped_medium = MovementEventMapper::get_volume_for_body_type(&Body::QuadrupedMedium(
        quadruped_medium::Body::random(),
    ));

    let quadruped_small = MovementEventMapper::get_volume_for_body_type(&Body::QuadrupedSmall(
        quadruped_small::Body::random(),
    ));

    let bird_small =
        MovementEventMapper::get_volume_for_body_type(&Body::BirdSmall(bird_small::Body::random()));

    assert!(quadruped_medium < human);
    assert!(quadruped_small < quadruped_medium);
    assert!(bird_small < quadruped_small);
}
