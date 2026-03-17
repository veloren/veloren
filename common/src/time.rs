use serde::Deserialize;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Hash)]
pub enum DayPeriod {
    Night,
    Morning,
    Noon,
    Evening,
}

impl From<f64> for DayPeriod {
    fn from(time_of_day: f64) -> Self {
        let tod = time_of_day.rem_euclid(60.0 * 60.0 * 24.0);
        if tod < 60.0 * 60.0 * 6.0 {
            DayPeriod::Night
        } else if tod < 60.0 * 60.0 * 11.0 {
            DayPeriod::Morning
        } else if tod < 60.0 * 60.0 * 16.0 {
            DayPeriod::Noon
        } else if tod < 60.0 * 60.0 * 19.0 {
            DayPeriod::Evening
        } else {
            DayPeriod::Night
        }
    }
}

impl DayPeriod {
    pub fn is_dark(&self) -> bool { *self == DayPeriod::Night }

    pub fn is_light(&self) -> bool { !self.is_dark() }
}

pub const DAYS_IN_MONTH: f64 = 40.0;

/// A value ranging 0.0..1.0, to indicate the orbit period of the moon.
pub struct MoonPeriod(pub f64);

impl From<f64> for MoonPeriod {
    fn from(value: f64) -> Self { Self((value / (crate::resources::DAY * DAYS_IN_MONTH)).fract()) }
}
