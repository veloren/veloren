use crate::{
    comp::{CharacterState, ItemKind::Tool, StateUpdate, ToolData},
    event::LocalEvent,
    states::*,
    sys::{character_behavior::JoinData, phys::GRAVITY},
    util::safe_slerp,
};
use vek::vec::Vec2;

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
// Gravity is 9.81 * 4, so this makes gravity equal to .15 //TODO: <- is wrong
//
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

    handle_orientation(data, update, 9.0);
}

pub fn handle_orientation(data: &JoinData, update: &mut StateUpdate, strength: f32) {
    // Set direction based on move direction
    let ori_dir = if update.character.is_attack() || update.character.is_block() {
        Vec2::from(data.inputs.look_dir)
    } else {
        Vec2::from(data.inputs.move_dir)
    };

    // Smooth orientation
    update.ori.0 = safe_slerp(update.ori.0, ori_dir.into(), strength * data.dt.0);
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

    handle_orientation(data, update, if data.physics.on_ground { 9.0 } else { 2.0 });

    // Swim
    if data.inputs.jump.is_pressed() {
        update.vel.0.z =
            (update.vel.0.z + data.dt.0 * GRAVITY * 2.25).min(BASE_HUMANOID_WATER_SPEED);
    }
}

/// First checks whether `primary` input is pressed, then
/// attempts to go into Equipping state, otherwise Idle
pub fn handle_primary_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed() {
        attempt_wield(data, update);
    }
}

/// If a tool is equipped, goes into Equipping state, otherwise goes to Idle
pub fn attempt_wield(data: &JoinData, update: &mut StateUpdate) {
    if let Some(Tool(tool)) = data.loadout.active_item.as_ref().map(|i| &i.item.kind) {
        update.character = CharacterState::Equipping(equipping::Data {
            time_left: tool.equip_time(),
        });
    } else {
        update.character = CharacterState::Idle;
    };
}

/// Checks that player can `Sit` and updates `CharacterState` if so
pub fn attempt_sit(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.on_ground && data.body.is_humanoid() {
        update.character = CharacterState::Sit;
    }
}

/// Checks that player can `Climb` and updates `CharacterState` if so
pub fn handle_climb(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.climb.is_some()
        && data.physics.on_wall.is_some()
        && !data.physics.on_ground
        //&& update.vel.0.z < 0.0
        && data.body.is_humanoid()
        && update.energy.current() > 100
    {
        update.character = CharacterState::Climb;
    }
}

/// Checks that player can Swap Weapons and updates `Loadout` if so
pub fn attempt_swap_loadout(data: &JoinData, update: &mut StateUpdate) {
    let mut new_loadout = data.loadout.clone();
    new_loadout.active_item = data.loadout.second_item.clone();
    new_loadout.second_item = data.loadout.active_item.clone();
    update.loadout = new_loadout;
}

/// Checks that player can glide and updates `CharacterState` if so
pub fn handle_glide(data: &JoinData, update: &mut StateUpdate) {
    if let CharacterState::Idle { .. } | CharacterState::Wielding { .. } = update.character {
        if data.inputs.glide.is_pressed()
            && !data.physics.on_ground
            && !data.physics.in_fluid
            && data.body.is_humanoid()
        {
            update.character = CharacterState::Glide;
        }
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.jump.is_pressed() && data.physics.on_ground && !data.physics.in_fluid {
        update
            .local_events
            .push_front(LocalEvent::Jump(data.entity));
    }
}

/// Will attempt to go into `loadout.active_item.ability1`
pub fn handle_ability1_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_just_pressed() {
        if let Some(ability) = data
            .loadout
            .active_item
            .as_ref()
            .and_then(|i| i.ability1.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = ability.into();
        }
    }
}

/// Will attempt to go into `loadout.active_item.ability2`
pub fn handle_ability2_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.secondary.is_just_pressed() {
        if let Some(ability) = data
            .loadout
            .active_item
            .as_ref()
            .and_then(|i| i.ability2.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = ability.into();
        }
    }
}

/// Will attempt to go into `loadout.active_item.ability3`
pub fn handle_ability3_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.ability3.is_just_pressed() {
        if let Some(ability) = data
            .loadout
            .active_item
            .as_ref()
            .and_then(|i| i.ability3.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = ability.into();
        }
    }
}

/// Checks that player can perform a dodge, then
/// attempts to go into `loadout.active_item.dodge_ability`
pub fn handle_dodge_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.roll.is_just_pressed() {
        if let Some(ability) = data
            .loadout
            .active_item
            .as_ref()
            .and_then(|i| i.dodge_ability.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = ability.into();
        }
    }
}

pub fn unwrap_tool_data<'a>(data: &'a JoinData) -> Option<&'a ToolData> {
    if let Some(Tool(tool)) = data.loadout.active_item.as_ref().map(|i| &i.item.kind) {
        Some(tool)
    } else {
        None
    }
}
