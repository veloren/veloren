use super::movement::ROLL_DURATION;
use crate::{
    comp::{
        self, item, projectile, ActionState, ActionState::*, Body, CharacterState, ControlEvent,
        Controller, ControllerInputs, Energy, EnergySource, HealthChange, HealthSource, ItemKind,
        Mounting, MovementState, MovementState::*, PhysicsState, Projectile, Stats, Vel,
    },
    event::{Emitter, EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::{Uid, UidAllocator},
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Entity, Join, Read, ReadStorage, System, WriteStorage,
};
use std::time::Duration;
use vek::*;

const CHARGE_COST: i32 = 200;
const ROLL_COST: i32 = 30;


/// # Controller System
/// #### Responsible for validating controller inputs and setting new Character
/// States ----
///
/// **Writes:**
/// `CharacterState`, `ControllerInputs`
///
/// **Reads:**
/// `Stats`, `Vel`, `PhysicsState`, `Uid`, `Mounting`
///
/// _TODO: Join ActionStates and MovementStates into one and have a handle()
/// trait / fn?_ _TODO: Move weapon action to trait fn?_
pub struct Sys;

impl Sys {
    /// Assumes `input.primary` has been pressed
    /// handles primary actions. ie. equipping, mainhand weapon attacks.
    ///
    /// Returns the `ActionState` that occurred
    fn handle_primary(
        inputs: &mut ControllerInputs,
        character: &mut CharacterState,
        stats: &Stats,
        entity: Entity,
        uid: &Uid,
        server_emitter: &mut Emitter<'_, ServerEvent>,
        local_emitter: &mut Emitter<'_, LocalEvent>,
    ) -> ActionState {
        match stats.equipment.main.as_ref().map(|i| &i.kind) {
            // Character is wielding something
            Some(ItemKind::Tool { kind, power, .. }) => {
                let attack_duration = kind.attack_duration();
                let wield_duration = kind.wield_duration();

                // Since primary input was pressed, set
                // action to new Wield, in case of
                // instant primary actions
                if character.action == Idle {
                    character.action = Wield {
                        time_left: wield_duration,
                    };
                }

                match kind {
                    item::Tool::Bow if character.action.is_action_finished() => {
                        // Immediately end the wield
                        server_emitter.emit(ServerEvent::Shoot {
                            entity,
                            dir: inputs.look_dir,
                            body: comp::Body::Object(comp::object::Body::Arrow),
                            light: None,
                            gravity: Some(comp::Gravity(0.3)),
                            projectile: Projectile {
                                owner: *uid,
                                hit_ground: vec![projectile::Effect::Stick],
                                hit_wall: vec![projectile::Effect::Stick],
                                hit_entity: vec![
                                    projectile::Effect::Damage(HealthChange {
                                        amount: -(*power as i32),
                                        cause: HealthSource::Attack { by: *uid },
                                    }),
                                    projectile::Effect::Vanish,
                                ],
                                time_left: Duration::from_secs(15),
                            },
                        });
                        Attack {
                            time_left: attack_duration,
                            applied: false, // We don't want to do a melee attack
                        }
                        //character.action
                    },
                    item::Tool::Debug(item::Debug::Boost) => {
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: inputs.look_dir * 7.0,
                        });
                        character.action
                    },

                    item::Tool::Debug(item::Debug::Possess)
                        if character.action.is_action_finished() =>
                    {
                        server_emitter.emit(ServerEvent::Shoot {
                            entity,
                            gravity: Some(comp::Gravity(0.1)),
                            dir: inputs.look_dir,
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

                        character.action
                    }
                    // All other weapons
                    _ if character.action.is_action_finished() => Attack {
                        time_left: attack_duration,
                        applied: false,
                    },
                    _ => {
                        // Return the new Wield action
                        character.action
                    },
                }
            },
            // Without a weapon
            None => {
                // Attack
                if !character.action.is_attack() {
                    Attack {
                        time_left: Duration::from_millis(250),
                        applied: false,
                    }
                } else {
                    character.action
                }
            },
            _ => character.action,
        }
    }

    /// Assumes `input.seconday` has been pressed
    /// handles seconday actions. ie. blocking, althand weapons
    ///
    /// Returns the `ActionState` that occurred
    fn handle_secondary(
        inputs: &mut ControllerInputs,
        character: &mut CharacterState,
        stats: &Stats,
        entity: Entity,
        uid: &Uid,
        server_emitter: &mut Emitter<'_, ServerEvent>,
        local_emitter: &mut Emitter<'_, LocalEvent>,
    ) -> ActionState {
        match stats.equipment.main.as_ref().map(|i| &i.kind) {
            // Character is wielding something
            Some(ItemKind::Tool { kind, power, .. }) => {
                let attack_duration = kind.attack_duration();
                let wield_duration = kind.wield_duration();

                // Since primary input was pressed, set
                // action to new Wield, in case of
                // instant primary actions
                if character.action == Idle {
                    character.action = Wield {
                        time_left: wield_duration,
                    };
                }

                match kind {
                    // Magical Bolt
                    item::Tool::Staff
                        if character.movement == Stand && character.action.is_action_finished() =>
                    {
                        server_emitter.emit(ServerEvent::Shoot {
                            entity,
                            dir: inputs.look_dir,
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
                                        amount: -(*power as i32),
                                        cause: HealthSource::Attack { by: *uid },
                                    }),
                                    projectile::Effect::Vanish,
                                ],
                                time_left: Duration::from_secs(5),
                            },
                        });
                        // TODO: Don't play melee animation
                        Attack {
                            time_left: attack_duration,
                            applied: true, // We don't want to do a melee attack
                        }
                    }

                    // Go upward
                    item::Tool::Debug(item::Debug::Boost) => {
                        local_emitter.emit(LocalEvent::Boost {
                            entity,
                            vel: Vec3::new(0.0, 0.0, 7.0),
                        });

                        character.action
                    },

                    // All other weapons block
                    _ if character.action.is_action_finished() => Block {
                        time_active: Duration::from_secs(0),
                    },

                    _ => character.action,
                }
            },

            _ => character.action,
        }
    }
}

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, CharacterState>,
        ReadStorage<'a, Stats>,
        WriteStorage<'a, Energy>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, Vel>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Mounting>,
    );

    fn run(
        &mut self,
        (
            entities,
            uid_allocator,
            server_bus,
            local_bus,
            dt,
            mut controllers,
            mut character_states,
            stats,
            mut energies,
            bodies,
            velocities,
            physics_states,
            uids,
            mountings,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();
        for (
            entity,
            uid,
            controller,
            mut character,
            stats,
            mut energy,
            body,
            vel,
            physics,
            mount,
        ) in (
            &entities,
            &uids,
            &mut controllers,
            &mut character_states,
            &stats,
            &mut energies.restrict_mut(),
            &bodies,
            &velocities,
            &physics_states,
            mountings.maybe(),
        )
            .join()
        {
            let inputs = &mut controller.inputs;

            // ---------------------------------------
            // Common actions for multiple states as closure fn's for convenience
            // Returns a Wield action, or Idle if nothing to wield
            let try_wield = |stats: &Stats| -> ActionState {
                // Get weapon to wield
                if let Some(ItemKind::Tool { kind, .. }) =
                    stats.equipment.main.as_ref().map(|i| &i.kind)
                {
                    let wield_duration = kind.wield_duration();
                    Wield {
                        time_left: wield_duration,
                    }
                } else {
                    Idle
                }
            };

            let get_state_from_move_dir = |move_dir: &Vec2<f32>| -> MovementState {
                if move_dir.magnitude_squared() > 0.0 {
                    Run
                } else {
                    Stand
                }
            };

            // End common actions
            // ---------------------------------------

            // Being dead overrides all other states
            if stats.is_dead {
                // Only options: click respawn
                // prevent instant-respawns (i.e. player was holding attack)
                // by disallowing while input is held down
                if inputs.respawn.is_pressed() && !inputs.respawn.is_held_down() {
                    server_emitter.emit(ServerEvent::Respawn(entity));
                }
                // Or do nothing
                continue;
            }

            // Process controller events
            for event in controller.events.drain(..) {
                match event {
                    ControlEvent::Mount(mountee_uid) => {
                        if let Some(mountee_entity) =
                            uid_allocator.retrieve_entity_internal(mountee_uid.id())
                        {
                            server_emitter.emit(ServerEvent::Mount(entity, mountee_entity));
                        }
                    },
                    ControlEvent::Unmount => server_emitter.emit(ServerEvent::Unmount(entity)),
                    ControlEvent::InventoryManip(manip) => {
                        server_emitter.emit(ServerEvent::InventoryManip(entity, manip))
                    }, /*ControlEvent::Respawn => {
                        if state.is_dead {
                          server_emitter.emit(ServerEvent::Respawn(entity)),
                        }
                       }*/
                }
            }

            // If mounted, character state is controlled by mount
            if mount.is_some() {
                character.movement = Sit;
                continue;
            }

            inputs.update_look_dir();
            inputs.update_move_dir();

            match (character.action, character.movement) {
                // Jumping, one frame state that calls jump server event
                (_, Jump) => {
                    character.movement = Fall;
                    local_emitter.emit(LocalEvent::Jump(entity));
                },
                // Charging + Any Movement, prioritizes finishing charge
                // over movement states
                (Charge { time_left }, _) => {
                    inputs.update_move_dir();
                    if time_left == Duration::default() || vel.0.magnitude_squared() < 10.0 {
                        character.action = try_wield(stats);
                    } else {
                        character.action = Charge {
                            time_left: time_left
                                .checked_sub(Duration::from_secs_f32(dt.0))
                                .unwrap_or_default(),
                        };
                    }
                    if let Some(uid_b) = physics.touch_entity {
                        server_emitter.emit(ServerEvent::Damage {
                            uid: uid_b,
                            change: HealthChange {
                                amount: -20,
                                cause: HealthSource::Attack { by: *uid },
                            },
                        });

                        character.action = try_wield(stats);
                    }
                },
                // Rolling + Any Movement, prioritizes finishing charge
                // over movement states
                (
                    Roll {
                        time_left,
                        was_wielding,
                    },
                    _,
                ) => {
                    if time_left == Duration::default() {
                        if was_wielding {
                            character.action = try_wield(stats);
                        } else {
                            character.action = Idle;
                        }
                    } else {
                        character.action = Roll {
                            time_left: time_left
                                .checked_sub(Duration::from_secs_f32(dt.0))
                                .unwrap_or_default(),
                            was_wielding,
                        }
                    }
                },
                // Any Action + Falling
                (action_state, Fall) => {
                    character.movement = get_state_from_move_dir(&inputs.move_dir);
                    if inputs.glide.is_pressed() && can_glide(body) {
                        character.movement = Glide;
                        continue;
                    }
                    // Try to climb
                    if let (true, Some(_wall_dir)) = (
                        (inputs.climb.is_pressed() | inputs.climb_down.is_pressed())
                            && can_climb(body),
                        physics.on_wall,
                    ) {
                        character.movement = Climb;
                        continue;
                    }
                    // Reset to Falling while not standing on ground,
                    // otherwise keep the state given above
                    if !physics.on_ground {
                        if physics.in_fluid {
                            character.movement = Swim;
                        } else {
                            character.movement = Fall;
                        }
                    } else {
                        character.movement = Stand;
                        continue;
                    }

                    match action_state {
                        // Unwield if buttons pressed
                        Wield { .. } | Attack { .. } => {
                            if inputs.toggle_wield.is_just_pressed() {
                                character.action = Idle;
                            }
                        },
                        // Try to wield if any of buttons pressed
                        Idle => {
                            if inputs.primary.is_pressed() || inputs.secondary.is_pressed() {
                                character.action = try_wield(stats);
                                continue;
                            }
                        },
                        // Cancel blocks
                        Block { .. } => {
                            character.action = try_wield(stats);
                            continue;
                        },
                        // Don't change action
                        Charge { .. } | Roll { .. } => {},
                    }
                    if inputs.primary.is_pressed() {
                        character.action = Self::handle_primary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    } else if inputs.secondary.is_pressed() {
                        character.action = Self::handle_secondary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    }
                },
                // Any Action + Swimming
                (_action_state, Swim) => {
                    character.movement = get_state_from_move_dir(&inputs.move_dir);

                    if !physics.on_ground && physics.in_fluid {
                        character.movement = Swim;
                    }
                    if inputs.primary.is_pressed() {
                        character.action = Self::handle_primary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    } else if inputs.secondary.is_pressed() {
                        character.action = Self::handle_secondary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    }
                },
                // Blocking, restricted look_dir compared to other states
                (Block { .. }, Stand) | (Block { .. }, Run) => {
                    character.movement = get_state_from_move_dir(&inputs.move_dir);

                    if !inputs.secondary.is_pressed() {
                        character.action = try_wield(stats);
                    } else {
                        character.action = Self::handle_secondary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    }

                    if !physics.on_ground {
                        if physics.in_fluid {
                            character.movement = Swim;
                        } else {
                            character.movement = Fall;
                        }
                    }
                },
                // Standing and Running states, typical states :shrug:
                (action_state, Run) | (action_state, Stand) => {
                    character.movement = get_state_from_move_dir(&inputs.move_dir);
                    // Try to sit
                    if inputs.sit.is_pressed() && physics.on_ground && body.is_humanoid() {
                        character.movement = Sit;
                        continue;
                    }

                    // Try to climb
                    
                        if let (true, Some(_wall_dir)) = (
                        (inputs.climb.is_pressed() | inputs.climb_down.is_pressed())
                            && can_climb(body),
                        physics.on_wall,)
                        {
                           
                                character.movement = Climb;
                            continue;
                        }

                    // Try to swim
                    if !physics.on_ground {
                        if physics.in_fluid {
                            character.movement = Swim;
                        } else {
                            character.movement = Fall;
                        }
                    }

                    // While on ground ...
                    if physics.on_ground {
                        // Try to jump
                        if inputs.jump.is_pressed() {
                            character.movement = Jump;
                            continue;
                        }

                        // Try to charge
                        if inputs.charge.is_pressed() && !inputs.charge.is_held_down() {
                            if energy
                                .get_mut_unchecked()
                                .try_change_by(-CHARGE_COST, EnergySource::CastSpell)
                                .is_ok()
                            {
                                character.action = Charge {
                                    time_left: Duration::from_millis(250),
                                };
                                
                            }
                            continue;
                        }

                        // Try to roll
                        if character.movement == Run
                            && inputs.roll.is_pressed()
                            && body.is_humanoid()
                        {
                            if energy
                                .get_mut_unchecked()
                                .try_change_by(-ROLL_COST, EnergySource::Roll)
                                .is_ok()
                            {
                                    character.action = Roll {
                                        time_left: ROLL_DURATION,
                                        was_wielding: character.action.is_wield(),
                                    };
                                    
                                    
                            }
                            continue;
                        }
                    }
                    // While not on ground ...
                    else {
                        // Try to glide
                        if physics.on_wall == None && inputs.glide.is_pressed() && can_glide(&body)
                        {
                            character.movement = Glide;
                            continue;
                        }
                    }

                    // Tool Actions
                    if inputs.toggle_wield.is_just_pressed() {
                        match action_state {
                            Wield { .. } | Attack { .. } => {
                                // Prevent instantaneous reequipping by checking
                                // for done wielding
                                if character.action.is_action_finished() {
                                    character.action = Idle;
                                }
                                continue;
                            },
                            Idle => {
                                character.action = try_wield(stats);
                                continue;
                            },
                            Charge { .. } | Roll { .. } | Block { .. } => {},
                        }
                    }
                    if inputs.primary.is_pressed() {
                        character.action = Self::handle_primary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    } else if inputs.secondary.is_pressed() {
                        character.action = Self::handle_secondary(
                            inputs,
                            character,
                            stats,
                            entity,
                            uid,
                            &mut server_emitter,
                            &mut local_emitter,
                        );
                    }
                },
                // Sitting
                (_, Sit) => {
                    character.action = Idle;
                    character.movement = get_state_from_move_dir(&inputs.move_dir);

                    // character.movement will be Stand after updating when
                    // no movement has occurred
                    if character.movement == Stand {
                        character.movement = Sit;
                    }
                    if inputs.jump.is_pressed() {
                        character.movement = Jump;
                        continue;
                    }
                    if !physics.on_ground {
                        character.movement = Fall;
                    }
                },
                // Any Action + Gliding, shouldnt care about action,
                // because should be Idle
                (_, Glide) => {
                    character.action = Idle;

                    if !inputs.glide.is_pressed() {
                        character.movement = Fall;
                    } else if let (Some(_wall_dir), true) = (physics.on_wall, can_climb(body)) {
                        character.movement = Climb;
                    }

                    if physics.on_ground {
                        character.movement = Stand
                    }
                },
                // Any Action + Climbing, shouldnt care about action,
                // because should be Idle
                (_, Climb) => {
                    character.action = Idle;
                    if let None = physics.on_wall {
                        if inputs.jump.is_pressed() {
                            character.movement = Jump;
                        } else {
                            character.movement = Fall;
                        }
                    }
                    if physics.on_ground {
                        character.movement = Stand;
                    }
                }, /* In case of adding new states
                    * (_, _) => {
                    *     println!("UNKNOWN STATE");
                    *     character.action = Idle;
                    *     character.movement = Fall;
                    * } */
            };
        }
    }
}

fn can_glide(body: &Body) -> bool { body.is_humanoid() }

fn can_climb(body: &Body) -> bool { body.is_humanoid() }
