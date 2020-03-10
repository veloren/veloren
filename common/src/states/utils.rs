use crate::{
    comp::{AbilityState, CharacterState, EnergySource, ItemKind::Tool, StateUpdate, ToolData},
    event::LocalEvent,
    sys::{character_behavior::JoinData, phys::GRAVITY},
};
use std::time::Duration;
use vek::vec::{Vec2, Vec3};

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;
const BASE_HUMANOID_ACCEL: f32 = 100.0;
const BASE_HUMANOID_SPEED: f32 = 150.0;
const BASE_HUMANOID_AIR_ACCEL: f32 = 15.0;
const BASE_HUMANOID_AIR_SPEED: f32 = 8.0;
const BASE_HUMANOID_WATER_ACCEL: f32 = 70.0;
const BASE_HUMANOID_WATER_SPEED: f32 = 120.0;
// const BASE_HUMANOID_CLIMB_ACCEL: f32 = 10.0;
// const ROLL_SPEED: f32 = 17.0;
// const CHARGE_SPEED: f32 = 20.0;
// const GLIDE_ACCEL: f32 = 15.0;
// const GLIDE_SPEED: f32 = 45.0;
// const BLOCK_ACCEL: f32 = 30.0;
// const BLOCK_SPEED: f32 = 75.0;
// // Gravity is 9.81 * 4, so this makes gravity equal to .15
// const GLIDE_ANTIGRAV: f32 = GRAVITY * 0.96;
// const CLIMB_SPEED: f32 = 5.0;
// const CLIMB_COST: i32 = 5;

/// Handles updating `Components` to move player based on state of `JoinData`
pub fn handle_move(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.in_fluid {
        swim_move(data, update);
    } else {
        basic_move(data, update);
    }
}

/// Updates components to move player as if theyre on ground or in air
fn basic_move(data: &JoinData, update: &mut StateUpdate) {
    let (accel, speed): (f32, f32) = if data.physics.on_ground {
        (BASE_HUMANOID_ACCEL, BASE_HUMANOID_SPEED)
    } else {
        (BASE_HUMANOID_AIR_ACCEL, BASE_HUMANOID_AIR_SPEED)
    };

    // Move player according to move_dir
    if update.vel.0.magnitude_squared() < speed.powf(2.0) {
        update.vel.0 = update.vel.0 + Vec2::broadcast(data.dt.0) * data.inputs.move_dir * accel;
        let mag2 = update.vel.0.magnitude_squared();
        if mag2 > speed.powf(2.0) {
            update.vel.0 = update.vel.0.normalized() * speed;
        }
    }

    // Set direction based on move direction
    let ori_dir = if update.character.is_wield()
        || update.character.is_attack()
        || update.character.is_block()
    {
        Vec2::from(data.inputs.look_dir).normalized()
    } else {
        Vec2::from(data.inputs.move_dir)
    };

    // Smooth orientation
    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 9.0 * data.dt.0);
    }
}

/// Updates components to move player as if theyre swimming
fn swim_move(data: &JoinData, update: &mut StateUpdate) {
    // Update velocity
    update.vel.0 += Vec2::broadcast(data.dt.0)
        * data.inputs.move_dir
        * if update.vel.0.magnitude_squared() < BASE_HUMANOID_WATER_SPEED.powf(2.0) {
            BASE_HUMANOID_WATER_ACCEL
        } else {
            0.0
        };

    // Set direction based on move direction when on the ground
    let ori_dir = if update.character.is_attack() || update.character.is_block() {
        Vec2::from(data.inputs.look_dir).normalized()
    } else {
        Vec2::from(update.vel.0)
    };

    if ori_dir.magnitude_squared() > 0.0001
        && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
            > 0.001
    {
        update.ori.0 = vek::ops::Slerp::slerp(
            update.ori.0,
            ori_dir.into(),
            if data.physics.on_ground { 9.0 } else { 2.0 } * data.dt.0,
        );
    }

    // Force players to pulse jump button to swim up
    if data.inputs.jump.is_pressed() && !data.inputs.jump.is_long_press(Duration::from_millis(600))
    {
        update.vel.0.z =
            (update.vel.0.z + data.dt.0 * GRAVITY * 1.25).min(BASE_HUMANOID_WATER_SPEED);
    }
}

/// First checks whether `primary` input is pressed, then
/// attempts to go into Equipping state, otherwise Idle
pub fn handle_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed() {
        attempt_wield(data, update);
    }
}

/// If a tool is equipped, goes into Equipping state, otherwise goes to Idle
pub fn attempt_wield(data: &JoinData, update: &mut StateUpdate) {
    if let Some(Tool(tool)) = data.stats.equipment.main.as_ref().map(|i| i.kind) {
        update.character = CharacterState::Equipping {
            tool,
            time_left: tool.equip_time(),
        };
    } else {
        update.character = CharacterState::Idle {};
    };
}

/// Checks that player can `Sit` and updates `CharacterState` if so
pub fn handle_sit(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.sit.is_pressed() && data.physics.on_ground && data.body.is_humanoid() {
        update.character = CharacterState::Sit {};
    }
}

/// Checks that player can `Climb` and updates `CharacterState` if so
pub fn handle_climb(data: &JoinData, update: &mut StateUpdate) {
    if (data.inputs.climb.is_pressed() || data.inputs.climb_down.is_pressed())
        && data.physics.on_wall.is_some()
        && !data.physics.on_ground
        //&& update.vel.0.z < 0.0
        && data.body.is_humanoid()
        && update.energy.current() > 100
    {
        update.character = CharacterState::Climb {};
    }
}

/// Checks that player can `Glide` and updates `CharacterState` if so
pub fn handle_unwield(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Wielding { .. } = update.character {
        if data.inputs.toggle_wield.is_pressed() {
            update.character = CharacterState::Idle {};
        }
    }
}

/// Checks that player can glide and updates `CharacterState` if so
pub fn handle_glide(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Idle { .. } | CharacterState::Wielding { .. } = update.character {
        if data.inputs.glide.is_pressed() && !data.physics.on_ground && data.body.is_humanoid() {
            update.character = CharacterState::Glide {};
        }
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.jump.is_pressed() && data.physics.on_ground {
        update
            .local_events
            .push_front(LocalEvent::Jump(data.entity));
    }
}

/// If `inputs.primary` is pressed and in `Wielding` state,
/// will attempt to go into `ability_pool.primary`
pub fn handle_primary_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed() {
        if let CharacterState::Wielding { .. } = update.character {
            attempt_primary_ability(data, update);
        }
    }
}

/// Attempts to go into `ability_pool.primary` if is `Some()` on `AbilityPool`
pub fn attempt_primary_ability(data: &JoinData, update: &mut StateUpdate) {
    if let Some(ability_state) = data.ability_pool.primary {
        update.character = ability_to_character_state(data, ability_state);
    }
}

/// If `inputs.secondary` is pressed and in `Wielding` state,
/// will attempt to go into `ability_pool.secondary`
pub fn handle_secondary_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.secondary.is_pressed() {
        if let CharacterState::Wielding { .. } = update.character {
            attempt_seconday_ability(data, update);
        }
    }
}

/// Attempts to go into `ability_pool.secondary` if is `Some()` on `AbilityPool`
pub fn attempt_seconday_ability(data: &JoinData, update: &mut StateUpdate) {
    if let Some(ability_state) = data.ability_pool.secondary {
        update.character = ability_to_character_state(data, ability_state);
    }
}

/// Checks that player can perform a dodge, then
/// attempts to go into `ability_pool.dodge`
pub fn handle_dodge_input(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Idle { .. } | CharacterState::Wielding { .. } = update.character {
        if data.inputs.roll.is_pressed()
            && data.physics.on_ground
            && data.body.is_humanoid()
            && update
                .energy
                .try_change_by(-200, EnergySource::Roll)
                .is_ok()
        {
            attempt_dodge_ability(data, update);
        }
    }
}

pub fn attempt_dodge_ability(data: &JoinData, update: &mut StateUpdate) {
    if let Some(ability_state) = data.ability_pool.dodge {
        update.character = ability_to_character_state(data, ability_state);
    }
}

// TODO: Wight need a fn `CharacterState::new(data, update)` if
// initialization gets too lengthy.

/// Maps from `AbilityState`s to `CharacterStates`s. Also handles intializing
/// the new `CharacterState`
pub fn ability_to_character_state(data: &JoinData, ability_state: AbilityState) -> CharacterState {
    match ability_state {
        AbilityState::BasicAttack { .. } => {
            if let Some(tool) = unwrap_tool_data(data) {
                CharacterState::BasicAttack {
                    exhausted: false,
                    remaining_duration: tool.attack_duration(),
                }
            } else {
                *data.character
            }
        },
        AbilityState::BasicBlock { .. } => CharacterState::BasicBlock {},
        AbilityState::Roll { .. } => CharacterState::Roll {
            remaining_duration: Duration::from_millis(600),
        },
        AbilityState::ChargeAttack { .. } => CharacterState::ChargeAttack {
            remaining_duration: Duration::from_millis(600),
        },
        AbilityState::TripleAttack { .. } => {
            if let Some(tool) = unwrap_tool_data(data) {
                CharacterState::TripleAttack {
                    tool,
                    stage: 1,
                    stage_time_active: Duration::default(),
                    stage_exhausted: false,
                    can_transition: false,
                }
            } else {
                *data.character
            }
        },

        // Do not use default match
        // _ => *data.character
    }
}

pub fn unwrap_tool_data(data: &JoinData) -> Option<ToolData> {
    if let Some(Tool(tool)) = data.stats.equipment.main.as_ref().map(|i| i.kind) {
        Some(tool)
    } else {
        None
    }
}
