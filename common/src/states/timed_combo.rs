use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate, ToolData},
    states::wielding,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::{collections::VecDeque, time::Duration};
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// Denotes what stage (of 3) the attack is in
    pub stage: i8,
    /// Whether current stage has exhausted its attack
    pub stage_exhausted: bool,
    /// How long state waits before it should deal damage
    pub buildup_duration: Duration,
    /// How long the state waits until exiting
    pub recover_duration: Duration,
    /// Tracks how long current stage has been active
    pub stage_time_active: Duration,
    /// `ToolData` to be sent to `Attacking` component
    pub tool: ToolData,
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

        let new_stage_time_active = self
            .stage_time_active
            .checked_add(Duration::from_secs_f32(data.dt.0))
            .unwrap_or(Duration::default());

        if self.stage < 3 {
            // Build up window
            if new_stage_time_active < self.buildup_duration {
                // If the player is pressing primary btn
                if data.inputs.primary.is_just_pressed() {
                    println!("Failed");
                    // They failed, go back to `Wielding`
                    update.character = CharacterState::Wielding(wielding::Data { tool: self.tool });
                }
                // Keep updating
                else {
                    update.character = CharacterState::TimedCombo(Data {
                        tool: self.tool,
                        stage: self.stage,
                        buildup_duration: self.buildup_duration,
                        recover_duration: self.recover_duration,
                        stage_exhausted: false,
                        stage_time_active: new_stage_time_active,
                    });
                }
            }
            // Hit attempt window
            else if !self.stage_exhausted {
                // Swing hits
                data.updater.insert(data.entity, Attacking {
                    weapon: Some(self.tool),
                    applied: false,
                    hit_count: 0,
                });

                update.character = CharacterState::TimedCombo(Data {
                    tool: self.tool,
                    stage: self.stage,
                    buildup_duration: self.buildup_duration,
                    recover_duration: self.recover_duration,
                    stage_exhausted: true,
                    stage_time_active: new_stage_time_active,
                });
            }
            // Swing recovery window
            else if new_stage_time_active
                < self
                    .buildup_duration
                    .checked_add(self.recover_duration)
                    .unwrap_or(Duration::default())
            {
                // Try to transition to next stage
                if data.inputs.primary.is_just_pressed() {
                    println!("Transition");
                    update.character = CharacterState::TimedCombo(Data {
                        tool: self.tool,
                        stage: self.stage + 1,
                        buildup_duration: self.buildup_duration,
                        recover_duration: self.recover_duration,
                        stage_exhausted: true,
                        stage_time_active: Duration::default(),
                    });
                }
                // Player didn't click this frame
                else {
                    // Update state
                    println!("Missed");
                    update.character = CharacterState::TimedCombo(Data {
                        tool: self.tool,
                        stage: self.stage,
                        buildup_duration: self.buildup_duration,
                        recover_duration: self.recover_duration,
                        stage_exhausted: true,
                        stage_time_active: new_stage_time_active,
                    });
                }
            }
            // Stage expired but missed transition to next stage
            else {
                // Back to `Wielding`
                update.character = CharacterState::Wielding(wielding::Data { tool: self.tool });
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
            }
        }
        // Made three successful hits!
        else {
            println!("Success!");
            // Back to `Wielding`
            update.character = CharacterState::Wielding(wielding::Data { tool: self.tool });
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        // Subtract energy on successful hit
        if let Some(attack) = data.attacking {
            if attack.applied && attack.hit_count > 0 {
                println!("Hit");
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(100, EnergySource::HitEnemy);
            }
        }

        update
    }
}
