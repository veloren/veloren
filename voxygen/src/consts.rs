// Library
use gfx::{
    handle::Buffer,
    traits::{FactoryExt, Pod},
};

// Crate
use crate::render_ctx::{RenderCtx, Resources};

type ConstBuffer<T> = Buffer<Resources, T>;

#[derive(Clone)]
pub struct ConstHandle<T: Copy + Pod> {
    buffer: ConstBuffer<T>,
}

impl<T: Copy + Pod> ConstHandle<T> {
    pub fn new(render_ctx: &mut RenderCtx) -> Self {
        Self {
            buffer: render_ctx
                .factory_mut()
                .create_constant_buffer(1),
        }
    }

    pub fn update(&self, render_ctx: &mut RenderCtx, consts: T) {
        render_ctx
            .encoder_mut()
            .update_buffer(&self.buffer, &[consts], 0)
            .unwrap();
    }

    pub fn buffer(&self) -> &ConstBuffer<T> { &self.buffer }
}
