use super::{
    super::{
        consts::Consts,
        pipelines::{lod_terrain, ui, GlobalModel, GlobalsBindGroup},
        texture::Texture,
    },
    Renderer,
};

impl Renderer {
    pub fn bind_globals(
        &self,
        global_model: &GlobalModel,
        lod_data: &lod_terrain::LodData,
    ) -> GlobalsBindGroup {
        let (point_shadow_map, directed_shadow_map) = match &self.shadow_map {
            Some(shadow_map) => (&shadow_map.point_depth, &shadow_map.directed_depth),
            None => (&self.noise_tex, &self.noise_tex),
        };

        self.layouts.global.bind(
            &self.device,
            global_model,
            lod_data,
            &self.noise_tex,
            point_shadow_map,
            directed_shadow_map,
        )
    }

    pub fn ui_bind_locals(&self, locals: &Consts<ui::Locals>) -> ui::LocalsBindGroup {
        self.layouts.ui.bind_locals(&self.device, locals)
    }

    pub fn ui_bind_texture(&self, texture: &Texture) -> ui::TextureBindGroup {
        self.layouts.ui.bind_texture(&self.device, texture)
    }
}
