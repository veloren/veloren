use crate::{render::Renderer, window::Window};
use vek::*;

// How the ui is scaled
pub enum ScaleMode {
    // Scale against physical size
    Absolute(f64),
    // Use the dpi factor provided by the windowing system (i.e. use logical size)
    DpiFactor,
    // Scale based on the window's physical size, but maintain aspect ratio of widgets
    // Contains width and height of the "default" window size (ie where there should be no scaling)
    RelativeToWindow(Vec2<f64>),
}

pub struct Scale {
    // Type of scaling to use
    mode: ScaleMode,
    // Current dpi factor
    dpi_factor: f64,
    // Current logical window size
    window_dims: Vec2<f64>,
}

impl Scale {
    pub fn new(window: &Window, mode: ScaleMode) -> Self {
        let window_dims = window.logical_size();
        let dpi_factor = window.renderer().get_resolution().x as f64 / window_dims.x;
        Scale {
            mode,
            dpi_factor,
            window_dims,
        }
    }
    // Change the scaling mode
    pub fn scaling_mode(&mut self, mode: ScaleMode) {
        self.mode = mode;
    }
    // Calculate factor to transform between logical coordinates and our scaled coordinates
    pub fn scale_factor_logical(&self) -> f64 {
        match self.mode {
            ScaleMode::Absolute(scale) => scale / self.dpi_factor,
            ScaleMode::DpiFactor => 1.0,
            ScaleMode::RelativeToWindow(dims) => {
                (self.window_dims.x / dims.x).min(self.window_dims.y / dims.y)
            }
        }
    }
    // Calculate factor to transform between physical coordinates and our scaled coordinates
    pub fn scale_factor_physical(&self) -> f64 {
        self.scale_factor_logical() * self.dpi_factor
    }
    // Updates internal window size (and/or dpi_factor)
    pub fn window_resized(&mut self, new_dims: Vec2<f64>, renderer: &Renderer) {
        self.dpi_factor = renderer.get_resolution().x as f64 / new_dims.x;
        self.window_dims = new_dims;
    }
    // Get scaled window size
    pub fn scaled_window_size(&self) -> Vec2<f64> {
        self.window_dims / self.scale_factor_logical()
    }
    // Transform point from logical to scaled coordinates
    pub fn scale_point(&self, point: Vec2<f64>) -> Vec2<f64> {
        point / self.scale_factor_logical()
    }
}
