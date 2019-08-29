use super::{
    combat::{ATTACK_DURATION, WIELD_DURATION},
    movement::ROLL_DURATION,
};
use crate::{
    comp::{
        item, ActionState::*, Body, CharacterState, Controller, Item, MovementState::*,
        PhysicsState, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
};
use specs::{Entities, Join, Read, ReadStorage, System, WriteStorage};
use std::time::Duration;
use vek::*;

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        WriteStorage<'a, CharacterState>,
    );

    fn run(
        &mut self,
        (
            entities,
            server_bus,
            local_bus,
            mut controllers,
            stats,
            bodies,
            velocities,
            physics_states,
            mut character_states,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();

        for (entity, controller, stats, body, vel, physics, mut character) in (
            &entities,
            &mut controllers,
            &stats,
            &bodies,
            &velocities,
            &physics_states,
            &mut character_states,
        )
            .join()
        {
            if stats.is_dead {
                // Respawn
                if controller.respawn {
                    server_emitter.emit(ServerEvent::Respawn(entity));
                }
                continue;
            }

            // Move
            controller.move_dir = if controller.move_dir.magnitude_squared() > 1.0 {
                controller.move_dir.normalized()
            } else {
                controller.move_dir
            };

            if character.movement == Stand && controller.move_dir.magnitude_squared() > 0.0 {
                character.movement = Run;
            } else if character.movement == Run && controller.move_dir.magnitude_squared() == 0.0 {
                character.movement = Stand;
            }

            // Look
            controller.look_dir = controller
                .look_dir
                .try_normalized()
                .unwrap_or(controller.move_dir.into());

            // Glide
            // TODO: Check for glide ability/item
            if controller.glide
                && !physics.on_ground
                && (character.action == Idle || character.action.is_wield())
                && character.movement == Jump
                && body.is_humanoid()
            {
                character.movement = Glide;
            } else if !controller.glide && character.movement == Glide {
                character.movement = Jump;
            }

            // Wield
            if controller.main
                && character.action == Idle
                && (character.movement == Stand || character.movement == Run)
            {
                character.action = Wield {
                    time_left: WIELD_DURATION,
                };
            }

            match stats.equipment.main {
                Some(Item::Tool { .. }) => {
                    // Attack
                    if controller.main
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                    {
                        // TODO: Check if wield ability exists
                        if let Wield { time_left } = character.action {
                            if time_left == Duration::default() {
                                character.action = Attack {
                                    time_left: ATTACK_DURATION,
                                    applied: false,
                                };
                            }
                        }
                    }

                    // Block
                    if controller.alt
                        && (character.movement == Stand || character.movement == Run)
                        && (character.action == Idle || character.action.is_wield())
                    {
                        character.action = Block {
                            time_left: Duration::from_secs(5),
                        };
                    } else if !controller.alt && character.action.is_block() {
                        character.action = Idle;
                    }
                }
                Some(Item::Debug(item::Debug::Teleport)) => {
                    if controller.main {
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: controller.look_dir * 7.0,
                        });
                    }
                    if controller.alt {
                        // Go upward
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: controller.look_dir * -7.0,
                        });
                    }
                }
                _ => {}
            }

            // Roll
            if controller.roll
                && (character.action == Idle || character.action.is_wield())
                && character.movement == Run
                && physics.on_ground
            {
                character.movement = Roll {
                    time_left: ROLL_DURATION,
                };
            }

            // Jump
            if controller.jump && physics.on_ground && vel.0.z <= 0.0 {
                local_emitter.emit(LocalEvent::Jump(entity));
            }
        }
    }
}
