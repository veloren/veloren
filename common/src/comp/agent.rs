use crate::{path::Chaser, sync::Uid};
use specs::{Component, Entity as EcsEntity, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
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
}

impl Alignment {
    // Always attacks
    pub fn hostile_towards(self, other: Alignment) -> bool {
        match (self, other) {
            (Alignment::Enemy, Alignment::Enemy) => false,
            (Alignment::Enemy, _) => true,
            (_, Alignment::Enemy) => true,
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
            (Alignment::Tame, Alignment::Npc) => true,
            (Alignment::Tame, Alignment::Tame) => true,
            _ => false,
        }
    }

    // TODO: Remove this hack
    pub fn is_friendly_to_players(&self) -> bool {
        match self {
            Alignment::Npc | Alignment::Tame | Alignment::Owned(_) => true,
            _ => false,
        }
    }
}

impl Component for Alignment {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Clone, Debug, Default)]
pub struct Agent {
    pub patrol_origin: Option<Vec3<f32>>,
    pub activity: Activity,
    /// Does the agent talk when e.g. hit by the player
    // TODO move speech patterns into a Behavior component
    pub can_speak: bool,
}

impl Agent {
    pub fn with_patrol_origin(mut self, origin: Vec3<f32>) -> Self {
        self.patrol_origin = Some(origin);
        self
    }

    pub fn new(origin: Vec3<f32>, can_speak: bool) -> Self {
        let patrol_origin = Some(origin);
        Agent {
            patrol_origin,
            can_speak,
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
    pub fn is_follow(&self) -> bool {
        match self {
            Activity::Follow { .. } => true,
            _ => false,
        }
    }

    pub fn is_attack(&self) -> bool {
        match self {
            Activity::Attack { .. } => true,
            _ => false,
        }
    }
}

impl Default for Activity {
    fn default() -> Self { Activity::Idle(Vec2::zero()) }
}
