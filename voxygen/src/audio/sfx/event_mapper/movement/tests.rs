use super::*;
use crate::audio::sfx::SfxEvent;
use common::{
    comp::{
        bird_large, character_state::AttackFilters, controller::InputKind, humanoid,
        quadruped_medium, quadruped_small, Body, CharacterState, Ori, PhysicsState,
    },
    states,
    terrain::{Block, BlockKind},
};
use std::time::{Duration, Instant};

#[test]
fn no_item_config_no_emit() {
    let previous_state = PreviousEntityState::default();
    let result = MovementEventMapper::should_emit(&previous_state, None);

    assert!(!result);
}

#[test]
fn config_but_played_since_threshold_no_emit() {
    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 1.0,
    };

    // Triggered a 'Run' 0 seconds ago
    let previous_state = PreviousEntityState {
        event: SfxEvent::Run(BlockKind::Grass),
        time: Instant::now(),
        on_ground: true,
        in_water: false,
        distance_travelled: 0.0,
    };

    let result = MovementEventMapper::should_emit(
        &previous_state,
        Some((&SfxEvent::Run(BlockKind::Grass), &trigger_item)),
    );

    assert!(!result);
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
        in_water: false,
        distance_travelled: 0.0,
    };

    let result = MovementEventMapper::should_emit(
        &previous_state,
        Some((&SfxEvent::Run(BlockKind::Grass), &trigger_item)),
    );

    assert!(result);
}

#[test]
fn same_previous_event_elapsed_emits() {
    let trigger_item = SfxTriggerItem {
        files: vec![String::from("some.path.to.sfx.file")],
        threshold: 0.5,
    };

    let previous_state = PreviousEntityState {
        event: SfxEvent::Run(BlockKind::Grass),
        time: Instant::now()
            .checked_sub(Duration::from_millis(1800))
            .unwrap(),
        on_ground: true,
        in_water: false,
        distance_travelled: 2.0,
    };

    let result = MovementEventMapper::should_emit(
        &previous_state,
        Some((&SfxEvent::Run(BlockKind::Grass), &trigger_item)),
    );

    assert!(result);
}

#[test]
fn maps_idle() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle(states::idle::Data::default()),
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::zero(),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_run_with_sufficient_velocity() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle(states::idle::Data::default()),
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::new(0.5, 0.8, 0.0),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Run(BlockKind::Grass));
}

#[test]
fn does_not_map_run_with_insufficient_velocity() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle(states::idle::Data::default()),
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: true,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::new(0.02, 0.0001, 0.0),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn does_not_map_run_with_sufficient_velocity_but_not_on_ground() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle(states::idle::Data::default()),
        &Default::default(),
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: false,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::new(0.5, 0.8, 0.0),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Idle);
}

#[test]
fn maps_roll() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Roll(states::roll::Data {
            static_data: states::roll::StaticData {
                buildup_duration: Duration::default(),
                movement_duration: Duration::default(),
                recover_duration: Duration::default(),
                roll_strength: 0.0,
                attack_immunities: AttackFilters {
                    melee: false,
                    projectiles: false,
                    beams: false,
                    ground_shockwaves: false,
                    air_shockwaves: false,
                    explosions: false,
                },
                ability_info: empty_ability_info(),
            },
            timer: Duration::default(),
            stage_section: states::utils::StageSection::Buildup,
            was_wielded: true,
            is_sneaking: false,
            was_combo: None,
        }),
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        &PreviousEntityState {
            event: SfxEvent::Run(BlockKind::Grass),
            time: Instant::now(),
            on_ground: true,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::new(0.5, 0.5, 0.0),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Roll);
}

#[test]
fn maps_land_on_ground_to_run() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Idle(states::idle::Data::default()),
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        &PreviousEntityState {
            event: SfxEvent::Idle,
            time: Instant::now(),
            on_ground: false,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::zero(),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Run(BlockKind::Grass));
}

#[test]
fn maps_glide() {
    let result = MovementEventMapper::map_movement_event(
        &CharacterState::Glide(states::glide::Data::new(10.0, 1.0, Ori::default())),
        &Default::default(),
        &PreviousEntityState {
            event: SfxEvent::Glide,
            time: Instant::now(),
            on_ground: false,
            in_water: false,
            distance_travelled: 0.0,
        },
        Vec3::zero(),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Glide);
}

#[test]
fn maps_quadrupeds_running() {
    let result = MovementEventMapper::map_non_humanoid_movement_event(
        &PhysicsState {
            on_ground: Some(Block::empty()),
            ..Default::default()
        },
        Vec3::new(0.5, 0.8, 0.0),
        BlockKind::Grass,
    );

    assert_eq!(result, SfxEvent::Run(BlockKind::Grass));
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

    let bird_large =
        MovementEventMapper::get_volume_for_body_type(&Body::BirdLarge(bird_large::Body::random()));

    assert!(quadruped_medium < human);
    assert!(quadruped_small < quadruped_medium);
    assert!(bird_large < quadruped_small);
}

fn empty_ability_info() -> states::utils::AbilityInfo {
    states::utils::AbilityInfo {
        tool: None,
        hand: None,
        input: InputKind::Primary,
        input_attr: None,
        ability_meta: Default::default(),
        ability: None,
    }
}
