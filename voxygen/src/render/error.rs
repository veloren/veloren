/// Used to represent one of many possible errors that may be omitted by the
/// rendering subsystem.
#[derive(Debug)]
pub enum RenderError {
    RequestDeviceError(wgpu::RequestDeviceError),
    MappingError(wgpu::BufferAsyncError),
    SwapChainError(wgpu::SwapChainError),
    CustomError(String),
    CouldNotFindAdapter,
    ErrorInitializingCompiler,
    ShaderError(shaderc::Error)
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
