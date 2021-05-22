use crate::{
    comp::{
        Body, CharacterState, EnergyChange, EnergySource, LightEmitter, ProjectileConstructor,
        StateUpdate,
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
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.3);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Buildup to attack
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transition to shoot
                    update.character = CharacterState::RepeaterRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Shoot,
                        ..*self
                    });
                }
            },
            StageSection::Shoot => {
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
                    && update.energy.current() as f32 >= self.static_data.energy_cost
                {
                    // Fire if input is pressed still
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let projectile = self.static_data.projectile.create_projectile(
                        Some(*data.uid),
                        crit_chance,
                        crit_mult,
                    );
                    update.server_events.push_front(ServerEvent::Shoot {
                        entity: data.entity,
                        // Provides slight variation to projectile direction
                        dir: data.inputs.look_dir,
                        body: self.static_data.projectile_body,
                        projectile,
                        light: self.static_data.projectile_light,
                        speed: self.static_data.projectile_speed,
                        object: None,
                    });

                    update.server_events.push_front(ServerEvent::EnergyChange {
                        entity: data.entity,
                        change: EnergyChange {
                            amount: -self.static_data.energy_cost as i32,
                            source: EnergySource::Ability,
                        },
                    });

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
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if !input_is_pressed(data, self.static_data.ability_info.input) {
            handle_state_interrupt(data, &mut update, false);
        }

        update
    }
}
