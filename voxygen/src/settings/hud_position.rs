use serde::{Deserialize, Serialize};
use vek::Vec2;

pub const DEFAULT_OWN_BAG_WIDTH: f64 = 424.0;
pub const DEFAULT_OWN_BAG_HEIGHT: f64 = 708.0;
pub const DEFAULT_OTHER_BAG_WIDTH: f64 = 424.0;
pub const DEFAULT_OTHER_BAG_HEIGHT: f64 = 548.0;

pub const DEFAULT_OWN_BAG_POSITION_MARGIN_BOTTOM: f64 = 70.0;
pub const DEFAULT_OWN_BAG_POSITION_MARGIN_RIGHT: f64 = 5.0;
pub const DEFAULT_OTHER_BAG_POSITION_MARGIN_BOTTOM: f64 = 230.0;
pub const DEFAULT_OTHER_BAG_POSITION_MARGIN_LEFT: f64 = 5.0;

pub const MINIMAP_POSITION_MARGIN_TOP: f64 = 5.0;
pub const MINIMAP_POSITION_MARGIN_RIGHT: f64 = 5.0;

pub const CRAFTING_POSITION_MARGIN_BOTTOM: f64 = 308.0;
pub const CRAFTING_POSITION_MARGIN_RIGHT: f64 = 450.0;

pub const SOCIAL_POSITION_MARGIN_LEFT: f64 = 25.0;
pub const SOCIAL_POSITION_MARGIN_BOTTOM: f64 = 308.0;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BagHudPosition {
    pub own: Vec2<f64>,
    pub other: Vec2<f64>,
}

impl Default for BagHudPosition {
    fn default() -> Self {
        Self {
            own: [
                DEFAULT_OWN_BAG_POSITION_MARGIN_RIGHT,
                DEFAULT_OWN_BAG_POSITION_MARGIN_BOTTOM,
            ]
            .into(),
            other: [
                DEFAULT_OTHER_BAG_POSITION_MARGIN_LEFT,
                DEFAULT_OTHER_BAG_POSITION_MARGIN_BOTTOM,
            ]
            .into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct HudPositionSettings {
    pub bag: BagHudPosition,
    pub minimap: Vec2<f64>,
    pub crafting: Vec2<f64>,
    pub social: Vec2<f64>,
}

impl Default for HudPositionSettings {
    fn default() -> Self {
        Self {
            bag: BagHudPosition::default(),
            minimap: [MINIMAP_POSITION_MARGIN_RIGHT, MINIMAP_POSITION_MARGIN_TOP].into(),
            crafting: [
                CRAFTING_POSITION_MARGIN_RIGHT,
                CRAFTING_POSITION_MARGIN_BOTTOM,
            ]
            .into(),
            social: [SOCIAL_POSITION_MARGIN_LEFT, SOCIAL_POSITION_MARGIN_BOTTOM].into(),
        }
    }
}
