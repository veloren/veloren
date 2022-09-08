use crate::{
    comp::{character_state::OutputEvents, CharacterState, Melee, MeleeConstructor, StateUpdate},
    event::LocalEvent,
    outcome::Outcome,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
        wielding,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How much energy is drained per second when charging
    pub energy_drain: f32,
    /// Energy cost per attack
    pub energy_cost: f32,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the weapon is swinging for
    pub swing_duration: Duration,
    /// At what fraction of the swing duration to apply the melee "hit"
    pub hit_timing: f32,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the melee attack to the frontend
    pub specifier: Option<FrontendSpecifier>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Checks what section a stage is in
    pub stage_section: StageSection,
    /// Timer for each stage
    pub timer: Duration,
    /// Whether the attack executed already
    pub exhausted: bool,
    /// How much the attack charged by
    pub charge_amount: f32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.7);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Charge => {
                if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                    && self.timer < self.static_data.charge_duration
                {
                    let charge = (self.timer.as_secs_f32()
                        / self.static_data.charge_duration.as_secs_f32())
                    .min(1.0);

                    // Charge the attack
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        charge_amount: charge,
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                {
                    // Maintains charge
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain as f32 * data.dt.0 / 5.0);
                } else {
                    // Transitions to swing
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Action,
                        timer: Duration::default(),
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer.as_millis() as f32
                    > self.static_data.hit_timing
                        * self.static_data.swing_duration.as_millis() as f32
                    && !self.exhausted
                {
                    // Swing
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        exhausted: true,
                        ..*self
                    });

                    let crit_data = get_crit_data(data, self.static_data.ability_info);
                    let buff_strength = get_buff_strength(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data
                            .melee_constructor
                            .handle_scaling(self.charge_amount)
                            .create_melee(crit_data, buff_strength),
                    );

                    if let Some(FrontendSpecifier::GroundCleave) = self.static_data.specifier {
                        // Send local event used for frontend shenanigans
                        output_events.emit_local(LocalEvent::CreateOutcome(Outcome::GroundSlam {
                            pos: data.pos.0
                                + *data.ori.look_dir()
                                    * (data.body.max_radius()
                                        + self.static_data.melee_constructor.range),
                        }));
                    }
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::ChargedMelee(Data {
                        stage_section: StageSection::Recover,
                        timer: Duration::default(),
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::ChargedMelee(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    update.character =
                        CharacterState::Wielding(wielding::Data { is_sneaking: false });
                    // Make sure attack component is removed
                    data.updater.remove::<Melee>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding(wielding::Data { is_sneaking: false });
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if !input_is_pressed(data, self.static_data.ability_info.input) {
            handle_state_interrupt(data, &mut update, false);
        }

        update
    }
}

/// Used to specify a particular effect for frontend purposes
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    GroundCleave,
}
