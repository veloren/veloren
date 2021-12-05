use serde::{Deserialize, Serialize};
use vek::*;

/// Type of scaling to use.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
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
    // Current pixel size of the window
    physical_resolution: Vec2<u32>,
    // TEMP
    extra_factor: f64,
}

impl Scale {
    pub fn new(
        physical_resolution: Vec2<u32>,
        scale_factor: f64,
        mode: ScaleMode,
        extra_factor: f64,
    ) -> Self {
        Scale {
            mode,
            scale_factor,
            physical_resolution,
            extra_factor,
        }
    }

    // Change the scaling mode.
    // Returns false if the mode matches the current mode
    pub fn set_scaling_mode(&mut self, mode: ScaleMode) -> bool {
        let old_mode = self.mode;
        self.mode = mode;
        old_mode != mode
    }

    // Get scaling mode transformed into absolute scaling
    pub fn scaling_mode_as_absolute(&self) -> ScaleMode {
        ScaleMode::Absolute(self.scale_factor_physical())
    }

    // Get scaling mode transformed to be relative to the window with the same
    // aspect ratio as the current window
    pub fn scaling_mode_as_relative(&self) -> ScaleMode {
        ScaleMode::RelativeToWindow(self.scaled_resolution())
    }

    /// Calculate factor to transform between physical coordinates and our
    /// scaled coordinates.
    /// Multiply by scaled coordinates to get the physical coordinates
    pub fn scale_factor_physical(&self) -> f64 {
        self.extra_factor
            * match self.mode {
                ScaleMode::Absolute(scale) => scale,
                ScaleMode::DpiFactor => 1.0 * self.scale_factor,
                ScaleMode::RelativeToWindow(dims) => (f64::from(self.physical_resolution.x)
                    / dims.x)
                    .min(f64::from(self.physical_resolution.y) / dims.y),
            }
    }

    /// Calculate factor to transform between logical coordinates and our scaled
    /// coordinates.
    /// Multiply by scaled coordinates to get the logical coordinates
    ///
    /// Used to scale coordinates from window events (e.g. the mouse cursor
    /// position)
    pub fn scale_factor_logical(&self) -> f64 { self.scale_factor_physical() / self.scale_factor }

    /// Updates window size
    /// Returns true if the value was changed
    pub fn surface_resized(&mut self, new_res: Vec2<u32>) -> bool {
        let old_res = self.physical_resolution;
        self.physical_resolution = new_res;
        old_res != self.physical_resolution
    }

    /// Updates scale factor
    /// Returns true if the value was changed
    pub fn scale_factor_changed(&mut self, scale_factor: f64) -> bool {
        let old_scale_factor = self.scale_factor;
        self.scale_factor = scale_factor;
        old_scale_factor != self.scale_factor
    }

    /// Get physical resolution.
    pub fn physical_resolution(&self) -> Vec2<u32> { self.physical_resolution }

    /// Get scaled window size.
    pub fn scaled_resolution(&self) -> Vec2<f64> {
        self.physical_resolution.map(f64::from) / self.scale_factor_physical()
    }

    // Transform point from logical to scaled coordinates.
    pub fn scale_point(&self, point: Vec2<f64>) -> Vec2<f64> { point / self.scale_factor_logical() }
}
