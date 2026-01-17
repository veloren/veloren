use crate::{
    Explosion, RadiusEffect,
    combat::{self, Attack, AttackDamage, Damage, DamageKind::Crushing, GroupTarget},
    comp::{
        CharacterState, MeleeConstructor, StateUpdate, ability::Dodgeable,
        character_state::OutputEvents, item::Reagent,
    },
    event::{ComboChangeEvent, ExplosionEvent},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until the state attacks
    pub buildup_duration: Duration,
    /// How long the state is in the swing duration
    pub swing_duration: Duration,
    /// How long until state ends
    pub recover_duration: Duration,
    /// Used to construct the Melee attack
    pub melee_constructor: MeleeConstructor,
    /// Energy cost per attack
    pub energy_cost: f32,
    /// Maximum number of consecutive strikes, if there is a max
    pub max_strikes: Option<u32>,
    pub move_modifier: f32,
    pub ori_modifier: f32,
    pub minimum_combo: u32,
    /// Used to indicate to the frontend what ability this is for any special
    /// effects
    pub frontend_specifier: Option<FrontendSpecifier>,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// How many spins it has done
    pub current_strike: u32,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the state can deal damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, self.static_data.ori_modifier, None);
        handle_move(data, &mut update, self.static_data.move_modifier);
        handle_interrupts(data, &mut update, output_events);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    // Transitions to swing section of stage
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Action;
                    }
                }
            },
            StageSection::Action => {
                if !self.exhausted {
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.exhausted = true;
                    }

                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                    data.updater.insert(
                        data.entity,
                        self.static_data.melee_constructor.clone().create_melee(
                            precision_mult,
                            tool_stats,
                            self.static_data.ability_info,
                        ),
                    );
                } else if self.timer < self.static_data.swing_duration {
                    // Swings
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if match self.static_data.max_strikes {
                    Some(max) => self.current_strike < max,
                    None => input_is_pressed(data, self.static_data.ability_info.input),
                } && update
                    .energy
                    .try_change_by(-self.static_data.energy_cost)
                    .is_ok()
                {
                    // TODO: Remove this, this should never have been added this way
                    if self.static_data.frontend_specifier == Some(FrontendSpecifier::CultistVortex)
                    {
                        let damage = AttackDamage::new(
                            Damage {
                                kind: Crushing,
                                value: 10.0,
                            },
                            Some(GroupTarget::OutOfGroup),
                            rand::random(),
                        );
                        let attack =
                            Attack::new(Some(self.static_data.ability_info)).with_damage(damage);
                        let explosion = Explosion {
                            effects: vec![RadiusEffect::Attack {
                                attack,
                                dodgeable: Dodgeable::Roll,
                            }],
                            radius: data.body.max_radius() * 4.0,
                            reagent: Some(Reagent::Purple),
                            min_falloff: 0.5,
                        };
                        let pos =
                            data.pos.0 + (*data.ori.look_dir() * (data.body.max_radius() * 3.0));
                        let explosition =
                            Vec3::new(pos.x, pos.y, pos.z + (data.body.height() / 4.0));
                        output_events.emit_server(ExplosionEvent {
                            pos: explosition,
                            explosion,
                            owner: Some(*data.uid),
                        });
                    }
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.current_strike += 1;
                        c.exhausted = false;
                    }
                } else {
                    // Transitions to recover section of stage
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                }

                // Consume combo if any was required
                if self.static_data.minimum_combo > 0 {
                    output_events.emit_server(ComboChangeEvent {
                        entity: data.entity,
                        change: -data.combo.map_or(0, |c| c.counter() as i32),
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover
                    if let CharacterState::RapidMelee(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
                } else {
                    // Done
                    end_melee_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_melee_ability(data, &mut update);
            },
        }

        update
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum FrontendSpecifier {
    CultistVortex,
    IceWhirlwind,
    ElephantVacuum,
}
