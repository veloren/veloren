use super::{Fluid, Ori};
use crate::{
    comp::{body::ship::figuredata::VoxelCollider, inventory::item::armor::Friction},
    consts::WATER_DENSITY,
    terrain::Block,
    uid::Uid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage, NullStorage, VecStorage};
use std::sync::Arc;
use vek::*;

/// Position
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pos(pub Vec3<f32>);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

/// Velocity
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vel(pub Vec3<f32>);

impl Vel {
    pub fn zero() -> Self { Vel(Vec3::zero()) }
}

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

/// Used to defer writes to Pos/Vel in nested join loops
#[derive(Copy, Clone, Debug)]
pub struct PosVelOriDefer {
    pub pos: Option<Pos>,
    pub vel: Option<Vel>,
    pub ori: Option<Ori>,
}

impl Component for PosVelOriDefer {
    type Storage = VecStorage<Self>;
}

/// Cache of Velocity (of last tick) * dt (of curent tick)
/// It's updated and read in physics sys to speed up entity<->entity collisions
/// no need to send it via network
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub struct PreviousPhysCache {
    pub velocity_dt: Vec3<f32>,
    /// Center of bounding sphere that encompasses the entity along its path for
    /// this tick
    pub center: Vec3<f32>,
    /// Calculates a Sphere over the Entity for quick boundary checking
    pub collision_boundary: f32,
    pub scale: f32,
    /// Approximate radius of cylinder of collider.
    pub scaled_radius: f32,
    /// Radius of stadium of collider.
    pub neighborhood_radius: f32,
    /// relative p0 and p1 of collider's statium, None if cylinder.
    pub origins: Option<(Vec2<f32>, Vec2<f32>)>,
    pub pos: Option<Pos>,
    pub ori: Quaternion<f32>,
}

impl Component for PreviousPhysCache {
    type Storage = VecStorage<Self>;
}

// Scale
#[derive(Copy, Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scale(pub f32);

impl Component for Scale {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

// Mass
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Mass(pub f32);

impl Default for Mass {
    fn default() -> Mass { Mass(1.0) }
}

impl Component for Mass {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

/// The average density (specific mass) of an entity.
/// Units used for reference is kg/m³
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Density(pub f32);

impl Default for Density {
    fn default() -> Density { Density(WATER_DENSITY) }
}

impl Component for Density {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

// Collider
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Collider {
    /// A volume based on an existing voxel asset.
    // TODO: pass the map from ids -> voxel data to get_radius
    // and get_z_limits to compute a bounding cylinder.
    Voxel {
        id: String,
    },
    /// A mutable volume.
    Volume(Arc<VoxelCollider>),
    /// Capsule prism with line segment from p0 to p1
    CapsulePrism {
        p0: Vec2<f32>,
        p1: Vec2<f32>,
        radius: f32,
        z_min: f32,
        z_max: f32,
    },
    Point,
}

impl Collider {
    pub fn is_voxel(&self) -> bool { matches!(self, Collider::Voxel { .. } | Collider::Volume(_)) }

    pub fn bounding_radius(&self) -> f32 {
        match self {
            Collider::Voxel { .. } | Collider::Volume(_) => 1.0,
            Collider::CapsulePrism { radius, p0, p1, .. } => {
                let a = p0.distance(*p1);
                a / 2.0 + *radius
            },
            Collider::Point => 0.0,
        }
    }

    pub fn get_height(&self) -> f32 {
        let (z_min, z_max) = self.get_z_limits(1.0);
        z_max - z_min
    }

    pub fn get_z_limits(&self, modifier: f32) -> (f32, f32) {
        match self {
            Collider::Voxel { .. } | Collider::Volume(_) => (0.0, 1.0),
            Collider::CapsulePrism { z_min, z_max, .. } => (*z_min * modifier, *z_max * modifier),
            Collider::Point => (0.0, 0.0),
        }
    }
}

impl Component for Collider {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sticky;

impl Component for Sticky {
    type Storage = DerefFlaggedStorage<Self, NullStorage<Self>>;
}

#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Immovable;

impl Component for Immovable {
    type Storage = DerefFlaggedStorage<Self, NullStorage<Self>>;
}

// PhysicsState
#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsState {
    pub on_ground: Option<Block>,
    pub on_ceiling: bool,
    pub on_wall: Option<Vec3<f32>>,
    pub touch_entities: HashMap<Uid, Vec3<f32>>,
    pub in_fluid: Option<Fluid>,
    pub ground_vel: Vec3<f32>,
    pub footwear: Friction,
    pub skating_last_height: f32,
    pub skating_active: bool,
}

impl PhysicsState {
    pub fn reset(&mut self) {
        // Avoid allocation overhead!
        let mut touch_entities = std::mem::take(&mut self.touch_entities);
        touch_entities.clear();
        *self = Self {
            touch_entities,
            ground_vel: self.ground_vel, /* Preserved, since it's the velocity of the last
                                          * contact point */
            ..Self::default()
        }
    }

    pub fn on_surface(&self) -> Option<Vec3<f32>> {
        self.on_ground
            .map(|_| -Vec3::unit_z())
            .or_else(|| self.on_ceiling.then_some(Vec3::unit_z()))
            .or(self.on_wall)
    }

    pub fn in_liquid(&self) -> Option<f32> { self.in_fluid.and_then(|fluid| fluid.depth()) }
}

impl Component for PhysicsState {
    type Storage = VecStorage<Self>;
}

/// Used to forcefully update the position, velocity, and orientation of the
/// client
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForceUpdate {
    flag: bool,
    counter: u64,
}

impl ForceUpdate {
    pub fn forced() -> Self {
        Self {
            flag: true,
            counter: 0,
        }
    }

    pub fn update(&mut self) {
        self.flag = true;
        self.counter = self.counter.wrapping_add(1);
    }

    pub fn clear(&mut self) { self.flag = false; }

    pub fn is_forced(&self) -> bool { self.flag }

    pub fn counter(&self) -> u64 { self.counter }
}

impl Component for ForceUpdate {
    type Storage = VecStorage<Self>;
}
