use crate::window::Window;
use serde::{Deserialize, Serialize};
use vek::*;

/// Type of scaling to use.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ScaleMode {
    // Scale against physical size.
    Absolute(f64),
    // Scale based on the window's physical size, but maintain aspect ratio of widgets.
    // Contains width and height of the "default" window size (ie where there should be no
    // scaling).
    RelativeToWindow(Vec2<f64>),
    // Use the dpi factor provided by the windowing system (i.e. use logical size).
    #[serde(other)]
    // Would be `RelativeToWindow([1920.0, 1080.0].into())`, but only supported on unit variants
    DpiFactor,
}

#[derive(Clone, Copy)]
pub struct Scale {
    mode: ScaleMode,
    // Current dpi factor
    scale_factor: f64,
    // Current logical window size
    window_dims: Vec2<f64>,
    // TEMP
    extra_factor: f64,
}

impl Scale {
    pub fn new(window: &Window, mode: ScaleMode, extra_factor: f64) -> Self {
        let window_dims = window.logical_size();
        let scale_factor = window.scale_factor();
        Scale {
            mode,
            scale_factor,
            window_dims,
            extra_factor,
        }
    }

    // Change the scaling mode.
    pub fn set_scaling_mode(&mut self, mode: ScaleMode) { self.mode = mode; }

    // Get scaling mode transformed into absolute scaling
    pub fn scaling_mode_as_absolute(&self) -> ScaleMode {
        ScaleMode::Absolute(self.scale_factor_physical())
    }

    // Get scaling mode transformed to be relative to the window with the same
    // aspect ratio as the current window
    pub fn scaling_mode_as_relative(&self) -> ScaleMode {
        let scale = self.scale_factor_logical();
        ScaleMode::RelativeToWindow(self.window_dims.map(|e| e / scale))
    }

    /// Calculate factor to transform between logical coordinates and our scaled
    /// coordinates.
    /// Multiply by scaled coordinates to get the logical coordinates
    pub fn scale_factor_logical(&self) -> f64 {
        self.extra_factor
            * match self.mode {
                ScaleMode::Absolute(scale) => scale / self.scale_factor,
                ScaleMode::DpiFactor => 1.0,
                ScaleMode::RelativeToWindow(dims) => {
                    (self.window_dims.x / dims.x).min(self.window_dims.y / dims.y)
                },
            }
    }

    /// Calculate factor to transform between physical coordinates and our
    /// scaled coordinates.
    /// Multiply by scaled coordinates to get the physical coordinates
    pub fn scale_factor_physical(&self) -> f64 { self.scale_factor_logical() * self.scale_factor }

    /// Updates window size
    /// Returns true if the value was changed
    #[allow(clippy::float_cmp)]
    pub fn window_resized(&mut self, new_dims: Vec2<f64>) -> bool {
        let old_window_dims = self.window_dims;
        self.window_dims = new_dims;
        old_window_dims != self.window_dims
    }

    /// Updates scale factor
    /// Returns true if the value was changed
    #[allow(clippy::float_cmp)]
    pub fn scale_factor_changed(&mut self, scale_factor: f64) -> bool {
        let old_scale_factor = self.scale_factor;
        self.scale_factor = scale_factor;
        old_scale_factor != self.scale_factor
    }

    /// Get scaled window size.
    pub fn scaled_resolution(&self) -> Vec2<f64> { self.window_dims / self.scale_factor_logical() }

    /// Get logical window size
    pub fn logical_resolution(&self) -> Vec2<f64> { self.window_dims }

    /// Get physical window size
    pub fn physical_resolution(&self) -> Vec2<u16> {
        (self.window_dims * self.scale_factor).map(|e| e.round() as u16)
    }

    // Transform point from logical to scaled coordinates.
    pub fn scale_point(&self, point: Vec2<f64>) -> Vec2<f64> { point / self.scale_factor_logical() }
}
