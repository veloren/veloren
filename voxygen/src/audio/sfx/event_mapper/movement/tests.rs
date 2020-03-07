use super::*;
use common::{
    assets,
    comp::{
        bird_small, humanoid, item::ToolKind, quadruped_medium, quadruped_small, Body,
        CharacterState, PhysicsState, Stats,
    },
    event::SfxEvent,
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
        weapon_drawn: false,
        on_ground: true,
    };

    let result =
        MovementEventMapper::should_emit(&previous_state, Some((&SfxEvent::Run, &trigger_item)));

    assert_eq!(result, false);
}

#[test]
fn config_and_not_played_since_threshold_emits() {
    let event = SfxEvent::Run;

    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let previous_state = PreviousEntityState {
        event: SfxEvent::Idle,
        time: Instant::now().checked_add(Duration::from_secs(1)).unwrap(),
        weapon_drawn: false,
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
        weapon_drawn: false,
        on_ground: true,
    };

    let result =
        MovementEventMapper::should_emit(&previous_state, Some((&SfxEvent::Run, &trigger_item)));

    assert_eq!(result, true);
}

#[test]
fn maps_idle() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_run_with_sufficient_velocity() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::new(0.5, 0.8, 0.0),
        &stats,
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn does_not_map_run_with_insufficient_velocity() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::new(0.02, 0.0001, 0.0),
        &stats,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn does_not_map_run_with_sufficient_velocity_but_not_on_ground() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::new(0.5, 0.8, 0.0),
        &stats,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_roll() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Roll {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Run,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Roll);
}

#[test]
fn maps_land_on_ground_to_run() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn maps_glider_open() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Glide {},
        &PhysicsState {
            on_ground: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Jump,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::GliderOpen);
}

#[test]
fn maps_glide() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Glide {},
        &PhysicsState {
            on_ground: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Glide);
}

#[test]
fn maps_glider_close_when_closing_mid_flight() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: false,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::GliderClose);
}

#[test]
fn maps_glider_close_when_landing() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: false,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::GliderClose);
}

#[test]
fn maps_wield() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        Some(assets::load_expect_cloned(
            "common.items.weapons.starter_axe",
        )),
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Equipping {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Wield(ToolKind::Axe));
}

#[test]
fn maps_unwield() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        Some(assets::load_expect_cloned(
            "common.items.weapons.starter_bow",
        )),
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::default(),
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: true,
            on_ground: true,
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Unwield(ToolKind::Bow));
}

#[test]
fn does_not_map_wield_when_no_main_weapon() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Wielding {},
        &PhysicsState {
            on_ground: true,
            on_wall: None,
            touch_entity: None,
            in_fluid: false,
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            weapon_drawn: false,
            on_ground: true,
        },
        Vec3::new(0.5, 0.8, 0.0),
        &stats,
    );

    assert_eq!(result, SfxEvent::Run);
}

#[test]
fn maps_quadrupeds_running() {
    let result = MovementEventMapper::map_non_humanoid_movement_event(
        &PhysicsState {
            on_ground: true,
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
