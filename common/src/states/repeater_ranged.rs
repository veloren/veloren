use crate::{
    combat::CombatEffect,
    comp::{
        character_state::OutputEvents, Body, CharacterState, LightEmitter, Pos,
        ProjectileConstructor, StateUpdate,
    },
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// How long we've readied the weapon
    pub buildup_duration: Duration,
    /// How long the state is shooting
    pub shoot_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Energy cost per projectile
    pub energy_cost: f32,
    /// Max speed that can be reached
    pub max_speed: f32,
    /// Projectiles required to reach half of max speed
    pub half_speed_at: u32,
    /// Projectile options
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
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
    /// Speed of the state while in shoot section
    pub speed: f32,
    /// Number of projectiles fired so far
    pub projectiles_fired: u32,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.3);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Buildup to attack
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transition to shoot
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
                if self.timer < self.static_data.shoot_duration {
                    // Draw projectile
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0 * self.speed))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else if input_is_pressed(data, self.static_data.ability_info.input)
                    && update.energy.current() >= self.static_data.energy_cost
                {
                    // Fire if input is pressed still
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let tool_stats = get_tool_stats(data, self.static_data.ability_info);
                    // Gets offsets
                    let body_offsets = data.body.projectile_offsets(update.ori.look_vec());
                    let pos = Pos(data.pos.0 + body_offsets);
                    let projectile = self.static_data.projectile.create_projectile(
                        Some(*data.uid),
                        crit_chance,
                        crit_mult,
                        tool_stats,
                        self.static_data.damage_effect,
                    );
                    output_events.emit_server(ServerEvent::Shoot {
                        entity: data.entity,
                        pos,
                        dir: data.inputs.look_dir,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        speed: self.static_data.projectile_speed,
                        object: None,
                    });

                    // Removes energy from character when arrow is fired
                    output_events.emit_server(ServerEvent::EnergyChange {
                        entity: data.entity,
                        change: -self.static_data.energy_cost,
                    });

                    // Sets new speed of shoot. Scales based off of the number of projectiles fired.
                    let new_speed = 1.0
                        + self.projectiles_fired as f32
                            / (self.static_data.half_speed_at as f32
                                + self.projectiles_fired as f32)
                            * self.static_data.max_speed;

                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        speed: new_speed,
                        projectiles_fired: self.projectiles_fired + 1,
                        ..*self
                    });
                } else {
                    // Transition to recover
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Recover from attack
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
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
