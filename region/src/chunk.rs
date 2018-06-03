use rand;

use {Volume, Voxel, Block, BlockMaterial};

pub struct Chunk {
    size: (i32, i32, i32),
    voxels: Vec<Block>,
}

impl Chunk {
    pub fn test(size: (i32, i32, i32)) -> Chunk {
        let mut voxels = Vec::new();
        for i in 0..(size.0 * size.1 * size.2) {
            voxels.push(Block::new(
                if rand::random::<bool>() {
                    BlockMaterial::Air
                } else {
                    BlockMaterial::Stone
                }
            ));
        }

        Chunk {
            size,
            voxels,
        }
    }
}

impl Volume for Chunk {
    type VoxelType = Block;

    fn empty() -> Self {
        Chunk {
            size: (0, 0, 0),
            voxels: Vec::new(),
        }
    }

    fn empty_with_size(size: (i32, i32, i32)) -> Self {
        Chunk {
            size,
            voxels: Vec::new(),
        }
    }

    fn size(&self) -> (i32, i32, i32) {
        self.size
    }

    fn at(&self, pos: (i32, i32, i32)) -> Option<Block> {
        if pos.0 < 0 || pos.1 < 0 || pos.2 < 0 ||
            pos.0 >= self.size.0 || pos.1 >= self.size.1 || pos.2 >= self.size.2
        {
            None
        } else {
            Some(self.voxels[(
                pos.0 * self.size.1 * self.size.2 +
                pos.1 * self.size.2 +
                pos.2
            ) as usize])
        }
    }
}
