use crate::{
    combat,
    comp::{
        Body, CharacterState, LightEmitter, MeleeConstructor, Pos, ProjectileConstructor,
        StateUpdate, character_state::OutputEvents,
    },
    event::ShootEvent,
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::{StageSection, *},
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    pub buildup_duration: Duration,
    pub buildup_melee_timing: f32,
    pub movement_duration: Duration,
    pub movement_ranged_timing: f32,
    pub land_timeout: Duration,
    pub recover_duration: Duration,
    pub melee: Option<MeleeConstructor>,
    pub melee_required: bool,
    pub projectile: ProjectileConstructor,
    pub projectile_body: Body,
    pub projectile_light: Option<LightEmitter>,
    pub projectile_speed: f32,
    pub horiz_leap_strength: f32,
    pub vert_leap_strength: f32,
    pub ability_info: AbilityInfo,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    pub static_data: StaticData,
    pub timer: Duration,
    pub stage_section: StageSection,
    pub melee_done: bool,
    pub ranged_done: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, 0.3);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    let frac = {
                        let raw = self.timer.as_secs_f32()
                            / self.static_data.buildup_duration.as_secs_f32();
                        raw.clamp(0.0, 1.0)
                    };
                    if self.static_data.melee.is_some()
                        && !self.melee_done
                        && frac > self.static_data.buildup_melee_timing
                    {
                        let precision_mult =
                            combat::compute_precision_mult(data.inventory, data.msm);
                        let tool_stats = get_tool_stats(data, self.static_data.ability_info);

                        if let Some(melee) = &self.static_data.melee {
                            data.updater.insert(
                                data.entity,
                                melee.clone().create_melee(
                                    precision_mult,
                                    tool_stats,
                                    self.static_data.ability_info,
                                ),
                            );
                        }

                        if let CharacterState::LeapRanged(c) = &mut update.character {
                            c.melee_done = true;
                        }
                    }

                    if let CharacterState::LeapRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else {
                    let early_exit = self.static_data.melee_required
                        && data.melee_attack.is_none_or(|melee| melee.hit_count == 0);
                    if let CharacterState::LeapRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = if early_exit {
                            StageSection::Recover
                        } else {
                            StageSection::Movement
                        };
                    }
                }
            },
            StageSection::Movement => {
                if self.timer < self.static_data.movement_duration {
                    let frac = {
                        let raw = self.timer.as_secs_f32()
                            / self.static_data.movement_duration.as_secs_f32();
                        raw.clamp(0.0, 1.0)
                    };
                    let progress = 1.0 - frac;
                    handle_forced_movement(data, &mut update, ForcedMovement::Leap {
                        vertical: self.static_data.vert_leap_strength,
                        forward: self.static_data.horiz_leap_strength,
                        progress,
                        direction: MovementDirection::AntiLook,
                    });

                    if !self.ranged_done && frac > self.static_data.movement_ranged_timing {
                        let precision_mult =
                            combat::compute_precision_mult(data.inventory, data.msm);

                        let projectile = self.static_data.projectile.clone().create_projectile(
                            Some(*data.uid),
                            precision_mult,
                            Some(self.static_data.ability_info),
                        );

                        let body_offsets = data.body.projectile_offsets(
                            update.ori.look_vec(),
                            data.scale.map_or(1.0, |s| s.0),
                        );
                        let pos = Pos(data.pos.0 + body_offsets);
                        output_events.emit_server(ShootEvent {
                            entity: Some(data.entity),
                            source_vel: Some(*data.vel),
                            pos,
                            dir: data.inputs.look_dir,
                            body: self.static_data.projectile_body,
                            projectile,
                            light: self.static_data.projectile_light,
                            speed: self.static_data.projectile_speed,
                            object: None,
                            marker: None,
                        });

                        if let CharacterState::LeapRanged(c) = &mut update.character {
                            c.ranged_done = true;
                        }
                    }

                    if let CharacterState::LeapRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(data, self.timer, None);
                    }
                } else if data.physics.on_ground.is_some()
                    | data.physics.in_liquid().is_some()
                    | (self.timer
                        > (self.static_data.movement_duration + self.static_data.land_timeout))
                {
                    if let CharacterState::LeapRanged(c) = &mut update.character {
                        c.timer = Duration::default();
                        c.stage_section = StageSection::Recover;
                    }
                } else if let CharacterState::LeapRanged(c) = &mut update.character {
                    c.timer = tick_attack_or_default(data, self.timer, None);
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    if let CharacterState::LeapRanged(c) = &mut update.character {
                        c.timer = tick_attack_or_default(
                            data,
                            self.timer,
                            Some(data.stats.recovery_speed_modifier),
                        );
                    }
                } else {
                    end_melee_ability(data, &mut update);
                }
            },
            _ => {
                end_melee_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}
