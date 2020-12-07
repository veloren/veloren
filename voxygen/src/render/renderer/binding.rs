use super::{
    super::{
        pipelines::{
            figure, fluid, lod_terrain, sprite, terrain, ui, ColLights, GlobalModel,
            GlobalsBindGroup,
        },
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
            None => (&self.dummy_shadow_cube_tex, &self.dummy_shadow_tex),
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

    pub fn create_figure_bound_locals(
        &mut self,
        locals: &[figure::Locals],
        bone_data: &[figure::BoneData],
    ) -> figure::BoundLocals {
        let locals = self.create_consts(locals);
        let bone_data = self.create_consts(bone_data);
        self.layouts
            .figure
            .bind_locals(&self.device, locals, bone_data)
    }

    pub fn create_terrain_bound_locals(
        &mut self,
        locals: &[terrain::Locals],
    ) -> terrain::BoundLocals {
        let locals = self.create_consts(locals);
        self.layouts.terrain.bind_locals(&self.device, locals)
    }

    pub fn create_sprite_bound_locals(&mut self, locals: &[sprite::Locals]) -> sprite::BoundLocals {
        let locals = self.create_consts(locals);
        self.layouts.sprite.bind_locals(&self.device, locals)
    }

    pub fn figure_bind_col_light(&self, col_light: Texture) -> ColLights<figure::Locals> {
        self.layouts.global.bind_col_light(&self.device, col_light)
    }

    pub fn terrain_bind_col_light(&self, col_light: Texture) -> ColLights<terrain::Locals> {
        self.layouts.global.bind_col_light(&self.device, col_light)
    }

    pub fn sprite_bind_col_light(&self, col_light: Texture) -> ColLights<sprite::Locals> {
        self.layouts.global.bind_col_light(&self.device, col_light)
    }

    pub fn fluid_bind_waves(&self, texture: Texture) -> fluid::BindGroup {
        self.layouts.fluid.bind(&self.device, texture)
    }
}
