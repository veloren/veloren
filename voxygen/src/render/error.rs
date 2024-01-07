/// Used to represent one of many possible errors that may be omitted by the
/// rendering subsystem.
pub enum RenderError {
    RequestDeviceError(wgpu::RequestDeviceError),
    MappingError(wgpu::BufferAsyncError),
    SurfaceError(wgpu::SurfaceError),
    CustomError(String),
    CouldNotFindAdapter,
    ErrorInitializingCompiler,
    ShaderError(String, shaderc::Error),
    ProfilerCreationError(wgpu_profiler::CreationError),
}

use std::fmt;
impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestDeviceError(err) => {
                f.debug_tuple("RequestDeviceError").field(err).finish()
            },
            Self::MappingError(err) => f.debug_tuple("MappingError").field(err).finish(),
            Self::SurfaceError(err) => f
                .debug_tuple("SurfaceError")
                // Use Display formatting for this error since they have nice descriptions
                .field(&format!("{}", err))
                .finish(),
            Self::CustomError(err) => f.debug_tuple("CustomError").field(err).finish(),
            Self::CouldNotFindAdapter => f.debug_tuple("CouldNotFindAdapter").finish(),
            Self::ErrorInitializingCompiler => f.debug_tuple("ErrorInitializingCompiler").finish(),
            Self::ShaderError(shader_name, err) => write!(
                f,
                "\"{shader_name}\" shader failed to compile due to the following error: {err}",
            ),
            RenderError::ProfilerCreationError(err) => write!(f, "Profiler creation error: {err}"),
        }
    }
}

impl From<wgpu::RequestDeviceError> for RenderError {
    fn from(err: wgpu::RequestDeviceError) -> Self { Self::RequestDeviceError(err) }
}

impl From<wgpu::BufferAsyncError> for RenderError {
    fn from(err: wgpu::BufferAsyncError) -> Self { Self::MappingError(err) }
}

impl From<wgpu::SurfaceError> for RenderError {
    fn from(err: wgpu::SurfaceError) -> Self { Self::SurfaceError(err) }
}
impl From<(&str, shaderc::Error)> for RenderError {
    fn from((shader_name, err): (&str, shaderc::Error)) -> Self {
        Self::ShaderError(shader_name.into(), err)
    }
}
