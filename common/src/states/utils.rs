use crate::{
    astar::Astar,
    combat,
    comp::{
        biped_large, biped_small,
        inventory::slot::{EquipSlot, Slot},
        item::{Hands, ItemKind, Tool, ToolKind},
        quadruped_low, quadruped_medium, quadruped_small,
        skills::{Skill, SwimSkill, SKILL_MODIFIERS},
        theropod, Body, CharacterAbility, CharacterState, Density, InputAttr, InputKind,
        InventoryAction, StateUpdate,
    },
    consts::{FRIC_GROUND, GRAVITY, MAX_PICKUP_RANGE},
    event::{LocalEvent, ServerEvent},
    states::{behavior::JoinData, *},
    util::Dir,
    vol::ReadVol,
};
use core::hash::BuildHasherDefault;
use fxhash::FxHasher64;
use serde::{Deserialize, Serialize};
use std::{
    ops::{Add, Div},
    time::Duration,
};
use strum_macros::Display;
use vek::*;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;

impl Body {
    pub fn base_accel(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 100.0,
            Body::QuadrupedSmall(body) => match body.species {
                quadruped_small::Species::Turtle => 30.0,
                quadruped_small::Species::Axolotl => 70.0,
                quadruped_small::Species::Pig => 70.0,
                quadruped_small::Species::Sheep => 70.0,
                quadruped_small::Species::Cat => 70.0,
                quadruped_small::Species::Truffler => 70.0,
                quadruped_small::Species::Fungome => 70.0,
                quadruped_small::Species::Goat => 80.0,
                _ => 125.0,
            },
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
                quadruped_medium::Species::Barghest => 80.0,
                quadruped_medium::Species::Cattle => 80.0,
                quadruped_medium::Species::Darkhound => 160.0,
                quadruped_medium::Species::Highland => 80.0,
                quadruped_medium::Species::Yak => 90.0,
                quadruped_medium::Species::Panda => 90.0,
                quadruped_medium::Species::Bear => 90.0,
                quadruped_medium::Species::Dreadhorn => 140.0,
                quadruped_medium::Species::Moose => 130.0,
                quadruped_medium::Species::Snowleopard => 160.0,
                quadruped_medium::Species::Mammoth => 180.0,
                quadruped_medium::Species::Ngoubou => 170.0,
                quadruped_medium::Species::Llama => 120.0,
                quadruped_medium::Species::Alpaca => 110.0,
            },
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Slysaurok => 100.0,
                biped_large::Species::Occultsaurok => 100.0,
                biped_large::Species::Mightysaurok => 100.0,
                biped_large::Species::Mindflayer => 90.0,
                biped_large::Species::Minotaur => 60.0,
                biped_large::Species::Huskbrute => 130.0,
                biped_large::Species::Cultistwarlord => 110.0,
                biped_large::Species::Cultistwarlock => 90.0,
                _ => 80.0,
            },
            Body::BirdMedium(_) => 80.0,
            Body::FishMedium(_) => 80.0,
            Body::Dragon(_) => 250.0,
            Body::BirdLarge(_) => 110.0,
            Body::FishSmall(_) => 60.0,
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::Haniwa => 65.0,
                _ => 80.0,
            },
            Body::Object(_) => 0.0,
            Body::Golem(_) => 60.0,
            Body::Theropod(_) => 135.0,
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 130.0,
                quadruped_low::Species::Alligator => 110.0,
                quadruped_low::Species::Salamander => 85.0,
                quadruped_low::Species::Monitor => 160.0,
                quadruped_low::Species::Asp => 110.0,
                quadruped_low::Species::Tortoise => 60.0,
                quadruped_low::Species::Rocksnapper => 70.0,
                quadruped_low::Species::Pangolin => 120.0,
                quadruped_low::Species::Maneater => 80.0,
                quadruped_low::Species::Sandshark => 160.0,
                quadruped_low::Species::Hakulaq => 140.0,
                quadruped_low::Species::Lavadrake => 100.0,
                quadruped_low::Species::Basilisk => 90.0,
                quadruped_low::Species::Deadwood => 140.0,
            },
            Body::Ship(_) => 0.0,
        }
    }

    pub fn air_accel(&self) -> f32 { self.base_accel() * 0.025 }

    /// Attempt to determine the maximum speed of the character
    /// when moving on the ground
    pub fn max_speed_approx(&self) -> f32 {
        // Inverse kinematics: at what velocity will acceleration
        // be cancelled out by friction drag?
        // Note: we assume no air, since it's such a small factor.
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

    /// The turn rate in 180°/s (or (rotations per second)/2)
    pub fn base_ori_rate(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 3.5,
            Body::QuadrupedSmall(_) => 3.0,
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Mammoth => 2.2,
                _ => 2.8,
            },
            Body::BirdMedium(_) => 6.0,
            Body::FishMedium(_) => 6.0,
            Body::Dragon(_) => 1.0,
            Body::BirdLarge(_) => 7.0,
            Body::FishSmall(_) => 7.0,
            Body::BipedLarge(_) => 2.7,
            Body::BipedSmall(_) => 3.5,
            Body::Object(_) => 2.0,
            Body::Golem(_) => 2.0,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 2.3,
                theropod::Species::Odonto => 2.3,
                theropod::Species::Ntouka => 2.3,
                _ => 2.5,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Asp => 2.2,
                quadruped_low::Species::Tortoise => 1.5,
                quadruped_low::Species::Rocksnapper => 1.8,
                quadruped_low::Species::Lavadrake => 1.7,
                _ => 2.0,
            },
            Body::Ship(_) => 0.035,
        }
    }

    /// Returns thrust force if the body type can swim, otherwise None
    pub fn swim_thrust(&self) -> Option<f32> {
        match self {
            Body::Object(_) | Body::Ship(_) => None,
            Body::BipedLarge(_) | Body::Golem(_) => Some(200.0 * self.mass().0),
            Body::BipedSmall(_) => Some(100.0 * self.mass().0),
            Body::BirdMedium(_) => Some(50.0 * self.mass().0),
            Body::BirdLarge(_) => Some(50.0 * self.mass().0),
            Body::FishMedium(_) => Some(50.0 * self.mass().0),
            Body::FishSmall(_) => Some(50.0 * self.mass().0),
            Body::Dragon(_) => Some(200.0 * self.mass().0),
            Body::Humanoid(_) => Some(2500.0 * self.mass().0),
            Body::Theropod(body) => match body.species {
                theropod::Species::Sandraptor
                | theropod::Species::Snowraptor
                | theropod::Species::Sunlizard
                | theropod::Species::Woodraptor
                | theropod::Species::Yale => Some(200.0 * self.mass().0),
                _ => Some(100.0 * self.mass().0),
            },
            Body::QuadrupedLow(_) => Some(300.0 * self.mass().0),
            Body::QuadrupedMedium(_) => Some(300.0 * self.mass().0),
            Body::QuadrupedSmall(_) => Some(300.0 * self.mass().0),
        }
    }

    /// Returns thrust force if the body type can fly, otherwise None
    pub fn fly_thrust(&self) -> Option<f32> {
        match self {
            Body::BirdMedium(_) => Some(GRAVITY * self.mass().0 * 2.0),
            Body::BirdLarge(_) => Some(GRAVITY * self.mass().0 * 0.5),
            Body::Dragon(_) => Some(200_000.0),
            Body::Ship(ship) if ship.can_fly() => Some(300_000.0),
            _ => None,
        }
    }

    /// Returns jump impulse if the body type can jump, otherwise None
    pub fn jump_impulse(&self) -> Option<f32> {
        match self {
            Body::Object(_) | Body::Ship(_) => None,
            Body::BipedLarge(_) | Body::Dragon(_) | Body::Golem(_) | Body::QuadrupedLow(_) => {
                Some(0.1 * self.mass().0)
            },
            Body::QuadrupedMedium(_) => Some(0.4 * self.mass().0),
            Body::Theropod(body) => match body.species {
                theropod::Species::Snowraptor
                | theropod::Species::Sandraptor
                | theropod::Species::Woodraptor => Some(0.4 * self.mass().0),
                _ => None,
            },
            _ => Some(0.4 * self.mass().0),
        }
        .map(|f| f * GRAVITY)
    }

    pub fn can_climb(&self) -> bool { matches!(self, Body::Humanoid(_)) }

    /// Returns how well a body can move backwards while strafing (0.0 = not at
    /// all, 1.0 = same as forward)
    pub fn reverse_move_factor(&self) -> f32 { 0.45 }

    /// Returns the position where a projectile should be fired relative to this
    /// body
    pub fn projectile_offsets(&self, ori: Vec3<f32>) -> Vec3<f32> {
        let body_offsets_z = match self {
            Body::Golem(_) => self.height() * 0.4,
            _ => self.eye_height(),
        };

        let dim = self.dimensions();
        // The width (shoulder to shoulder) and length (nose to tail)
        let (width, length) = (dim.x, dim.y);
        let body_radius = if length > width {
            // Dachshund-like
            self.max_radius()
        } else {
            // Cyclops-like
            self.min_radius()
        };

        Vec3::new(
            body_radius * ori.x * 1.1,
            body_radius * ori.y * 1.1,
            body_offsets_z,
        )
    }
}

/// Handles updating `Components` to move player based on state of `JoinData`
pub fn handle_move(data: &JoinData<'_>, update: &mut StateUpdate, efficiency: f32) {
    let submersion = data
        .physics
        .in_liquid()
        .map(|depth| depth / data.body.height());

    if input_is_pressed(data, InputKind::Fly)
        && submersion.map_or(true, |sub| sub < 1.0)
        && (data.physics.on_ground.is_none() || data.body.jump_impulse().is_none())
        && data.body.fly_thrust().is_some()
    {
        fly_move(data, update, efficiency);
    } else if let Some(submersion) = (data.physics.on_ground.is_none()
        && data.body.swim_thrust().is_some())
    .then_some(submersion)
    .flatten()
    {
        swim_move(data, update, efficiency, submersion);
    } else {
        basic_move(data, update, efficiency);
    }
}

/// Updates components to move player as if theyre on ground or in air
fn basic_move(data: &JoinData<'_>, update: &mut StateUpdate, efficiency: f32) {
    let efficiency = efficiency * data.stats.move_speed_modifier * data.stats.friction_modifier;

    let accel = if data.physics.on_ground.is_some() {
        data.body.base_accel()
    } else {
        data.body.air_accel()
    } * efficiency;

    // Should ability to backpedal be separate from ability to strafe?
    update.vel.0 += Vec2::broadcast(data.dt.0)
        * accel
        * if data.body.can_strafe() {
            data.inputs.move_dir
                * if is_strafing(data, update) {
                    Lerp::lerp(
                        Vec2::from(update.ori)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            .dot(
                                data.inputs
                                    .move_dir
                                    .try_normalized()
                                    .unwrap_or_else(Vec2::zero),
                            )
                            .add(1.0)
                            .div(2.0)
                            .max(0.0),
                        1.0,
                        data.body.reverse_move_factor(),
                    )
                } else {
                    1.0
                }
        } else {
            let fw = Vec2::from(update.ori);
            fw * data.inputs.move_dir.dot(fw).max(0.0)
        };
}

/// Handles forced movement
pub fn handle_forced_movement(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    movement: ForcedMovement,
) {
    match movement {
        ForcedMovement::Forward { strength } => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|_| data.body.base_accel()) {
                update.vel.0 += Vec2::broadcast(data.dt.0)
                    * accel
                    * (data.inputs.move_dir + Vec2::from(update.ori))
                    * strength;
            }
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
}

pub fn handle_orientation(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    efficiency: f32,
    dir_override: Option<Dir>,
) {
    // Direction is set to the override if one is provided, else if entity is
    // strafing or attacking the horiontal component of the look direction is used,
    // else the current horizontal movement direction is used
    let dir = if let Some(dir_override) = dir_override {
        dir_override
    } else if is_strafing(data, update) || update.character.is_attack() {
        data.inputs.look_dir.to_horizontal().unwrap_or_default()
    } else {
        Dir::from_unnormalized(data.inputs.move_dir.into())
            .unwrap_or_else(|| data.ori.to_horizontal().look_dir())
    };
    let rate = {
        let angle = update.ori.look_dir().angle_between(*dir);
        data.body.base_ori_rate() * efficiency * std::f32::consts::PI / angle
    };
    update.ori = update
        .ori
        .slerped_towards(dir.into(), (data.dt.0 * rate).min(1.0));
}

/// Updates components to move player as if theyre swimming
fn swim_move(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    efficiency: f32,
    submersion: f32,
) -> bool {
    let efficiency = efficiency * data.stats.move_speed_modifier * data.stats.friction_modifier;
    if let Some(force) = data.body.swim_thrust() {
        let force = efficiency * force;
        let mut water_accel = force / data.mass.0;

        if let Ok(Some(level)) = data.skill_set.skill_level(Skill::Swim(SwimSkill::Speed)) {
            let modifiers = SKILL_MODIFIERS.general_tree.swim;
            water_accel *= modifiers.speed.powi(level.into());
        }

        let dir = if data.body.can_strafe() {
            data.inputs.move_dir
        } else {
            let fw = Vec2::from(update.ori);
            fw * data.inputs.move_dir.dot(fw).max(0.0)
        };

        // Autoswim to stay afloat
        let move_z = if submersion < 1.0 && data.inputs.move_z.abs() < std::f32::EPSILON {
            (submersion - 0.1).max(0.0)
        } else {
            data.inputs.move_z
        };

        update.vel.0 += Vec3::broadcast(data.dt.0)
            * Vec3::new(dir.x, dir.y, move_z)
                .try_normalized()
                .unwrap_or_default()
            * water_accel
            * (submersion - 0.2).clamp(0.0, 1.0).powi(2);

        true
    } else {
        false
    }
}

/// Updates components to move entity as if it's flying
pub fn fly_move(data: &JoinData<'_>, update: &mut StateUpdate, efficiency: f32) -> bool {
    let efficiency = efficiency * data.stats.move_speed_modifier * data.stats.friction_modifier;

    let glider = match data.character {
        CharacterState::Glide(data) => Some(data),
        _ => None,
    };
    if let Some(force) = data
        .body
        .fly_thrust()
        .or_else(|| glider.is_some().then_some(0.0))
    {
        let thrust = efficiency * force;
        let accel = thrust / data.mass.0;

        handle_orientation(data, update, efficiency, None);

        // Elevation control
        match data.body {
            // flappy flappy
            Body::Dragon(_) | Body::BirdLarge(_) | Body::BirdMedium(_) => {
                let anti_grav = GRAVITY * (1.0 + data.inputs.move_z.min(0.0));
                update.vel.0.z += data.dt.0 * (anti_grav + accel * data.inputs.move_z.max(0.0));
            },
            // floaty floaty
            Body::Ship(ship) if ship.can_fly() => {
                let regulate_density = |min: f32, max: f32, def: f32, rate: f32| -> Density {
                    // Reset to default on no input
                    let change = if data.inputs.move_z.abs() > std::f32::EPSILON {
                        -data.inputs.move_z
                    } else {
                        (def - data.density.0).max(-1.0).min(1.0)
                    };
                    Density((update.density.0 + data.dt.0 * rate * change).clamp(min, max))
                };
                let def_density = ship.density().0;
                if data.physics.in_liquid().is_some() {
                    let hull_density = ship.hull_density().0;
                    update.density.0 =
                        regulate_density(def_density * 0.6, hull_density, hull_density, 25.0).0;
                } else {
                    update.density.0 =
                        regulate_density(def_density * 0.5, def_density * 1.5, def_density, 0.5).0;
                };
            },
            // oopsie woopsie
            // TODO: refactor to make this state impossible
            _ => {},
        };

        update.vel.0 += Vec2::broadcast(data.dt.0)
            * accel
            * if data.body.can_strafe() {
                data.inputs.move_dir
            } else {
                let fw = Vec2::from(update.ori);
                fw * data.inputs.move_dir.dot(fw).max(0.0)
            };

        true
    } else {
        false
    }
}

/// Checks if an input related to an attack is held. If one is, moves entity
/// into wielding state
pub fn handle_wield(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.controller.queued_inputs.keys().any(|i| i.is_ability()) {
        attempt_wield(data, update);
    }
}

/// If a tool is equipped, goes into Equipping state, otherwise goes to Idle
pub fn attempt_wield(data: &JoinData<'_>, update: &mut StateUpdate) {
    // Closure to get equip time provided an equip slot if a tool is equipped in
    // equip slot
    let equip_time = |equip_slot| {
        data.inventory
            .and_then(|inv| inv.equipped(equip_slot))
            .and_then(|item| match item.kind() {
                ItemKind::Tool(tool) => Some((item, tool)),
                _ => None,
            })
            .map(|(item, tool)| tool.equip_time(data.msm, item.components()))
    };

    // Calculates time required to equip weapons, if weapon in mainhand and offhand,
    // uses maximum duration
    let mainhand_equip_time = equip_time(EquipSlot::ActiveMainhand);
    let offhand_equip_time = equip_time(EquipSlot::ActiveOffhand);
    let equip_time = match (mainhand_equip_time, offhand_equip_time) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    };

    // Moves entity into equipping state if there is some equip time, else moves
    // intantly into wield
    if let Some(equip_time) = equip_time {
        update.character = CharacterState::Equipping(equipping::Data {
            static_data: equipping::StaticData {
                buildup_duration: equip_time,
            },
            timer: Duration::default(),
        });
    } else {
        update.character = CharacterState::Wielding;
    };
}

/// Checks that player can `Sit` and updates `CharacterState` if so
pub fn attempt_sit(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.physics.on_ground.is_some() {
        update.character = CharacterState::Sit;
    }
}

pub fn attempt_dance(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.physics.on_ground.is_some() && data.body.is_humanoid() {
        update.character = CharacterState::Dance;
    }
}

pub fn attempt_talk(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.physics.on_ground.is_some() {
        update.character = CharacterState::Talk;
    }
}

pub fn attempt_sneak(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.physics.on_ground.is_some() && data.body.is_humanoid() {
        update.character = CharacterState::Sneak;
    }
}

/// Checks that player can `Climb` and updates `CharacterState` if so
pub fn handle_climb(data: &JoinData<'_>, update: &mut StateUpdate) -> bool {
    if data.inputs.climb.is_some()
        && data.physics.on_wall.is_some()
        && data.physics.on_ground.is_none()
        && !data
            .physics
            .in_liquid()
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        //&& update.vel.0.z < 0.0
        && data.body.can_climb()
        && update.energy.current() > 100
    {
        update.character = CharacterState::Climb(climb::Data::create_adjusted_by_skills(data));
        true
    } else {
        false
    }
}

/// Checks that player can Swap Weapons and updates `Loadout` if so
pub fn attempt_swap_equipped_weapons(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data
        .inventory
        .and_then(|inv| inv.equipped(EquipSlot::InactiveMainhand))
        .is_some()
        || data
            .inventory
            .and_then(|inv| inv.equipped(EquipSlot::InactiveOffhand))
            .is_some()
    {
        update.swap_equipped_weapons = true;
    }
}

/// Handles inventory manipulations that affect the loadout
pub fn handle_manipulate_loadout(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    inv_action: InventoryAction,
) {
    match inv_action {
        InventoryAction::Use(Slot::Inventory(inv_slot)) => {
            // If inventory action is using a slot, and slot is in the inventory
            // TODO: Do some non lazy way of handling the possibility that items equipped in
            // the loadout will have effects that are desired to be non-instantaneous
            use use_item::ItemUseKind;
            if let Some((item_kind, item)) = data
                .inventory
                .and_then(|inv| inv.get(inv_slot))
                .and_then(|item| Option::<ItemUseKind>::from(item.kind()).zip(Some(item)))
            {
                let (buildup_duration, use_duration, recover_duration) = item_kind.durations();
                // If item returns a valid kind for item use, do into use item character state
                update.character = CharacterState::UseItem(use_item::Data {
                    static_data: use_item::StaticData {
                        buildup_duration,
                        use_duration,
                        recover_duration,
                        inv_slot,
                        item_kind,
                        item_definition_id: item.item_definition_id().to_string(),
                        was_wielded: matches!(data.character, CharacterState::Wielding),
                        was_sneak: matches!(data.character, CharacterState::Sneak),
                    },
                    timer: Duration::default(),
                    stage_section: StageSection::Buildup,
                });
            } else {
                // Else emit inventory action instantnaneously
                update
                    .server_events
                    .push_front(ServerEvent::InventoryManip(data.entity, inv_action.into()));
            }
        },
        InventoryAction::Collect(sprite_pos) => {
            let sprite_pos_f32 = sprite_pos.map(|x| x as f32 + 0.5);
            // Closure to check if distance between a point and the sprite is less than
            // MAX_PICKUP_RANGE and the radius of the body
            let sprite_range_check = |pos: Vec3<f32>| {
                (sprite_pos_f32 - pos).magnitude_squared()
                    < (MAX_PICKUP_RANGE + data.body.max_radius()).powi(2)
            };

            // Checks if player's feet or head is near to sprite
            let close_to_sprite = sprite_range_check(data.pos.0)
                || sprite_range_check(data.pos.0 + Vec3::new(0.0, 0.0, data.body.height()));
            if close_to_sprite {
                // First, get sprite data for position, if there is a sprite
                use sprite_interact::SpriteInteractKind;
                let sprite_at_pos = data
                    .terrain
                    .get(sprite_pos)
                    .ok()
                    .copied()
                    .and_then(|b| b.get_sprite());

                // Checks if position has a collectible sprite as well as what sprite is at the
                // position
                let sprite_interact = sprite_at_pos.and_then(Option::<SpriteInteractKind>::from);

                if let Some(sprite_interact) = sprite_interact {
                    // Do a check that a path can be found between sprite and entity
                    // interacting with sprite Use manhattan distance * 1.5 for number
                    // of iterations
                    let iters =
                        (3.0 * (sprite_pos_f32 - data.pos.0).map(|x| x.abs()).sum()) as usize;
                    // Heuristic compares manhattan distance of start and end pos
                    let heuristic =
                        move |pos: &Vec3<i32>| (sprite_pos - pos).map(|x| x.abs()).sum() as f32;

                    let mut astar = Astar::new(
                        iters,
                        data.pos.0.map(|x| x.floor() as i32),
                        heuristic,
                        BuildHasherDefault::<FxHasher64>::default(),
                    );

                    // Neighbors are all neighboring blocks that are air
                    let neighbors = |pos: &Vec3<i32>| {
                        const DIRS: [Vec3<i32>; 6] = [
                            Vec3::new(1, 0, 0),
                            Vec3::new(-1, 0, 0),
                            Vec3::new(0, 1, 0),
                            Vec3::new(0, -1, 0),
                            Vec3::new(0, 0, 1),
                            Vec3::new(0, 0, -1),
                        ];
                        let pos = *pos;
                        DIRS.iter().map(move |dir| dir + pos).filter(|pos| {
                            data.terrain
                                .get(*pos)
                                .ok()
                                .map_or(false, |block| !block.is_filled())
                        })
                    };
                    // Transition uses manhattan distance as the cost, with a slightly lower cost
                    // for z transitions
                    let transition = |a: &Vec3<i32>, b: &Vec3<i32>| {
                        let (a, b) = (a.map(|x| x as f32), b.map(|x| x as f32));
                        ((a - b) * Vec3::new(1.0, 1.0, 0.9)).map(|e| e.abs()).sum()
                    };
                    // Pathing satisfied when it reaches the sprite position
                    let satisfied = |pos: &Vec3<i32>| *pos == sprite_pos;

                    let not_blocked_by_terrain = astar
                        .poll(iters, heuristic, neighbors, transition, satisfied)
                        .into_path()
                        .is_some();

                    // If path can be found between entity interacting with sprite and entity, start
                    // interaction with sprite
                    if not_blocked_by_terrain {
                        // If the sprite is collectible, enter the sprite interaction character
                        // state TODO: Handle cases for sprite being
                        // interactible, but not collectible (none currently
                        // exist)
                        let (buildup_duration, use_duration, recover_duration) =
                            sprite_interact.durations();

                        update.character = CharacterState::SpriteInteract(sprite_interact::Data {
                            static_data: sprite_interact::StaticData {
                                buildup_duration,
                                use_duration,
                                recover_duration,
                                sprite_pos,
                                sprite_kind: sprite_interact,
                                was_wielded: matches!(data.character, CharacterState::Wielding),
                                was_sneak: matches!(data.character, CharacterState::Sneak),
                            },
                            timer: Duration::default(),
                            stage_section: StageSection::Buildup,
                        })
                    }
                }
            }
        },
        _ => {
            // Else just do event instantaneously
            update
                .server_events
                .push_front(ServerEvent::InventoryManip(data.entity, inv_action.into()));
        },
    }
}

/// Checks that player can wield the glider and updates `CharacterState` if so
pub fn attempt_glide_wield(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data
        .inventory
        .and_then(|inv| inv.equipped(EquipSlot::Glider))
        .is_some()
        && !data
            .physics
            .in_liquid()
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        && data.body.is_humanoid()
    {
        update.character = CharacterState::GlideWield(glide_wield::Data::from(data));
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(data: &JoinData<'_>, update: &mut StateUpdate, strength: f32) -> bool {
    (input_is_pressed(data, InputKind::Jump) && data.physics.on_ground.is_some())
        .then(|| data.body.jump_impulse())
        .flatten()
        .map(|impulse| {
            update.local_events.push_front(LocalEvent::Jump(
                data.entity,
                strength * impulse / data.mass.0 * data.stats.move_speed_modifier,
            ));
        })
        .is_some()
}

fn handle_ability(data: &JoinData<'_>, update: &mut StateUpdate, input: InputKind) {
    let hands = get_hands(data);

    // Mouse1 and Skill1 always use the MainHand slot
    let always_main_hand = matches!(input, InputKind::Primary | InputKind::Ability(0));
    let no_main_hand = hands.0.is_none();
    // skill_index used to select ability for the AbilityKey::Skill2 input
    let (equip_slot, skill_index) = if no_main_hand {
        (Some(EquipSlot::ActiveOffhand), 1)
    } else if always_main_hand {
        (Some(EquipSlot::ActiveMainhand), 0)
    } else {
        match hands {
            (Some(Hands::Two), _) => (Some(EquipSlot::ActiveMainhand), 1),
            (_, Some(Hands::One)) => (Some(EquipSlot::ActiveOffhand), 0),
            (Some(Hands::One), _) => (Some(EquipSlot::ActiveMainhand), 1),
            (_, _) => (None, 0),
        }
    };

    let unlocked = |(s, a): (Option<Skill>, CharacterAbility)| {
        s.map_or(true, |s| data.skill_set.has_skill(s)).then_some(a)
    };

    if let Some(equip_slot) = equip_slot {
        if let Some(ability) = data
            .inventory
            .and_then(|inv| inv.equipped(equip_slot))
            .map(|i| &i.item_config_expect().abilities)
            .and_then(|abilities| match input {
                InputKind::Primary => Some(abilities.primary.clone()),
                InputKind::Secondary => Some(abilities.secondary.clone()),
                InputKind::Ability(0) => abilities.abilities.get(0).cloned().and_then(unlocked),
                InputKind::Ability(i) => abilities
                    .abilities
                    .get(if i < 2 { skill_index } else { i })
                    .cloned()
                    .and_then(unlocked),
                InputKind::Roll | InputKind::Jump | InputKind::Fly | InputKind::Block => None,
            })
            .map(|a| {
                let tool = unwrap_tool_data(data, equip_slot).map(|t| t.kind);
                a.adjusted_by_skills(data.skill_set, tool)
            })
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::from_input(
                    data,
                    matches!(equip_slot, EquipSlot::ActiveOffhand),
                    input,
                ),
                data,
            ));
        }
    }
}

pub fn handle_ability_input(data: &JoinData<'_>, update: &mut StateUpdate) {
    if let Some(input) = data
        .controller
        .queued_inputs
        .keys()
        .find(|i| i.is_ability())
    {
        handle_ability(data, update, *input);
    }
}

pub fn handle_input(data: &JoinData<'_>, update: &mut StateUpdate, input: InputKind) {
    match input {
        InputKind::Primary | InputKind::Secondary | InputKind::Ability(_) => {
            handle_ability(data, update, input)
        },
        InputKind::Roll => handle_dodge_input(data, update),
        InputKind::Jump => {
            handle_jump(data, update, 1.0);
        },
        InputKind::Block => handle_block_input(data, update),
        InputKind::Fly => {},
    }
}

pub fn attempt_input(data: &JoinData<'_>, update: &mut StateUpdate) {
    // TODO: look into using first() when it becomes stable
    if let Some(input) = data.controller.queued_inputs.keys().next() {
        handle_input(data, update, *input);
    }
}

/// Checks that player can block, then attempts to block
pub fn handle_block_input(data: &JoinData<'_>, update: &mut StateUpdate) {
    let can_block =
        |equip_slot| matches!(unwrap_tool_data(data, equip_slot), Some(tool) if tool.can_block());
    let hands = get_hands(data);
    if input_is_pressed(data, InputKind::Block)
        && (can_block(EquipSlot::ActiveMainhand)
            || (hands.0.is_none() && can_block(EquipSlot::ActiveOffhand)))
    {
        let ability = CharacterAbility::default_block();
        if ability.requirements_paid(data, update) {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::from_input(data, false, InputKind::Roll),
                data,
            ));
        }
    }
}

/// Checks that player can perform a dodge, then
/// attempts to perform their dodge ability
pub fn handle_dodge_input(data: &JoinData<'_>, update: &mut StateUpdate) {
    if input_is_pressed(data, InputKind::Roll) && data.body.is_humanoid() {
        let ability = CharacterAbility::default_roll().adjusted_by_skills(data.skill_set, None);
        if ability.requirements_paid(data, update) {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::from_input(data, false, InputKind::Roll),
                data,
            ));
            if let CharacterState::ComboMelee(c) = data.character {
                if let CharacterState::Roll(roll) = &mut update.character {
                    roll.was_combo = Some((c.static_data.ability_info.input, c.stage));
                    roll.was_wielded = true;
                }
            } else if data.character.is_wield() {
                if let CharacterState::Roll(roll) = &mut update.character {
                    roll.was_wielded = true;
                }
            } else if data.character.is_stealthy() {
                if let CharacterState::Roll(roll) = &mut update.character {
                    roll.was_sneak = true;
                }
            }
        }
    }
}

pub fn is_strafing(data: &JoinData<'_>, update: &StateUpdate) -> bool {
    // TODO: Don't always check `character.is_aimed()`, allow the frontend to
    // control whether the player strafes during an aimed `CharacterState`.
    (update.character.is_aimed() || update.should_strafe) && data.body.can_strafe()
}

pub fn unwrap_tool_data<'a>(data: &'a JoinData, equip_slot: EquipSlot) -> Option<&'a Tool> {
    if let Some(ItemKind::Tool(tool)) = data
        .inventory
        .and_then(|inv| inv.equipped(equip_slot))
        .map(|i| i.kind())
    {
        Some(tool)
    } else {
        None
    }
}

pub fn get_hands(data: &JoinData<'_>) -> (Option<Hands>, Option<Hands>) {
    let hand = |slot| {
        if let Some(ItemKind::Tool(tool)) = data
            .inventory
            .and_then(|inv| inv.equipped(slot))
            .map(|i| i.kind())
        {
            Some(tool.hands)
        } else {
            None
        }
    };
    (
        hand(EquipSlot::ActiveMainhand),
        hand(EquipSlot::ActiveOffhand),
    )
}

/// Returns (critical chance, critical multiplier) which is calculated from
/// equipped weapon and equipped armor respectively
pub fn get_crit_data(data: &JoinData<'_>, ai: AbilityInfo) -> (f32, f32) {
    const DEFAULT_CRIT_CHANCE: f32 = 0.1;

    let crit_chance = ai
        .hand
        .map(|hand| match hand {
            HandInfo::TwoHanded | HandInfo::MainHand => EquipSlot::ActiveMainhand,
            HandInfo::OffHand => EquipSlot::ActiveOffhand,
        })
        .and_then(|slot| data.inventory.and_then(|inv| inv.equipped(slot)))
        .and_then(|item| {
            if let ItemKind::Tool(tool) = item.kind() {
                Some(tool.base_crit_chance(data.msm, item.components()))
            } else {
                None
            }
        })
        .unwrap_or(DEFAULT_CRIT_CHANCE);

    let crit_mult = combat::compute_crit_mult(data.inventory);

    (crit_chance, crit_mult)
}

/// Returns buff strength from the weapon used in the ability
pub fn get_buff_strength(data: &JoinData<'_>, ai: AbilityInfo) -> f32 {
    ai.hand
        .map(|hand| match hand {
            HandInfo::TwoHanded | HandInfo::MainHand => EquipSlot::ActiveMainhand,
            HandInfo::OffHand => EquipSlot::ActiveOffhand,
        })
        .and_then(|slot| data.inventory.and_then(|inv| inv.equipped(slot)))
        .and_then(|item| {
            if let ItemKind::Tool(tool) = item.kind() {
                Some(tool.base_buff_strength(data.msm, item.components()))
            } else {
                None
            }
        })
        .unwrap_or(1.0)
}

pub fn handle_state_interrupt(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    attacks_interrupt: bool,
) {
    if attacks_interrupt {
        handle_ability_input(data, update);
    }
    handle_dodge_input(data, update);
}

pub fn input_is_pressed(data: &JoinData<'_>, input: InputKind) -> bool {
    data.controller.queued_inputs.contains_key(&input)
}

/// Checked `Duration` addition. Computes `timer` + `dt`, applying relevant stat
/// attack modifiers and `other_modifiers`, returning None if overflow occurred.
pub fn checked_tick_attack(
    data: &JoinData<'_>,
    timer: Duration,
    other_modifier: Option<f32>,
) -> Option<Duration> {
    timer.checked_add(Duration::from_secs_f32(
        data.dt.0 * data.stats.attack_speed_modifier * other_modifier.unwrap_or(1.0),
    ))
}
/// Ticks `timer` by `dt`, applying relevant stat attack modifiers and
/// `other_modifier`. Returns `Duration::default()` if overflow occurs
pub fn tick_attack_or_default(
    data: &JoinData<'_>,
    timer: Duration,
    other_modifier: Option<f32>,
) -> Duration {
    checked_tick_attack(data, timer, other_modifier).unwrap_or_default()
}
/// Determines what portion a state is in. Used in all attacks (eventually). Is
/// used to control aspects of animation code, as well as logic within the
/// character states.
#[derive(Clone, Copy, Debug, Display, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum StageSection {
    Buildup,
    Recover,
    Charge,
    Movement,
    Action,
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
    pub fn get_2d_dir(self, data: &JoinData<'_>) -> Vec2<f32> {
        use MovementDirection::*;
        match self {
            Look => data
                .inputs
                .look_dir
                .to_horizontal()
                .unwrap_or_default()
                .xy(),
            Move => data.inputs.move_dir,
        }
        .try_normalized()
        .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AbilityInfo {
    pub tool: Option<ToolKind>,
    pub hand: Option<HandInfo>,
    pub input: InputKind,
    pub input_attr: Option<InputAttr>,
}

impl AbilityInfo {
    pub fn from_input(data: &JoinData<'_>, from_offhand: bool, input: InputKind) -> Self {
        let tool_data = if from_offhand {
            unwrap_tool_data(data, EquipSlot::ActiveOffhand)
        } else {
            unwrap_tool_data(data, EquipSlot::ActiveMainhand)
        };
        let (tool, hand) = (
            tool_data.map(|t| t.kind),
            tool_data.map(|t| HandInfo::from_main_tool(t, from_offhand)),
        );

        Self {
            tool,
            hand,
            input,
            input_attr: data.controller.queued_inputs.get(&input).copied(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum HandInfo {
    TwoHanded,
    MainHand,
    OffHand,
}

impl HandInfo {
    pub fn from_main_tool(tool: &Tool, from_offhand: bool) -> Self {
        match tool.hands {
            Hands::Two => Self::TwoHanded,
            Hands::One => {
                if from_offhand {
                    Self::OffHand
                } else {
                    Self::MainHand
                }
            },
        }
    }
}
