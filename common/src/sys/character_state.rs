use crate::{
    comp::{
        Body, CharacterState, Controller, EcsStateData, Mounting, MoveState::*, Ori,
        OverrideAction, OverrideMove, OverrideState, PhysicsState, Pos, SitState, StateHandle,
        Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
};

use specs::{Entities, Join, LazyUpdate, Read, ReadStorage, System, WriteStorage};
use sphynx::{Uid, UidAllocator};

/// # Character StateHandle System
/// #### Updates then detemrines next Character States based on ControllerInputs
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        Entities<'a>,
        Read<'a, UidAllocator>,
        Read<'a, EventBus<ServerEvent>>,
        Read<'a, EventBus<LocalEvent>>,
        Read<'a, DeltaTime>,
        Read<'a, LazyUpdate>,
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
        ReadStorage<'a, OverrideState>,
        ReadStorage<'a, OverrideMove>,
        ReadStorage<'a, OverrideAction>,
    );
    fn run(
        &mut self,
        (
            entities,
            _uid_allocator,
            server_bus,
            local_bus,
            dt,
            updater,
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
            state_overrides,
            move_overrides,
            action_overrides,
        ): Self::SystemData,
    ) {
        for (
            entity,
            uid,
            mut character,
            pos,
            vel,
            ori,
            controller,
            stats,
            body,
            physics,
            maybe_mount,
            maybe_move_override,
            maybe_action_override,
            (),
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
            move_overrides.maybe(),
            action_overrides.maybe(),
            !&state_overrides,
        )
            .join()
        {
            let inputs = &controller.inputs;

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
            if maybe_mount.is_some() {
                character.move_state = Sit(SitState);
                continue;
            }

            // Determine new move state if can move
            if !maybe_move_override.is_some() && !character.move_disabled {
                let state_update = character.move_state.handle(&EcsStateData {
                    entity: &entity,
                    uid,
                    character,
                    pos,
                    vel,
                    ori,
                    dt: &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    updater: &updater,
                    server_bus: &server_bus,
                    local_bus: &local_bus,
                });

                *character = state_update.character;
                *pos = state_update.pos;
                *vel = state_update.vel;
                *ori = state_update.ori;
            }

            // Determine new action if can_act
            if !maybe_action_override.is_some() && !character.action_disabled {
                let state_update = character.action_state.handle(&EcsStateData {
                    entity: &entity,
                    uid,
                    character,
                    pos,
                    vel,
                    ori,
                    dt: &dt,
                    inputs,
                    stats,
                    body,
                    physics,
                    updater: &updater,
                    server_bus: &server_bus,
                    local_bus: &local_bus,
                });

                *character = state_update.character;
                *pos = state_update.pos;
                *vel = state_update.vel;
                *ori = state_update.ori;
            }
        }
    }
}
