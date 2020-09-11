use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    /// Specifies which stage the combo attack is in
    pub stage: u32,
    /// Initial damage of stage
    pub base_damage: u32,
    /// Max damage of stage
    pub max_damage: u32,
    /// Damage scaling per combo
    pub damage_increase: u32,
    /// Knockback of stage
    pub knockback: f32,
    /// Range of attack
    pub range: f32,
    /// Angle of attack
    pub angle: f32,
    /// Initial buildup duration of stage (how long until state can deal damage)
    pub base_buildup_duration: Duration,
    /// Duration of stage spent in swing (controls animation stuff, and can also
    /// be used to handle movement separately to buildup)
    pub base_swing_duration: Duration,
    /// Initial recover duration of stage (how long until character exits state)
    pub base_recover_duration: Duration,
    /// How much forward movement there is in the swing portion of the stage
    pub forward_movement: f32,
}

/// A sequence of attacks that can incrementally become faster and more
/// damaging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Indicates what stage the combo is in
    pub stage: u32,
    /// Indicates number of stages in combo
    pub num_stages: u32,
    /// Number of consecutive strikes
    pub combo: u32,
    /// Data for first stage
    pub stage_data: Vec<Stage>,
    /// Initial energy gain per strike
    pub initial_energy_gain: u32,
    /// Max energy gain per strike
    pub max_energy_gain: u32,
    /// Energy gain increase per combo
    pub energy_increase: u32,
    /// Duration for the next stage to be activated
    pub combo_duration: Duration,
    /// Timer for each stage
    pub timer: Duration,
    /// Checks what section a stage is in
    pub stage_section: StageSection,
    /// Whether the state should go onto the next stage
    pub next_stage: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.1);

        let stage_index = (self.stage - 1) as usize;

        if self.stage_section == StageSection::Buildup
            && self.timer < self.stage_data[stage_index].base_buildup_duration
        {
            // Build up
            update.character = CharacterState::ComboMelee(Data {
                stage: self.stage,
                num_stages: self.num_stages,
                combo: self.combo,
                stage_data: self.stage_data.clone(),
                initial_energy_gain: self.initial_energy_gain,
                max_energy_gain: self.max_energy_gain,
                energy_increase: self.energy_increase,
                combo_duration: self.combo_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
                next_stage: self.next_stage,
            });
        } else if self.stage_section == StageSection::Buildup {
            // Transitions to swing section of stage
            update.character = CharacterState::ComboMelee(Data {
                stage: self.stage,
                num_stages: self.num_stages,
                combo: self.combo,
                stage_data: self.stage_data.clone(),
                initial_energy_gain: self.initial_energy_gain,
                max_energy_gain: self.max_energy_gain,
                energy_increase: self.energy_increase,
                combo_duration: self.combo_duration,
                timer: Duration::default(),
                stage_section: StageSection::Swing,
                next_stage: self.next_stage,
            });

            // Hit attempt
            data.updater.insert(data.entity, Attacking {
                base_healthchange: -((self.stage_data[stage_index].max_damage.min(
                    self.stage_data[stage_index].base_damage
                        + self.combo / self.num_stages
                            * self.stage_data[stage_index].damage_increase,
                )) as i32),
                range: self.stage_data[stage_index].range,
                max_angle: self.stage_data[stage_index].angle.to_radians(),
                applied: false,
                hit_count: 0,
                knockback: self.stage_data[stage_index].knockback,
            });
        } else if self.stage_section == StageSection::Swing
            && self.timer < self.stage_data[stage_index].base_swing_duration
        {
            // Forward movement
            forward_move(
                data,
                &mut update,
                0.1,
                self.stage_data[stage_index].forward_movement,
            );

            // Swings
            update.character = CharacterState::ComboMelee(Data {
                stage: self.stage,
                num_stages: self.num_stages,
                combo: self.combo,
                stage_data: self.stage_data.clone(),
                initial_energy_gain: self.initial_energy_gain,
                max_energy_gain: self.max_energy_gain,
                energy_increase: self.energy_increase,
                combo_duration: self.combo_duration,
                timer: self
                    .timer
                    .checked_add(Duration::from_secs_f32(data.dt.0))
                    .unwrap_or_default(),
                stage_section: self.stage_section,
                next_stage: self.next_stage,
            });
        } else if self.stage_section == StageSection::Swing {
            // Transitions to recover section of stage
            update.character = CharacterState::ComboMelee(Data {
                stage: self.stage,
                num_stages: self.num_stages,
                combo: self.combo,
                stage_data: self.stage_data.clone(),
                initial_energy_gain: self.initial_energy_gain,
                max_energy_gain: self.max_energy_gain,
                energy_increase: self.energy_increase,
                combo_duration: self.combo_duration,
                timer: Duration::default(),
                stage_section: StageSection::Recover,
                next_stage: self.next_stage,
            });
        } else if self.stage_section == StageSection::Recover
            && self.timer < self.stage_data[stage_index].base_recover_duration
        {
            // Recovers
            if data.inputs.primary.is_pressed() {
                // Checks if state will transition to next stage after recover
                update.character = CharacterState::ComboMelee(Data {
                    stage: self.stage,
                    num_stages: self.num_stages,
                    combo: self.combo,
                    stage_data: self.stage_data.clone(),
                    initial_energy_gain: self.initial_energy_gain,
                    max_energy_gain: self.max_energy_gain,
                    energy_increase: self.energy_increase,
                    combo_duration: self.combo_duration,
                    timer: self
                        .timer
                        .checked_add(Duration::from_secs_f32(data.dt.0))
                        .unwrap_or_default(),
                    stage_section: self.stage_section,
                    next_stage: true,
                });
            } else {
                update.character = CharacterState::ComboMelee(Data {
                    stage: self.stage,
                    num_stages: self.num_stages,
                    combo: self.combo,
                    stage_data: self.stage_data.clone(),
                    initial_energy_gain: self.initial_energy_gain,
                    max_energy_gain: self.max_energy_gain,
                    energy_increase: self.energy_increase,
                    combo_duration: self.combo_duration,
                    timer: self
                        .timer
                        .checked_add(Duration::from_secs_f32(data.dt.0))
                        .unwrap_or_default(),
                    stage_section: self.stage_section,
                    next_stage: self.next_stage,
                });
            }
        } else if self.next_stage {
            // Transitions to buildup section of next stage
            update.character = CharacterState::ComboMelee(Data {
                stage: (self.stage % self.num_stages) + 1,
                num_stages: self.num_stages,
                combo: self.combo + 1,
                stage_data: self.stage_data.clone(),
                initial_energy_gain: self.initial_energy_gain,
                max_energy_gain: self.max_energy_gain,
                energy_increase: self.energy_increase,
                combo_duration: self.combo_duration,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                next_stage: false,
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
            // Make sure attack component is removed
            data.updater.remove::<Attacking>(data.entity);
        }

        // Grant energy on successful hit
        if let Some(attack) = data.attacking {
            if attack.applied && attack.hit_count > 0 {
                let energy = self
                    .max_energy_gain
                    .min(self.initial_energy_gain + self.combo * self.energy_increase)
                    as i32;
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(energy, EnergySource::HitEnemy);
            }
        }

        update
    }
}
