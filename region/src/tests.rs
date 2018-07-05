// Standard
use std::io::ErrorKind::UnexpectedEof;
use std::net::{TcpStream, TcpListener, Shutdown::Both};
use std::thread;
use std::sync::{Mutex};
use std::time::Duration;

//Library
use coord::prelude::*;

// Parent
use super::{Volume, Voxel, Model};
use super::collision::{resolve_collision, Collidable, Cuboid};

fn newmodel(middle: Vec3<f64>, size: Vec3<f64>) -> Collidable {
    let col = Collidable::Cuboid{ cuboid: Cuboid::new(middle, size) };
    return col;
}

#[test]
fn might_colide_simple() {
    let m1 = newmodel(vec3!(0.5, 0.5, 0.5), vec3!(1.0, 1.0, 1.0));
    let m2 = newmodel(vec3!(1.5, 0.5, 0.5), vec3!(1.0, 1.0, 1.0));
    let res = resolve_collision(m1, m2).unwrap();
    assert_eq!(res.collision, vec3!(0.0, 0.0, 0.0));
    assert_eq!(res.a_correction, vec3!(0.0, 0.0, 0.001));
    assert_eq!(res.b_correction, vec3!(0.0, 0.0, 0.001));

    let m1 = newmodel(vec3!(0.5, 0.5, 0.5), vec3!(1.0, 1.0, 1.0));
    let m2 = newmodel(vec3!(2.5, 0.5, 0.5), vec3!(1.0, 1.0, 1.0));
    let res = resolve_collision(m1, m2);
    assert!(res.is_none());
}
