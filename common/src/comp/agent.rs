use crate::{
    comp::{humanoid, quadruped_low, quadruped_medium, quadruped_small, Body},
    path::Chaser,
    rtsim::RtSimController,
    uid::Uid,
};
use specs::{Component, Entity as EcsEntity};
use specs_idvs::IdvStorage;
use std::collections::VecDeque;
use vek::*;

pub const DEFAULT_INTERACTION_TIME: f32 = 3.0;

#[derive(Eq, PartialEq)]
pub enum Tactic {
    Melee,
    Axe,
    Hammer,
    Sword,
    Bow,
    Staff,
    StoneGolemBoss,
    CircleCharge { radius: u32, circle_time: u32 },
    QuadLowRanged,
    TailSlap,
    QuadLowQuick,
    QuadLowBasic,
    QuadLowBeam,
    QuadMedJump,
    QuadMedBasic,
    Lavadrake,
    Theropod,
    Turret,
    FixedTurret,
    RotatingTurret,
}

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
                    humanoid::Species::Dwarf => 0.8,
                    humanoid::Species::Elf => 0.7,
                    humanoid::Species::Human => 0.6,
                    humanoid::Species::Orc => 0.9,
                    humanoid::Species::Undead => 0.9,
                },
                Body::QuadrupedSmall(quadruped_small) => match quadruped_small.species {
                    quadruped_small::Species::Pig => 0.5,
                    quadruped_small::Species::Fox => 0.3,
                    quadruped_small::Species::Sheep => 0.5,
                    quadruped_small::Species::Boar => 0.8,
                    quadruped_small::Species::Jackalope => 0.4,
                    quadruped_small::Species::Skunk => 0.6,
                    quadruped_small::Species::Cat => 0.2,
                    quadruped_small::Species::Batfox => 0.6,
                    quadruped_small::Species::Raccoon => 0.4,
                    quadruped_small::Species::Quokka => 0.4,
                    quadruped_small::Species::Dodarock => 0.9,
                    quadruped_small::Species::Holladon => 1.0,
                    quadruped_small::Species::Hyena => 0.4,
                    quadruped_small::Species::Rabbit => 0.1,
                    quadruped_small::Species::Truffler => 0.8,
                    quadruped_small::Species::Frog => 0.4,
                    quadruped_small::Species::Hare => 0.2,
                    _ => 0.0,
                },
                Body::QuadrupedMedium(quadruped_medium) => match quadruped_medium.species {
                    quadruped_medium::Species::Tuskram => 0.7,
                    quadruped_medium::Species::Frostfang => 0.9,
                    quadruped_medium::Species::Mouflon => 0.7,
                    quadruped_medium::Species::Catoblepas => 0.8,
                    quadruped_medium::Species::Deer => 0.6,
                    quadruped_medium::Species::Hirdrasil => 0.7,
                    quadruped_medium::Species::Donkey => 0.7,
                    quadruped_medium::Species::Camel => 0.7,
                    quadruped_medium::Species::Zebra => 0.7,
                    quadruped_medium::Species::Antelope => 0.6,
                    quadruped_medium::Species::Horse => 0.7,
                    quadruped_medium::Species::Cattle => 0.7,
                    quadruped_medium::Species::Darkhound => 0.9,
                    _ => 0.5,
                },
                Body::QuadrupedLow(quadruped_low) => match quadruped_low.species {
                    quadruped_low::Species::Salamander => 0.7,
                    quadruped_low::Species::Monitor => 0.7,
                    quadruped_low::Species::Asp => 0.9,
                    quadruped_low::Species::Pangolin => 0.4,
                    _ => 0.6,
                },
                Body::BipedSmall(_) => 0.5,
                Body::BirdMedium(_) => 0.5,
                Body::BirdSmall(_) => 0.4,
                Body::FishMedium(_) => 0.15,
                Body::FishSmall(_) => 0.0,
                Body::BipedLarge(_) => 1.0,
                Body::Object(_) => 1.0,
                Body::Golem(_) => 1.0,
                Body::Theropod(_) => 1.0,
                Body::Dragon(_) => 1.0,
            },
        }
    }
}

#[derive(Clone, Debug)]
/// Events that affect agent behavior from other entities/players/environment
pub enum AgentEvent {
    /// Engage in conversation with entity with Uid
    Talk(Uid),
    Trade(Uid),
    // Add others here
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub rtsim_controller: RtSimController,
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
    pub target: Option<(EcsEntity, bool)>,
    /// Does the agent talk when e.g. hit by the player
    // TODO move speech patterns into a Behavior component
    pub can_speak: bool,
    pub psyche: Psyche,
    pub inbox: VecDeque<AgentEvent>,
    pub interaction_timer: f32,
    pub action_timer: f32,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn new(
        patrol_origin: Option<Vec3<f32>>,
        can_speak: bool,
        body: &Body,
        no_flee: bool,
    ) -> Self {
        Agent {
            patrol_origin,
            can_speak,
            psyche: if no_flee {
                Psyche { aggro: 1.0 }
            } else {
                Psyche::from(body)
            },
            ..Default::default()
        }
    }
}

impl Component for Agent {
    type Storage = IdvStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Activity {
    pub idle: bool,
    pub interact: bool,
    pub flee: bool,
    pub follow: bool,
    pub attack: bool,
    pub choose_target: bool,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            idle: true,
            interact: false,
            flee: false,
            follow: false,
            attack: false,
            choose_target: false,
        }
    }
}

impl Activity {
    pub fn reset(&mut self) {
        self.idle = false;
        self.interact = false;
        self.flee = false;
        self.follow = false;
        self.attack = false;
        self.choose_target = false;
    }

    pub fn idle(&mut self) {
        self.idle = true;
        self.interact = false;
        self.flee = false;
        self.follow = false;
        self.attack = false;
        self.choose_target = false;
    }

    pub fn interact(&mut self) {
        self.idle = false;
        self.interact = true;
        self.flee = false;
        self.follow = false;
        self.attack = false;
        self.choose_target = false;
    }

    pub fn flee(&mut self) {
        self.idle = false;
        self.interact = false;
        self.flee = true;
        self.follow = false;
        self.attack = false;
        self.choose_target = false;
    }

    pub fn follow(&mut self) {
        self.idle = false;
        self.interact = false;
        self.flee = false;
        self.follow = true;
        self.attack = false;
        self.choose_target = false;
    }

    pub fn attack(&mut self) {
        self.idle = false;
        self.interact = false;
        self.flee = false;
        self.follow = false;
        self.attack = true;
        self.choose_target = false;
    }

    pub fn choose_target(&mut self) {
        self.idle = false;
        self.interact = false;
        self.flee = false;
        self.follow = false;
        self.attack = false;
        self.choose_target = true;
    }
}

//pub enum Activity {
//    Interact {
//        timer: f32,
//        interaction: AgentEvent,
//    },
//    Idle {
//        bearing: Vec2<f32>,
//        chaser: Chaser,
//    },
//    Follow {
//        target: EcsEntity,
//        chaser: Chaser,
//    },
//    Attack {
//        target: EcsEntity,
//        chaser: Chaser,
//        time: f64,
//        been_close: bool,
//        powerup: f32,
//    },
//    Flee {
//        target: EcsEntity,
//        chaser: Chaser,
//        timer: f32,
//    },
//}
//
//impl Activity {
//    pub fn is_follow(&self) -> bool { matches!(self, Activity::Follow { .. })
// }
//
//    pub fn is_attack(&self) -> bool { matches!(self, Activity::Attack { .. })
// }
//
//    pub fn is_flee(&self) -> bool { matches!(self, Activity::Flee { .. }) }
//}
//
//impl Default for Activity {
//    fn default() -> Self {
//        Activity::Idle {
//            bearing: Vec2::zero(),
//            chaser: Chaser::default(),
//        }
//    }
//}
