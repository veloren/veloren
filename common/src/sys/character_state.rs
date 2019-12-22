use super::movement::ROLL_DURATION;
use super::phys::GRAVITY;

const HUMANOID_ACCEL: f32 = 50.0;
const HUMANOID_SPEED: f32 = 120.0;
const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;
const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;
const HUMANOID_CLIMB_ACCEL: f32 = 5.0;
const ROLL_SPEED: f32 = 17.0;
const CHARGE_SPEED: f32 = 20.0;
const GLIDE_ACCEL: f32 = 15.0;
const GLIDE_SPEED: f32 = 45.0;
const BLOCK_ACCEL: f32 = 30.0;
const BLOCK_SPEED: f32 = 75.0;
// Gravity is 9.81 * 4, so this makes gravity equal to .15
const GLIDE_ANTIGRAV: f32 = GRAVITY * 0.96;
const CLIMB_SPEED: f32 = 5.0;

pub const MOVEMENT_THRESHOLD_VEL: f32 = 3.0;
use crate::{
    comp::{
        self, item, projectile, ActionState, ActionState::*, Body, CharacterState, ClimbData,
        ControlEvent, Controller, ControllerInputs, FallData, GlideData, HealthChange,
        HealthSource, ItemKind, JumpData, Mounting, MovementState, MovementState::*, Ori,
        PhysicsState, Pos, Projectile, RunData, SitData, StandData, Stats, SwimData, Vel,
    },
    event::{Emitter, EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
};
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entities, Entity, Join, Read, ReadStorage, System, WriteStorage,
};
use sphynx::{Uid, UidAllocator};
use std::time::Duration;
use vek::*;

struct CharacterStateData();

/// # Character State System
/// #### Updates then detemrines next Character States based on ControllerInputs
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        ReadStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Body>,
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
            mut character_states,
            mut positions,
            mut velocities,
            mut orientations,
            controllers,
            stats,
            bodies,
            physics_states,
            uids,
            mountings,
        ): Self::SystemData,
    ) {
        for (
            entity,
            uid,
            mut character,
            mut pos,
            mut vel,
            mut ori,
            controller,
            stats,
            body,
            physics,
            mount,
        ) in (
            &entities,
            &uids,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &controllers,
            &stats,
            &bodies,
            &physics_states,
            mountings.maybe(),
        )
            .join()
        {
            let inputs = &controller.inputs;
            // println!("{:?}", character);
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
                    Run(RunData)
                } else {
                    Stand(StandData)
                }
            };

            // Being dead overrides all other states
            if stats.is_dead {
                // Only options: click respawn
                // prevent instant-respawns (i.e. player was holding attack)
                // by disallowing while input is held down
                if inputs.respawn.is_pressed() && !inputs.respawn.is_held_down() {
                    server_bus.emitter().emit(ServerEvent::Respawn(entity));
                }
                // Or do nothing
                continue;
            }
            // If mounted, character state is controlled by mount
            // TODO: Make mounting a state
            if mount.is_some() {
                character.movement = Sit(SitData);
                continue;
            }

            // Update Action States
            match character.action {
                Attack {
                    ref mut time_left, ..
                } => {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
                Roll {
                    ref mut time_left, ..
                } => {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
                Charge { ref mut time_left } => {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
                Wield { ref mut time_left } => {
                    *time_left = time_left
                        .checked_sub(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
                Block {
                    ref mut time_active,
                } => {
                    *time_active = time_active
                        .checked_add(Duration::from_secs_f32(dt.0))
                        .unwrap_or_default();
                }
                Idle => {}
            }

            // Determine new state
            *character = match character.movement {
                Stand(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Run(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Jump(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Climb(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Glide(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Swim(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Fall(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ),
                Sit(data) => data.handle(
                    &entity,
                    character,
                    pos,
                    vel,
                    ori,
                    &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    &server_bus,
                    &local_bus,
                ), // Charging + Any Movement, prioritizes finishing charge
                   // over movement states
                   // (Charge { time_left }, _) => {
                   //     if let Some(uid_b) = physics.touch_entity {
                   //         server_emitter.emit(ServerEvent::Damage {
                   //             uid: uid_b,
                   //             change: HealthChange {
                   //                 amount: -20,
                   //                 cause: HealthSource::Attack { by: *uid },
                   //             },
                   //         });

                   //         character.action = try_wield(stats);
                   //     } else if time_left == Duration::default() || vel.0.magnitude_squared() < 10.0 {
                   //         character.action = try_wield(stats);
                   //     }
                   // }
                   // Rolling + Any Movement, prioritizes finishing charge
                   // over movement states
                   // (
                   //     Roll {
                   //         time_left,
                   //         was_wielding,
                   //     },
                   //     _,
                   // ) => {
                   //     if time_left == Duration::default() {
                   //         if was_wielding {
                   //             character.action = try_wield(stats);
                   //         } else {
                   //             character.action = Idle;
                   //         }
                   //     }
                   // }
            };
        }
    }
}

pub trait State {
    fn handle(
        &self,
        entity: &Entity,
        character: &CharacterState,
        pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        stats: &Stats,
        body: &Body,
        physics: &PhysicsState,
        server_bus: &EventBus<ServerEvent>,
        local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState;
}

impl State for RunData {
    fn handle(
        &self,
        _entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        stats: &Stats,
        body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Move player according to move_dir
        vel.0 += Vec2::broadcast(dt.0)
            * inputs.move_dir
            * if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) {
                HUMANOID_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir = if character.action.is_attack() || character.action.is_block() {
            Vec2::from(inputs.look_dir).normalized()
        } else {
            Vec2::from(vel.0)
        };

        if ori_dir.magnitude_squared() > 0.0001
            && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared() > 0.001
        {
            ori.0 = vek::ops::Slerp::slerp(ori.0, ori_dir.into(), 9.0 * dt.0);
        }

        // Try to sit
        if inputs.sit.is_pressed() && physics.on_ground && body.is_humanoid() {
            return CharacterState {
                movement: Sit(SitData),
                action: Idle,
            };
        }

        // Try to climb
        if let (true, Some(_wall_dir)) = (
            inputs.climb.is_pressed() | inputs.climb_down.is_pressed() && body.is_humanoid(),
            physics.on_wall,
        ) {
            return CharacterState {
                movement: Climb(ClimbData),
                action: Idle,
            };
        }

        // Try to swim
        if !physics.on_ground && physics.in_fluid {
            return CharacterState {
                action: character.action,
                movement: Swim(SwimData),
            };
        }

        // While on ground ...
        if physics.on_ground {
            // Try to jump
            if inputs.jump.is_pressed() && !inputs.jump.is_held_down() {
                return CharacterState {
                    action: character.action,
                    movement: Jump(JumpData),
                };
            }

            // Try to charge
            if inputs.charge.is_pressed() && !inputs.charge.is_held_down() {
                return CharacterState {
                    action: Charge {
                        time_left: Duration::from_millis(250),
                    },
                    movement: Run(RunData),
                };
            }

            // Try to roll
            if inputs.roll.is_pressed() && body.is_humanoid() {
                return CharacterState {
                    action: Roll {
                        time_left: Duration::from_millis(600),
                        was_wielding: character.action.is_wield(),
                    },
                    movement: Run(RunData),
                };
            }
        }
        // While not on ground ...
        else {
            // Try to glide
            if physics.on_wall == None
                && inputs.glide.is_pressed()
                && !inputs.glide.is_held_down()
                && body.is_humanoid()
            {
                return CharacterState {
                    action: Idle,
                    movement: Glide(GlideData),
                };
            }
            return CharacterState {
                action: character.action,
                movement: Fall(FallData),
            };
        }

        // Tool Actions
        if inputs.toggle_wield.is_just_pressed() {
            match character.action {
                Wield { .. } | Attack { .. } => {
                    // Prevent instantaneous reequipping by checking
                    // for done wielding
                    if character.action.is_action_finished() {
                        return CharacterState {
                            action: Idle,
                            movement: character.movement,
                        };
                    }
                }
                Idle => {
                    return CharacterState {
                        // Try to wield if an item is equipped in main hand
                        action: if let Some(ItemKind::Tool { kind, .. }) =
                            stats.equipment.main.as_ref().map(|i| &i.kind)
                        {
                            let wield_duration = kind.wield_duration();
                            Wield {
                                time_left: wield_duration,
                            }
                        } else {
                            Idle
                        },
                        movement: character.movement,
                    };
                }
                Charge { .. } | Roll { .. } | Block { .. } => {}
            }
        }
        if inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }

        if inputs.move_dir.magnitude_squared() > 0.0 {
            return CharacterState {
                action: character.action,
                movement: Run(RunData),
            };
        } else {
            return CharacterState {
                action: character.action,
                movement: Stand(StandData),
            };
        }
    }
}

impl State for StandData {
    fn handle(
        &self,
        _entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        _vel: &mut Vel,
        _ori: &mut Ori,
        _dt: &DeltaTime,
        inputs: &ControllerInputs,
        stats: &Stats,
        body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Try to sit
        if inputs.sit.is_pressed() && physics.on_ground && body.is_humanoid() {
            return CharacterState {
                movement: Sit(SitData),
                action: Idle,
            };
        }

        // Try to climb
        if let (true, Some(_wall_dir)) = (
            inputs.climb.is_pressed() | inputs.climb_down.is_pressed() && body.is_humanoid(),
            physics.on_wall,
        ) {
            return CharacterState {
                movement: Climb(ClimbData),
                action: Idle,
            };
        }

        // Try to swim
        if !physics.on_ground && physics.in_fluid {
            return CharacterState {
                action: character.action,
                movement: Swim(SwimData),
            };
        }

        // While on ground ...
        if physics.on_ground {
            // Try to jump
            if inputs.jump.is_pressed() {
                return CharacterState {
                    action: character.action,
                    movement: Jump(JumpData),
                };
            }

            // Try to charge
            if inputs.charge.is_pressed() && !inputs.charge.is_held_down() {
                return CharacterState {
                    action: Charge {
                        time_left: Duration::from_millis(250),
                    },
                    movement: Run(RunData),
                };
            }

            // Try to roll
            if inputs.roll.is_pressed() && body.is_humanoid() {
                return CharacterState {
                    action: Roll {
                        time_left: Duration::from_millis(600),
                        was_wielding: character.action.is_wield(),
                    },
                    movement: Run(RunData),
                };
            }
        }
        // While not on ground ...
        else {
            // Try to glide
            if physics.on_wall == None
                && inputs.glide.is_pressed()
                && !inputs.glide.is_held_down()
                && body.is_humanoid()
            {
                return CharacterState {
                    action: Idle,
                    movement: Glide(GlideData),
                };
            }
            return CharacterState {
                action: character.action,
                movement: Fall(FallData),
            };
        }

        // Tool Actions
        if inputs.toggle_wield.is_just_pressed() {
            match character.action {
                Wield { .. } | Attack { .. } => {
                    // Prevent instantaneous reequipping by checking
                    // for done wielding
                    if character.action.is_action_finished() {
                        return CharacterState {
                            action: Idle,
                            movement: character.movement,
                        };
                    }
                }
                Idle => {
                    return CharacterState {
                        // Try to wield if an item is equipped in main hand
                        action: if let Some(ItemKind::Tool { kind, .. }) =
                            stats.equipment.main.as_ref().map(|i| &i.kind)
                        {
                            let wield_duration = kind.wield_duration();
                            Wield {
                                time_left: wield_duration,
                            }
                        } else {
                            Idle
                        },
                        movement: character.movement,
                    };
                }
                Charge { .. } | Roll { .. } | Block { .. } => {}
            }
        }
        if inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }

        if inputs.move_dir.magnitude_squared() > 0.0 {
            return CharacterState {
                action: character.action,
                movement: Run(RunData),
            };
        } else {
            return CharacterState {
                action: character.action,
                movement: Stand(StandData),
            };
        }
    }
}

impl State for SitData {
    fn handle(
        &self,
        _entity: &Entity,
        _character: &CharacterState,
        _pos: &mut Pos,
        _vel: &mut Vel,
        _ori: &mut Ori,
        _dt: &DeltaTime,
        inputs: &ControllerInputs,
        _stats: &Stats,
        _body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Falling
        // Idk, maybe the ground disappears,
        // suddenly maybe a water spell appears.
        // Can't hurt to be safe :shrug:
        if !physics.on_ground {
            if physics.in_fluid {
                return CharacterState {
                    action: Idle,
                    movement: Swim(SwimData),
                };
            } else {
                return CharacterState {
                    action: Idle,
                    movement: Fall(FallData),
                };
            }
        }
        // Jumping
        if inputs.jump.is_pressed() {
            return CharacterState {
                action: Idle,
                movement: Jump(JumpData),
            };
        }

        // Moving
        if inputs.move_dir.magnitude_squared() > 0.0 {
            return CharacterState {
                action: Idle,
                movement: Run(RunData),
            };
        }

        // Standing back up (unsitting)
        if inputs.sit.is_just_pressed() {
            return CharacterState {
                action: Idle,
                movement: Stand(StandData),
            };
        }

        // no movement has occurred
        return CharacterState {
            action: Idle,
            movement: Sit(SitData),
        };
    }
}

impl State for JumpData {
    fn handle(
        &self,
        entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        _vel: &mut Vel,
        _ori: &mut Ori,
        _dt: &DeltaTime,
        _inputs: &ControllerInputs,
        _stats: &Stats,
        _body: &Body,
        _physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        local_bus.emitter().emit(LocalEvent::Jump(*entity));

        return CharacterState {
            action: character.action,
            movement: Fall(FallData),
        };
    }
}

impl State for FallData {
    fn handle(
        &self,
        _entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        stats: &Stats,
        _body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Move player according to move_dir
        vel.0 += Vec2::broadcast(dt.0)
            * inputs.move_dir
            * if vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) {
                HUMANOID_AIR_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir = if character.action.is_attack() || character.action.is_block() {
            Vec2::from(inputs.look_dir).normalized()
        } else {
            Vec2::from(vel.0)
        };

        if ori_dir.magnitude_squared() > 0.0001
            && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared() > 0.001
        {
            ori.0 = vek::ops::Slerp::slerp(ori.0, ori_dir.into(), 2.0 * dt.0);
        }

        let mut new_action = character.action;

        // Update actions
        match character.action {
            // Unwield if buttons pressed
            Wield { .. } | Attack { .. } => {
                if inputs.toggle_wield.is_just_pressed() {
                    new_action = Idle;
                }
            }
            // Try to wield if any of buttons pressed
            Idle => {
                if inputs.primary.is_pressed() || inputs.secondary.is_pressed() {
                    new_action = if let Some(ItemKind::Tool { kind, .. }) =
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
            }
            // Cancel blocks
            Block { .. } => {
                new_action = if let Some(ItemKind::Tool { kind, .. }) =
                    stats.equipment.main.as_ref().map(|i| &i.kind)
                {
                    let wield_duration = kind.wield_duration();
                    Wield {
                        time_left: wield_duration,
                    }
                } else {
                    Idle
                };
            }
            // Don't change action
            Charge { .. } | Roll { .. } => {}
        };

        // Gliding
        if inputs.glide.is_pressed() && !inputs.glide.is_held_down() {
            return CharacterState {
                action: Idle,
                movement: Glide(GlideData),
            };
        }

        // Reset to Falling while not standing on ground,
        // otherwise keep the state given above
        if !physics.on_ground {
            if physics.in_fluid {
                return CharacterState {
                    action: new_action,
                    movement: Swim(SwimData),
                };
            } else {
                return CharacterState {
                    action: new_action,
                    movement: Fall(FallData),
                };
            }
        }
        // On ground
        else {
            // Return to running or standing based on move inputs
            return CharacterState {
                action: new_action,
                movement: if inputs.move_dir.magnitude_squared() > 0.0 {
                    Run(RunData)
                } else {
                    Stand(StandData)
                },
            };
        }
    }
}
impl State for GlideData {
    fn handle(
        &self,
        _entity: &Entity,
        _character: &CharacterState,
        _pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        _stats: &Stats,
        _body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Move player according to move_dir
        vel.0 += Vec2::broadcast(dt.0)
            * inputs.move_dir
            * if vel.0.magnitude_squared() < GLIDE_SPEED.powf(2.0) {
                GLIDE_ACCEL
            } else {
                0.0
            };

        let ori_dir = Vec2::from(vel.0);

        if ori_dir.magnitude_squared() > 0.0001
            && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared() > 0.001
        {
            ori.0 = vek::ops::Slerp::slerp(ori.0, ori_dir.into(), 2.0 * dt.0);
        }

        // Apply Glide lift
        if Vec2::<f32>::from(vel.0).magnitude_squared() < GLIDE_SPEED.powf(2.0) && vel.0.z < 0.0 {
            let lift = GLIDE_ANTIGRAV + vel.0.z.abs().powf(2.0) * 0.15;
            vel.0.z += dt.0
                * lift
                * (Vec2::<f32>::from(vel.0).magnitude() * 0.075)
                    .min(1.0)
                    .max(0.2);
        }

        if !inputs.glide.is_pressed() {
            return CharacterState {
                action: Idle,
                movement: Fall(FallData),
            };
        } else if let Some(_wall_dir) = physics.on_wall {
            return CharacterState {
                action: Idle,
                movement: Climb(ClimbData),
            };
        }

        if physics.on_ground {
            return CharacterState {
                action: Idle,
                movement: Stand(StandData),
            };
        }

        return CharacterState {
            action: Idle,
            movement: Glide(GlideData),
        };
    }
}
impl State for ClimbData {
    fn handle(
        &self,
        _entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        _stats: &Stats,
        _body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Move player according to move_dir
        vel.0 += Vec2::broadcast(dt.0)
            * inputs.move_dir
            * if vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) {
                HUMANOID_CLIMB_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir = if let Some(wall_dir) = physics.on_wall {
            if Vec2::<f32>::from(wall_dir).magnitude_squared() > 0.001 {
                Vec2::from(wall_dir).normalized()
            } else {
                Vec2::from(vel.0)
            }
        } else {
            Vec2::from(vel.0)
        };

        if ori_dir.magnitude_squared() > 0.0001
            && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared() > 0.001
        {
            ori.0 = vek::ops::Slerp::slerp(
                ori.0,
                ori_dir.into(),
                if physics.on_ground { 9.0 } else { 2.0 } * dt.0,
            );
        }

        // Apply Vertical Climbing Movement
        if let (true, Some(_wall_dir)) = (
            (inputs.climb.is_pressed() | inputs.climb_down.is_pressed()) && vel.0.z <= CLIMB_SPEED,
            physics.on_wall,
        ) {
            if inputs.climb_down.is_pressed() && !inputs.climb.is_pressed() {
                vel.0 -= dt.0 * vel.0.map(|e| e.abs().powf(1.5) * e.signum() * 6.0);
            } else if inputs.climb.is_pressed() && !inputs.climb_down.is_pressed() {
                vel.0.z = (vel.0.z + dt.0 * GRAVITY * 1.25).min(CLIMB_SPEED);
            } else {
                vel.0.z = vel.0.z + dt.0 * GRAVITY * 1.5;
                vel.0 = Lerp::lerp(
                    vel.0,
                    Vec3::zero(),
                    30.0 * dt.0 / (1.0 - vel.0.z.min(0.0) * 5.0),
                );
            }
        }

        if let None = physics.on_wall {
            if inputs.jump.is_pressed() {
                return CharacterState {
                    action: Idle,
                    movement: Jump(JumpData),
                };
            } else {
                return CharacterState {
                    action: Idle,
                    movement: Fall(FallData),
                };
            }
        }
        if physics.on_ground {
            return CharacterState {
                action: Idle,
                movement: Stand(StandData),
            };
        }

        return *character;
    }
}
impl State for SwimData {
    fn handle(
        &self,
        _entity: &Entity,
        character: &CharacterState,
        _pos: &mut Pos,
        vel: &mut Vel,
        ori: &mut Ori,
        dt: &DeltaTime,
        inputs: &ControllerInputs,
        _stats: &Stats,
        _body: &Body,
        physics: &PhysicsState,
        _server_bus: &EventBus<ServerEvent>,
        _local_bus: &EventBus<LocalEvent>,
    ) -> CharacterState {
        // Update velocity
        vel.0 += Vec2::broadcast(dt.0)
            * inputs.move_dir
            * if vel.0.magnitude_squared() < HUMANOID_WATER_SPEED.powf(2.0) {
                HUMANOID_WATER_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir = if character.action.is_attack() || character.action.is_block() {
            Vec2::from(inputs.look_dir).normalized()
        } else {
            Vec2::from(vel.0)
        };

        if ori_dir.magnitude_squared() > 0.0001
            && (ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared() > 0.001
        {
            ori.0 = vek::ops::Slerp::slerp(
                ori.0,
                ori_dir.into(),
                if physics.on_ground { 9.0 } else { 2.0 } * dt.0,
            );
        }

        if inputs.jump.is_pressed() {
            vel.0.z = (vel.0.z + dt.0 * GRAVITY * 1.25).min(HUMANOID_WATER_SPEED);
        }

        if inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }

        // Not on ground
        if !physics.on_ground {
            return CharacterState {
                action: character.action,
                movement: Swim(SwimData),
            };
        }
        // On ground
        else {
            // Return to running or standing based on move inputs
            return CharacterState {
                action: character.action,
                movement: if inputs.move_dir.magnitude_squared() > 0.0 {
                    Run(RunData)
                } else {
                    Stand(StandData)
                },
            };
        }
    }
}
