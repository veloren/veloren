use rand;
use noise::{NoiseFn, OpenSimplex, Seedable};

use {Volume, Voxel, Block, BlockMaterial};

pub struct Chunk {
    size: (i32, i32, i32),
    voxels: Vec<Block>,
}

impl Chunk {
    pub fn test(size: (i32, i32, i32)) -> Chunk {

        let mut noise0 = OpenSimplex::new().set_seed(1337);
        let mut noise1 = OpenSimplex::new().set_seed(1338);
        let mut noise2 = OpenSimplex::new().set_seed(1339);
        let mut noise3 = OpenSimplex::new().set_seed(1340);

        let mut voxels = Vec::new();

        for i in 0..size.0 {
            for j in 0..size.1 {
                for k in 0..size.2 {
                    let (x, y) = (
                        i as f64 + noise2.get([i as f64 * 0.02, j as f64 * 0.02, k as f64 * 0.05]) * 16.0,
                        j as f64 + noise3.get([i as f64 * 0.02, j as f64 * 0.02, k as f64 * 0.05]) * 16.0
                    );
                    let noise = noise0.get([x as f64 * 0.02, y as f64 * 0.02]) + 0.2 * noise1.get([x as f64 * 0.1, y as f64 * 0.1]);
                    let height = ((noise * 0.5 + 0.5) * size.2 as f64) as i32;

                    voxels.push(Block::new(
                        if k <= height {
                            if k < height - 4 {
                                BlockMaterial::Stone
                            } else if k < height {
                                BlockMaterial::Earth
                            } else if k <= size.2 / 3 + 3 {
                                BlockMaterial::Sand
                            } else if k > (size.2 * 2) / 3 {
                                BlockMaterial::Stone
                            } else {
                                BlockMaterial::Grass
                            }
                        } else {
                            if k <= size.2 / 3 {
                                BlockMaterial::Water
                            } else {
                                BlockMaterial::Air
                            }
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
