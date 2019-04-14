use vek::*;
use serde_derive::{Serialize, Deserialize};

// Crate
use crate::vol::Vox;

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    kind: u8,
    color: [u8; 3],
}

impl Block {
    pub fn new(kind: u8, color: Rgb<u8>) -> Self {
        Self {
            kind,
            color: color.into_array(),
        }
    }

    pub fn get_color(&self) -> Option<Rgb<u8>> {
        if self.is_empty() {
            None
        } else {
            Some(self.color.into())
        }
    }
}

impl Vox for Block {
    fn empty() -> Self {
        Self {
            kind: 0,
            color: [0; 3],
        }
    }

    fn is_empty(&self) -> bool {
        self.kind == 0
    }
}
