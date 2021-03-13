// impls of `InterpolatableComponent` on things defined in `common`, since
// `common_net` is downstream of `common`, and an `InterpolationSystem` that
// applies them
use super::InterpolatableComponent;
use common::comp::{Ori, Pos, Vel};
use specs::Component;
use specs_idvs::IdvStorage;
use tracing::warn;
use vek::ops::{Lerp, Slerp};

#[derive(Debug, Default)]
pub struct InterpBuffer<T> {
    pub buf: [(f64, T); 4],
    pub i: usize,
}

impl<T> InterpBuffer<T> {
    fn push(&mut self, time: f64, x: T) {
        let InterpBuffer {
            ref mut buf,
            ref mut i,
        } = self;
        *i += 1;
        *i %= buf.len();
        buf[*i] = (time, x);
    }
}

impl<T: 'static + Send + Sync> Component for InterpBuffer<T> {
    type Storage = IdvStorage<Self>;
}

// 0 is pure physics, 1 is pure extrapolation
const PHYSICS_VS_EXTRAPOLATION_FACTOR: f32 = 0.2;

impl InterpolatableComponent for Pos {
    type InterpData = InterpBuffer<Pos>;
    type ReadData = Vel;

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64) {
        interp_data.push(time, *self);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, _vel: &Vel) -> Self {
        // lerp to test interface, do hermite spline later
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        let lerp_factor = 1.0 + ((t2 - t1) / (t1 - t0)) as f32;
        let mut out = Lerp::lerp_unclamped(p0.0, p1.0, lerp_factor);
        if out.map(|x| x.is_nan()).reduce_or() {
            warn!(
                "interpolation output is nan: {}, {}, {:?}",
                t2, lerp_factor, buf
            );
            out = p1.0;
        }

        Pos(Lerp::lerp(self.0, out, PHYSICS_VS_EXTRAPOLATION_FACTOR))
    }
}

impl InterpolatableComponent for Vel {
    type InterpData = InterpBuffer<Vel>;
    type ReadData = ();

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64) {
        interp_data.push(time, *self);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, _: &()) -> Self {
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        let lerp_factor = 1.0 + ((t2 - t1) / (t1 - t0)) as f32;
        let mut out = Lerp::lerp_unclamped(p0.0, p1.0, lerp_factor);
        if out.map(|x| x.is_nan()).reduce_or() {
            warn!(
                "interpolation output is nan: {}, {}, {:?}",
                t2, lerp_factor, buf
            );
            out = p1.0;
        }

        Vel(Lerp::lerp(self.0, out, PHYSICS_VS_EXTRAPOLATION_FACTOR))
    }
}

impl InterpolatableComponent for Ori {
    type InterpData = InterpBuffer<Ori>;
    type ReadData = ();

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64) {
        interp_data.push(time, *self);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, _: &()) -> Self {
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        let lerp_factor = 1.0 + ((t2 - t1) / (t1 - t0)) as f32;
        let mut out = Slerp::slerp_unclamped(p0.0, p1.0, lerp_factor);
        if out.into_vec4().map(|x| x.is_nan()).reduce_or() {
            warn!(
                "interpolation output is nan: {}, {}, {:?}",
                t2, lerp_factor, buf
            );
            out = p1.0;
        }

        Ori(Slerp::slerp(self.0, out, PHYSICS_VS_EXTRAPOLATION_FACTOR).normalized())
    }
}
