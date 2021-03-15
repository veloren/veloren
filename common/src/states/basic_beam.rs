use crate::{
    combat::{
        Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement, Damage, DamageSource,
        GroupTarget,
    },
    comp::{beam, CharacterState, EnergyChange, EnergySource, Ori, Pos, StateUpdate},
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::*;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage or heal
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How long each beam segment persists for
    pub beam_duration: Duration,
    /// Base damage per tick
    pub damage: f32,
    /// Ticks per second
    pub tick_rate: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Lifesteal efficiency (0 gives 0% conversion of damage to health, 1 gives
    /// 100% conversion of damage to health)
    pub lifesteal_eff: f32,
    /// Energy regenerated per tick
    pub energy_regen: f32,
    /// Energy drained per second
    pub energy_drain: f32,
    /// Used to dictate how orientation functions in this state
    pub orientation_behavior: MovementBehavior,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Used to specify the beam to the frontend
    pub specifier: beam::FrontendSpecifier,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        match self.static_data.orientation_behavior {
            MovementBehavior::Normal => {},
            MovementBehavior::Turret => {
                update.ori = Ori::from(data.inputs.look_dir);
            },
        }

        handle_move(data, &mut update, 0.4);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Creates beam
                    data.updater.insert(data.entity, beam::Beam {
                        hit_entities: Vec::<Uid>::new(),
                        tick_dur: Duration::from_secs_f32(1.0 / self.static_data.tick_rate),
                        timer: Duration::default(),
                    });
                    // Build up
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Cast,
                        ..*self
                    });
                }
            },
            StageSection::Cast => {
                if input_is_pressed(data, self.static_data.ability_info.input)
                    && (self.static_data.energy_drain <= f32::EPSILON
                        || update.energy.current() > 0)
                {
                    let speed =
                        self.static_data.range / self.static_data.beam_duration.as_secs_f32();

                    let energy = AttackEffect::new(
                        None,
                        CombatEffect::EnergyReward(self.static_data.energy_regen),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);
                    let lifesteal = CombatEffect::Lifesteal(self.static_data.lifesteal_eff);
                    let damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Energy,
                            value: self.static_data.damage,
                        },
                        Some(GroupTarget::OutOfGroup),
                    )
                    .with_effect(lifesteal);
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_crit(crit_chance, crit_mult)
                        .with_effect(energy)
                        .with_combo_increment();

                    let properties = beam::Properties {
                        attack,
                        angle: self.static_data.max_angle.to_radians(),
                        speed,
                        duration: self.static_data.beam_duration,
                        owner: Some(*data.uid),
                        specifier: self.static_data.specifier,
                    };
                    // Gets offsets
                    let body_offsets = Vec3::new(
                        (data.body.radius() + 1.0) * data.inputs.look_dir.x,
                        (data.body.radius() + 1.0) * data.inputs.look_dir.y,
                        data.body.eye_height() * 0.6,
                    );
                    let pos = Pos(data.pos.0 + body_offsets);
                    // Create beam segment
                    update.server_events.push_front(ServerEvent::BeamSegment {
                        properties,
                        pos,
                        ori: Ori::from(data.inputs.look_dir),
                    });
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });

                    // Consumes energy if there's enough left and ability key is held down
                    update.energy.change_by(EnergyChange {
                        amount: -(self.static_data.energy_drain as f32 * data.dt.0) as i32,
                        source: EnergySource::Ability,
                    });
                } else {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    update.character = CharacterState::BasicBeam(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<beam::Beam>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<beam::Beam>(data.entity);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if !input_is_pressed(data, self.static_data.ability_info.input) {
            handle_state_interrupt(data, &mut update, false);
        }

        update
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MovementBehavior {
    Normal,
    Turret,
}
