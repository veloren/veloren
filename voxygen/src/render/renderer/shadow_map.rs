use super::super::{pipelines::shadow, texture::Texture};

/// A type that holds shadow map data.  Since shadow mapping may not be
/// supported on all platforms, we try to keep it separate.
pub struct ShadowMapRenderer {
    // directed_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    // point_encoder: gfx::Encoder<gfx_backend::Resources, gfx_backend::CommandBuffer>,
    pub directed_depth: Texture,

    pub point_depth: Texture,

    pub point_pipeline: shadow::PointShadowPipeline,
    pub terrain_directed_pipeline: shadow::ShadowPipeline,
    pub figure_directed_pipeline: shadow::ShadowFigurePipeline,
    pub layout: shadow::ShadowLayout,
}

pub enum ShadowMap {
    Enabled(ShadowMapRenderer),
    Disabled {
        dummy_point: Texture, // Cube texture
        dummy_directed: Texture,
    },
}

impl ShadowMap {
    pub fn textures(&self) -> (&Texture, &Texture) {
        match self {
            Self::Enabled(renderer) => (&renderer.point_depth, &renderer.directed_depth),
            Self::Disabled {
                dummy_point,
                dummy_directed,
            } => (dummy_point, dummy_directed),
        }
    }

    pub fn is_enabled(&self) -> bool { matches!(self, Self::Enabled(_)) }
}
