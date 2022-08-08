// impls of `InterpolatableComponent` on things defined in `common`, since
// `common_net` is downstream of `common`, and an `InterpolationSystem` that
// applies them
use super::InterpolatableComponent;
use common::comp::{Ori, Pos, Vel};
use specs::Component;
use tracing::warn;
use vek::ops::{Lerp, Slerp};

#[derive(Debug)]
pub struct InterpBuffer<T> {
    pub buf: [(f64, T); 4],
    pub i: usize,
}

impl<T: Clone> InterpBuffer<T> {
    pub fn new(x: T) -> Self {
        Self {
            buf: [
                (0.0, x.clone()),
                (0.0, x.clone()),
                (0.0, x.clone()),
                (0.0, x),
            ],
            i: 0,
        }
    }

    fn push(&mut self, time: f64, x: T) {
        let InterpBuffer {
            ref mut buf,
            ref mut i,
        } = self;
        *i += 1;
        *i %= buf.len();
        buf[*i] = (time, x);
    }

    fn force_update(&mut self, time: f64, x: T) {
        for i in 0..self.buf.len() {
            self.buf[i] = (time, x.clone());
        }
    }

    fn update(&mut self, time: f64, x: T, force_update: bool) {
        if force_update {
            self.force_update(time, x);
        } else {
            self.push(time, x);
        }
    }
}

impl<T: 'static + Send + Sync> Component for InterpBuffer<T> {
    type Storage = specs::VecStorage<Self>;
}

// 0 is pure physics, 1 is pure extrapolation
const PHYSICS_VS_EXTRAPOLATION_FACTOR: f32 = 0.1;
const POSITION_INTERP_SANITY: Option<f32> = None;
const VELOCITY_INTERP_SANITY: Option<f32> = None;
const ENABLE_POSITION_HERMITE: bool = false;

impl InterpolatableComponent for Pos {
    type InterpData = InterpBuffer<Pos>;
    type ReadData = InterpBuffer<Vel>;

    fn new_data(x: Self) -> Self::InterpData { InterpBuffer::new(x) }

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64, force_update: bool) {
        interp_data.update(time, *self, force_update);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, vel: &InterpBuffer<Vel>) -> Self {
        // lerp to test interface, do hermite spline later
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        if POSITION_INTERP_SANITY
            .map_or(false, |limit| p0.0.distance_squared(p1.0) > limit.powf(2.0))
        {
            warn!("position delta exceeded sanity check, clamping");
            return p1;
        }
        let (t0prime, m0) = vel.buf[(i + vel.buf.len() - 1) % vel.buf.len()];
        let (t1prime, m1) = vel.buf[i % vel.buf.len()];
        let t = (t2 - t0) / (t1 - t0);
        let mut out = if ENABLE_POSITION_HERMITE
            && ((t0 - t0prime).abs() < f64::EPSILON && (t1 - t1prime).abs() < f64::EPSILON)
        {
            let h00 = |t: f64| (2.0 * t.powf(3.0) - 3.0 * t.powf(2.0) + 1.0) as f32;
            let h10 = |t: f64| (t.powf(3.0) - 2.0 * t.powf(2.0) + t) as f32;
            let h01 = |t: f64| (-2.0 * t.powf(3.0) + 3.0 * t.powf(2.0)) as f32;
            let h11 = |t: f64| (t.powf(3.0) - t.powf(2.0)) as f32;
            let dt = (t1 - t0) as f32;
            h00(t) * p0.0 + h10(t) * dt * m0.0 + h01(t) * p1.0 + h11(t) * dt * m1.0
        } else {
            if ENABLE_POSITION_HERMITE {
                warn!(
                    "timestamps for pos and vel don't match ({:?}, {:?}), falling back to lerp",
                    interp_data, vel
                );
            }
            Lerp::lerp_unclamped(p0.0, p1.0, t as f32)
        };

        if out.map(|x| x.is_nan()).reduce_or() {
            warn!("interpolation output is nan: {}, {}, {:?}", t2, t, buf);
            out = p1.0;
        }

        Pos(Lerp::lerp(self.0, out, PHYSICS_VS_EXTRAPOLATION_FACTOR))
    }
}

impl InterpolatableComponent for Vel {
    type InterpData = InterpBuffer<Vel>;
    type ReadData = ();

    fn new_data(x: Self) -> Self::InterpData { InterpBuffer::new(x) }

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64, force_update: bool) {
        interp_data.update(time, *self, force_update);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, _: &()) -> Self {
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        if VELOCITY_INTERP_SANITY
            .map_or(false, |limit| p0.0.distance_squared(p1.0) > limit.powf(2.0))
        {
            warn!("velocity delta exceeded sanity check, clamping");
            return p1;
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

    fn new_data(x: Self) -> Self::InterpData { InterpBuffer::new(x) }

    fn update_component(&self, interp_data: &mut Self::InterpData, time: f64, force_update: bool) {
        interp_data.update(time, *self, force_update);
    }

    fn interpolate(self, interp_data: &Self::InterpData, t2: f64, _: &()) -> Self {
        let InterpBuffer { ref buf, ref i } = interp_data;
        let (t0, p0) = buf[(i + buf.len() - 1) % buf.len()];
        let (t1, p1) = buf[i % buf.len()];
        if (t1 - t0).abs() < f64::EPSILON {
            return self;
        }
        let lerp_factor = 1.0 + ((t2 - t1) / (t1 - t0)) as f32;
        let mut out = Slerp::slerp_unclamped(p0.to_quat(), p1.to_quat(), lerp_factor);
        if out.into_vec4().map(|x| x.is_nan()).reduce_or() {
            warn!(
                "interpolation output is nan: {}, {}, {:?}",
                t2, lerp_factor, buf
            );
            out = p1.to_quat();
        }

        Ori::new(Slerp::slerp(self.to_quat(), out, PHYSICS_VS_EXTRAPOLATION_FACTOR).normalized())
    }
}
