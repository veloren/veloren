// Standard
use std::marker::PhantomData;

// Local
use super::Pipeline;

/// Represents a mesh that has been sent to the CPU
pub struct Model<P: Pipeline> {
    phantom: PhantomData<P>,
}

impl<P: Pipeline> Model<P> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}
