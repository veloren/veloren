use super::{
    combat::{ATTACK_DURATION, WIELD_DURATION},
    movement::ROLL_DURATION,
};
use crate::{
    comp::{
        self, item, projectile, ActionState::*, Body, CharacterState, ControlEvent, Controller,
        HealthChange, HealthSource, Item, MovementState::*, PhysicsState, Projectile, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Join, Read, ReadStorage, System, WriteStorage,
};
use sphynx::{Uid, UidAllocator};
use std::time::Duration;
use vek::*;

/// This system is responsible for validating controller inputs
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        Read<'a, UidAllocator>,
        Entities<'a>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Uid>,
        WriteStorage<'a, CharacterState>,
    );

    fn run(
        &mut self,
        (
            uid_allocator,
            entities,
            server_bus,
            local_bus,
            mut controllers,
            stats,
            bodies,
            velocities,
            physics_states,
            uids,
            mut character_states,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();

        for (entity, uid, controller, stats, body, vel, physics, mut character) in (
            &entities,
            &uids,
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

            // Sit
            if controller.sit
                && physics.on_ground
                && character.action == Idle
                && character.movement != Sit
                && body.is_humanoid()
            {
                character.movement = Sit;
            } else if character.movement == Sit
                && (controller.move_dir.magnitude_squared() > 0.0 || !physics.on_ground)
            {
                character.movement = Run;
            }

            // Wield
            if controller.primary
                && character.action == Idle
                && (character.movement == Stand || character.movement == Run)
            {
                character.action = Wield {
                    time_left: WIELD_DURATION,
                };
            }

            match stats.equipment.main {
                Some(Item::Tool {
                    kind: item::Tool::Bow,
                    power,
                    ..
                }) => {
                    if controller.primary
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                    {
                        if let Wield { time_left } = character.action {
                            if time_left == Duration::default() {
                                // Immediately end the wield
                                character.action = Idle;
                                server_emitter.emit(ServerEvent::Shoot {
                                    entity,
                                    dir: controller.look_dir,
                                    body: comp::Body::Object(comp::object::Body::Arrow),
                                    light: None,
                                    gravity: Some(comp::Gravity(0.3)),
                                    projectile: Projectile {
                                        owner: *uid,
                                        hit_ground: vec![projectile::Effect::Stick],
                                        hit_wall: vec![projectile::Effect::Stick],
                                        hit_entity: vec![
                                            projectile::Effect::Damage(HealthChange {
                                                amount: -(power as i32),
                                                cause: HealthSource::Attack { by: *uid },
                                            }),
                                            projectile::Effect::Vanish,
                                        ],
                                        time_left: Duration::from_secs(30),
                                    },
                                });
                            }
                        }
                    }
                }
                Some(Item::Tool {
                    kind: item::Tool::Staff,
                    power,
                    ..
                }) => {
                    // Melee Attack
                    if controller.primary
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                    {
                        if let Wield { time_left } = character.action {
                            if time_left == Duration::default() {
                                character.action = Attack {
                                    time_left: ATTACK_DURATION,
                                    applied: false,
                                };
                            }
                        }
                    }
                    // Magical Bolt
                    if controller.secondary
                        && (
                            character.movement == Stand
                            //|| character.movement == Run
                            //|| character.movement == Jump
                        )
                    {
                        if let Wield { time_left } = character.action {
                            if time_left == Duration::default() {
                                character.action = Attack {
                                    time_left: ATTACK_DURATION,
                                    applied: false,
                                };
                                server_emitter.emit(ServerEvent::Shoot {
                                    entity,
                                    dir: controller.look_dir,
                                    body: comp::Body::Object(comp::object::Body::BoltFire),
                                    gravity: Some(comp::Gravity(0.0)),
                                    light: Some(comp::LightEmitter {
                                        col: (0.72, 0.11, 0.11).into(),
                                        strength: 10.0,
                                        offset: Vec3::new(0.0, -5.0, 2.0),
                                    }),
                                    projectile: Projectile {
                                        owner: *uid,
                                        hit_ground: vec![projectile::Effect::Vanish],
                                        hit_wall: vec![projectile::Effect::Vanish],
                                        hit_entity: vec![
                                            projectile::Effect::Damage(HealthChange {
                                                amount: -(power as i32),
                                                cause: HealthSource::Attack { by: *uid },
                                            }),
                                            projectile::Effect::Vanish,
                                        ],
                                        time_left: Duration::from_secs(5),
                                    },
                                });
                            }
                        }
                    }
                }
                Some(Item::Tool { .. }) => {
                    // Melee Attack
                    if controller.primary
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                    {
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
                    if controller.secondary
                        && (character.movement == Stand || character.movement == Run)
                        && character.action.is_wield()
                    {
                        character.action = Block {
                            time_left: Duration::from_secs(5),
                        };
                    } else if !controller.secondary && character.action.is_block() {
                        character.action = Wield {
                            time_left: Duration::default(),
                        };
                    }
                }
                Some(Item::Debug(item::Debug::Boost)) => {
                    if controller.primary {
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: controller.look_dir * 7.0,
                        });
                    }
                    if controller.secondary {
                        // Go upward
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: Vec3::new(0.0, 0.0, 7.0),
                        });
                    }
                }
                Some(Item::Debug(item::Debug::Possess)) => {
                    if controller.primary
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                    {
                        if let Wield { time_left } = character.action {
                            if time_left == Duration::default() {
                                // Immediately end the wield
                                character.action = Idle;
                                server_emitter.emit(ServerEvent::Shoot {
                                    entity,
                                    gravity: Some(comp::Gravity(0.1)),
                                    dir: controller.look_dir,
                                    body: comp::Body::Object(comp::object::Body::ArrowSnake),
                                    light: Some(comp::LightEmitter {
                                        col: (0.0, 1.0, 0.3).into(),
                                        ..Default::default()
                                    }),
                                    projectile: Projectile {
                                        owner: *uid,
                                        hit_ground: vec![projectile::Effect::Stick],
                                        hit_wall: vec![projectile::Effect::Stick],
                                        hit_entity: vec![
                                            projectile::Effect::Stick,
                                            projectile::Effect::Possess,
                                        ],
                                        time_left: Duration::from_secs(10),
                                    },
                                });
                            }
                        }
                    }
                    // Block
                    if controller.secondary
                        && (character.movement == Stand || character.movement == Run)
                        && character.action.is_wield()
                    {
                        character.action = Block {
                            time_left: Duration::from_secs(5),
                        };
                    } else if !controller.secondary && character.action.is_block() {
                        character.action = Wield {
                            time_left: Duration::default(),
                        };
                    }
                }
                None => {
                    // Attack
                    if controller.primary
                        && (character.movement == Stand
                            || character.movement == Run
                            || character.movement == Jump)
                        && !character.action.is_attack()
                    {
                        character.action = Attack {
                            time_left: ATTACK_DURATION,
                            applied: false,
                        };
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
            if controller.jump
                && physics.on_ground
                && vel.0.z <= 0.0
                && !character.movement.is_roll()
            {
                local_emitter.emit(LocalEvent::Jump(entity));
            }

            // Wall leap
            if controller.wall_leap {
                if let (Some(_wall_dir), Climb) = (physics.on_wall, character.movement) {
                    //local_emitter.emit(LocalEvent::WallLeap { entity, wall_dir });
                }
            }

            // Process controller events
            for event in std::mem::replace(&mut controller.events, Vec::new()) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) =
                            uid_allocator.retrieve_entity_internal(mountee_uid.id())
                        {
                            server_emitter.emit(ServerEvent::Mount(entity, mountee_entity));
                        }
                    }
                    ControlEvent::Unmount => server_emitter.emit(ServerEvent::Unmount(entity)),
                }
            }
        }
    }
}
