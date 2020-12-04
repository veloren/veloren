use crate::render::{
    GlobalModel, Globals, GlobalsBindGroup, Light, LodData, Renderer, Shadow, ShadowLocals,
};

pub struct Scene {
    // global_data: GlobalModel,
    // lod_data: LodData,
    bind_group: GlobalsBindGroup,
}

impl Scene {
    pub fn new(renderer: &mut Renderer) -> Self {
        let global_data = GlobalModel {
            globals: renderer.create_consts(&[Globals::default()]),
            lights: renderer.create_consts(&[Light::default(); 32]),
            shadows: renderer.create_consts(&[Shadow::default(); 32]),
            shadow_mats: renderer.create_consts(&[ShadowLocals::default(); 6]),
        };

        let lod_data = LodData::dummy(renderer);

        let bind_group = renderer.bind_globals(&global_data, &lod_data);

        Self { bind_group }
    }

    pub fn global_bind_group(&self) -> &GlobalsBindGroup { &self.bind_group }
}
