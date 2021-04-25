use crate::{
    comp::{
        biped_large, biped_small,
        inventory::slot::EquipSlot,
        item::{Hands, ItemKind, Tool, ToolKind},
        quadruped_low, quadruped_medium, quadruped_small, ship,
        skills::{Skill, SwimSkill},
        theropod, Body, CharacterAbility, CharacterState, Density, InputAttr, InputKind,
        InventoryAction, StateUpdate,
    },
    consts::{FRIC_GROUND, GRAVITY},
    event::{LocalEvent, ServerEvent},
    states::{behavior::JoinData, *},
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
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
            },
            Body::BipedLarge(body) => match body.species {
                biped_large::Species::Slysaurok => 100.0,
                biped_large::Species::Occultsaurok => 100.0,
                biped_large::Species::Mightysaurok => 100.0,
                biped_large::Species::Mindflayer => 90.0,
                biped_large::Species::Minotaur => 90.0,
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
                quadruped_low::Species::Asp => 130.0,
                quadruped_low::Species::Tortoise => 60.0,
                quadruped_low::Species::Rocksnapper => 70.0,
                quadruped_low::Species::Pangolin => 120.0,
                quadruped_low::Species::Maneater => 80.0,
                quadruped_low::Species::Sandshark => 160.0,
                quadruped_low::Species::Hakulaq => 140.0,
                quadruped_low::Species::Lavadrake => 100.0,
                quadruped_low::Species::Basilisk => 120.0,
                quadruped_low::Species::Deadwood => 140.0,
            },
            Body::Ship(_) => 0.0,
        }
    }

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
            Body::Humanoid(_) => 4.0,
            Body::QuadrupedSmall(_) => 3.0,
            Body::QuadrupedMedium(_) => 1.6,
            Body::BirdMedium(_) => 6.0,
            Body::FishMedium(_) => 6.0,
            Body::Dragon(_) => 1.0,
            Body::BirdLarge(_) => 7.0,
            Body::FishSmall(_) => 7.0,
            Body::BipedLarge(_) => 1.6,
            Body::BipedSmall(_) => 2.4,
            Body::Object(_) => 2.0,
            Body::Golem(_) => 0.8,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 0.5,
                theropod::Species::Odonto => 0.5,
                theropod::Species::Ntouka => 0.5,
                _ => 1.4,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Monitor => 1.8,
                quadruped_low::Species::Asp => 1.6,
                quadruped_low::Species::Tortoise => 0.6,
                quadruped_low::Species::Rocksnapper => 0.8,
                quadruped_low::Species::Maneater => 1.0,
                quadruped_low::Species::Lavadrake => 0.8,
                _ => 1.2,
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
            Body::Humanoid(_) => Some(200.0 * self.mass().0),
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
            Body::Ship(ship::Body::DefaultAirship) => Some(300_000.0),
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
}

/// Handles updating `Components` to move player based on state of `JoinData`
pub fn handle_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    let submersion = data
        .physics
        .in_liquid()
        .map(|depth| depth / data.body.height());

    if input_is_pressed(data, InputKind::Fly)
        && submersion.map_or(true, |sub| sub < 1.0)
        && (!data.physics.on_ground || data.body.jump_impulse().is_none())
        && data.body.fly_thrust().is_some()
    {
        fly_move(data, update, efficiency);
    } else if let Some(submersion) = (!data.physics.on_ground && data.body.swim_thrust().is_some())
        .then_some(submersion)
        .flatten()
    {
        swim_move(data, update, efficiency, submersion);
    } else {
        basic_move(data, update, efficiency);
    }
}

/// Updates components to move player as if theyre on ground or in air
#[allow(clippy::assign_op_pattern)] // TODO: Pending review in #587
fn basic_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    handle_orientation(data, update, efficiency);

    if let Some(accel) = data
        .physics
        .on_ground
        .then_some(data.body.base_accel() * efficiency)
    {
        // Should ability to backpedal be separate from ability to strafe?
        update.vel.0 += Vec2::broadcast(data.dt.0)
            * accel
            * if data.body.can_strafe() {
                data.inputs.move_dir
            } else {
                let fw = Vec2::from(update.ori);
                fw * data.inputs.move_dir.dot(fw).max(0.0)
            };
    }
}

/// Handles forced movement
pub fn handle_forced_movement(
    data: &JoinData,
    update: &mut StateUpdate,
    movement: ForcedMovement,
    efficiency: f32,
) {
    handle_orientation(data, update, efficiency);

    match movement {
        ForcedMovement::Forward { strength } => {
            if let Some(accel) = data.physics.on_ground.then_some(data.body.base_accel()) {
                update.vel.0 += Vec2::broadcast(data.dt.0)
                    * accel
                    * (data.inputs.move_dir * efficiency + Vec2::from(update.ori) * strength);
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

pub fn handle_orientation(data: &JoinData, update: &mut StateUpdate, efficiency: f32) {
    let strafe_aim = update.character.is_aimed() && data.body.can_strafe();
    if let Some(dir) = (strafe_aim || update.character.is_attack())
        .then(|| data.inputs.look_dir.to_horizontal().unwrap_or_default())
        .or_else(|| Dir::from_unnormalized(data.inputs.move_dir.into()))
    {
        let rate = {
            let angle = update.ori.look_dir().angle_between(*dir);
            data.body.base_ori_rate() * efficiency * std::f32::consts::PI / angle
        };
        update.ori = update
            .ori
            .slerped_towards(dir.into(), (data.dt.0 * rate).min(0.1));
    };
}

/// Updates components to move player as if theyre swimming
fn swim_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32, submersion: f32) -> bool {
    if let Some(force) = data.body.swim_thrust() {
        handle_orientation(data, update, efficiency * 0.2);

        let force = efficiency * force;
        let mut water_accel = force / data.mass.0;

        if let Ok(Some(level)) = data.skill_set.skill_level(Skill::Swim(SwimSkill::Speed)) {
            water_accel *= 1.4_f32.powi(level.into());
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
pub fn fly_move(data: &JoinData, update: &mut StateUpdate, efficiency: f32) -> bool {
    if let Some(force) = data.body.fly_thrust() {
        let thrust = efficiency * force;

        let accel = thrust / data.mass.0;

        handle_orientation(data, update, efficiency);

        // Elevation control
        match data.body {
            // flappy flappy
            Body::Dragon(_) | Body::BirdMedium(_) | Body::BirdLarge(_) => {
                let anti_grav = GRAVITY * (1.0 + data.inputs.move_z.min(0.0));
                update.vel.0.z += data.dt.0 * (anti_grav + accel * data.inputs.move_z.max(0.0));
            },
            // floaty floaty
            Body::Ship(ship @ ship::Body::DefaultAirship) => {
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
pub fn handle_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.controller.queued_inputs.keys().any(|i| i.is_ability()) {
        attempt_wield(data, update);
    }
}

/// If a tool is equipped, goes into Equipping state, otherwise goes to Idle
pub fn attempt_wield(data: &JoinData, update: &mut StateUpdate) {
    if let Some((item, ItemKind::Tool(tool))) = data
        .inventory
        .equipped(EquipSlot::Mainhand)
        .map(|i| (i, i.kind()))
    {
        update.character = CharacterState::Equipping(equipping::Data {
            static_data: equipping::StaticData {
                buildup_duration: tool.equip_time(data.msm, item.components()),
            },
            timer: Duration::default(),
        });
    } else {
        update.character = CharacterState::Wielding;
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

pub fn attempt_talk(data: &JoinData, update: &mut StateUpdate) {
    if data.physics.on_ground {
        update.character = CharacterState::Talk;
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
            .in_liquid()
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        //&& update.vel.0.z < 0.0
        && data.body.can_climb()
        && update.energy.current() > 100
    {
        update.character = CharacterState::Climb(climb::Data::create_adjusted_by_skills(data));
    }
}

/// Checks that player can Swap Weapons and updates `Loadout` if so
pub fn attempt_swap_equipped_weapons(data: &JoinData, update: &mut StateUpdate) {
    if data.inventory.equipped(EquipSlot::Offhand).is_some() {
        update.swap_equipped_weapons = true;
    }
}

/// Handles inventory manipulations that affect the loadout
pub fn handle_manipulate_loadout(
    data: &JoinData,
    update: &mut StateUpdate,
    inv_action: InventoryAction,
) {
    update
        .server_events
        .push_front(ServerEvent::InventoryManip(data.entity, inv_action.into()));
}

/// Checks that player can wield the glider and updates `CharacterState` if so
pub fn attempt_glide_wield(data: &JoinData, update: &mut StateUpdate) {
    if data.inventory.equipped(EquipSlot::Glider).is_some()
        && !data
            .physics
            .in_liquid()
            .map(|depth| depth > 1.0)
            .unwrap_or(false)
        && data.body.is_humanoid()
    {
        update.character = CharacterState::GlideWield;
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(data: &JoinData, update: &mut StateUpdate, strength: f32) -> bool {
    (input_is_pressed(data, InputKind::Jump) && data.physics.on_ground)
        .then(|| data.body.jump_impulse())
        .flatten()
        .map(|impulse| {
            update.local_events.push_front(LocalEvent::Jump(
                data.entity,
                strength * impulse / data.mass.0,
            ));
        })
        .is_some()
}

fn handle_ability(data: &JoinData, update: &mut StateUpdate, input: InputKind) {
    let hands = |equip_slot| match data.inventory.equipped(equip_slot).map(|i| i.kind()) {
        Some(ItemKind::Tool(tool)) => Some(tool.hands),
        _ => None,
    };

    // Mouse1 and Skill1 always use the MainHand slot
    let always_main_hand = matches!(input, InputKind::Primary | InputKind::Ability(0));
    let no_main_hand = hands(EquipSlot::Mainhand).is_none();
    // skill_index used to select ability for the AbilityKey::Skill2 input
    let (equip_slot, skill_index) = if no_main_hand {
        (Some(EquipSlot::Offhand), 1)
    } else if always_main_hand {
        (Some(EquipSlot::Mainhand), 0)
    } else {
        let hands = (hands(EquipSlot::Mainhand), hands(EquipSlot::Offhand));
        match hands {
            (Some(Hands::Two), _) => (Some(EquipSlot::Mainhand), 1),
            (_, Some(Hands::One)) => (Some(EquipSlot::Offhand), 0),
            (Some(Hands::One), _) => (Some(EquipSlot::Mainhand), 1),
            (_, _) => (None, 0),
        }
    };

    let unlocked = |(s, a): (Option<Skill>, CharacterAbility)| {
        s.map_or(true, |s| data.skill_set.has_skill(s)).then_some(a)
    };

    if let Some(equip_slot) = equip_slot {
        if let Some(ability) = data
            .inventory
            .equipped(equip_slot)
            .map(|i| &i.item_config_expect().abilities)
            .and_then(|abilities| match input {
                InputKind::Primary => Some(abilities.primary.clone()),
                InputKind::Secondary => Some(abilities.secondary.clone()),
                InputKind::Ability(0) => abilities.abilities.get(0).cloned().and_then(unlocked),
                InputKind::Ability(_) => abilities
                    .abilities
                    .get(skill_index)
                    .cloned()
                    .and_then(unlocked),
                InputKind::Roll | InputKind::Jump | InputKind::Fly => None,
            })
            .map(|a| {
                let tool = unwrap_tool_data(data, equip_slot).map(|t| t.kind);
                a.adjusted_by_skills(&data.skill_set, tool)
            })
            .filter(|ability| ability.requirements_paid(data, update))
        {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::from_input(data, matches!(equip_slot, EquipSlot::Offhand), input),
            ));
        }
    }
}

pub fn handle_ability_input(data: &JoinData, update: &mut StateUpdate) {
    if let Some(input) = data
        .controller
        .queued_inputs
        .keys()
        .find(|i| i.is_ability())
    {
        handle_ability(data, update, *input);
    }
}

pub fn handle_input(data: &JoinData, update: &mut StateUpdate, input: InputKind) {
    match input {
        InputKind::Primary | InputKind::Secondary | InputKind::Ability(_) => {
            handle_ability(data, update, input)
        },
        InputKind::Roll => handle_dodge_input(data, update),
        InputKind::Jump => {
            handle_jump(data, update, 1.0);
        },
        InputKind::Fly => {},
    }
}

pub fn attempt_input(data: &JoinData, update: &mut StateUpdate) {
    // TODO: look into using first() when it becomes stable
    if let Some(input) = data.controller.queued_inputs.keys().next() {
        handle_input(data, update, *input);
    }
}

/// Checks that player can perform a dodge, then
/// attempts to perform their dodge ability
pub fn handle_dodge_input(data: &JoinData, update: &mut StateUpdate) {
    if input_is_pressed(data, InputKind::Roll) && data.body.is_humanoid() {
        let ability = CharacterAbility::default_roll().adjusted_by_skills(&data.skill_set, None);
        if ability.requirements_paid(data, update) {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::from_input(data, false, InputKind::Roll),
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

pub fn unwrap_tool_data<'a>(data: &'a JoinData, equip_slot: EquipSlot) -> Option<&'a Tool> {
    if let Some(ItemKind::Tool(tool)) = data.inventory.equipped(equip_slot).map(|i| i.kind()) {
        Some(&tool)
    } else {
        None
    }
}

pub fn get_crit_data(data: &JoinData, ai: AbilityInfo) -> (f32, f32) {
    const DEFAULT_CRIT_DATA: (f32, f32) = (0.5, 1.3);
    use HandInfo::*;
    let slot = match ai.hand {
        Some(TwoHanded) | Some(MainHand) => EquipSlot::Mainhand,
        Some(OffHand) => EquipSlot::Offhand,
        None => return DEFAULT_CRIT_DATA,
    };
    if let Some(item) = data.inventory.equipped(slot) {
        if let ItemKind::Tool(tool) = item.kind() {
            let crit_chance = tool.base_crit_chance(data.msm, item.components());
            let crit_mult = tool.base_crit_mult(data.msm, item.components());
            return (crit_chance, crit_mult);
        }
    }
    DEFAULT_CRIT_DATA
}

pub fn handle_state_interrupt(data: &JoinData, update: &mut StateUpdate, attacks_interrupt: bool) {
    if attacks_interrupt {
        handle_ability_input(data, update);
    }
    handle_dodge_input(data, update);
}

pub fn input_is_pressed(data: &JoinData, input: InputKind) -> bool {
    data.controller.queued_inputs.contains_key(&input)
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
    pub fn from_input(data: &JoinData, from_offhand: bool, input: InputKind) -> Self {
        let tool_data = if from_offhand {
            unwrap_tool_data(data, EquipSlot::Offhand)
        } else {
            unwrap_tool_data(data, EquipSlot::Mainhand)
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
