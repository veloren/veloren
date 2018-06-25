use std::collections::HashMap;

use nalgebra::{Vector3};

use model_object::{ModelObject};

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
