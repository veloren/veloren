use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
#[repr(u16)]
pub enum CalendarEvent {
    Christmas = 0,
    Halloween = 1,
    AprilFools = 2,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Calendar {
    events: Vec<CalendarEvent>,
}

impl Calendar {
    pub fn is_event(&self, event: CalendarEvent) -> bool { self.events.contains(&event) }

    pub fn events(&self) -> impl ExactSizeIterator<Item = &CalendarEvent> + '_ {
        self.events.iter()
    }

    pub fn from_events(events: Vec<CalendarEvent>) -> Self { Self { events } }

    pub fn from_tz(tz: Option<Tz>) -> Self {
        let mut this = Self::default();

        let now = match tz {
            Some(tz) => {
                let utc = Utc::now().naive_utc();
                DateTime::<Tz>::from_utc(utc, tz.offset_from_utc_datetime(&utc)).naive_local()
            },
            None => Local::now().naive_local(),
        };

        if now.month() == 12 && (20..=30).contains(&now.day()) {
            this.events.push(CalendarEvent::Christmas);
        }

        if now.month() == 10 && (24..=31).contains(&now.day()) {
            this.events.push(CalendarEvent::Halloween);
        }

        if now.month() == 4 && now.day() == 1 {
            this.events.push(CalendarEvent::AprilFools);
        }

        this
    }
}
