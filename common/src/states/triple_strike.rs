use crate::{
    comp::{StateUpdate, ToolData},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::{collections::VecDeque, time::Duration};

// In millis
const STAGE_DURATION: u64 = 600;

/// ### A sequence of 3 incrementally increasing attacks.
///
/// While holding down the `primary` button, perform a series of 3 attacks,
/// each one pushes the player forward as the character steps into the swings.
/// The player can let go of the left mouse button at any time
/// and stop their attacks by interrupting the attack animation.
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// The tool this state will read to handle damage, etc.
    pub tool: ToolData,
    /// `int` denoting what stage (of 3) the attack is in.
    pub stage: i8,
    /// How long current stage has been active
    pub stage_time_active: Duration,
    /// Whether current stage has exhausted its attack
    stage_exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            energy: *data.energy,
            character: *data.character,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        let new_stage_exhausted = self.stage_exhausted;
        let new_stage_time_active = self
            .stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        // If player stops holding input,
        if !data.inputs.primary.is_pressed() {
            attempt_wield(data, &mut update);
            return update;
        }

        if self.stage < 3 {
            if new_stage_time_active < Duration::from_millis(STAGE_DURATION / 3) {
                // Move player forward while in first third of each stage
                handle_move(data, &mut update);
            } else if new_stage_time_active > Duration::from_millis(STAGE_DURATION / 2)
                && !new_stage_exhausted
            {
                // Try to deal damage in second half of stage
                // TODO: deal damage
            }
        }

        update
    }
}
