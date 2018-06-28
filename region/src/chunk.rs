use noise::{NoiseFn, OpenSimplex, Seedable};
use coord::prelude::*;

use {Volume, Voxel, Block, BlockMaterial};

pub struct Chunk {
    size: Vec3<i64>,
    offset: Vec3<i64>,
    voxels: Vec<Block>,
}

impl Chunk {
    pub fn test(offset: Vec3<i64>, size: Vec3<i64>) -> Chunk {

        let noise0 = OpenSimplex::new().set_seed(1337);
        let noise1 = OpenSimplex::new().set_seed(1338);
        let noise2 = OpenSimplex::new().set_seed(1339);
        let noise3 = OpenSimplex::new().set_seed(1340);

        let mut voxels = Vec::new();

        for i in 0..size.x {
            for j in 0..size.y {
                for k in 0..size.z {
                    let (x, y) = (
                        (i+offset.x) as f64 + noise2.get([(i+offset.x) as f64 * 0.02, (j+offset.y) as f64 * 0.02, (k+offset.z) as f64 * 0.05]) * 16.0,
                        (j+offset.y) as f64 + noise3.get([(i+offset.x) as f64 * 0.02, (j+offset.y) as f64 * 0.02, (k+offset.z) as f64 * 0.05]) * 16.0
                    );
                    let noise = noise0.get([x as f64 * 0.02, y as f64 * 0.02]) + 0.2 * noise1.get([x as f64 * 0.1, y as f64 * 0.1]);
                    let height = ((noise * 0.5 + 0.5) * size.z as f64) as i64;

                    voxels.push(Block::new(
                        if k <= height {
                            if k < height - 4 {
                                BlockMaterial::Stone
                            } else if k < height {
                                BlockMaterial::Earth
                            } else if k <= size.z / 3 + 3 {
                                BlockMaterial::Sand
                            } else if k > (size.z * 2) / 3 {
                                BlockMaterial::Stone
                            } else {
                                BlockMaterial::Grass
                            }
                        } else {
                            if k <= size.z / 3 {
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
            offset,
            voxels,
        }
    }

    fn pos_to_index(&self, pos: Vec3<i64>) -> usize {
        (pos.x * self.size.y * self.size.z + pos.y * self.size.z + pos.z) as usize
    }
}

impl Volume for Chunk {
    type VoxelType = Block;

    fn new() -> Self {
        Chunk {
            size: Vec3::from((0, 0, 0)),
            offset: Vec3::from((0, 0, 0)),
            voxels: Vec::new(),
        }
    }

    fn fill(&mut self, block: Block) {
        for v in self.voxels.iter_mut() {
            *v = block;
        };
    }

    fn size(&self) -> Vec3<i64> {
        self.size
    }

    fn offset(&self) -> Vec3<i64> {
        self.offset
    }

    fn rotation(&self) -> Vec3<f64> {
        Vec3::new(0.0, 0.0, 0.0)
    }

    fn scale(&self) -> Vec3<f64> {
        Vec3::new(1.0, 1.0, 1.0)
    }

    fn set_size(&mut self, size: Vec3<i64>) {
        self.size = size;
        self.voxels.resize((size.x * size.y * size.z) as usize, Block::new(BlockMaterial::Air));
    }

    fn set_offset(&mut self, offset: Vec3<i64>) {
        self.offset = offset;
    }

    fn at(&self, pos: Vec3<i64>) -> Option<Block> {
        if pos.x < 0 || pos.y < 0 || pos.z < 0 ||
            pos.x >= self.size.x || pos.y >= self.size.y || pos.z >= self.size.z
        {
            None
        } else {
            Some(self.voxels[self.pos_to_index(pos)])
        }
    }

    fn set(&mut self, pos: Vec3<i64>, vt: Block) {
        if pos.x < 0 || pos.y < 0 || pos.z < 0 ||
            pos.x >= self.size.x || pos.y >= self.size.y || pos.z >= self.size.z
        {
        } else {
            let i = self.pos_to_index(pos);
            self.voxels[i] = vt;
        }
    }
}
