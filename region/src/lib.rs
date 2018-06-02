pub trait Voxel: Copy + Clone {
    type Material: Copy + Clone;
    fn empty() -> Self;
    fn is_solid(&self) -> bool;
    fn material(&self) -> Self::Material;
}

#[repr(u16)]
#[derive(Copy, Clone, PartialEq)]
pub enum BlockMaterial {
    Air,
    Grass,
    Stone,
}

#[derive(Copy, Clone)]
pub struct Block {
    mat: BlockMaterial,
}

impl Voxel for Block {
    type Material = BlockMaterial;

    fn empty() -> Self {
        Block {
            mat: BlockMaterial::Air,
        }
    }

    fn is_solid(&self) -> bool {
        self.mat != BlockMaterial::Air
    }

    fn material(&self) -> Self::Material {
        self.mat
    }
}

pub trait Volume {
    type VoxelType: Copy + Clone;
    fn empty() -> Self;
}

pub struct Chunk<T: Voxel> {
    voxels: Vec<T>,
}

impl<T: Voxel> Volume for Chunk<T> {
    type VoxelType = T;

    fn empty() -> Self {
        Chunk::<T> {
            voxels: Vec::new(),
        }
    }
}

pub type TerrainChunk = Chunk<Block>;
