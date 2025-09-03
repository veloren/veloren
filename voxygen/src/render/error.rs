/// Used to represent one of many possible errors that may be omitted by the
/// rendering subsystem.
pub enum RenderError {
    RequestDeviceError(wgpu::RequestDeviceError),
    MappingError(wgpu::BufferAsyncError),
    SurfaceError(wgpu::SurfaceError),
    CustomError(String),
    CouldNotFindAdapter,
    RequestAdapterError(wgpu::RequestAdapterError),
    ErrorInitializingCompiler,
    ShaderShaderCError(String, shaderc::Error),
    ShaderWgpuError(String, wgpu::Error),
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
            Self::RequestAdapterError(err) => f
                .debug_tuple("RequestAdapterError")
                // Use Display formatting for this error since they have nice descriptions
                .field(&err.to_string())
                .finish(),
            Self::ErrorInitializingCompiler => f.debug_tuple("ErrorInitializingCompiler").finish(),
            Self::ShaderShaderCError(shader_name, err) => write!(
                f,
                "\"{shader_name}\" shader failed to compile with shaderc due to the following \
                 error: {err}",
            ),
            Self::ShaderWgpuError(shader_name, err) => write!(
                f,
                "\"{shader_name}\" shader failed to compile with wgpu due to the following error: \
                 {err}",
            ),
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

impl From<wgpu::RequestAdapterError> for RenderError {
    fn from(err: wgpu::RequestAdapterError) -> Self { Self::RequestAdapterError(err) }
}

impl From<(&str, shaderc::Error)> for RenderError {
    fn from((shader_name, err): (&str, shaderc::Error)) -> Self {
        Self::ShaderShaderCError(shader_name.into(), err)
    }
}
