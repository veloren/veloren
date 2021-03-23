use super::{
    body::{object, Body},
    Density, Vel,
};
use crate::{
    consts::{AIR_DENSITY, WATER_DENSITY},
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use vek::*;

/// Fluid medium in which the entity exists
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fluid {
    Air { vel: Vel, elevation: f32 },
    Water { vel: Vel, depth: f32 },
}

impl Fluid {
    /// Specific mass
    pub fn density(&self) -> Density {
        match self {
            Self::Air { .. } => Density(AIR_DENSITY),
            Self::Water { .. } => Density(WATER_DENSITY),
        }
    }

    /// Pressure from entity velocity
    pub fn dynamic_pressure(&self, vel: &Vel) -> f32 {
        0.5 * self.density().0 * self.relative_flow(vel).0.magnitude_squared()
    }

    /*
        pub fn static_pressure(&self) -> f32 {
            match self {
                Self::Air { elevation, .. } => Self::air_pressure(*elevation),
                Self::Water { depth, .. } => Self::water_pressure(*depth),
            }
        }

        /// Absolute static pressure of air at elevation
        pub fn air_pressure(elevation: f32) -> f32 {
            // At low altitudes above sea level, the pressure decreases by about 1.2 kPa for
            // every 100 metres.
            // https://en.wikipedia.org/wiki/Atmospheric_pressure#Altitude_variation
            ATMOSPHERE - elevation / 12.0
        }

        /// Absolute static pressure of water at depth
        pub fn water_pressure(depth: f32) -> f32 { WATER_DENSITY * GRAVITY * depth + ATMOSPHERE }
    */
    /// Velocity of fluid, if applicable
    pub fn flow_vel(&self) -> Vel {
        match self {
            Self::Air { vel, .. } => *vel,
            Self::Water { vel, .. } => *vel,
        }
    }

    // Very simple but useful in reducing mental overhead
    pub fn relative_flow(&self, vel: &Vel) -> Vel { Vel(self.flow_vel().0 - vel.0) }

    pub fn is_liquid(&self) -> bool { matches!(self, Fluid::Water { .. }) }

    pub fn elevation(&self) -> Option<f32> {
        match self {
            Fluid::Air { elevation, .. } => Some(*elevation),
            _ => None,
        }
    }

    pub fn depth(&self) -> Option<f32> {
        match self {
            Fluid::Water { depth, .. } => Some(*depth),
            _ => None,
        }
    }
}

impl Default for Fluid {
    fn default() -> Self {
        Self::Air {
            elevation: 0.0,
            vel: Vel::zero(),
        }
    }
}

impl Body {
    pub fn aerodynamic_forces(&self, rel_flow: &Vel, fluid_density: f32) -> Vec3<f32> {
        let v_sq = rel_flow.0.magnitude_squared();
        if v_sq < 0.25 {
            // don't bother with miniscule forces
            Vec3::zero()
        } else {
            let rel_flow_dir = Dir::new(rel_flow.0 / v_sq.sqrt());
            // All the coefficients come pre-multiplied by their reference area
            0.5 * fluid_density * v_sq * self.parasite_drag_coefficient() * *rel_flow_dir
        }
    }

    /// Parasite drag is the sum of pressure drag and skin friction.
    /// Skin friction is the drag arising from the shear forces between a fluid
    /// and a surface, while pressure drag is due to flow separation. Both are
    /// viscous effects.
    fn parasite_drag_coefficient(&self) -> f32 {
        // Reference area and drag coefficient assumes best-case scenario of the
        // orientation producing least amount of drag
        match self {
            // Cross-section, head/feet first
            Body::BipedLarge(_) | Body::BipedSmall(_) | Body::Golem(_) | Body::Humanoid(_) => {
                let dim = self.dimensions().xy().map(|a| a * 0.5);
                0.7 * PI * dim.x * dim.y
            },

            // Cross-section, nose/tail first
            Body::Theropod(_)
            | Body::QuadrupedMedium(_)
            | Body::QuadrupedSmall(_)
            | Body::QuadrupedLow(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                let cd = if matches!(self, Body::QuadrupedLow(_)) {
                    0.7
                } else {
                    1.0
                };
                cd * std::f32::consts::PI * dim.x * dim.z
            },

            // Cross-section, zero-lift angle; exclude the wings (width * 0.2)
            Body::BirdMedium(_) | Body::BirdSmall(_) | Body::Dragon(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                let cd = match self {
                    Body::BirdMedium(_) => 0.2,
                    Body::BirdSmall(_) => 0.4,
                    _ => 0.7,
                };
                cd * std::f32::consts::PI * dim.x * 0.2 * dim.z
            },

            // Cross-section, zero-lift angle; exclude the fins (width * 0.2)
            Body::FishMedium(_) | Body::FishSmall(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                0.031 * std::f32::consts::PI * dim.x * 0.2 * dim.z
            },

            Body::Object(object) => match object {
                // very streamlined objects
                object::Body::Arrow
                | object::Body::ArrowSnake
                | object::Body::FireworkBlue
                | object::Body::FireworkGreen
                | object::Body::FireworkPurple
                | object::Body::FireworkRed
                | object::Body::FireworkWhite
                | object::Body::FireworkYellow
                | object::Body::MultiArrow => {
                    let dim = self.dimensions().map(|a| a * 0.5);
                    0.02 * std::f32::consts::PI * dim.x * dim.z
                },

                // spherical-ish objects
                object::Body::BoltFire
                | object::Body::BoltFireBig
                | object::Body::BoltNature
                | object::Body::Bomb
                | object::Body::PotionBlue
                | object::Body::PotionGreen
                | object::Body::PotionRed
                | object::Body::Pouch
                | object::Body::Pumpkin
                | object::Body::Pumpkin2
                | object::Body::Pumpkin3
                | object::Body::Pumpkin4
                | object::Body::Pumpkin5 => {
                    let dim = self.dimensions().map(|a| a * 0.5);
                    0.5 * std::f32::consts::PI * dim.x * dim.z
                },

                _ => {
                    let dim = self.dimensions();
                    2.0 * (std::f32::consts::PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)
                },
            },

            Body::Ship(_) => {
                // Airships tend to use the square of the cube root of its volume for
                // reference area
                let dim = self.dimensions();
                (std::f32::consts::PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)
            },
        }
    }
}

/*
## References:

1. "Field Estimates of Body Drag Coefficient on the Basis of Dives in Passerine Birds",
    Anders Hedenström and Felix Liechti, 2001
2. "A Simple Method to Determine Drag Coefficients in Aquatic Animals",
    D. Bilo and W. Nachtigall, 1980
*/
