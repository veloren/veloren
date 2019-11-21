/// Used to represent one of many possible errors that may be omitted by the
/// rendering subsystem.
#[derive(Debug)]
pub enum RenderError {
    PipelineError(gfx::PipelineStateError<String>),
    UpdateError(gfx::UpdateError<usize>),
    TexUpdateError(gfx::UpdateError<[u16; 3]>),
    CombinedError(gfx::CombinedError),
    BufferCreationError(gfx::buffer::CreationError),
    IncludeError(glsl_include::Error),
    MappingError(gfx::mapping::Error),
    CopyError(gfx::CopyError<[u16; 3], usize>),
}

impl From<gfx::PipelineStateError<String>> for RenderError {
    fn from(err: gfx::PipelineStateError<String>) -> Self { Self::PipelineError(err) }
}

impl From<gfx::PipelineStateError<&str>> for RenderError {
    fn from(err: gfx::PipelineStateError<&str>) -> Self {
        // This is horrid. We do it to get rid of the `&str`'s lifetime bound by turning it into a `String`.
        match err {
            gfx::PipelineStateError::DescriptorInit(err) => {
                gfx::PipelineStateError::DescriptorInit(match err {
                    gfx::pso::InitError::VertexImport(s, x) => {
                        gfx::pso::InitError::VertexImport(s.to_string(), x)
                    }
                    gfx::pso::InitError::ConstantBuffer(s, x) => {
                        gfx::pso::InitError::ConstantBuffer(
                            s.to_string(),
                            x.map(|x| match x {
                                gfx::pso::ElementError::NotFound(s) => {
                                    gfx::pso::ElementError::NotFound(s.to_string())
                                }
                                gfx::pso::ElementError::Offset {
                                    name,
                                    shader_offset,
                                    code_offset,
                                } => gfx::pso::ElementError::Offset {
                                    name: name.to_string(),
                                    shader_offset,
                                    code_offset,
                                },
                                gfx::pso::ElementError::Format {
                                    name,
                                    shader_format,
                                    code_format,
                                } => gfx::pso::ElementError::Format {
                                    name: name.to_string(),
                                    shader_format,
                                    code_format,
                                },
                            }),
                        )
                    }
                    gfx::pso::InitError::GlobalConstant(s, x) => {
                        gfx::pso::InitError::GlobalConstant(s.to_string(), x)
                    }
                    gfx::pso::InitError::ResourceView(s, x) => {
                        gfx::pso::InitError::ResourceView(s.to_string(), x)
                    }
                    gfx::pso::InitError::UnorderedView(s, x) => {
                        gfx::pso::InitError::UnorderedView(s.to_string(), x)
                    }
                    gfx::pso::InitError::Sampler(s, x) => {
                        gfx::pso::InitError::Sampler(s.to_string(), x)
                    }
                    gfx::pso::InitError::PixelExport(s, x) => {
                        gfx::pso::InitError::PixelExport(s.to_string(), x)
                    }
                })
            }
            gfx::PipelineStateError::Program(p) => gfx::PipelineStateError::Program(p),
            gfx::PipelineStateError::DeviceCreate(c) => gfx::PipelineStateError::DeviceCreate(c),
        }
        .into()
    }
}
impl From<gfx::shade::ProgramError> for RenderError {
    fn from(err: gfx::shade::ProgramError) -> Self {
        gfx::PipelineStateError::<String>::Program(err).into()
    }
}
impl From<gfx::UpdateError<usize>> for RenderError {
    fn from(err: gfx::UpdateError<usize>) -> Self { Self::UpdateError(err) }
}

impl From<gfx::UpdateError<[u16; 3]>> for RenderError {
    fn from(err: gfx::UpdateError<[u16; 3]>) -> Self { Self::TexUpdateError(err) }
}

impl From<gfx::CombinedError> for RenderError {
    fn from(err: gfx::CombinedError) -> Self { Self::CombinedError(err) }
}

impl From<gfx::TargetViewError> for RenderError {
    fn from(err: gfx::TargetViewError) -> Self { Self::CombinedError(err.into()) }
}

impl From<gfx::ResourceViewError> for RenderError {
    fn from(err: gfx::ResourceViewError) -> Self { Self::CombinedError(err.into()) }
}

impl From<gfx::texture::CreationError> for RenderError {
    fn from(err: gfx::texture::CreationError) -> Self { Self::CombinedError(err.into()) }
}

impl From<gfx::buffer::CreationError> for RenderError {
    fn from(err: gfx::buffer::CreationError) -> Self { Self::BufferCreationError(err) }
}

impl From<glsl_include::Error> for RenderError {
    fn from(err: glsl_include::Error) -> Self { Self::IncludeError(err) }
}

impl From<gfx::mapping::Error> for RenderError {
    fn from(err: gfx::mapping::Error) -> Self { Self::MappingError(err) }
}

impl From<gfx::CopyError<[u16; 3], usize>> for RenderError {
    fn from(err: gfx::CopyError<[u16; 3], usize>) -> Self { Self::CopyError(err) }
}
