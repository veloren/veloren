use crate::settings::ModerationSettings;
use authc::Uuid;
use censor::Censor;
use common::comp::AdminRole;
use hashbrown::HashMap;
use std::time::{Duration, Instant};
use tracing::info;

pub const MAX_BYTES_CHAT_MSG: usize = 256;

pub enum ActionNote {
    SpamWarn,
}

pub enum ActionErr {
    BannedWord,
    TooLong,
    SpamMuted(Duration),
}

pub struct AutoMod {
    settings: ModerationSettings,
    censor: Censor,
    players: HashMap<Uuid, PlayerState>,
}

impl AutoMod {
    pub fn new(settings: &ModerationSettings, banned_words: Vec<String>) -> Self {
        if settings.automod {
            info!(
                "Automod enabled, players{} will be subject to automated spam/content filters",
                if settings.admins_exempt {
                    ""
                } else {
                    " (and admins)"
                }
            );
        } else {
            info!("Automod disabled");
        }

        Self {
            settings: settings.clone(),
            censor: Censor::Custom(banned_words.into_iter().collect()),
            players: HashMap::default(),
        }
    }

    pub fn enabled(&self) -> bool { self.settings.automod }

    fn player_mut(&mut self, player: Uuid) -> &mut PlayerState {
        self.players.entry(player).or_default()
    }

    pub fn validate_chat_msg(
        &mut self,
        player: Uuid,
        role: Option<AdminRole>,
        now: Instant,
        msg: &str,
    ) -> Result<Option<ActionNote>, ActionErr> {
        // TODO: Consider using grapheme cluster count instead of size in bytes
        if msg.len() > MAX_BYTES_CHAT_MSG {
            Err(ActionErr::TooLong)
        } else if !self.settings.automod || (role.is_some() && self.settings.admins_exempt) {
            Ok(None)
        } else if self.censor.check(msg) {
            Err(ActionErr::BannedWord)
        } else {
            let volume = self.player_mut(player).enforce_message_volume(now);

            if let Some(until) = self.player_mut(player).muted_until {
                Err(ActionErr::SpamMuted(until.saturating_duration_since(now)))
            } else if volume > 0.75 {
                Ok(Some(ActionNote::SpamWarn))
            } else {
                Ok(None)
            }
        }
    }
}

/// The period, in seconds, over which chat volume should be tracked to detect
/// spam.
const CHAT_VOLUME_PERIOD: f32 = 30.0;
/// The maximum permitted average number of chat messages over the chat volume
/// period.
const MAX_AVG_MSG_PER_SECOND: f32 = 1.0 / 7.0; // No more than a message every 7 seconds on average
/// The period for which a player should be muted when they exceed the message
/// spam threshold.
const SPAM_MUTE_PERIOD: Duration = Duration::from_secs(180);

#[derive(Default)]
pub struct PlayerState {
    last_msg_time: Option<Instant>,
    /// The average number of messages per second over the last N seconds.
    chat_volume: f32,
    muted_until: Option<Instant>,
}

impl PlayerState {
    // 0.0 => message is permitted, nothing unusual
    // >=1.0 => message is not permitted, chat volume exceeded
    pub fn enforce_message_volume(&mut self, now: Instant) -> f32 {
        if self.muted_until.map_or(false, |u| u <= now) {
            self.muted_until = None;
        }

        if let Some(time_since_last) = self
            .last_msg_time
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
        {
            let time_proportion = (time_since_last / CHAT_VOLUME_PERIOD).min(1.0);
            self.chat_volume = self.chat_volume * (1.0 - time_proportion)
                + (1.0 / time_since_last) * time_proportion;
        } else {
            self.chat_volume = 0.0;
        }
        self.last_msg_time = Some(now);

        let min_level = 1.0 / CHAT_VOLUME_PERIOD;
        let max_level = MAX_AVG_MSG_PER_SECOND;

        let volume = ((self.chat_volume - min_level) / (max_level - min_level)).max(0.0);

        if volume > 1.0 && self.muted_until.is_none() {
            self.muted_until = now.checked_add(SPAM_MUTE_PERIOD);
        }

        volume
    }
}
