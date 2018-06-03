use nalgebra::Vector4;

use region::{Voxel, Volume, Block, BlockMaterial, Chunk};

pub trait RenderVoxel: Voxel {
    fn get_color(&self) -> Vector4<f32>;
    fn is_opaque(&self) -> bool;
}

pub trait RenderVolume: Volume
    where Self::VoxelType: RenderVoxel
{}

// Implementations for common structures

impl RenderVoxel for Block {
    fn get_color(&self) -> Vector4<f32> {
        let color_map = enum_map! {
            BlockMaterial::Air => Vector4::new(0.0, 0.0, 0.0, 0.0),
            BlockMaterial::Grass => Vector4::new(0.0, 1.0, 0.0, 1.0),
            BlockMaterial::Stone => Vector4::new(0.5, 0.5, 0.5, 1.0),
        };

        color_map[self.material()]
    }

    fn is_opaque(&self) -> bool {
        self.material() != BlockMaterial::Air
    }
}

impl RenderVolume for Chunk {}
