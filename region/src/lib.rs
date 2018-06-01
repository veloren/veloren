#[repr(u16)]
#[derive(PartialEq)]
pub enum BlockMaterial {
    Air,
    Grass,
    Stone,
}

pub struct Block {
    mat: BlockMaterial,
}

impl Block {
    pub fn is_solid(&self) -> bool {
        self.mat != BlockMaterial::Air
    }
}

pub struct Volume<T> {
    blocks: Vec<T>,
}

pub type Chunk = Volume<Block>;
