use crate::{
    comp::{
        AbilityPool, Body, CharacterState, Controller, EcsStateData, Mounting, Ori, PhysicsState,
        Pos, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    sync::{Uid, UidAllocator},
};

use specs::{Entities, Join, LazyUpdate, Read, ReadStorage, System, WriteStorage};

/// ## Character State System
/// #### Calls updates to `CharacterState`s. Acts on tuples of ( `CharacterState`, `Pos`, `Vel`, and `Ori` ).
///
/// _System forms `EcsStateData` tuples and passes those to `ActionState` `update()` fn,
/// then does the same for `MoveState` `update`_
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
        ReadStorage<'a, AbilityPool>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Mounting>,
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
            ability_pools,
            uids,
            mountings,
        ): Self::SystemData,
    ) {
        for (
            entity,
            uid,
            character,
            pos,
            vel,
            ori,
            controller,
            stats,
            body,
            physics,
            ability_pool,
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
            &ability_pools,
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
                *character = CharacterState::Sit(None);
                return;
            }

            let mut state_update = character.update(&EcsStateData {
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
                ability_pool,
            });

            *character = state_update.character;
            *pos = state_update.pos;
            *vel = state_update.vel;
            *ori = state_update.ori;
            local_bus.emitter().append(&mut state_update.local_events);
            server_bus.emitter().append(&mut state_update.server_events);
        }
    }
}
