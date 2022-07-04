use crate::render::pipelines::rain_occlusion;

use super::{
    super::{
        pipelines::{
            debug, figure, lod_terrain, shadow, sprite, terrain, ui, ColLights, GlobalModel,
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
        self.layouts
            .global
            .bind(&self.device, global_model, lod_data, &self.noise_tex)
    }

    pub fn bind_sprite_globals(
        &self,
        global_model: &GlobalModel,
        lod_data: &lod_terrain::LodData,
        sprite_verts: &sprite::SpriteVerts,
    ) -> sprite::SpriteGlobalsBindGroup {
        self.layouts.sprite.bind_globals(
            &self.device,
            global_model,
            lod_data,
            &self.noise_tex,
            sprite_verts,
        )
    }

    pub fn create_debug_bound_locals(&mut self, vals: &[debug::Locals]) -> debug::BoundLocals {
        let locals = self.create_consts(vals);
        self.layouts.debug.bind_locals(&self.device, locals)
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

    pub fn create_shadow_bound_locals(&mut self, locals: &[shadow::Locals]) -> shadow::BoundLocals {
        let locals = self.create_consts(locals);
        self.layouts.shadow.bind_locals(&self.device, locals)
    }

    pub fn create_rain_occlusion_bound_locals(
        &mut self,
        locals: &[rain_occlusion::Locals],
    ) -> rain_occlusion::BoundLocals {
        let locals = self.create_consts(locals);
        self.layouts
            .rain_occlusion
            .bind_locals(&self.device, locals)
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
}
