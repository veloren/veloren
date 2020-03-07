use crate::{
    comp::{
        AbilityPool, Body, CharacterState, Controller, ControllerInputs, Energy, Mounting, Ori,
        PhysicsState, Pos, StateUpdate, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    states,
    sync::{Uid, UidAllocator},
};

use specs::{Entities, Entity, Join, LazyUpdate, Read, ReadStorage, System, WriteStorage};

use std::collections::VecDeque;

/// Read-Only Data sent from Character Behavior System to bahvior fn's
pub struct JoinData<'a> {
    pub entity: Entity,
    pub uid: &'a Uid,
    pub character: &'a CharacterState,
    pub pos: &'a Pos,
    pub vel: &'a Vel,
    pub ori: &'a Ori,
    pub dt: &'a DeltaTime,
    pub controller: &'a Controller,
    pub inputs: &'a ControllerInputs,
    pub stats: &'a Stats,
    pub energy: &'a Energy,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub ability_pool: &'a AbilityPool,
    pub updater: &'a LazyUpdate,
}

pub type JoinTuple<'a> = (
    Entity,
    &'a Uid,
    &'a mut CharacterState,
    &'a mut Pos,
    &'a mut Vel,
    &'a mut Ori,
    &'a mut Energy,
    &'a Controller,
    &'a Stats,
    &'a Body,
    &'a PhysicsState,
    &'a AbilityPool,
);

impl<'a> JoinData<'a> {
    fn new(j: &'a JoinTuple<'a>, updater: &'a LazyUpdate, dt: &'a DeltaTime) -> Self {
        Self {
            entity: j.0,
            uid: j.1,
            character: j.2,
            pos: j.3,
            vel: j.4,
            ori: j.5,
            energy: j.6,
            controller: j.7,
            inputs: &j.7.inputs,
            stats: j.8,
            body: j.9,
            physics: j.10,
            ability_pool: j.11,
            updater,
            dt,
        }
    }
}

/// /// ## Character State System
/// #### Calls updates to `CharacterState`s. Acts on tuples of (
/// `CharacterState`, `Pos`, `Vel`, and `Ori` ).
///
/// _System forms `CharacterEntityData` tuples and passes those to `ActionState`
/// `update()` fn, then does the same for `MoveState` `update`_
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
        WriteStorage<'a, Energy>,
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
            mut energies,
            controllers,
            stats,
            bodies,
            physics_states,
            ability_pools,
            uids,
            mountings,
        ): Self::SystemData,
    ) {
        let mut join_iter = (
            &entities,
            &uids,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &mut energies,
            &controllers,
            &stats,
            &bodies,
            &physics_states,
            &ability_pools,
        )
            .join();

        while let Some(tuple) = join_iter.next() {
            let j = JoinData::new(&tuple, &updater, &dt);
            let inputs = &j.inputs;

            // Being dead overrides all other states
            if j.stats.is_dead {
                // Only options: click respawn
                // prevent instant-respawns (i.e. player was holding attack)
                // by disallowing while input is held down
                if inputs.respawn.is_pressed() && !inputs.respawn.is_held_down() {
                    server_bus.emitter().emit(ServerEvent::Respawn(j.entity));
                }
                // Or do nothing
                return;
            }
            // If mounted, character state is controlled by mount
            // TODO: Make mounting a state
            if let Some(Mounting(_)) = mountings.get(j.entity) {
                *tuple.2 = CharacterState::Sit {};
                return;
            }

            let mut state_update = match j.character {
                CharacterState::Idle { .. } => states::idle::behavior(&j),
                CharacterState::Climb { .. } => states::climb::behavior(&j),
                CharacterState::Glide { .. } => states::glide::behavior(&j),
                CharacterState::Roll { .. } => states::roll::behavior(&j),
                CharacterState::Wielding { .. } => states::wielding::behavior(&j),
                CharacterState::Equipping { .. } => states::equipping::behavior(&j),
                CharacterState::BasicAttack { .. } => states::basic_attack::behavior(&j),
                CharacterState::BasicBlock { .. } => states::basic_block::behavior(&j),
                CharacterState::Sit { .. } => states::sit::behavior(&j),

                _ => StateUpdate {
                    character: *j.character,
                    pos: *j.pos,
                    vel: *j.vel,
                    ori: *j.ori,
                    energy: *j.energy,
                    local_events: VecDeque::new(),
                    server_events: VecDeque::new(),
                },
            };

            *tuple.2 = state_update.character;
            *tuple.3 = state_update.pos;
            *tuple.4 = state_update.vel;
            *tuple.5 = state_update.ori;
            *tuple.6 = state_update.energy;
            local_bus.emitter().append(&mut state_update.local_events);
            server_bus.emitter().append(&mut state_update.server_events);
        }
    }
}
