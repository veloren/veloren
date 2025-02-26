use crate::{
    combat::{self, CombatEffect},
    comp::{
        CharacterState, LightEmitter, Pos, ProjectileConstructor, StateUpdate,
        character_state::OutputEvents, item::ToolKind, slot::EquipSlot,
    },
    event::ThrowEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProjectileDir {
    // Projectile is thrown in the direction the entity is looking
    LookDir,
    // Projectile is thrown in the direction of an upwards facing vector slerped by some factor
    // towards the look direction
    Upwards(f32),
}

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the weapon needs to be prepared for
    pub buildup_duration: Duration,
    /// How long it takes to charge the weapon to max damage and knockback
    pub charge_duration: Duration,
    /// How long the action stage lasts
    pub throw_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How much energy is drained per second when charging
    pub energy_drain: f32,
    /// Projectile information
    pub projectile: ProjectileConstructor,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_dir: ProjectileDir,
    pub initial_projectile_speed: f32,
    pub scaled_projectile_speed: f32,
    /// Move speed efficiency
    pub move_speed: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
    /// Inventory slot to use item from
    pub equip_slot: EquipSlot,
    /// Item hash, used to verify that slot still has the correct item
    pub item_hash: u64,
    /// Type of tool thrown, stored here since it cannot be recalculated once
    /// the thrown item is removed from the entity's inventory
    pub tool_kind: ToolKind,
    /// Hand info for the thrown tool, stored here since it cannot be
    /// recalculated once the thrown item is removed from the entity's inventory
    pub hand_info: HandInfo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the attack fired already
    pub exhausted: bool,
}

impl Data {
    /// How complete the charge is, on a scale of 0.0 to 1.0
    pub fn charge_frac(&self) -> f32 {
        if let StageSection::Charge = self.stage_section {
            (self.timer.as_secs_f32() / self.static_data.charge_duration.as_secs_f32()).min(1.0)
        } else {
            0.0
        }
    }
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, self.static_data.move_speed);
        handle_jump(data, output_events, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Throw(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::Throw(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Charge,
                        ..*self
                    });
                }
            },
            StageSection::Charge => {
                if !input_is_pressed(data, self.static_data.ability_info.input)
                    && !self.exhausted
                    && data
                        .inventory
                        .and_then(|inv| inv.equipped(self.static_data.equip_slot))
                        .is_some_and(|item| item.item_hash() == self.static_data.item_hash)
                {
                    let charge_frac = self.charge_frac();

                    let body_offsets = data
                        .body
                        .projectile_offsets(update.ori.look_vec(), data.scale.map_or(1.0, |s| s.0));
                    let pos = Pos(data.pos.0 + body_offsets);

                    let dir = match self.static_data.projectile_dir {
                        ProjectileDir::LookDir => data.inputs.look_dir,
                        ProjectileDir::Upwards(interpolation_factor) => {
                            Dir::up().slerped_to(data.inputs.look_dir, interpolation_factor)
                        },
                    };

                    let precision_mult = combat::compute_precision_mult(data.inventory, data.msm);
                    let projectile = if self.static_data.projectile.scaled.is_some() {
                        self.static_data.projectile.handle_scaling(charge_frac)
                    } else {
                        self.static_data.projectile
                    };
                    let projectile = projectile.create_projectile(
                        Some(*data.uid),
                        precision_mult,
                        data.stats,
                        Some(self.static_data.ability_info),
                    );

                    // Removes the thrown item from the entity's inventory and creates a
                    // projectile
                    output_events.emit_server(ThrowEvent {
                        entity: data.entity,
                        pos,
                        dir,
                        projectile,
                        light: self.static_data.projectile_light,
                        speed: self.static_data.initial_projectile_speed
                            + charge_frac * self.static_data.scaled_projectile_speed,
                        object: None,
                        equip_slot: self.static_data.equip_slot,
                    });

                    // Exhausts ability and transitions to action
                    update.character = CharacterState::Throw(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        exhausted: true,
                        ..*self
                    });
                } else if self.timer < self.static_data.charge_duration
                    && input_is_pressed(data, self.static_data.ability_info.input)
                {
                    // Charges
                    update.character = CharacterState::Throw(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and input is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0);
                } else if input_is_pressed(data, self.static_data.ability_info.input) {
                    // Holds charge
                    update.character = CharacterState::Throw(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });

                    // Consumes energy if there's enough left and RMB is held down
                    update
                        .energy
                        .change_by(-self.static_data.energy_drain * data.dt.0 / 5.0);
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.throw_duration {
                    // Throws
                    update.character = CharacterState::Throw(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transition to recover
                    update.character = CharacterState::Throw(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::Throw(Data {
                        timer: tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        ),
                        ..*self
                    });
                } else {
                    // Done
                    end_ability(data, &mut update);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}
