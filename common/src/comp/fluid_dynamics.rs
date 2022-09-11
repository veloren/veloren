#[cfg(not(target_arch = "wasm32"))]
use super::body::{object, Body};
use super::{Density, Ori, Vel};
use crate::{
    consts::{AIR_DENSITY, LAVA_DENSITY, WATER_DENSITY},
    util::{Dir, Plane, Projection},
};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use vek::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiquidKind {
    Water,
    Lava,
}

impl LiquidKind {
    /// If an entity is in multiple overlapping liquid blocks, which one takes
    /// precedence? (should be a rare edge case, since checkerboard patterns of
    /// water and lava shouldn't show up in worldgen)
    #[inline]
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        use LiquidKind::{Lava, Water};
        match (self, other) {
            (Water, Water) => Water,
            (Water, Lava) => Lava,
            (Lava, _) => Lava,
        }
    }
}

/// Fluid medium in which the entity exists
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Fluid {
    Air {
        vel: Vel,
        elevation: f32,
    },
    Liquid {
        kind: LiquidKind,
        vel: Vel,
        depth: f32,
    },
}

impl Fluid {
    /// Specific mass
    pub fn density(&self) -> Density {
        match self {
            Self::Air { .. } => Density(AIR_DENSITY),
            Self::Liquid {
                kind: LiquidKind::Water,
                ..
            } => Density(WATER_DENSITY),
            Self::Liquid {
                kind: LiquidKind::Lava,
                ..
            } => Density(LAVA_DENSITY),
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
            Self::Liquid { vel, .. } => *vel,
        }
    }

    // Very simple but useful in reducing mental overhead
    pub fn relative_flow(&self, vel: &Vel) -> Vel { Vel(self.flow_vel().0 - vel.0) }

    pub fn is_liquid(&self) -> bool { matches!(self, Fluid::Liquid { .. }) }

    pub fn elevation(&self) -> Option<f32> {
        match self {
            Fluid::Air { elevation, .. } => Some(*elevation),
            _ => None,
        }
    }

    pub fn depth(&self) -> Option<f32> {
        match self {
            Fluid::Liquid { depth, .. } => Some(*depth),
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

pub struct Wings {
    pub aspect_ratio: f32,
    pub planform_area: f32,
    pub ori: Ori,
}

#[cfg(not(target_arch = "wasm32"))]
impl Body {
    pub fn aerodynamic_forces(
        &self,
        rel_flow: &Vel,
        fluid_density: f32,
        wings: Option<&Wings>,
    ) -> Vec3<f32> {
        let v_sq = rel_flow.0.magnitude_squared();
        if v_sq < 0.25 {
            // don't bother with minuscule forces
            Vec3::zero()
        } else {
            let rel_flow_dir = Dir::new(rel_flow.0 / v_sq.sqrt());
            0.5 * fluid_density
                * v_sq
                * match wings {
                    Some(&Wings {
                        aspect_ratio,
                        planform_area,
                        ori,
                    }) => {
                        if aspect_ratio > 25.0 {
                            tracing::warn!(
                                "Calculating lift for wings with an aspect ratio of {}. The \
                                 formulas are only valid for aspect ratios below 25.",
                                aspect_ratio
                            )
                        };
                        let ar = aspect_ratio.min(24.0);
                        // We have an elliptical wing; proceed to calculate its lift and drag

                        // aoa will be positive when we're pitched up and negative otherwise
                        let aoa = angle_of_attack(&ori, &rel_flow_dir);
                        // c_l will be positive when aoa is positive (we have positive lift,
                        // producing an upward force) and negative otherwise
                        let c_l = lift_coefficient(ar, aoa);

                        // lift dir will be orthogonal to the local relative flow vector.
                        // Local relative flow is the resulting vector of (relative) freestream
                        // flow + downwash (created by the vortices
                        // of the wing tips)
                        let lift_dir: Dir = {
                            // induced angle of attack
                            let aoa_i = c_l / (PI * ar);
                            // effective angle of attack; the aoa as seen by aerofoil after
                            // downwash
                            let aoa_eff = aoa - aoa_i;
                            // Angle between chord line and local relative wind is aoa_eff
                            // radians. Direction of lift is
                            // perpendicular to local relative wind.
                            // At positive lift, local relative wind will be below our cord line
                            // at an angle of aoa_eff. Thus if
                            // we pitch down by aoa_eff radians then
                            // our chord line will be colinear with local relative wind vector
                            // and our up will be the direction
                            // of lift.
                            ori.pitched_down(aoa_eff).up()
                        };

                        // induced drag coefficient (drag due to lift)
                        let cdi = {
                            // Oswald's efficiency factor (empirically derived--very magical)
                            // (this definition should not be used for aspect ratios > 25)
                            let e = 1.78 * (1.0 - 0.045 * ar.powf(0.68)) - 0.64;
                            c_l.powi(2) / (PI * e * ar)
                        };

                        // drag coefficient
                        let c_d = zero_lift_drag_coefficient() + cdi;
                        debug_assert!(c_d.is_sign_positive());
                        debug_assert!(c_l.is_sign_positive() || aoa.is_sign_negative());

                        planform_area * (c_l * *lift_dir + c_d * *rel_flow_dir)
                            + self.parasite_drag() * *rel_flow_dir
                    },

                    _ => self.parasite_drag() * *rel_flow_dir,
                }
        }
    }

    /// Parasite drag is the sum of pressure drag and skin friction.
    /// Skin friction is the drag arising from the shear forces between a fluid
    /// and a surface, while pressure drag is due to flow separation. Both are
    /// viscous effects.
    fn parasite_drag(&self) -> f32 {
        // Reference area and drag coefficient assumes best-case scenario of the
        // orientation producing least amount of drag
        match self {
            // Cross-section, head/feet first
            Body::BipedLarge(_) | Body::BipedSmall(_) | Body::Golem(_) | Body::Humanoid(_) => {
                let dim = self.dimensions().xy().map(|a| a * 0.5);
                const CD: f32 = 0.7;
                CD * PI * dim.x * dim.y
            },

            // Cross-section, nose/tail first
            Body::Theropod(_)
            | Body::QuadrupedMedium(_)
            | Body::QuadrupedSmall(_)
            | Body::QuadrupedLow(_)
            | Body::Arthropod(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                let cd: f32 = if matches!(self, Body::QuadrupedLow(_)) {
                    0.7
                } else {
                    1.0
                };
                cd * PI * dim.x * dim.z
            },

            // Cross-section, zero-lift angle; exclude the wings (width * 0.2)
            Body::BirdMedium(_) | Body::BirdLarge(_) | Body::Dragon(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                let cd: f32 = match self {
                    // "Field Estimates of Body Drag Coefficient
                    // on the Basis of Dives in Passerine Birds",
                    // Anders Hedenström and Felix Liechti, 2001
                    Body::BirdLarge(_) | Body::BirdMedium(_) => 0.2,
                    // arbitrary
                    _ => 0.7,
                };
                cd * PI * dim.x * 0.2 * dim.z
            },

            // Cross-section, zero-lift angle; exclude the fins (width * 0.2)
            Body::FishMedium(_) | Body::FishSmall(_) => {
                let dim = self.dimensions().map(|a| a * 0.5);
                // "A Simple Method to Determine Drag Coefficients in Aquatic Animals",
                // D. Bilo and W. Nachtigall, 1980
                const CD: f32 = 0.031;
                CD * PI * dim.x * 0.2 * dim.z
            },

            Body::Object(object) => match object {
                // very streamlined objects
                object::Body::Arrow
                | object::Body::ArrowSnake
                | object::Body::ArrowTurret
                | object::Body::FireworkBlue
                | object::Body::FireworkGreen
                | object::Body::FireworkPurple
                | object::Body::FireworkRed
                | object::Body::FireworkWhite
                | object::Body::FireworkYellow
                | object::Body::MultiArrow
                | object::Body::Dart => {
                    let dim = self.dimensions().map(|a| a * 0.5);
                    const CD: f32 = 0.02;
                    CD * PI * dim.x * dim.z
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
                    const CD: f32 = 0.5;
                    CD * PI * dim.x * dim.z
                },

                _ => {
                    let dim = self.dimensions();
                    const CD: f32 = 2.0;
                    CD * (PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)
                },
            },

            Body::ItemDrop(_) => {
                let dim = self.dimensions();
                const CD: f32 = 2.0;
                CD * (PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)
            },

            Body::Ship(_) => {
                // Airships tend to use the square of the cube root of its volume for
                // reference area
                let dim = self.dimensions();
                (PI / 6.0 * dim.x * dim.y * dim.z).powf(2.0 / 3.0)
            },
        }
    }
}

/// Geometric angle of attack
///
/// # Note
/// This ignores spanwise flow (i.e. we remove the spanwise flow component).
/// With greater yaw comes greater loss of accuracy as more flow goes
/// unaccounted for.
pub fn angle_of_attack(ori: &Ori, rel_flow_dir: &Dir) -> f32 {
    rel_flow_dir
        .projected(&Plane::from(ori.right()))
        .map(|flow_dir| PI / 2.0 - ori.up().angle_between(flow_dir.to_vec()))
        .unwrap_or(0.0)
}

/// Total lift coefficient for a finite wing of symmetric aerofoil shape and
/// elliptical pressure distribution.
pub fn lift_coefficient(aspect_ratio: f32, aoa: f32) -> f32 {
    let aoa_abs = aoa.abs();
    let stall_angle = PI * 0.1;
    if aoa_abs < stall_angle {
        lift_slope(aspect_ratio, None) * aoa
    } else {
        // This is when flow separation and turbulence starts to kick in.
        // Going to just make something up (based on some data), as the alternative is
        // to just throw your hands up and return 0
        let aoa_s = aoa.signum();
        let c_l_max = lift_slope(aspect_ratio, None) * stall_angle;
        let deg_45 = PI / 4.0;
        if aoa_abs < deg_45 {
            // drop directly to 0.6 * max lift at stall angle
            // then climb back to max at 45°
            Lerp::lerp(0.6 * c_l_max, c_l_max, aoa_abs / deg_45) * aoa_s
        } else {
            // let's just say lift goes down linearly again until we're at 90°
            Lerp::lerp(c_l_max, 0.0, (aoa_abs - deg_45) / deg_45) * aoa_s
        }
    }
}

/// The zero-lift profile drag coefficient is the parasite drag on the wings
/// at the angle of attack which generates no lift
pub fn zero_lift_drag_coefficient() -> f32 { 0.026 }

/// The change in lift over change in angle of attack¹. Multiplying by angle
/// of attack gives the lift coefficient (for a finite wing, not aerofoil).
/// Aspect ratio is the ratio of total wing span squared over planform area.
///
/// # Notes
/// Only valid for symmetric, elliptical wings at small² angles of attack³.
/// Does not apply to twisted, cambered or delta wings. (It still gives a
/// reasonably accurate approximation if the wing shape is not truly
/// elliptical.)
/// 1. geometric angle of attack, i.e. the pitch angle relative to
/// freestream flow
/// 2. up to around ~18°, at which point maximum lift has been achieved and
/// thereafter falls precipitously, causing a stall (this is the stall
/// angle)
/// 3. effective aoa, i.e. geometric aoa - induced aoa; assumes
/// no sideslip
// TODO: Look into handling tapered wings
fn lift_slope(aspect_ratio: f32, sweep_angle: Option<f32>) -> f32 {
    // lift slope for a thin aerofoil, given by Thin Aerofoil Theory
    let a0 = 2.0 * PI;
    if let Some(sweep) = sweep_angle {
        // for swept wings we use Kuchemann's modification to Helmbold's
        // equation
        let a0_cos_sweep = a0 * sweep.cos();
        let x = a0_cos_sweep / (PI * aspect_ratio);
        a0_cos_sweep / ((1.0 + x.powi(2)).sqrt() + x)
    } else if aspect_ratio < 4.0 {
        // for low aspect ratio wings (AR < 4) we use Helmbold's equation
        let x = a0 / (PI * aspect_ratio);
        a0 / ((1.0 + x.powi(2)).sqrt() + x)
    } else {
        // for high aspect ratio wings (AR > 4) we use the equation given by
        // Prandtl's lifting-line theory
        a0 / (1.0 + (a0 / (PI * aspect_ratio)))
    }
}
