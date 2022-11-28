use common_net::msg::PresenceKind;
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use specs::{Component, NullStorage};
use std::time::{Duration, Instant};
use vek::*;

#[derive(Debug)]
pub struct Presence {
    pub terrain_view_distance: ViewDistance,
    pub entity_view_distance: ViewDistance,
    pub kind: PresenceKind,
    pub lossy_terrain_compression: bool,
}

impl Presence {
    pub fn new(view_distances: common::ViewDistances, kind: PresenceKind) -> Self {
        let now = Instant::now();
        Self {
            terrain_view_distance: ViewDistance::new(view_distances.terrain, now),
            entity_view_distance: ViewDistance::new(view_distances.entity, now),
            kind,
            lossy_terrain_compression: false,
        }
    }
}

impl Component for Presence {
    type Storage = specs::DenseVecStorage<Self>;
}

// Distance from fuzzy_chunk before snapping to current chunk
pub const CHUNK_FUZZ: u32 = 2;
// Distance out of the range of a region before removing it from subscriptions
pub const REGION_FUZZ: u32 = 16;

#[derive(Clone, Debug)]
pub struct RegionSubscription {
    pub fuzzy_chunk: Vec2<i32>,
    pub last_entity_view_distance: u32,
    pub regions: HashSet<Vec2<i32>>,
}

impl Component for RegionSubscription {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositionOnChunkLoad;

impl Component for RepositionOnChunkLoad {
    type Storage = NullStorage<Self>;
}

#[derive(PartialEq, Debug, Clone, Copy)]
enum Direction {
    Up,
    Down,
}

/// Distance from the [Presence] from which the world is loaded and information
/// is synced to clients.
///
/// We limit the frequency that changes in the view distance change direction
/// (e.g. shifting from increasing the value to decreasing it). This is useful
/// since we want to avoid rapid cycles of shrinking and expanding of the view
/// distance.
#[derive(Debug)]
pub struct ViewDistance {
    direction: Direction,
    last_direction_change_time: Instant,
    target: Option<u32>,
    current: u32,
}

impl ViewDistance {
    /// Minimum time allowed between changes in direction of value adjustments.
    const TIME_PER_DIR_CHANGE: Duration = Duration::from_millis(300);

    pub fn new(start_value: u32, now: Instant) -> Self {
        Self {
            direction: Direction::Up,
            last_direction_change_time: now.checked_sub(Self::TIME_PER_DIR_CHANGE).unwrap_or(now),
            target: None,
            current: start_value,
        }
    }

    /// Returns the current value.
    pub fn current(&self) -> u32 { self.current }

    /// Applies deferred change based on the whether the time to apply it has
    /// been reached.
    pub fn update(&mut self, now: Instant) {
        if let Some(target_val) = self.target {
            if now.saturating_duration_since(self.last_direction_change_time)
                > Self::TIME_PER_DIR_CHANGE
            {
                self.last_direction_change_time = now;
                self.current = target_val;
                self.target = None;
            }
        }
    }

    /// Sets the target value.
    ///
    /// If this hasn't been changed recently or it is in the same direction as
    /// the previous change it will be applied immediately. Otherwise, it
    /// will be deferred to a later time (limiting the frequency of changes
    /// in the change direction).
    pub fn set_target(&mut self, new_target: u32, now: Instant) {
        use core::cmp::Ordering;
        let new_direction = match new_target.cmp(&self.current) {
            Ordering::Equal => return, // No change needed.
            Ordering::Less => Direction::Down,
            Ordering::Greater => Direction::Up,
        };

        // Change is in the same direction as before so we can just apply it.
        if new_direction == self.direction {
            self.current = new_target;
            self.target = None;
        // If it has already been a while since the last direction change we can
        // directly apply the request and switch the direction.
        } else if now.saturating_duration_since(self.last_direction_change_time)
            > Self::TIME_PER_DIR_CHANGE
        {
            self.direction = new_direction;
            self.last_direction_change_time = now;
            self.current = new_target;
            self.target = None;
        // Otherwise, we need to defer the request.
        } else {
            self.target = Some(new_target);
        }
    }
}
