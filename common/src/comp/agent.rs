use crate::{
    comp::{humanoid, quadruped_low, quadruped_medium, quadruped_small, Body},
    path::Chaser,
    sync::Uid,
};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IdvStorage;
use vek::*;
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Alignment {
    /// Wild animals and gentle giants
    Wild,
    /// Dungeon cultists and bandits
    Enemy,
    /// Friendly folk in villages
    Npc,
    /// Farm animals and pets of villagers
    Tame,
    /// Pets you've tamed with a collar
    Owned(Uid),
    /// Passive objects like training dummies
    Passive,
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, Alignment::Wild) => false,
            (Alignment::Wild, Alignment::Enemy) => false,
            (Alignment::Wild, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Wild) => false,
            (Alignment::Npc, Alignment::Enemy) => true,
            (_, Alignment::Enemy) => true,
            (Alignment::Enemy, _) => true,
            _ => false,
        }
    }

    // Never attacks
    pub fn passive_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => true,
            (Alignment::Owned(a), Alignment::Owned(b)) if a == b => true,
            (Alignment::Npc, Alignment::Npc) => true,
            (Alignment::Npc, Alignment::Tame) => true,
            (Alignment::Enemy, Alignment::Wild) => true,
            (Alignment::Wild, Alignment::Enemy) => true,
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            (_, Alignment::Passive) => true,
            _ => false,
        }
    }

    // TODO: Remove this hack
    pub fn is_friendly_to_players(&self) -> bool {
        matches!(self, Alignment::Npc | Alignment::Tame | Alignment::Owned(_))
    }
}

impl Component for Alignment {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Psyche {
    pub aggro: f32, // 0.0 = always flees, 1.0 = always attacks, 0.5 = flee at 50% health
}
impl<'a> From<&'a Body> for Psyche {
    fn from(body: &'a Body) -> Self {
        Self {
            aggro: match body {
                Body::Humanoid(humanoid) => match humanoid.species {
                    humanoid::Species::Danari => 0.9,
                    humanoid::Species::Dwarf => 0.9,
                    humanoid::Species::Elf => 0.95,
                    humanoid::Species::Human => 0.95,
                    humanoid::Species::Orc => 1.0,
                    humanoid::Species::Undead => 1.0,
                },
                Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                    quadruped_small::Species::Pig => 0.5,
                    quadruped_small::Species::Fox => 0.4,
                    quadruped_small::Species::Sheep => 0.5,
                    quadruped_small::Species::Boar => 1.0,
                    quadruped_small::Species::Jackalope => 0.4,
                    quadruped_small::Species::Skunk => 0.8,
                    quadruped_small::Species::Cat => 0.2,
                    quadruped_small::Species::Batfox => 0.7,
                    quadruped_small::Species::Raccoon => 0.4,
                    quadruped_small::Species::Quokka => 0.7,
                    quadruped_small::Species::Dodarock => 0.9,
                    quadruped_small::Species::Holladon => 1.0,
                    quadruped_small::Species::Hyena => 0.4,
                    quadruped_small::Species::Rabbit => 0.1,
                    quadruped_small::Species::Truffler => 0.8,
                    quadruped_small::Species::Frog => 0.6,
                    _ => 1.0,
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Tuskram => 0.8,
                    quadruped_medium::Species::Frostfang => 0.9,
                    quadruped_medium::Species::Mouflon => 0.8,
                    quadruped_medium::Species::Catoblepas => 0.8,
                    _ => 1.0,
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    quadruped_low::Species::Crocodile => 1.0,
                    quadruped_low::Species::Alligator => 1.0,
                    quadruped_low::Species::Salamander => 0.8,
                    quadruped_low::Species::Monitor => 0.9,
                    quadruped_low::Species::Asp => 0.9,
                    quadruped_low::Species::Tortoise => 1.0,
                    quadruped_low::Species::Rocksnapper => 1.0,
                    quadruped_low::Species::Pangolin => 0.6,
                    quadruped_low::Species::Maneater => 1.0,
                },
                Body::BirdMedium(_) => 1.0,
                Body::BirdSmall(_) => 0.4,
                Body::FishMedium(_) => 0.15,
                Body::FishSmall(_) => 0.0,
                Body::BipedLarge(_) => 1.0,
                Body::Object(_) => 1.0,
                Body::Golem(_) => 1.0,
                Body::Theropod(theropod) => match theropod.species {
                    _ => 0.4,
                },
                Body::Dragon(_) => 1.0,
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
    /// Does the agent talk when e.g. hit by the player
    // TODO move speech patterns into a Behavior component
    pub can_speak: bool,
    pub psyche: Psyche,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn new(origin: Vec3<f32>, can_speak: bool, body: &Body) -> Self {
        let patrol_origin = Some(origin);
        Agent {
            patrol_origin,
            can_speak,
            psyche: Psyche::from(body),
            ..Default::default()
        }
    }
}

impl Component for Agent {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Debug)]
pub enum Activity {
    Idle(Vec2<f32>),
    Follow {
        target: EcsEntity,
        chaser: Chaser,
    },
    Attack {
        target: EcsEntity,
        chaser: Chaser,
        time: f64,
        been_close: bool,
        powerup: f32,
    },
}

impl Activity {
    pub fn is_follow(&self) -> bool { matches!(self, Activity::Follow { .. }) }

    pub fn is_attack(&self) -> bool { matches!(self, Activity::Attack { .. }) }
}

impl Default for Activity {
    fn default() -> Self { Activity::Idle(Vec2::zero()) }
}
