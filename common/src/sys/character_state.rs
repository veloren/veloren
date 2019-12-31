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

/// # Character State System
/// #### Updates tuples of ( `CharacterState`, `Pos`, `Vel`, and `Ori` ) in parallel.
/// _Each update for a single character involves first passing an `EcsStateData` struct of ECS components
///  to the character's `MoveState`, then the character's `ActionState`. State update logic is
///  is encapsulated in state's `handle()` fn, impl'd by the `StateHandle` trait. `handle()` fn's
///  return a `StateUpdate` tuple containing new ( `CharacterState`, `Pos`, `Vel`, and `Ori` ) components.
///  Since `handle()` accepts readonly components, component updates are contained within this system and ECS
///  behavior constraints are satisfied._
///
///  _This mimics the typical OOP style state machine pattern, but remains performant
///  under ECS since trait fn's are syntactic sugar for static fn's that accept their implementor's
///  object type as its first parameter. See `StateHandle` for more information._
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
        for (entity, uid, mut character, pos, vel, ori, controller, stats, body, physics, ()) in (
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
                return;
            }
            // If mounted, character state is controlled by mount
            // TODO: Make mounting a state
            if let Some(Mounting(_)) = mountings.get(entity) {
                character.move_state = Sit(SitState);
                return;
            }

            // Determine new move state if character can move
            if let (None, false) = (
                move_overrides.get(entity),
                character.move_disabled_this_tick,
            ) {
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

            // Reset disabled every tick. Should be
            // set every tick by states that use it.
            character.move_disabled_this_tick = false;

            // Determine new action if character can act
            if let (None, false) = (
                action_overrides.get(entity),
                character.action_disabled_this_tick,
            ) {
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

            // Reset disabled every tick. Should
            // be set every tick by states that use it.
            character.action_disabled_this_tick = false;
        }
    }
}
