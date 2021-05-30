use crate::{
    comp::{Body, CharacterState, LightEmitter, ProjectileConstructor, StateUpdate},
    event::ServerEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    util::Dir,
};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How much buildup is required before the attack
    pub buildup_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// How much spread there is when more than 1 projectile is created
    pub projectile_spread: f32,
    /// Projectile variables
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    /// How many projectiles are simultaneously fired
    pub num_projectiles: u32,
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
    /// Whether the attack fired already
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::BasicRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::BasicRanged(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if !self.exhausted {
                    // Fire
                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);
                    let projectile = self.static_data.projectile.create_projectile(
                        Some(*data.uid),
                        crit_chance,
                        crit_mult,
                    );
                    // Shoots all projectiles simultaneously
                    for i in 0..self.static_data.num_projectiles {
                        // Adds a slight spread to the projectiles. First projectile has no spread,
                        // and spread increases linearly with number of projectiles created.
                        let dir = Dir::from_unnormalized(data.inputs.look_dir.map(|x| {
                            let offset = (2.0 * thread_rng().gen::<f32>() - 1.0)
                                * self.static_data.projectile_spread
                                * i as f32;
                            x + offset
                        }))
                        .unwrap_or(data.inputs.look_dir);
                        // Tells server to create and shoot the projectile
                        update.server_events.push_front(ServerEvent::Shoot {
                            entity: data.entity,
                            dir,
                            body: self.static_data.projectile_body,
                            projectile: projectile.clone(),
                            light: self.static_data.projectile_light,
                            speed: self.static_data.projectile_speed,
                            object: None,
                        });
                    }

                    update.character = CharacterState::BasicRanged(Data {
                        exhausted: true,
                        ..*self
                    });
                } else if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::BasicRanged(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, &mut update);
                    } else {
                        update.character = CharacterState::Wielding;
                    }
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

fn reset_state(data: &Data, join: &JoinData, update: &mut StateUpdate) {
    handle_input(join, update, data.static_data.ability_info.input);
}
