use specs::{Component, FlaggedStorage, HashMapStorage};
//use specs_idvs::IDVStorage;
use std::{
    time::Duration,
    hash::{Hash, Hasher},
    ops::MulAssign,
};
use vek::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum MovementState {
    Stand,
    Sit,
    Run,
    Jump,
    Glide { oriq: OriQ, rotq: OriQ },
    Roll { time_left: Duration },
    Swim,
    Climb,
}

impl MovementState {
    pub fn is_roll(&self) -> bool {
        if let Self::Roll { .. } = self {
            true
        } else {
            false
        }
    }
    
    pub fn is_glide(&self) -> bool {
        if let Self::Glide { .. } = self {
            true
        } else {
            false
        }
    }
    
    pub fn start_glide(&mut self, ori: Vec3<f32>) {
        *self = Self::Glide { oriq: OriQ::from(ori), rotq: OriQ::new() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum ActionState {
    Idle,
    Wield { time_left: Duration },
    Attack { time_left: Duration, applied: bool },
    Block { time_left: Duration },
    //Carry,
}

impl ActionState {
    pub fn is_wield(&self) -> bool {
        if let Self::Wield { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_attack(&self) -> bool {
        if let Self::Attack { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn is_block(&self) -> bool {
        if let Self::Block { .. } = self {
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct CharacterState {
    pub movement: MovementState,
    pub action: ActionState,
}

impl CharacterState {
    pub fn is_same_movement(&self, other: &Self) -> bool {
        // Check if enum item is the same without looking at the inner data
        std::mem::discriminant(&self.movement) == std::mem::discriminant(&other.movement)
    }
    pub fn is_same_action(&self, other: &Self) -> bool {
        // Check if enum item is the same without looking at the inner data
        std::mem::discriminant(&self.action) == std::mem::discriminant(&other.action)
    }
    pub fn is_same_state(&self, other: &Self) -> bool {
        self.is_same_movement(other) && self.is_same_action(other)
    }
}

impl Default for CharacterState {
    fn default() -> Self {
        Self {
            movement: MovementState::Jump,
            action: ActionState::Idle,
        }
    }
}

impl Component for CharacterState {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OriQ {
    oriq: Quaternion<f32>
}

impl OriQ {
    pub fn new() -> Self {
        Self {
            oriq: Quaternion::identity()
        }
    }
    
    pub fn val(&self) -> Quaternion<f32> {
        self.oriq
    }
    
    pub fn ori(&self) -> Vec3<f32> {
        self.oriq * Vec3::unit_y()
    }
    
    pub fn left(&self) -> Vec3<f32> {
        self.oriq * -Vec3::unit_x()
    }
    
    pub fn set(&mut self, q: Quaternion<f32>) {
        self.oriq = q
    }
}

impl Eq for OriQ {}

impl Hash for OriQ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        0.hash(state) // Two figures with different orientation quaternions are to be considered the same
    }
}

impl From<Vec3<f32>> for OriQ {
    fn from(v: Vec3<f32>) -> Self {
        Self {
            oriq: Quaternion::rotation_from_to_3d(Vec3::unit_y(), v)
        }
    }
}

impl From<Quaternion<f32>> for OriQ {
    fn from(q: Quaternion<f32>) -> Self {
        let l = q.magnitude_squared();
        Self {
            oriq: if !l.is_finite() || l == 0.0 { Quaternion::identity() } else { q.normalized() }
        }
    }
}

// Representing multiplying this on the left by other
impl MulAssign<Quaternion<f32>> for OriQ {
    fn mul_assign(&mut self, other: Quaternion<f32>) {
        *self = Self {
            oriq: other * self.val()
        }
    }
}