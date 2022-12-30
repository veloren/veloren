use super::{
    super::{
        consts::Consts,
        pipelines::{bloom, clouds, postprocess},
    },
    Layouts,
};

pub struct BloomParams<'a> {
    pub locals: [Consts<bloom::Locals>; bloom::NUM_SIZES],
    pub src_views: [&'a wgpu::TextureView; bloom::NUM_SIZES],
    pub final_tgt_view: &'a wgpu::TextureView,
}

pub struct Locals {
    pub clouds: Consts<clouds::Locals>,
    pub clouds_bind: clouds::BindGroup,

    pub bloom_binds: Option<[bloom::BindGroup; bloom::NUM_SIZES]>,

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
        tgt_mat_view: &wgpu::TextureView,
        tgt_depth_view: &wgpu::TextureView,
        bloom: Option<BloomParams>,
        tgt_color_pp_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        depth_sampler: &wgpu::Sampler,
    ) -> Self {
        let clouds_bind = layouts.clouds.bind(
            device,
            tgt_color_view,
            tgt_mat_view,
            tgt_depth_view,
            sampler,
            depth_sampler,
            &clouds_locals,
        );

        let postprocess_bind = layouts.postprocess.bind(
            device,
            tgt_color_pp_view,
            tgt_depth_view,
            bloom.as_ref().map(|b| b.final_tgt_view),
            sampler,
            depth_sampler,
            &postprocess_locals,
        );

        let bloom_binds = bloom.map(|bloom| {
            bloom
                .src_views
                .zip(bloom.locals) // zip arrays
                .map(|(view, locals)| layouts.bloom.bind(device, view, sampler, locals))
        });

        Self {
            clouds: clouds_locals,
            clouds_bind,
            bloom_binds,
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
        tgt_mat_view: &wgpu::TextureView,
        tgt_depth_view: &wgpu::TextureView,
        bloom: Option<BloomParams>,
        tgt_color_pp_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        depth_sampler: &wgpu::Sampler,
    ) {
        self.clouds_bind = layouts.clouds.bind(
            device,
            tgt_color_view,
            tgt_mat_view,
            tgt_depth_view,
            sampler,
            depth_sampler,
            &self.clouds,
        );
        self.postprocess_bind = layouts.postprocess.bind(
            device,
            tgt_color_pp_view,
            tgt_depth_view,
            bloom.as_ref().map(|b| b.final_tgt_view),
            sampler,
            depth_sampler,
            &self.postprocess,
        );
        self.bloom_binds = bloom.map(|bloom| {
            bloom
                .src_views
                .zip(bloom.locals) // zip arrays
                .map(|(view, locals)| layouts.bloom.bind(device, view, sampler, locals))
        });
    }
}
