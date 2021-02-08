use std::ops::Range;
use enum_iterator::IntoEnumIterator;
use vek::*;
use crate::util::math::close;

#[derive(Copy, Clone, Debug, IntoEnumIterator)]
pub enum ForestKind {
    Palm,
    Acacia,
    Baobab,
    Oak,
    Pine,
    Birch,
    Mangrove,
    Giant,
    Swamp,
}

pub struct Environment {
    pub humid: f32,
    pub temp: f32,
    pub near_water: f32,
}

impl ForestKind {
    pub fn humid_range(&self) -> Range<f32> {
        match self {
            ForestKind::Palm => 0.25..1.4,
            ForestKind::Acacia => 0.05..0.55,
            ForestKind::Baobab => 0.2..0.6,
            ForestKind::Oak => 0.35..1.5,
            ForestKind::Pine => 0.2..1.4,
            ForestKind::Birch => 0.0..0.6,
            ForestKind::Mangrove => 0.65..1.3,
            ForestKind::Swamp => 0.5..1.1,
            _ => 0.0..0.0
        }
    }

    pub fn temp_range(&self) -> Range<f32> {
        match self {
            ForestKind::Palm => 0.4..1.6,
            ForestKind::Acacia => 0.3..1.6,
            ForestKind::Baobab => 0.4..0.9,
            ForestKind::Oak => -0.35..0.6,
            ForestKind::Pine => -1.8..-0.2,
            ForestKind::Birch => -0.7..0.25,
            ForestKind::Mangrove => 0.4..1.6,
            ForestKind::Swamp => -0.6..0.8,
            _ => 0.0..0.0,
        }
    }

    pub fn near_water_range(&self) -> Option<Range<f32>> {
        match self {
            ForestKind::Palm => Some(0.35..1.8),
            ForestKind::Swamp => Some(0.5..1.8),
            _ => None,
        }
    }

    /// The relative rate at which this tree appears under ideal conditions
    pub fn ideal_proclivity(&self) -> f32 {
        match self {
            ForestKind::Palm => 0.4,
            ForestKind::Acacia => 0.6,
            ForestKind::Baobab => 0.2,
            ForestKind::Oak => 1.0,
            ForestKind::Pine => 1.0,
            ForestKind::Birch => 0.65,
            ForestKind::Mangrove => 1.0,
            ForestKind::Swamp => 1.0,
            _ => 0.0,
        }
    }

    pub fn proclivity(&self, env: &Environment) -> f32 {
        self.ideal_proclivity()
        * close(env.humid, self.humid_range())
        * close(env.temp, self.temp_range())
        * self.near_water_range().map_or(1.0, |near_water_range| close(env.near_water, near_water_range))
    }
}

pub struct TreeAttr {
    pub pos: Vec2<i32>,
    pub seed: u32,
    pub scale: f32,
    pub forest_kind: ForestKind,
    pub lanterns: bool,
}
