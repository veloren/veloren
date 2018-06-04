use rand;
use noise::{NoiseFn, OpenSimplex, Seedable};

use {Volume, Voxel, Block, BlockMaterial};

pub struct Chunk {
    size: (i32, i32, i32),
    voxels: Vec<Block>,
}

impl Chunk {
    pub fn test(size: (i32, i32, i32)) -> Chunk {

        let mut noise = OpenSimplex::new().set_seed(1337);

        let mut voxels = Vec::new();

        for i in 0..size.0 {
            for j in 0..size.1 {
                let height = ((noise.get([i as f64 * 0.1, j as f64 * 0.1]) * 0.5 + 0.5) * size.2 as f64) as i32;
                for k in 0..size.2 {
                    voxels.push(Block::new(
                        if k <= height {
                            if k < height - 4 {
                                BlockMaterial::Stone
                            } else if k < height {
                                BlockMaterial::Earth
                            } else {
                                BlockMaterial::Grass
                            }
                        } else {
                            BlockMaterial::Air
                        }
                    ));
                }
            }
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
