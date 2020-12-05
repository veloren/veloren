use super::{
    super::{
        consts::Consts,
        pipelines::{lod_terrain, ui, GlobalModel, GlobalsBindGroup},
        texture::Texture,
    },
    Renderer,
};

impl Renderer {
    // TODO: rework this to use the Bound type?
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

    pub fn create_ui_bound_locals(&mut self, vals: &[ui::Locals]) -> ui::BoundLocals {
        let locals = self.create_consts(vals);
        self.layouts.ui.bind_locals(&self.device, locals)
    }

    pub fn ui_bind_texture(&self, texture: &Texture) -> ui::TextureBindGroup {
        self.layouts.ui.bind_texture(&self.device, texture)
    }
}
