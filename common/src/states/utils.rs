use crate::{
    comp::{
        inventory::slot::EquipSlot,
        item::{Hands, ItemKind, Tool},
        quadruped_low, quadruped_medium, theropod, Body, CharacterState, StateUpdate,
    },
    consts::{FRIC_GROUND, GRAVITY},
    event::LocalEvent,
    states::{behavior::JoinData, *},
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::*;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;
const BASE_HUMANOID_AIR_ACCEL: f32 = 8.0;
const BASE_FLIGHT_ACCEL: f32 = 16.0;
const BASE_HUMANOID_WATER_ACCEL: f32 = 150.0;
const BASE_HUMANOID_WATER_SPEED: f32 = 180.0;
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

impl Body {
    pub fn base_accel(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 100.0,
            Body::QuadrupedSmall(_) => 125.0,
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Grolgar => 110.0,
                quadruped_medium::Species::Saber => 180.0,
                quadruped_medium::Species::Tiger => 150.0,
                quadruped_medium::Species::Tuskram => 160.0,
                quadruped_medium::Species::Lion => 170.0,
                quadruped_medium::Species::Tarasque => 100.0,
                quadruped_medium::Species::Wolf => 180.0,
                quadruped_medium::Species::Frostfang => 180.0,
                quadruped_medium::Species::Mouflon => 100.0,
                quadruped_medium::Species::Catoblepas => 70.0,
                quadruped_medium::Species::Bonerattler => 130.0,
                quadruped_medium::Species::Deer => 150.0,
                quadruped_medium::Species::Hirdrasil => 160.0,
                quadruped_medium::Species::Roshwalr => 160.0,
                quadruped_medium::Species::Donkey => 110.0,
                quadruped_medium::Species::Camel => 75.0,
                quadruped_medium::Species::Zebra => 150.0,
                quadruped_medium::Species::Antelope => 185.0,
                quadruped_medium::Species::Kelpie => 180.0,
                quadruped_medium::Species::Horse => 180.0,
            },
            Body::BirdMedium(_) => 80.0,
            Body::FishMedium(_) => 80.0,
            Body::Dragon(_) => 250.0,
            Body::BirdSmall(_) => 75.0,
            Body::FishSmall(_) => 60.0,
            Body::BipedLarge(_) => 75.0,
            Body::Object(_) => 40.0,
            Body::Golem(_) => 60.0,
            Body::Theropod(_) => 135.0,
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 130.0,
                quadruped_low::Species::Alligator => 110.0,
                quadruped_low::Species::Salamander => 85.0,
                quadruped_low::Species::Monitor => 160.0,
                quadruped_low::Species::Asp => 130.0,
                quadruped_low::Species::Tortoise => 60.0,
                quadruped_low::Species::Rocksnapper => 70.0,
                quadruped_low::Species::Pangolin => 120.0,
                quadruped_low::Species::Maneater => 80.0,
                quadruped_low::Species::Sandshark => 160.0,
                quadruped_low::Species::Hakulaq => 140.0,
                quadruped_low::Species::Lavadrake => 100.0,
            },
        }
    }

    /// Attempt to determine the maximum speed of the character
    /// when moving on the ground
    pub fn max_speed_approx(&self) -> f32 {
        // Inverse kinematics: at what velocity will acceleration
        // be cancelled out by friction drag?
        // Note: we assume no air (this is fine, current physics
        // uses max(air_drag, ground_drag)).
        // Derived via...
        // v = (v + dv / 30) * (1 - drag).powi(2) (accel cancels drag)
        // => 1 = (1 + (dv / 30) / v) * (1 - drag).powi(2)
        // => 1 / (1 - drag).powi(2) = 1 + (dv / 30) / v
        // => 1 / (1 - drag).powi(2) - 1 = (dv / 30) / v
        // => 1 / (1 / (1 - drag).powi(2) - 1) = v / (dv / 30)
        // => (dv / 30) / (1 / (1 - drag).powi(2) - 1) = v
        let v = (-self.base_accel() / 30.0) / ((1.0 - FRIC_GROUND).powi(2) - 1.0);
        debug_assert!(v >= 0.0, "Speed must be positive!");
        v
    }

    pub fn base_ori_rate(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 20.0,
            Body::QuadrupedSmall(_) => 15.0,
            Body::QuadrupedMedium(_) => 8.0,
            Body::BirdMedium(_) => 30.0,
            Body::FishMedium(_) => 5.0,
            Body::Dragon(_) => 5.0,
            Body::BirdSmall(_) => 35.0,
            Body::FishSmall(_) => 10.0,
            Body::BipedLarge(_) => 12.0,
            Body::Object(_) => 5.0,
            Body::Golem(_) => 8.0,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 2.5,
                theropod::Species::Odonto => 2.5,
                _ => 7.0,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Monitor => 9.0,
                quadruped_low::Species::Asp => 8.0,
                quadruped_low::Species::Tortoise => 3.0,
                quadruped_low::Species::Rocksnapper => 4.0,
                quadruped_low::Species::Maneater => 5.0,
                quadruped_low::Species::Lavadrake => 4.0,
                _ => 6.0,
            },
        }
    }

    pub fn can_fly(&self) -> bool {
        matches!(
            self,
            Body::BirdMedium(_) | Body::Dragon(_) | Body::BirdSmall(_)
        )
    }

    pub fn can_climb(&self) -> bool { matches!(self, Body::Humanoid(_)) }
}

/// Handles updating `Components` to move player based on state of `JoinData`
pub fn handle_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    if let Some(depth) = data.physics.in_liquid {
        swim_move(data, update, efficiency, depth);
    } else if data.inputs.fly.is_pressed() && !data.physics.on_ground && data.body.can_fly() {
        fly_move(data, update, efficiency);
    } else {
        basic_move(data, update, efficiency);
    }
}

/// Updates components to move player as if theyre on ground or in air
#[allow(clippy::assign_op_pattern)] // TODO: Pending review in #587
fn basic_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    let accel = if data.physics.on_ground {
        data.body.base_accel()
    } else {
        BASE_HUMANOID_AIR_ACCEL
    };

    update.vel.0 =
        update.vel.0 + Vec2::broadcast(data.dt.0) * data.inputs.move_dir * accel * efficiency;

    handle_orientation(data, update, data.body.base_ori_rate());
}

/// Handles forced movement
pub fn handle_forced_movement(
    data: &JoinData,
    update: &mut StateUpdate,
    movement: ForcedMovement,
    efficiency: f32,
) {
    match movement {
        ForcedMovement::Forward { strength } => {
            let accel = if data.physics.on_ground {
                data.body.base_accel()
            } else {
                BASE_HUMANOID_AIR_ACCEL
            };

            update.vel.0 += Vec2::broadcast(data.dt.0)
                * accel
                * (data.inputs.move_dir * efficiency + (*update.ori.0).xy() * strength);
        },
        ForcedMovement::Leap {
            vertical,
            forward,
            progress,
            direction,
        } => {
            let dir = direction.get_2d_dir(data);
            // Apply jumping force
            update.vel.0 = Vec3::new(
                dir.x,
                dir.y,
                vertical,
            )
                // Multiply decreasing amount linearly over time (with average of 1)
                * 2.0 * progress
                // Apply direction
                + Vec3::from(dir)
                // Multiply by forward leap strength
                    * forward
                // Control forward movement based on look direction.
                // This allows players to stop moving forward when they
                // look downward at target
                    * (1.0 - data.inputs.look_dir.z.abs());
        },
        ForcedMovement::Hover { move_input } => {
            update.vel.0 = Vec3::new(data.vel.0.x, data.vel.0.y, 0.0)
                + move_input * data.inputs.move_dir.try_normalized().unwrap_or_default();
        },
    }
    handle_orientation(data, update, data.body.base_ori_rate() * efficiency);
}

pub fn handle_orientation(data: &JoinData, update: &mut StateUpdate, rate: f32) {
    // Set direction based on move direction
    let ori_dir = if (update.character.is_aimed() && data.body.can_strafe())
        || update.character.is_attack()
    {
        data.inputs.look_dir.xy()
    } else if !data.inputs.move_dir.is_approx_zero() {
        data.inputs.move_dir
    } else {
        update.ori.0.xy()
    };

    // Smooth orientation
    update.ori.0 = Dir::slerp_to_vec3(update.ori.0, ori_dir.into(), rate * data.dt.0);
}

/// Updates components to move player as if theyre swimming
fn swim_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32, depth: f32) {
    // Update velocity
    update.vel.0 += Vec2::broadcast(data.dt.0)
        * data.inputs.move_dir
        * if update.vel.0.magnitude_squared() < BASE_HUMANOID_WATER_SPEED.powi(2) {
            BASE_HUMANOID_WATER_ACCEL
        } else {
            0.0
        }
        * efficiency;

    handle_orientation(data, update, if data.physics.on_ground { 9.0 } else { 2.0 });

    // Swim
    update.vel.0.z = (update.vel.0.z
        + data.dt.0
            * GRAVITY
            * 4.0
            * data
                .inputs
                .move_z
                .clamped(-1.0, depth.clamped(0.0, 1.0).powi(3)))
    .min(BASE_HUMANOID_WATER_SPEED);
}

/// Updates components to move entity as if it's flying
fn fly_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    // Update velocity (counteract gravity with lift)
    // TODO: Do this better
    update.vel.0 += Vec3::unit_z() * data.dt.0 * GRAVITY
        + Vec3::new(
            data.inputs.move_dir.x,
            data.inputs.move_dir.y,
            data.inputs.move_z,
        ) * data.dt.0
            * BASE_FLIGHT_ACCEL
            * efficiency;

    handle_orientation(data, update, 1.0);
}

/// First checks whether `primary`, `secondary` or `ability3` input is pressed,
/// then attempts to go into Equipping state, otherwise Idle
pub fn handle_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed()
        || data.inputs.secondary.is_pressed()
        || data.inputs.ability3.is_pressed()
    {
        attempt_wield(data, update);
    }
}

/// If a tool is equipped, goes into Equipping state, otherwise goes to Idle
pub fn attempt_wield(data: &JoinData, update: &mut StateUpdate) {
    if let Some(ItemKind::Tool(tool)) = data
        .inventory
        .equipped(EquipSlot::Mainhand)
        .map(|i| i.kind())
    {
        update.character = CharacterState::Equipping(equipping::Data {
            static_data: equipping::StaticData {
                buildup_duration: tool.equip_time(),
            },
            timer: Duration::default(),
        });
    } else {
        update.character = CharacterState::Idle;
    };
}

/// Checks that player can `Sit` and updates `CharacterState` if so
pub fn attempt_sit(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.on_ground {
        update.character = CharacterState::Sit;
    }
}

pub fn attempt_dance(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.on_ground && data.body.is_humanoid() {
        update.character = CharacterState::Dance;
    }
}

pub fn attempt_sneak(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.on_ground && data.body.is_humanoid() {
        update.character = CharacterState::Sneak;
    }
}

/// Checks that player can `Climb` and updates `CharacterState` if so
pub fn handle_climb(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.climb.is_some()
        && data.physics.on_wall.is_some()
        && !data.physics.on_ground
        && !data
            .physics
            .in_liquid
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        //&& update.vel.0.z < 0.0
        && data.body.can_climb()
        && update.energy.current() > 100
    {
        update.character = CharacterState::Climb;
    }
}

/// Checks that player can Swap Weapons and updates `Loadout` if so
pub fn attempt_swap_loadout(data: &JoinData, update: &mut StateUpdate) {
    if data.inventory.equipped(EquipSlot::Offhand).is_some() {
        update.swap_loadout = true;
    }
}

/// Checks that player can wield the glider and updates `CharacterState` if so
pub fn attempt_glide_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inventory.equipped(EquipSlot::Glider).is_some()
        && !data
            .physics
            .in_liquid
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        && data.body.is_humanoid()
    {
        update.character = CharacterState::GlideWield;
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.jump.is_pressed()
        && data.physics.on_ground
        && !data
            .physics
            .in_liquid
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
    {
        update
            .local_events
            .push_front(LocalEvent::Jump(data.entity));
    }
}

pub fn handle_ability1_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.primary.is_pressed() {
        if let Some(ability) = data
            .inventory
            .equipped(EquipSlot::Mainhand)
            .and_then(|i| i.item_config_expect().ability1.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = (ability, AbilityKey::Mouse1).into();
        }
    }
}

pub fn handle_ability2_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.secondary.is_pressed() {
        let active_tool_kind = match data
            .inventory
            .equipped(EquipSlot::Mainhand)
            .map(|i| i.kind())
        {
            Some(ItemKind::Tool(Tool { kind, .. })) => Some(kind),
            _ => None,
        };

        let second_tool_kind = match data
            .inventory
            .equipped(EquipSlot::Offhand)
            .map(|i| i.kind())
        {
            Some(ItemKind::Tool(Tool { kind, .. })) => Some(kind),
            _ => None,
        };

        match (
            active_tool_kind.map(|tk| tk.hands()),
            second_tool_kind.map(|tk| tk.hands()),
        ) {
            (Some(Hands::TwoHand), _) => {
                if let Some(ability) = data
                    .inventory
                    .equipped(EquipSlot::Mainhand)
                    .and_then(|i| i.item_config_expect().ability2.as_ref())
                    .filter(|ability| ability.requirements_paid(data, update))
                {
                    update.character = (ability, AbilityKey::Mouse2).into();
                }
            },
            (_, Some(Hands::OneHand)) => {
                if let Some(ability) = data
                    .inventory
                    .equipped(EquipSlot::Offhand)
                    .and_then(|i| i.item_config_expect().ability2.as_ref())
                    .filter(|ability| ability.requirements_paid(data, update))
                {
                    update.character = (ability, AbilityKey::Mouse2).into();
                }
            },
            (_, _) => {},
        };
    }
}

pub fn handle_ability3_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.ability3.is_pressed() {
        if let Some(ability) = data
            .inventory
            .equipped(EquipSlot::Mainhand)
            .and_then(|i| i.item_config_expect().ability3.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = (ability, AbilityKey::Skill1).into();
        }
    }
}

/// Checks that player can perform a dodge, then
/// attempts to perform their dodge ability
pub fn handle_dodge_input(data: &JoinData, update: &mut StateUpdate) {
    if data.inputs.roll.is_pressed() && data.body.is_humanoid() {
        if let Some(ability) = data
            .inventory
            .equipped(EquipSlot::Mainhand)
            .and_then(|i| i.item_config_expect().dodge_ability.as_ref())
            .filter(|ability| ability.requirements_paid(data, update))
        {
            if data.character.is_wield() {
                update.character = (ability, AbilityKey::Dodge).into();
                if let CharacterState::Roll(roll) = &mut update.character {
                    roll.was_wielded = true;
                }
            } else if data.character.is_stealthy() {
                update.character = (ability, AbilityKey::Dodge).into();
                if let CharacterState::Roll(roll) = &mut update.character {
                    roll.was_sneak = true;
                }
            } else {
                update.character = (ability, AbilityKey::Dodge).into();
            }
        }
    }
}

pub fn unwrap_tool_data<'a>(data: &'a JoinData) -> Option<&'a Tool> {
    if let Some(ItemKind::Tool(tool)) = data
        .inventory
        .equipped(EquipSlot::Mainhand)
        .map(|i| i.kind())
    {
        Some(&tool)
    } else {
        None
    }
}

pub fn handle_interrupt(data: &JoinData, update: &mut StateUpdate, attacks_interrupt: bool) {
    if attacks_interrupt {
        handle_ability1_input(data, update);
        handle_ability2_input(data, update);
        handle_ability3_input(data, update);
    }
    handle_dodge_input(data, update);
}

pub fn ability_key_is_pressed(data: &JoinData, ability_key: AbilityKey) -> bool {
    match ability_key {
        AbilityKey::Mouse1 => data.inputs.primary.is_pressed(),
        AbilityKey::Mouse2 => data.inputs.secondary.is_pressed(),
        AbilityKey::Skill1 => data.inputs.ability3.is_pressed(),
        AbilityKey::Dodge => data.inputs.roll.is_pressed(),
    }
}

pub fn continue_combo(data: &JoinData, update: &mut StateUpdate, combo_data: (u32, u32)) {
    handle_ability1_input(data, update);
    if let CharacterState::ComboMelee(data) = &mut update.character {
        data.stage = combo_data.0;
        data.combo = combo_data.1;
    }
}

/// Determines what portion a state is in. Used in all attacks (eventually). Is
/// used to control aspects of animation code, as well as logic within the
/// character states.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum StageSection {
    Buildup,
    Swing,
    Recover,
    Charge,
    Cast,
    Shoot,
    Movement,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum AbilityKey {
    Mouse1,
    Mouse2,
    Skill1,
    Dodge,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ForcedMovement {
    Forward {
        strength: f32,
    },
    Leap {
        vertical: f32,
        forward: f32,
        progress: f32,
        direction: MovementDirection,
    },
    Hover {
        move_input: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementDirection {
    Look,
    Move,
}

impl MovementDirection {
    pub fn get_2d_dir(self, data: &JoinData) -> Vec2<f32> {
        use MovementDirection::*;
        match self {
            Look => data.inputs.look_dir.xy(),
            Move => data.inputs.move_dir,
        }
        .try_normalized()
        .unwrap_or_default()
    }
}
