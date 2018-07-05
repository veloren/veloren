use std::cmp;
use coord::prelude::*;
use {Volume, Voxel, Cell};

pub struct Cuboid {
    middle: Vec3<f64>,
    radius: Vec3<f64>,
}

pub struct CollisionResolution {
    pub collision: Vec3<f64>,
    pub a_correction: Vec3<f64>,
    pub b_correction: Vec3<f64>,
}

pub enum Collidable {
    Cuboid { cuboid: Cuboid },
    //add more here
}

pub fn resolve_collision(a: Collidable, b: Collidable) -> Option<CollisionResolution> {
    match a {
        Collidable::Cuboid { cuboid: a } => {
            match b {
                Collidable::Cuboid { cuboid: b } => {
                    cuboid_cuboid_col(a,b)
                },
            }
        },
    }
}

impl Cuboid {
    pub fn new(middle: Vec3<f64>, radius: Vec3<f64>) -> Self {
        Cuboid {
            middle,
            radius,
        }
    }

    pub fn lower(&self) -> Vec3<f64> {
        self.middle - self.radius
    }

    pub fn upper(&self) -> Vec3<f64> {
        self.middle + self.radius
    }
}

fn cuboid_cuboid_col(a: Cuboid, b: Cuboid) -> Option<CollisionResolution> {
    let la = a.lower();
    let ua = a.upper();
    let lb = b.lower();
    let ub = b.upper();
    if (ua.x > lb.x && la.x < ub.x &&
        ua.y > lb.y && la.y < ub.y &&
        ua.z > lb.z && la.z < ub.z) {
            // we collide
            return Some(CollisionResolution{
                collision: vec3!(0.0, 0.0, 0.0),
                a_correction: vec3!(0.0, 0.0, 0.001), // hack this stuff for now
                b_correction: vec3!(0.0, 0.0, 0.001),
            });
        };
    None
}
