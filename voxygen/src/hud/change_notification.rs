use serde::{Deserialize, Serialize};

use std::time::Duration;

/// Default initial alpha of a Notify
const NOTIF_START_ALPHA: f32 = 1.0;

/// Default time to live of a notify
const NOTIF_LIFETIME: f32 = 2.0;
/// Default fading time of a notify
const NOTIF_FADETIME: f32 = 1.5;

/// The reason this notification is being shown: the setting is being enabled or
/// disabled, or we are reminding the player of the current state.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum NotificationReason {
    Remind = 2,
    Enable = 1,
    #[serde(other)]
    Disable = 0,
}
/// A temporal, fading message that a setting
/// or other state was changed (probably by direct player input)
#[derive(Default)]
pub struct ChangeNotification {
    pub reason: Option<NotificationReason>,
    pub alpha: f32,
    lifetime: Duration,
    fadetime: Duration,
    initial_fadetime: Duration,
}

impl ChangeNotification {
    pub fn new(
        reason: Option<NotificationReason>,
        alpha: f32,
        lifetime: Duration,
        fadetime: Duration,
    ) -> Result<Self, Duration> {
        if fadetime.is_zero() {
            Err(fadetime)
        } else {
            Ok(Self {
                reason,
                alpha,
                lifetime,
                fadetime,
                initial_fadetime: fadetime,
            })
        }
    }

    pub fn from_reason(reason: NotificationReason) -> Self {
        ChangeNotification::new(
            Some(reason),
            NOTIF_START_ALPHA,
            Duration::from_secs_f32(NOTIF_LIFETIME),
            Duration::from_secs_f32(NOTIF_FADETIME),
        )
        .unwrap()
    }

    pub fn from_state(state: bool) -> Self {
        ChangeNotification::from_reason(match state {
            true => NotificationReason::Enable,
            false => NotificationReason::Disable,
        })
    }

    pub fn update(&mut self, dt: Duration) {
        if self.reason.is_some() {
            // Timer before fade
            if !self.lifetime.is_zero() {
                self.lifetime = self.lifetime.saturating_sub(dt);
            // Lifetime expired, start to fade
            } else if !self.fadetime.is_zero() {
                self.fadetime = self.fadetime.saturating_sub(dt);
                // alpha as elapsed duration fraction, multiply with this for nice fade curve
                self.alpha = self.fadetime.as_secs_f32() / self.initial_fadetime.as_secs_f32();
            // Done fading
            } else {
                self.reason = None;
            }
        }
    }
}
