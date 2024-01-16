use crate::render::{
    GlobalModel, Globals, GlobalsBindGroup, Light, LodData, PointLightMatrix, RainOcclusionLocals,
    Renderer, Shadow, ShadowLocals,
};

pub struct Scene {
    bind_group: GlobalsBindGroup,
}

impl Scene {
    pub fn new(renderer: &mut Renderer) -> Self {
        let global_data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),
            lights: renderer.create_consts(&[Light::default(); 32]),
            shadows: renderer.create_consts(&[Shadow::default(); 32]),
            shadow_mats: renderer.create_shadow_bound_locals(&[ShadowLocals::default()]),
            rain_occlusion_mats: renderer
                .create_rain_occlusion_bound_locals(&[RainOcclusionLocals::default()]),
            point_light_matrices: Box::new([PointLightMatrix::default(); 126]),
        };

        let lod_data = LodData::dummy(renderer);

        let bind_group = renderer.bind_globals(&global_data, &lod_data);

        Self { bind_group }
    }

    pub fn global_bind_group(&self) -> &GlobalsBindGroup { &self.bind_group }
}
