use crate::{resources::Time, uid::Uid};
use serde::{Deserialize, Serialize};
use specs::Component;
use vek::*;

#[derive(Copy, Clone, Debug)]
pub struct Waypoint {
    pos: Vec3<f32>,
    last_save: Time,
}

impl Waypoint {
    // TODO: add actual fix and remove this method
    pub fn temp_new(pos: Vec3<f32>, last_save: Time) -> Self {
        Self {
            pos: pos + Vec3::from(0.25f32).map(|e| e * rand::random::<f32>() - 0.25 / 2.0),
            last_save,
        }
    }

    pub fn new(pos: Vec3<f32>, last_save: Time) -> Self { Self { pos, last_save } }

    pub fn get_pos(&self) -> Vec3<f32> { self.pos }

    /// Time in seconds since this waypoint was saved
    pub fn elapsed(&self, time: Time) -> f64 { time.0 - self.last_save.0 }
}

impl Component for Waypoint {
    type Storage = specs::VecStorage<Self>;
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WaypointArea(f32);

impl WaypointArea {
    pub fn radius(&self) -> f32 { self.0 }
}

impl Component for WaypointArea {
    type Storage = specs::VecStorage<Self>;
}

impl Default for WaypointArea {
    fn default() -> Self { Self(5.0) }
}

/// Marker on the map, used for sharing waypoint with group and
/// persisting it server side.
#[derive(Copy, Clone, Debug)]
pub struct MapMarker(pub Vec2<i32>);

impl Component for MapMarker {
    type Storage = specs::VecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum MapMarkerChange {
    Update(Vec2<i32>),
    Remove,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MapMarkerUpdate {
    Owned(MapMarkerChange),
    GroupMember(Uid, MapMarkerChange),
    ClearGroup,
}
