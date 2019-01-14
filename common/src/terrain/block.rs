// Crate
use crate::vol::Vox;

#[derive(Copy, Clone, Debug)]
pub struct Block {
    kind: u8,
    color: [u8; 3],
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
