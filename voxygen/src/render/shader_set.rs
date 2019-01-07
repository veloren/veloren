// Standard
use std::marker::PhantomData;

// Local
use super::{
    Pipeline,
    RenderErr
};

pub struct ShaderSet<P: Pipeline> {
    phantom: PhantomData<P>,
}

impl<P: Pipeline> ShaderSet<P> {
    pub fn new(
        vs: &[u8],
        fs: &[u8],
    ) -> Result<Self, RenderErr> {
        Ok(Self {
            phantom: PhantomData,
        })
    }
}
