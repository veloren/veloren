use super::*;
use common::{
    assets,
    comp::{humanoid, item::Tool, ActionState, Body, MovementState, Stats},
    event::SfxEvent,
};
use std::time::{Duration, Instant};

#[test]
fn no_item_config_no_emit() {
    let last_sfx_event = LastSfxEvent {
        event: SfxEvent::Idle,
        weapon_drawn: false,
        time: Instant::now(),
    };

    let result = MovementEventMapper::should_emit(&last_sfx_event, None);

    assert_eq!(result, false);
}

#[test]
fn config_but_played_since_threshold_no_emit() {
    let event = SfxEvent::Run;

    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 1.0,
    };

    // Triggered a 'Run' 0 seconds ago
    let last_sfx_event = LastSfxEvent {
        event: SfxEvent::Run,
        weapon_drawn: false,
        time: Instant::now(),
    };

    let result = MovementEventMapper::should_emit(&last_sfx_event, Some((&event, &trigger_item)));

    assert_eq!(result, false);
}

#[test]
fn config_and_not_played_since_threshold_emits() {
    let event = SfxEvent::Run;

    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let last_sfx_event = LastSfxEvent {
        event: SfxEvent::Idle,
        weapon_drawn: false,
        time: Instant::now().checked_add(Duration::from_secs(1)).unwrap(),
    };

    let result = MovementEventMapper::should_emit(&last_sfx_event, Some((&event, &trigger_item)));

    assert_eq!(result, true);
}

#[test]
fn same_previous_event_elapsed_emits() {
    let event = SfxEvent::Run;

    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let last_sfx_event = LastSfxEvent {
        event: SfxEvent::Run,
        weapon_drawn: false,
        time: Instant::now()
            .checked_sub(Duration::from_millis(500))
            .unwrap(),
    };

    let result = MovementEventMapper::should_emit(&last_sfx_event, Some((&event, &trigger_item)));

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
        &CharacterState {
            movement: MovementState::Stand,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Run,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Run,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: false,
            time: Instant::now(),
        },
        Vec3::new(0.02, 0.0001, 0.0),
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
        &CharacterState {
            action: ActionState::Roll {
                time_left: Duration::new(1, 0),
                was_wielding: false,
            },
            movement: MovementState::Run,
        },
        &LastSfxEvent {
            event: SfxEvent::Run,
            weapon_drawn: false,
            time: Instant::now(),
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Roll);
}

#[test]
fn maps_fall() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState {
            movement: MovementState::Fall,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Fall,
            weapon_drawn: false,
            time: Instant::now(),
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Fall);
}

#[test]
fn maps_land_on_ground_to_run() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState {
            movement: MovementState::Stand,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Fall,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Glide,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Jump,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Glide,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Glide,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Fall,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Glide,
            weapon_drawn: false,
            time: Instant::now(),
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
        &CharacterState {
            movement: MovementState::Stand,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Glide,
            weapon_drawn: false,
            time: Instant::now(),
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
            "common.items.weapons.starter_sword",
        )),
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState {
            movement: MovementState::Stand,
            action: ActionState::Wield {
                time_left: Duration::from_millis(800),
            },
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: false,
            time: Instant::now(),
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Wield(Tool::Sword));
}

#[test]
fn maps_unwield() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        Some(assets::load_expect_cloned(
            "common.items.weapons.starter_axe",
        )),
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState {
            movement: MovementState::Stand,
            action: ActionState::Idle,
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: true,
            time: Instant::now(),
        },
        Vec3::zero(),
        &stats,
    );

    assert_eq!(result, SfxEvent::Unwield(Tool::Axe));
}

#[test]
fn does_not_map_wield_when_no_main_weapon() {
    let stats = Stats::new(
        String::from("test"),
        Body::Humanoid(humanoid::Body::random()),
        None,
    );

    let result = MovementEventMapper::map_movement_event(
        &CharacterState {
            movement: MovementState::Run,
            action: ActionState::Wield {
                time_left: Duration::from_millis(600),
            },
        },
        &LastSfxEvent {
            event: SfxEvent::Idle,
            weapon_drawn: false,
            time: Instant::now(),
        },
        Vec3::new(0.5, 0.8, 0.0),
        &stats,
    );

    assert_eq!(result, SfxEvent::Run);
}
