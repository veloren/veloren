/// Used to represent one of many possible errors that may be omitted by the
/// rendering subsystem.
pub enum RenderError {
    RequestDeviceError(wgpu::RequestDeviceError),
    MappingError(wgpu::BufferAsyncError),
    SwapChainError(wgpu::SwapChainError),
    CustomError(String),
    CouldNotFindAdapter,
    ErrorInitializingCompiler,
    ShaderError(shaderc::Error),
}

use std::fmt;
impl fmt::Debug for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestDeviceError(err) => {
                f.debug_tuple("RequestDeviceError").field(err).finish()
            },
            Self::MappingError(err) => f.debug_tuple("MappingError").field(err).finish(),
            Self::SwapChainError(err) => f.debug_tuple("SwapChainError").field(err).finish(),
            Self::CustomError(err) => f.debug_tuple("CustomError").field(err).finish(),
            Self::CouldNotFindAdapter => f.debug_tuple("CouldNotFindAdapter").finish(),
            Self::ErrorInitializingCompiler => f.debug_tuple("ErrorInitializingCompiler").finish(),
            Self::ShaderError(err) => write!(f, "{}", err),
        }
    }
}

impl From<wgpu::RequestDeviceError> for RenderError {
    fn from(err: wgpu::RequestDeviceError) -> Self { Self::RequestDeviceError(err) }
}

impl From<wgpu::BufferAsyncError> for RenderError {
    fn from(err: wgpu::BufferAsyncError) -> Self { Self::MappingError(err) }
}

impl From<wgpu::SwapChainError> for RenderError {
    fn from(err: wgpu::SwapChainError) -> Self { Self::SwapChainError(err) }
}

impl From<shaderc::Error> for RenderError {
    fn from(err: shaderc::Error) -> Self { Self::ShaderError(err) }
}
