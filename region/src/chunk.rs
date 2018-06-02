use {Volume, Voxel, Block};

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
