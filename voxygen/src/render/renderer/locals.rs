use super::{
    super::{
        consts::Consts,
        pipelines::{clouds, postprocess},
    },
    Layouts,
};

pub struct Locals {
    pub clouds: Consts<clouds::Locals>,
    pub clouds_bind: clouds::BindGroup,

    pub postprocess: Consts<postprocess::Locals>,
    pub postprocess_bind: postprocess::BindGroup,
}

impl Locals {
    pub(super) fn new(
        device: &wgpu::Device,
        layouts: &Layouts,
        clouds_locals: Consts<clouds::Locals>,
        postprocess_locals: Consts<postprocess::Locals>,
        tgt_color_view: &wgpu::TextureView,
        tgt_depth_view: &wgpu::TextureView,
        tgt_color_pp_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        depth_sampler: &wgpu::Sampler,
    ) -> Self {
        let clouds_bind = layouts.clouds.bind(
            device,
            tgt_color_view,
            tgt_depth_view,
            sampler,
            depth_sampler,
            &clouds_locals,
        );
        let postprocess_bind =
            layouts
                .postprocess
                .bind(device, tgt_color_pp_view, sampler, &postprocess_locals);

        Self {
            clouds: clouds_locals,
            clouds_bind,
            postprocess: postprocess_locals,
            postprocess_bind,
        }
    }

    pub(super) fn rebind(
        &mut self,
        device: &wgpu::Device,
        layouts: &Layouts,
        // Call when these are recreated and need to be rebound
        // e.g. resizing
        tgt_color_view: &wgpu::TextureView,
        tgt_depth_view: &wgpu::TextureView,
        tgt_color_pp_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        depth_sampler: &wgpu::Sampler,
    ) {
        self.clouds_bind = layouts.clouds.bind(
            device,
            tgt_color_view,
            tgt_depth_view,
            sampler,
            depth_sampler,
            &self.clouds,
        );
        self.postprocess_bind =
            layouts
                .postprocess
                .bind(device, tgt_color_pp_view, sampler, &self.postprocess);
    }
}
