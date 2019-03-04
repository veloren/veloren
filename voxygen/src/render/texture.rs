// Standard
use std::marker::PhantomData;

// Library
use gfx::{
    self,
    traits::Factory,
};
use image::{
    DynamicImage,
    GenericImageView,
};

// Local
use super::{
    RenderError,
    Pipeline,
    gfx_backend,
};

type ShaderFormat = (gfx::format::R8_G8_B8_A8, gfx::format::Srgb);

/// Represents an image that has been uploaded to the GPU.
pub struct Texture<P: Pipeline> {
    pub tex: gfx::handle::Texture<gfx_backend::Resources, <ShaderFormat as gfx::format::Formatted>::Surface>,
    pub srv: gfx::handle::ShaderResourceView<gfx_backend::Resources, <ShaderFormat as gfx::format::Formatted>::View>,
    pub sampler: gfx::handle::Sampler<gfx_backend::Resources>,
    _phantom: PhantomData<P>,
}

impl<P: Pipeline> Texture<P> {
    pub fn new(
        factory: &mut gfx_backend::Factory,
        image: &DynamicImage,
    ) -> Result<Self, RenderError> {
        let (tex, srv) = factory.create_texture_immutable_u8::<ShaderFormat>(
            gfx::texture::Kind::D2(
                image.width() as u16,
                image.height() as u16,
                gfx::texture::AaMode::Single,
            ),
            gfx::texture::Mipmap::Provided,
            &[&image.to_rgba().into_raw()],
        )
            .map_err(|err| RenderError::CombinedError(err))?;

        Ok(Self {
            tex,
            srv,
            sampler: factory.create_sampler(gfx::texture::SamplerInfo::new(
                gfx::texture::FilterMethod::Scale,
                gfx::texture::WrapMode::Clamp,
            )),
            _phantom: PhantomData,
        })
    }
}
