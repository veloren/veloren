use enum_map::EnumMap;

use Voxel;

#[repr(u16)]
#[derive(Copy, Clone, PartialEq, EnumMap)]
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

    fn new(mat: Self::Material) -> Self {
        Block {
            mat,
        }
    }

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
