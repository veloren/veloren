pub struct Bound<T> {
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) with: T,
}

impl<T> std::ops::Deref for Bound<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target { &self.with }
}

impl<T> std::ops::DerefMut for Bound<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.with }
}
