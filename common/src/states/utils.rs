use crate::{
    astar::Astar,
    comp::{
        ability::{AbilityInitEvent, AbilityMeta, Capability, SpecifiedAbility, Stance},
        arthropod, biped_large, biped_small, bird_medium,
        buff::{BuffCategory, BuffChange},
        character_state::OutputEvents,
        controller::InventoryManip,
        golem,
        inventory::slot::{ArmorSlot, EquipSlot, Slot},
        item::{
            armor::Friction,
            tool::{self, AbilityContext},
            Hands, ItemKind, ToolKind,
        },
        quadruped_low, quadruped_medium, quadruped_small, ship,
        skills::{Skill, SwimSkill, SKILL_MODIFIERS},
        theropod, Body, CharacterState, Density, InputAttr, InputKind, InventoryAction, Melee,
        StateUpdate,
    },
    consts::{FRIC_GROUND, GRAVITY, MAX_PICKUP_RANGE},
    event::{LocalEvent, ServerEvent},
    mounting::Volume,
    outcome::Outcome,
    states::{behavior::JoinData, utils::CharacterState::Idle, *},
    terrain::{Block, TerrainGrid, UnlockKind},
    util::Dir,
    vol::ReadVol,
};
use core::hash::BuildHasherDefault;
use fxhash::FxHasher64;
use serde::{Deserialize, Serialize};
use std::{
    f32::consts::PI,
    ops::{Add, Div, Mul},
    time::Duration,
};
use strum::Display;
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
                quadruped_small::Species::Truffler => 70.0,
                quadruped_small::Species::Fungome => 70.0,
                quadruped_small::Species::Goat => 80.0,
                quadruped_small::Species::Raccoon => 100.0,
                quadruped_small::Species::Frog => 150.0,
                quadruped_small::Species::Porcupine => 100.0,
                quadruped_small::Species::Beaver => 100.0,
                quadruped_small::Species::Rabbit => 110.0,
                quadruped_small::Species::Cat => 150.0,
                quadruped_small::Species::Quokka => 100.0,
                quadruped_small::Species::MossySnail => 20.0,
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
                quadruped_medium::Species::Akhlut => 90.0,
                quadruped_medium::Species::Bristleback => 135.0,
                quadruped_medium::Species::ClaySteed => 120.0,
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
                biped_large::Species::Gigasfrost => 45.0,
                _ => 80.0,
            },
            Body::BirdMedium(_) => 80.0,
            Body::FishMedium(_) => 80.0,
            Body::Dragon(_) => 250.0,
            Body::BirdLarge(_) => 110.0,
            Body::FishSmall(_) => 60.0,
            Body::BipedSmall(biped_small) => match biped_small.species {
                biped_small::Species::Haniwa => 65.0,
                biped_small::Species::Boreal => 100.0,
                _ => 80.0,
            },
            Body::Object(_) => 0.0,
            Body::ItemDrop(_) => 0.0,
            Body::Golem(body) => match body.species {
                golem::Species::ClayGolem => 120.0,
                _ => 60.0,
            },
            Body::Theropod(_) => 135.0,
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Crocodile => 130.0,
                quadruped_low::Species::SeaCrocodile => 120.0,
                quadruped_low::Species::Alligator => 110.0,
                quadruped_low::Species::Salamander => 85.0,
                quadruped_low::Species::Elbst => 85.0,
                quadruped_low::Species::Monitor => 160.0,
                quadruped_low::Species::Asp => 110.0,
                quadruped_low::Species::Tortoise => 60.0,
                quadruped_low::Species::Rocksnapper => 70.0,
                quadruped_low::Species::Rootsnapper => 70.0,
                quadruped_low::Species::Reefsnapper => 70.0,
                quadruped_low::Species::Pangolin => 90.0,
                quadruped_low::Species::Maneater => 80.0,
                quadruped_low::Species::Sandshark => 160.0,
                quadruped_low::Species::Hakulaq => 140.0,
                quadruped_low::Species::Dagon => 140.0,
                quadruped_low::Species::Lavadrake => 100.0,
                quadruped_low::Species::Icedrake => 100.0,
                quadruped_low::Species::Basilisk => 90.0,
                quadruped_low::Species::Deadwood => 140.0,
                quadruped_low::Species::Mossdrake => 100.0,
                quadruped_low::Species::Driggle => 120.0,
                quadruped_low::Species::HermitAlligator => 120.0,
            },
            Body::Ship(ship::Body::Carriage) => 40.0,
            Body::Ship(_) => 0.0,
            Body::Arthropod(arthropod) => match arthropod.species {
                arthropod::Species::Tarantula => 135.0,
                arthropod::Species::Blackwidow => 110.0,
                arthropod::Species::Antlion => 120.0,
                arthropod::Species::Hornbeetle => 80.0,
                arthropod::Species::Leafbeetle => 80.0,
                arthropod::Species::Stagbeetle => 80.0,
                arthropod::Species::Weevil => 110.0,
                arthropod::Species::Cavespider => 110.0,
                arthropod::Species::Moltencrawler => 70.0,
                arthropod::Species::Mosscrawler => 70.0,
                arthropod::Species::Sandcrawler => 70.0,
                arthropod::Species::Dagonite => 70.0,
                arthropod::Species::Emberfly => 75.0,
            },
            Body::Crustacean(_) => 80.0,
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
        let v = match self {
            Body::Ship(ship) => ship.get_speed(),
            _ => (-self.base_accel() / 30.0) / ((1.0 - FRIC_GROUND).powi(2) - 1.0),
        };
        debug_assert!(v >= 0.0, "Speed must be positive!");
        v
    }

    /// The turn rate in 180°/s (or (rotations per second)/2)
    pub fn base_ori_rate(&self) -> f32 {
        match self {
            Body::Humanoid(_) => 3.5,
            Body::QuadrupedSmall(_) => 3.0,
            Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                quadruped_medium::Species::Mammoth => 1.0,
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
            Body::ItemDrop(_) => 2.0,
            Body::Golem(_) => 2.0,
            Body::Theropod(theropod) => match theropod.species {
                theropod::Species::Archaeos => 2.3,
                theropod::Species::Odonto => 2.3,
                theropod::Species::Ntouka => 2.3,
                theropod::Species::Dodarock => 2.0,
                _ => 2.5,
            },
            Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                quadruped_low::Species::Asp => 2.2,
                quadruped_low::Species::Tortoise => 1.5,
                quadruped_low::Species::Rocksnapper => 1.8,
                quadruped_low::Species::Rootsnapper => 1.8,
                quadruped_low::Species::Lavadrake => 1.7,
                quadruped_low::Species::Icedrake => 1.7,
                quadruped_low::Species::Mossdrake => 1.7,
                _ => 2.0,
            },
            Body::Ship(ship::Body::Carriage) => 0.04,
            Body::Ship(ship) if ship.has_water_thrust() => 5.0 / self.dimensions().y,
            Body::Ship(_) => 6.0 / self.dimensions().y,
            Body::Arthropod(_) => 3.5,
            Body::Crustacean(_) => 3.5,
        }
    }

    /// Returns thrust force if the body type can swim, otherwise None
    pub fn swim_thrust(&self) -> Option<f32> {
        // Swim thrust is proportional to the frontal area of the creature, since we
        // assume that strength roughly scales according to square laws. Also,
        // it happens to make balancing against drag much simpler.
        let front_profile = self.dimensions().x * self.dimensions().z;
        Some(
            match self {
                Body::Object(_) => return None,
                Body::ItemDrop(_) => return None,
                Body::Ship(ship::Body::Submarine) => 1000.0 * self.mass().0,
                Body::Ship(ship) if ship.has_water_thrust() => 500.0 * self.mass().0,
                Body::Ship(_) => return None,
                Body::BipedLarge(_) => 120.0 * self.mass().0,
                Body::Golem(_) => 100.0 * self.mass().0,
                Body::BipedSmall(_) => 1000.0 * self.mass().0,
                Body::BirdMedium(_) => 400.0 * self.mass().0,
                Body::BirdLarge(_) => 400.0 * self.mass().0,
                Body::FishMedium(_) => 200.0 * self.mass().0,
                Body::FishSmall(_) => 300.0 * self.mass().0,
                Body::Dragon(_) => 50.0 * self.mass().0,
                // Humanoids are a bit different: we try to give them thrusts that result in similar
                // speeds for gameplay reasons
                Body::Humanoid(_) => 4_000_000.0 / self.mass().0,
                Body::Theropod(body) => match body.species {
                    theropod::Species::Sandraptor
                    | theropod::Species::Snowraptor
                    | theropod::Species::Sunlizard
                    | theropod::Species::Woodraptor
                    | theropod::Species::Dodarock
                    | theropod::Species::Axebeak
                    | theropod::Species::Yale => 500.0 * self.mass().0,
                    _ => 150.0 * self.mass().0,
                },
                Body::QuadrupedLow(_) => 1200.0 * self.mass().0,
                Body::QuadrupedMedium(body) => match body.species {
                    quadruped_medium::Species::Mammoth => 150.0 * self.mass().0,
                    _ => 1000.0 * self.mass().0,
                },
                Body::QuadrupedSmall(_) => 1500.0 * self.mass().0,
                Body::Arthropod(_) => 500.0 * self.mass().0,
                Body::Crustacean(_) => 400.0 * self.mass().0,
            } * front_profile,
        )
    }

    /// Returns thrust force if the body type can fly, otherwise None
    pub fn fly_thrust(&self) -> Option<f32> {
        match self {
            Body::BirdMedium(body) => match body.species {
                bird_medium::Species::Bat => Some(GRAVITY * self.mass().0 * 0.5),
                _ => Some(GRAVITY * self.mass().0 * 2.0),
            },
            Body::BirdLarge(_) => Some(GRAVITY * self.mass().0 * 0.5),
            Body::Dragon(_) => Some(200_000.0),
            Body::Ship(ship) if ship.can_fly() => Some(300_000.0),
            _ => None,
        }
    }

    /// Returns jump impulse if the body type can jump, otherwise None
    pub fn jump_impulse(&self) -> Option<f32> {
        match self {
            Body::Object(_) | Body::Ship(_) | Body::ItemDrop(_) => None,
            Body::BipedLarge(_) | Body::Dragon(_) => Some(0.6 * self.mass().0),
            Body::Golem(_) | Body::QuadrupedLow(_) => Some(0.4 * self.mass().0),
            Body::QuadrupedMedium(_) => Some(0.4 * self.mass().0),
            Body::Theropod(body) => match body.species {
                theropod::Species::Snowraptor
                | theropod::Species::Sandraptor
                | theropod::Species::Woodraptor => Some(0.4 * self.mass().0),
                _ => None,
            },
            Body::Arthropod(_) => Some(1.0 * self.mass().0),
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
    pub fn projectile_offsets(&self, ori: Vec3<f32>, scale: f32) -> Vec3<f32> {
        let body_offsets_z = match self {
            Body::Golem(_) => self.height() * 0.4,
            _ => self.eye_height(scale),
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

/// set footwear in idle data and potential state change to Skate
pub fn handle_skating(data: &JoinData, update: &mut StateUpdate) {
    if let Idle(idle::Data {
        is_sneaking,
        time_entered,
        mut footwear,
    }) = data.character
    {
        if footwear.is_none() {
            footwear = data.inventory.and_then(|inv| {
                inv.equipped(EquipSlot::Armor(ArmorSlot::Feet))
                    .map(|armor| match armor.kind().as_ref() {
                        ItemKind::Armor(a) => {
                            a.stats(data.msm, armor.stats_durability_multiplier())
                                .ground_contact
                        },
                        _ => Friction::Normal,
                    })
            });
            update.character = Idle(idle::Data {
                is_sneaking: *is_sneaking,
                time_entered: *time_entered,
                footwear,
            });
        }
        if data.physics.skating_active {
            update.character =
                CharacterState::Skate(skate::Data::new(data, footwear.unwrap_or(Friction::Normal)));
        }
    }
}

/// Handles updating `Components` to move player based on state of `JoinData`
pub fn handle_move(data: &JoinData<'_>, update: &mut StateUpdate, efficiency: f32) {
    if data.volume_mount_data.is_some() {
        return;
    }
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
    } else if let Some(submersion) = (data.physics.in_liquid().is_some()
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

    let accel = if let Some(block) = data.physics.on_ground {
        // FRIC_GROUND temporarily used to normalize things around expected values
        data.body.base_accel()
            * data.scale.map_or(1.0, |s| s.0.sqrt())
            * block.get_traction()
            * block.get_friction()
            / FRIC_GROUND
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
        ForcedMovement::Forward(strength) => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|block| {
                // FRIC_GROUND temporarily used to normalize things around expected values
                data.body.base_accel() * block.get_traction() * block.get_friction() / FRIC_GROUND
            }) {
                update.vel.0 += Vec2::broadcast(data.dt.0)
                    * accel
                    * data.scale.map_or(1.0, |s| s.0.sqrt())
                    * Vec2::from(*data.ori)
                    * strength;
            }
        },
        ForcedMovement::Reverse(strength) => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|block| {
                // FRIC_GROUND temporarily used to normalize things around expected values
                data.body.base_accel() * block.get_traction() * block.get_friction() / FRIC_GROUND
            }) {
                update.vel.0 += Vec2::broadcast(data.dt.0)
                    * accel
                    * data.scale.map_or(1.0, |s| s.0.sqrt())
                    * -Vec2::from(*data.ori)
                    * strength;
            }
        },
        ForcedMovement::Sideways(strength) => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|block| {
                // FRIC_GROUND temporarily used to normalize things around expected values
                data.body.base_accel() * block.get_traction() * block.get_friction() / FRIC_GROUND
            }) {
                let direction = {
                    // Left if positive, else right
                    let side = Vec2::from(*data.ori)
                        .rotated_z(PI / 2.)
                        .dot(data.inputs.move_dir)
                        .signum();
                    if side > 0.0 {
                        Vec2::from(*data.ori).rotated_z(PI / 2.)
                    } else {
                        -Vec2::from(*data.ori).rotated_z(PI / 2.)
                    }
                };

                update.vel.0 += Vec2::broadcast(data.dt.0)
                    * accel
                    * data.scale.map_or(1.0, |s| s.0.sqrt())
                    * direction
                    * strength;
            }
        },
        ForcedMovement::DirectedReverse(strength) => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|block| {
                // FRIC_GROUND temporarily used to normalize things around expected values
                data.body.base_accel() * block.get_traction() * block.get_friction() / FRIC_GROUND
            }) {
                let direction = if Vec2::from(*data.ori).dot(data.inputs.move_dir).signum() > 0.0 {
                    data.inputs.move_dir.reflected(Vec2::from(*data.ori))
                } else {
                    data.inputs.move_dir
                }
                .try_normalized()
                .unwrap_or_else(|| -Vec2::from(*data.ori));
                update.vel.0 += direction * strength * accel * data.dt.0;
            }
        },
        ForcedMovement::AntiDirectedForward(strength) => {
            let strength = strength * data.stats.move_speed_modifier * data.stats.friction_modifier;
            if let Some(accel) = data.physics.on_ground.map(|block| {
                // FRIC_GROUND temporarily used to normalize things around expected values
                data.body.base_accel() * block.get_traction() * block.get_friction() / FRIC_GROUND
            }) {
                let direction = if Vec2::from(*data.ori).dot(data.inputs.move_dir).signum() < 0.0 {
                    data.inputs.move_dir.reflected(Vec2::from(*data.ori))
                } else {
                    data.inputs.move_dir
                }
                .try_normalized()
                .unwrap_or_else(|| Vec2::from(*data.ori));
                let direction = direction.reflected(Vec2::from(*data.ori).rotated_z(PI / 2.));
                update.vel.0 += direction * strength * accel * data.dt.0;
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
                * data.scale.map_or(1.0, |s| s.0.sqrt())
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
                + move_input
                    * data.scale.map_or(1.0, |s| s.0.sqrt())
                    * data.inputs.move_dir.try_normalized().unwrap_or_default();
        },
    }
}

pub fn handle_orientation(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    efficiency: f32,
    dir_override: Option<Dir>,
) {
    /// first check for horizontal
    fn to_horizontal_fast(ori: &crate::comp::Ori) -> crate::comp::Ori {
        if ori.to_quat().into_vec4().xy().is_approx_zero() {
            *ori
        } else {
            ori.to_horizontal()
        }
    }
    /// compute an upper limit for the difference of two orientations
    fn ori_absdiff(a: &crate::comp::Ori, b: &crate::comp::Ori) -> f32 {
        (a.to_quat().into_vec4() - b.to_quat().into_vec4()).reduce(|a, b| a.abs() + b.abs())
    }

    let (tilt_ori, efficiency) = if let Body::Ship(ship) = data.body
        && ship.has_wheels()
    {
        let height_at = |rpos| {
            data.terrain
                .ray(
                    data.pos.0 + rpos + Vec3::unit_z() * 4.0,
                    data.pos.0 + rpos - Vec3::unit_z() * 4.0,
                )
                .until(Block::is_solid)
                .cast()
                .0
        };

        // Do some cheap raycasting with the ground to determine the appropriate
        // orientation for the vehicle
        let x_diff = (height_at(data.ori.to_horizontal().right().to_vec() * 3.0)
            - height_at(data.ori.to_horizontal().right().to_vec() * -3.0))
            / 10.0;
        let y_diff = (height_at(data.ori.to_horizontal().look_dir().to_vec() * -4.5)
            - height_at(data.ori.to_horizontal().look_dir().to_vec() * 4.5))
            / 10.0;

        (
            Quaternion::rotation_y(x_diff.atan()) * Quaternion::rotation_x(y_diff.atan()),
            (data.vel.0 - data.physics.ground_vel)
                .xy()
                .magnitude()
                .max(3.0)
                * efficiency,
        )
    } else {
        (Quaternion::identity(), efficiency)
    };

    // Direction is set to the override if one is provided, else if entity is
    // strafing or attacking the horiontal component of the look direction is used,
    // else the current horizontal movement direction is used
    let target_ori = if let Some(dir_override) = dir_override {
        dir_override.into()
    } else if is_strafing(data, update) || update.character.is_attack() {
        data.inputs
            .look_dir
            .to_horizontal()
            .unwrap_or_default()
            .into()
    } else {
        Dir::from_unnormalized(data.inputs.move_dir.into())
            .map_or_else(|| to_horizontal_fast(data.ori), |dir| dir.into())
    }
    .rotated(tilt_ori);
    // unit is multiples of 180°
    let half_turns_per_tick = data.body.base_ori_rate() / data.scale.map_or(1.0, |s| s.0.sqrt())
        * efficiency
        * if data.physics.on_ground.is_some() {
            1.0
        } else if data.physics.in_liquid().is_some() {
            0.4
        } else {
            0.2
        }
        * data.dt.0;
    // very rough guess
    let ticks_from_target_guess = ori_absdiff(&update.ori, &target_ori) / half_turns_per_tick;
    let instantaneous = ticks_from_target_guess < 1.0;
    update.ori = if data.volume_mount_data.is_some() {
        update.ori
    } else if instantaneous {
        target_ori
    } else {
        let target_fraction = {
            // Angle factor used to keep turning rate approximately constant by
            // counteracting slerp turning more with a larger angle
            let angle_factor = 2.0 / (1.0 - update.ori.dot(target_ori)).sqrt();

            half_turns_per_tick * angle_factor
        };
        update
            .ori
            .slerped_towards(target_ori, target_fraction.min(1.0))
    };

    // Look at things
    update.character_activity.look_dir = Some(data.controller.inputs.look_dir);
}

/// Updates components to move player as if theyre swimming
fn swim_move(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    efficiency: f32,
    submersion: f32,
) -> bool {
    let efficiency = efficiency * data.stats.swim_speed_modifier * data.stats.friction_modifier;
    if let Some(force) = data.body.swim_thrust() {
        let force = efficiency * force * data.scale.map_or(1.0, |s| s.0);
        let mut water_accel = force / data.mass.0;

        if let Ok(level) = data.skill_set.skill_level(Skill::Swim(SwimSkill::Speed)) {
            let modifiers = SKILL_MODIFIERS.general_tree.swim;
            water_accel *= modifiers.speed.powi(level.into());
        }

        let dir = if data.body.can_strafe() {
            data.inputs.move_dir
        } else {
            let fw = Vec2::from(update.ori);
            fw * data.inputs.move_dir.dot(fw).max(0.0)
        };

        // Automatically tread water to stay afloat
        let move_z = if submersion < 1.0
            && data.inputs.move_z.abs() < f32::EPSILON
            && data.physics.on_ground.is_none()
        {
            submersion.max(0.0) * 0.1
        } else {
            data.inputs.move_z
        };

        // Assume that feet/flippers get less efficient as we become less submerged
        let move_z = move_z.min((submersion * 1.5 - 0.5).clamp(0.0, 1.0).powi(2));

        update.vel.0 += Vec3::new(dir.x, dir.y, move_z)
                // TODO: Should probably be normalised, but creates odd discrepancies when treading water
                // .try_normalized()
                // .unwrap_or_default()
            * water_accel
            // Gives a good balance between submerged and surface speed
            * submersion.clamp(0.0, 1.0).sqrt()
            // Good approximate compensation for dt-dependent effects
            * data.dt.0 * 0.04;

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
                    let change = if data.inputs.move_z.abs() > f32::EPSILON {
                        -data.inputs.move_z
                    } else {
                        (def - data.density.0).clamp(-1.0, 1.0)
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
            .and_then(|item| match &*item.kind() {
                ItemKind::Tool(tool) => Some(Duration::from_secs_f32(
                    tool.stats(item.stats_durability_multiplier())
                        .equip_time_secs,
                )),
                _ => None,
            })
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
    // instantly into wield
    if let Some(equip_time) = equip_time {
        update.character = CharacterState::Equipping(equipping::Data {
            static_data: equipping::StaticData {
                buildup_duration: equip_time,
            },
            timer: Duration::default(),
            is_sneaking: update.character.is_stealthy(),
        });
    } else {
        update.character = CharacterState::Wielding(wielding::Data {
            is_sneaking: update.character.is_stealthy(),
        });
    }
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
        update.character = Idle(idle::Data {
            is_sneaking: true,
            time_entered: *data.time,
            footwear: data.character.footwear(),
        });
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
        // *All* entities can climb when in liquids, to let them climb out near the surface
        && (data.body.can_climb() || data.physics.in_liquid().is_some())
        && update.energy.current() > 1.0
    {
        update.character = CharacterState::Climb(climb::Data::create_adjusted_by_skills(data));
        true
    } else {
        false
    }
}

pub fn handle_wallrun(data: &JoinData<'_>, update: &mut StateUpdate) -> bool {
    if data.physics.on_wall.is_some()
        && data.physics.on_ground.is_none()
        && data.physics.in_liquid().is_none()
        && data.body.can_climb()
    {
        update.character = CharacterState::Wallrun(wallrun::Data {
            was_wielded: data.character.is_wield() || data.character.was_wielded(),
        });
        true
    } else {
        false
    }
}
/// Checks that player can Swap Weapons and updates `Loadout` if so
pub fn attempt_swap_equipped_weapons(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    output_events: &mut OutputEvents,
) {
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
        loadout_change_hook(data, output_events, false);
    }
}

/// Checks if a block can be reached from a position.
fn can_reach_block(
    player_pos: Vec3<f32>,
    block_pos: Vec3<i32>,
    range: f32,
    body: &Body,
    terrain: &TerrainGrid,
) -> bool {
    let block_pos_f32 = block_pos.map(|x| x as f32 + 0.5);
    // Closure to check if distance between a point and the block is less than
    // MAX_PICKUP_RANGE and the radius of the body
    let block_range_check = |pos: Vec3<f32>| {
        (block_pos_f32 - pos).magnitude_squared() < (range + body.max_radius()).powi(2)
    };

    // Checks if player's feet or head is near to block
    let close_to_block = block_range_check(player_pos)
        || block_range_check(player_pos + Vec3::new(0.0, 0.0, body.height()));
    if close_to_block {
        // Do a check that a path can be found between sprite and entity
        // interacting with sprite Use manhattan distance * 1.5 for number
        // of iterations
        let iters = (3.0 * (block_pos_f32 - player_pos).map(|x| x.abs()).sum()) as usize;
        // Heuristic compares manhattan distance of start and end pos
        let heuristic =
            move |pos: &Vec3<i32>, _: &Vec3<i32>| (block_pos - pos).map(|x| x.abs()).sum() as f32;

        let mut astar = Astar::new(
            iters,
            player_pos.map(|x| x.floor() as i32),
            BuildHasherDefault::<FxHasher64>::default(),
        );

        // Transition uses manhattan distance as the cost, with a slightly lower cost
        // for z transitions
        let transition = |a: Vec3<i32>, b: Vec3<i32>| {
            let (a, b) = (a.map(|x| x as f32), b.map(|x| x as f32));
            ((a - b) * Vec3::new(1.0, 1.0, 0.9)).map(|e| e.abs()).sum()
        };
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
            DIRS.iter()
                .map(move |dir| {
                    let dest = dir + pos;
                    (dest, transition(pos, dest))
                })
                .filter(|(pos, _)| {
                    terrain
                        .get(*pos)
                        .ok()
                        .map_or(false, |block| !block.is_filled())
                })
        };
        // Pathing satisfied when it reaches the sprite position
        let satisfied = |pos: &Vec3<i32>| *pos == block_pos;

        astar
            .poll(iters, heuristic, neighbors, satisfied)
            .into_path()
            .is_some()
    } else {
        false
    }
}

/// Handles inventory manipulations that affect the loadout
pub fn handle_manipulate_loadout(
    data: &JoinData<'_>,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
    inv_action: InventoryAction,
) {
    loadout_change_hook(data, output_events, true);
    match inv_action {
        InventoryAction::Use(slot @ Slot::Inventory(inv_slot)) => {
            // If inventory action is using a slot, and slot is in the inventory
            // TODO: Do some non lazy way of handling the possibility that items equipped in
            // the loadout will have effects that are desired to be non-instantaneous
            use use_item::ItemUseKind;
            if let Some((item_kind, item)) = data
                .inventory
                .and_then(|inv| inv.get(inv_slot))
                .and_then(|item| Option::<ItemUseKind>::from(&*item.kind()).zip(Some(item)))
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
                        item_hash: item.item_hash(),
                        was_wielded: data.character.is_wield(),
                        was_sneak: data.character.is_stealthy(),
                    },
                    timer: Duration::default(),
                    stage_section: StageSection::Buildup,
                });
            } else {
                // Else emit inventory action instantaneously
                let inv_manip = InventoryManip::Use(slot);
                output_events.emit_server(ServerEvent::InventoryManip(data.entity, inv_manip));
            }
        },
        InventoryAction::Collect(sprite_pos) => {
            // First, get sprite data for position, if there is a sprite
            let sprite_at_pos = data
                .terrain
                .get(sprite_pos)
                .ok()
                .copied()
                .and_then(|b| b.get_sprite());
            // Checks if position has a collectible sprite as well as what sprite is at the
            // position
            let sprite_interact =
                sprite_at_pos.and_then(Option::<sprite_interact::SpriteInteractKind>::from);
            if let Some(sprite_interact) = sprite_interact {
                if can_reach_block(
                    data.pos.0,
                    sprite_pos,
                    MAX_PICKUP_RANGE,
                    data.body,
                    data.terrain,
                ) {
                    let sprite_chunk_pos = TerrainGrid::chunk_offs(sprite_pos);
                    let sprite_cfg = data
                        .terrain
                        .pos_chunk(sprite_pos)
                        .and_then(|chunk| chunk.meta().sprite_cfg_at(sprite_chunk_pos));
                    let required_item =
                        sprite_at_pos.and_then(|s| match s.unlock_condition(sprite_cfg.cloned()) {
                            UnlockKind::Free => None,
                            UnlockKind::Requires(item) => Some((item, false)),
                            UnlockKind::Consumes(item) => Some((item, true)),
                        });

                    // None: An required items exist but no available
                    // Some(None): No required items
                    // Some(Some(_)): Required items satisfied, contains info about them
                    let has_required_items = match required_item {
                        // Produces `None` if we can't find the item or `Some(Some(_))` if we can
                        Some((item_id, consume)) => data
                            .inventory
                            .and_then(|inv| inv.get_slot_of_item_by_def_id(&item_id))
                            .map(|slot| Some((item_id, slot, consume))),
                        None => Some(None),
                    };
                    if let Some(required_item) = has_required_items {
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
                                was_wielded: data.character.is_wield(),
                                was_sneak: data.character.is_stealthy(),
                                required_item,
                            },
                            timer: Duration::default(),
                            stage_section: StageSection::Buildup,
                        })
                    } else {
                        output_events.emit_local(LocalEvent::CreateOutcome(
                            Outcome::FailedSpriteUnlock { pos: sprite_pos },
                        ));
                    }
                }
            }
        },
        // For inventory actions without a dedicated character state, just do action instantaneously
        InventoryAction::Swap(equip, slot) => {
            let inv_manip = InventoryManip::Swap(Slot::Equip(equip), slot);
            output_events.emit_server(ServerEvent::InventoryManip(data.entity, inv_manip));
        },
        InventoryAction::Drop(equip) => {
            let inv_manip = InventoryManip::Drop(Slot::Equip(equip));
            output_events.emit_server(ServerEvent::InventoryManip(data.entity, inv_manip));
        },
        InventoryAction::Sort => {
            output_events.emit_server(ServerEvent::InventoryManip(
                data.entity,
                InventoryManip::Sort,
            ));
        },
        InventoryAction::Use(slot @ Slot::Equip(_)) => {
            let inv_manip = InventoryManip::Use(slot);
            output_events.emit_server(ServerEvent::InventoryManip(data.entity, inv_manip));
        },
        InventoryAction::Use(Slot::Overflow(_)) => {
            // Items in overflow slots cannot be used until moved to a real slot
        },
        InventoryAction::ToggleSpriteLight(pos, enable) => {
            if matches!(pos.kind, Volume::Terrain) {
                let sprite_interact = sprite_interact::SpriteInteractKind::ToggleLight(enable);

                let (buildup_duration, use_duration, recover_duration) =
                    sprite_interact.durations();

                update.character = CharacterState::SpriteInteract(sprite_interact::Data {
                    static_data: sprite_interact::StaticData {
                        buildup_duration,
                        use_duration,
                        recover_duration,
                        sprite_pos: pos.pos,
                        sprite_kind: sprite_interact,
                        was_wielded: data.character.is_wield(),
                        was_sneak: data.character.is_stealthy(),
                        required_item: None,
                    },
                    timer: Duration::default(),
                    stage_section: StageSection::Buildup,
                });
            }
        },
    }
}

/// Checks that player can wield the glider and updates `CharacterState` if so
pub fn attempt_glide_wield(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    output_events: &mut OutputEvents,
) {
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
        output_events.emit_local(LocalEvent::CreateOutcome(Outcome::Glider {
            pos: data.pos.0,
            wielded: true,
        }));
        update.character = CharacterState::GlideWield(glide_wield::Data::from(data));
    }
}

/// Checks that player can jump and sends jump event if so
pub fn handle_jump(
    data: &JoinData<'_>,
    output_events: &mut OutputEvents,
    _update: &mut StateUpdate,
    strength: f32,
) -> bool {
    input_is_pressed(data, InputKind::Jump)
        .then(|| data.body.jump_impulse())
        .flatten()
        .and_then(|impulse| {
            if data.physics.in_liquid().is_some() {
                if data.physics.on_wall.is_some() {
                    // Allow entities to make a small jump when at the edge of a body of water,
                    // allowing them to path out of it
                    Some(impulse * 0.75)
                } else {
                    None
                }
            } else if data.physics.on_ground.is_some() {
                Some(impulse)
            } else {
                None
            }
        })
        .map(|impulse| {
            output_events.emit_local(LocalEvent::Jump(
                data.entity,
                strength * impulse / data.mass.0
                    * data.scale.map_or(1.0, |s| s.0.powf(13.0).powf(0.25))
                    * data.stats.jump_modifier,
            ));
        })
        .is_some()
}

fn handle_ability(
    data: &JoinData<'_>,
    update: &mut StateUpdate,
    output_events: &mut OutputEvents,
    input: InputKind,
) -> bool {
    let context = AbilityContext::from(data.stance, data.inventory, data.combo);
    if let Some(ability_input) = input.into() {
        if let Some((ability, from_offhand, spec_ability)) = data
            .active_abilities
            .and_then(|a| {
                a.activate_ability(
                    ability_input,
                    data.inventory,
                    data.skill_set,
                    Some(data.body),
                    Some(data.character),
                    &context,
                )
            })
            .filter(|(ability, _, _)| ability.requirements_paid(data, update))
        {
            update.character = CharacterState::from((
                &ability,
                AbilityInfo::new(
                    data,
                    from_offhand,
                    input,
                    Some(spec_ability),
                    ability.ability_meta(),
                ),
                data,
            ));
            if let Some(init_event) = ability.ability_meta().init_event {
                match init_event {
                    AbilityInitEvent::EnterStance(stance) => {
                        output_events.emit_server(ServerEvent::ChangeStance {
                            entity: data.entity,
                            stance,
                        });
                    },
                }
            }
            if let CharacterState::Roll(roll) = &mut update.character {
                if data.character.is_wield() || data.character.was_wielded() {
                    roll.was_wielded = true;
                }
                if data.character.is_stealthy() {
                    roll.is_sneaking = true;
                }
                if data.character.is_aimed() {
                    roll.prev_aimed_dir = Some(data.controller.inputs.look_dir);
                }
            }
            return true;
        }
    }
    false
}

pub fn handle_input(
    data: &JoinData<'_>,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
    input: InputKind,
) {
    match input {
        InputKind::Primary
        | InputKind::Secondary
        | InputKind::Ability(_)
        | InputKind::Block
        | InputKind::Roll => {
            handle_ability(data, update, output_events, input);
        },
        InputKind::Jump => {
            handle_jump(data, output_events, update, 1.0);
        },
        InputKind::Fly => {},
    }
}

pub fn attempt_input(
    data: &JoinData<'_>,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
) {
    // TODO: look into using first() when it becomes stable
    if let Some(input) = data.controller.queued_inputs.keys().next() {
        handle_input(data, output_events, update, *input);
    }
}

/// Returns whether an interrupt occurred
pub fn handle_interrupts(
    data: &JoinData,
    update: &mut StateUpdate,
    output_events: &mut OutputEvents,
) -> bool {
    let can_dodge = matches!(
        data.character.stage_section(),
        Some(StageSection::Buildup | StageSection::Recover)
    );
    let can_block = data
        .character
        .ability_info()
        .map(|info| info.ability_meta)
        .map_or(false, |meta| {
            meta.capabilities.contains(Capability::BLOCK_INTERRUPT)
        });
    if can_dodge && input_is_pressed(data, InputKind::Roll) {
        handle_ability(data, update, output_events, InputKind::Roll)
    } else if can_block && input_is_pressed(data, InputKind::Block) {
        handle_ability(data, update, output_events, InputKind::Block)
    } else {
        false
    }
}

pub fn is_strafing(data: &JoinData<'_>, update: &StateUpdate) -> bool {
    // TODO: Don't always check `character.is_aimed()`, allow the frontend to
    // control whether the player strafes during an aimed `CharacterState`.
    (update.character.is_aimed() || update.should_strafe) && data.body.can_strafe()
    // no strafe with music instruments equipped in ActiveMainhand
    && !matches!(unwrap_tool_data(data, EquipSlot::ActiveMainhand),
        Some((ToolKind::Instrument, _)))
}

/// Returns tool and components
pub fn unwrap_tool_data(data: &JoinData, equip_slot: EquipSlot) -> Option<(ToolKind, Hands)> {
    if let Some(ItemKind::Tool(tool)) = data
        .inventory
        .and_then(|inv| inv.equipped(equip_slot))
        .map(|i| i.kind())
        .as_deref()
    {
        Some((tool.kind, tool.hands))
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
            .as_deref()
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

pub fn get_tool_stats(data: &JoinData<'_>, ai: AbilityInfo) -> tool::Stats {
    ai.hand
        .map(|hand| match hand {
            HandInfo::TwoHanded | HandInfo::MainHand => EquipSlot::ActiveMainhand,
            HandInfo::OffHand => EquipSlot::ActiveOffhand,
        })
        .and_then(|slot| data.inventory.and_then(|inv| inv.equipped(slot)))
        .and_then(|item| {
            if let ItemKind::Tool(tool) = &*item.kind() {
                Some(tool.stats(item.stats_durability_multiplier()))
            } else {
                None
            }
        })
        .unwrap_or(tool::Stats::one())
}

pub fn input_is_pressed(data: &JoinData<'_>, input: InputKind) -> bool {
    data.controller.queued_inputs.contains_key(&input)
}

/// Checked `Duration` addition. Computes `timer` + `dt`, applying relevant stat
/// attack modifiers and `otcompute_precision_multeturning None if overflow
/// occurred.
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
    Forward(f32),
    Reverse(f32),
    Sideways(f32),
    DirectedReverse(f32),
    AntiDirectedForward(f32),
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

impl Mul<f32> for ForcedMovement {
    type Output = Self;

    fn mul(self, scalar: f32) -> Self {
        use ForcedMovement::*;
        match self {
            Forward(x) => Forward(x * scalar),
            Reverse(x) => Reverse(x * scalar),
            Sideways(x) => Sideways(x * scalar),
            DirectedReverse(x) => DirectedReverse(x * scalar),
            AntiDirectedForward(x) => AntiDirectedForward(x * scalar),
            Leap {
                vertical,
                forward,
                progress,
                direction,
            } => Leap {
                vertical: vertical * scalar,
                forward: forward * scalar,
                progress,
                direction,
            },
            Hover { move_input } => Hover { move_input },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub ability_meta: AbilityMeta,
    pub ability: Option<SpecifiedAbility>,
}

impl AbilityInfo {
    pub fn new(
        data: &JoinData<'_>,
        from_offhand: bool,
        input: InputKind,
        ability: Option<SpecifiedAbility>,
        ability_meta: AbilityMeta,
    ) -> Self {
        let tool_data = if from_offhand {
            unwrap_tool_data(data, EquipSlot::ActiveOffhand)
        } else {
            unwrap_tool_data(data, EquipSlot::ActiveMainhand)
        };
        let (tool, hand) = tool_data.map_or((None, None), |(kind, hands)| {
            (
                Some(kind),
                Some(HandInfo::from_main_tool(hands, from_offhand)),
            )
        });

        Self {
            tool,
            hand,
            input,
            input_attr: data.controller.queued_inputs.get(&input).copied(),
            ability_meta,
            ability,
        }
    }
}

pub fn end_ability(data: &JoinData<'_>, update: &mut StateUpdate) {
    if data.character.is_wield() || data.character.was_wielded() {
        update.character = CharacterState::Wielding(wielding::Data {
            is_sneaking: data.character.is_stealthy(),
        });
    } else {
        update.character = CharacterState::Idle(idle::Data {
            is_sneaking: data.character.is_stealthy(),
            footwear: None,
            time_entered: *data.time,
        });
    }
    if let CharacterState::Roll(roll) = data.character {
        if let Some(dir) = roll.prev_aimed_dir {
            update.ori = dir.into();
        }
    }
}

pub fn end_melee_ability(data: &JoinData<'_>, update: &mut StateUpdate) {
    end_ability(data, update);
    data.updater.remove::<Melee>(data.entity);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandInfo {
    TwoHanded,
    MainHand,
    OffHand,
}

impl HandInfo {
    pub fn from_main_tool(tool_hands: Hands, from_offhand: bool) -> Self {
        match tool_hands {
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

pub fn leave_stance(data: &JoinData<'_>, output_events: &mut OutputEvents) {
    if !matches!(data.stance, Some(Stance::None)) {
        output_events.emit_server(ServerEvent::ChangeStance {
            entity: data.entity,
            stance: Stance::None,
        });
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingKind {
    // Reaches a scaling of 1 when at minimum combo, and a scaling of 2 when at double minimum
    // combo
    Linear,
    // Reaches a scaling of 1 when at minimum combo, and a scaling of 2 when at 4x minimum combo
    Sqrt,
}

impl ScalingKind {
    pub fn factor(&self, val: f32, norm: f32) -> f32 {
        match self {
            Self::Linear => val / norm,
            Self::Sqrt => (val / norm).sqrt(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ComboConsumption {
    #[default]
    All,
    Half,
}

impl ComboConsumption {
    pub fn consume(&self, data: &JoinData, output_events: &mut OutputEvents) {
        let combo = data.combo.map_or(0, |c| c.counter());
        let to_consume = match self {
            Self::All => combo,
            Self::Half => (combo + 1) / 2,
        };
        output_events.emit_server(ServerEvent::ComboChange {
            entity: data.entity,
            change: -(to_consume as i32),
        });
    }
}

fn loadout_change_hook(data: &JoinData<'_>, output_events: &mut OutputEvents, clear_combo: bool) {
    if clear_combo {
        // Reset combo to 0
        output_events.emit_server(ServerEvent::ComboChange {
            entity: data.entity,
            change: -data.combo.map_or(0, |c| c.counter() as i32),
        });
    }
    // Clear any buffs from equipped weapons
    output_events.emit_server(ServerEvent::Buff {
        entity: data.entity,
        buff_change: BuffChange::RemoveByCategory {
            all_required: vec![BuffCategory::RemoveOnLoadoutChange],
            any_required: vec![],
            none_required: vec![],
        },
    });
}
