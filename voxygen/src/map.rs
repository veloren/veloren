use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
//use std::f32::{sin, cos};

use nalgebra::{Vector2, Vector3, Matrix4, Translation3};
use glutin::ElementState;

use client::{Client, ClientMode};
use camera::Camera;
use window::{RenderWindow, Event};
use model_object::{ModelObject, Constants};
use mesh::{Mesh, Vertex};
use region::Chunk;
use key_state::KeyState;
use std::sync::RwLock;
use std::sync::RwLockWriteGuard;

pub struct Map {
    chunks: HashMap<Vector3<u64>, ModelObject>
}

impl Map {
    pub fn new() -> Map {
        Map {
            chunks: HashMap::new(),
        }
    }

    pub fn chunks(&mut self) -> &mut HashMap<Vector3<u64>, ModelObject> {
        &mut self.chunks
    }
}
