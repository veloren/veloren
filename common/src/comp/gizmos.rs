//! Gizmos are used to visually debug server-side values.

use enum_map::EnumMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage};
use vek::{Rgba, Sphere, Vec3};

use crate::{resources::Time, uid::Uid};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Shape {
    Sphere(Sphere<f32, f32>),
    LineStrip(Vec<Vec3<f32>>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Gizmos {
    pub shape: Shape,
    pub color: Rgba<u8>,
    /// If `None`, this ends when the next batch of gizmos are recieved.
    pub end_time: Option<Time>,
}

impl Gizmos {
    pub fn sphere(pos: Vec3<f32>, radius: f32, color: impl Into<Rgba<u8>>) -> Self {
        Self {
            shape: Shape::Sphere(Sphere {
                center: pos,
                radius,
            }),
            color: color.into(),
            end_time: None,
        }
    }

    pub fn line(a: Vec3<f32>, b: Vec3<f32>, color: impl Into<Rgba<u8>>) -> Self {
        Self {
            shape: Shape::LineStrip(vec![a, b]),
            color: color.into(),
            end_time: None,
        }
    }

    pub fn line_strip(points: Vec<Vec3<f32>>, color: impl Into<Rgba<u8>>) -> Self {
        Self {
            shape: Shape::LineStrip(points),
            color: color.into(),
            end_time: None,
        }
    }

    /// If specified, this shape will be displayed up to `time`.
    ///
    /// If not specified this will end the next time a batch of gizmos are sent.
    pub fn with_end_time(mut self, time: Time) -> Self {
        self.end_time = Some(time);
        self
    }
}

#[derive(
    Serialize,
    Deserialize,
    strum::EnumString,
    strum::Display,
    strum::EnumIter,
    PartialEq,
    enum_map::Enum,
    Clone,
    Copy,
)]
pub enum GizmoSubscription {
    PathFinding,
    Rtsim,
}

#[derive(Default, Clone)]
pub enum GizmoContext {
    #[default]
    Disabled,
    Enabled,
    /// If applicable, only draw debug information with this specific target.
    EnabledWithTarget(Uid),
}

/// A component to subscribe to certain visual debug information.
pub struct GizmoSubscriber {
    pub gizmos: EnumMap<GizmoSubscription, GizmoContext>,
    /// The range of the debug drawing, in world space.
    pub range: f32,
}

/// A resource storing rtsim-gizmos.
#[derive(Default)]
pub struct RtsimGizmos {
    pub tracked: slotmap::SecondaryMap<crate::rtsim::NpcId, Vec<Gizmos>>,
}

impl Default for GizmoSubscriber {
    fn default() -> Self {
        GizmoSubscriber {
            gizmos: EnumMap::default(),
            range: 32.0,
        }
    }
}

impl Component for GizmoSubscriber {
    type Storage = DenseVecStorage<GizmoSubscriber>;
}
