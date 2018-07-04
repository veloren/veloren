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
use super::physic::{might_colide, do_colide, CuboidColission};

fn newmodel(size: Vec3<i64>, offset: Vec3<i64>, rotation: Vec3<f64>) -> Model {
    let mut model = Model::new();
    model.set_size(size);
    model.set_offset(offset);
    model.set_rotation(rotation);
    return model;
}

#[test]
fn middle1() {
    let m = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.middle(),vec3!(0.5, 0.5, 0.5));
    let m = newmodel(vec3!(1, 1, 1), vec3!(2, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.middle(),vec3!(2.5, 0.5, 0.5));
    let m = newmodel(vec3!(1, 1, 1), vec3!(2, 0, 30), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.middle(),vec3!(2.5, 0.5, 30.5));
    let m = newmodel(vec3!(10, 10, 10), vec3!(2, 4, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.middle(),vec3!(7.0, 9.0, 5.0));
    let m = newmodel(vec3!(10, 10, 10), vec3!(2, 4, 0), vec3!(2.0, 0.0, 0.0));
    assert_eq!(m.middle(),vec3!(7.0, 9.0, 5.0));
    let m = newmodel(vec3!(10, 10, 10), vec3!(2, 4, 0), vec3!(0.0, 1.0, 0.5));
    assert_eq!(m.middle(),vec3!(7.0, 9.0, 5.0));
    let m = newmodel(vec3!(10, 10, 10), vec3!(2, 4, 0), vec3!(0.2, 0.4, 0.1));
    assert_eq!(m.middle(),vec3!(7.0, 9.0, 5.0));
}

#[test]
fn radius1() {
    let m = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.radius(),vec3!(0.5, 0.5, 0.5));
    let m = newmodel(vec3!(1, 3, 1), vec3!(2, 0, 30), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.radius(),vec3!(0.5, 1.5, 0.5));
    let m = newmodel(vec3!(10, 10, 4), vec3!(2, 4, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(m.radius(),vec3!(5.0, 5.0, 2.0));
    let m = newmodel(vec3!(5, 10, 10), vec3!(2, 4, 0), vec3!(0.2, 0.4, 0.1));
    assert_eq!(m.radius(),vec3!(2.5, 5.0, 5.0));
}

#[test]
fn rotation1() {
    let m = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert_eq!(<Model as CuboidColission>::rotation(&m),vec3!(0.0, 0.0, 0.0));
    let m = newmodel(vec3!(5, 10, 10), vec3!(2, 4, 0), vec3!(0.2, 0.4, 0.1));
    assert_eq!(<Model as CuboidColission>::rotation(&m),vec3!(0.2, 0.4, 0.1));
}

#[test]
fn might_colide_simple() {
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2));
    let m1 = newmodel(vec3!(10, 10, 10), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(10, 10, 10), vec3!(10, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2)); //should exactly colide
    let m1 = newmodel(vec3!(10, 10, 10), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(10, 10, 10), vec3!(11, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2)); //colide because could be rotated
    let m1 = newmodel(vec3!(10, 10, 10), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(10, 10, 10), vec3!(18, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(!might_colide(m1, m2)); // no longer collide
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(1, 1, 1), vec3!(2, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(!might_colide(m1, m2));
}

#[test]
fn might_colide_maxradius() {
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(1, 1, 1), vec3!(1, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2));
    let m1 = newmodel(vec3!(1, 2, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(1, 1, 1), vec3!(1, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2));
}

#[test]
fn might_colide3() {
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2));
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(2, 2, 2), vec3!(1, 1, 0), vec3!(0.0, 0.0, 0.0));
    assert!(might_colide(m1, m2));
    let m1 = newmodel(vec3!(1, 1, 1), vec3!(0, 0, 0), vec3!(0.0, 0.0, 0.0));
    let m2 = newmodel(vec3!(2, 2, 2), vec3!(2, 2, 0), vec3!(0.0, 0.0, 0.0));
    assert!(!might_colide(m1, m2));
}
