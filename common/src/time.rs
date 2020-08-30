#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DayPeriod {
    Night,
    Morning,
    Noon,
    Evening,
}

impl DayPeriod {
    pub fn is_dark(&self) -> bool {
        *self == DayPeriod::Night
    }

    pub fn is_light(&self) -> bool {
        !self.is_dark()
    }
}
