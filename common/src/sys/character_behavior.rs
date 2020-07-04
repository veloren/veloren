use crate::{
    comp::{
        Attacking, Body, CharacterState, ControlAction, Controller, ControllerInputs, Energy,
        Loadout, Mounting, Ori, PhysicsState, Pos, StateUpdate, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    state::DeltaTime,
    states,
    sync::{Uid, UidAllocator},
};

use specs::{Entities, Entity, Join, LazyUpdate, Read, ReadStorage, System, WriteStorage};

// use std::collections::VecDeque;

pub trait CharacterBehavior {
    fn behavior(&self, data: &JoinData) -> StateUpdate;
    // Impl these to provide behavior for these inputs
    fn swap_loadout(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn wield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn glide_wield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn unwield(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn sit(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn dance(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn stand(&self, data: &JoinData) -> StateUpdate { StateUpdate::from(data) }
    fn handle_event(&self, data: &JoinData, event: ControlAction) -> StateUpdate {
        match event {
            ControlAction::SwapLoadout => self.swap_loadout(data),
            ControlAction::Wield => self.wield(data),
            ControlAction::GlideWield => self.glide_wield(data),
            ControlAction::Unwield => self.unwield(data),
            ControlAction::Sit => self.sit(data),
            ControlAction::Dance => self.dance(data),
            ControlAction::Stand => self.stand(data),
        }
    }
    // fn init(data: &JoinData) -> CharacterState;
}

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
    pub loadout: &'a Loadout,
    pub body: &'a Body,
    pub physics: &'a PhysicsState,
    pub attacking: Option<&'a Attacking>,
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
    &'a mut Loadout,
    &'a mut Controller,
    &'a Stats,
    &'a Body,
    &'a PhysicsState,
    Option<&'a Attacking>,
);

fn incorporate_update(tuple: &mut JoinTuple, state_update: StateUpdate) {
    *tuple.2 = state_update.character;
    *tuple.3 = state_update.pos;
    *tuple.4 = state_update.vel;
    *tuple.5 = state_update.ori;
    *tuple.6 = state_update.energy;
    *tuple.7 = state_update.loadout;
}

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
            loadout: j.7,
            controller: j.8,
            inputs: &j.8.inputs,
            stats: j.9,
            body: j.10,
            physics: j.11,
            attacking: j.12,
            updater,
            dt,
        }
    }
}

/// ## Character Behavior System
/// Passes `JoinData` to `CharacterState`'s `behavior` handler fn's. Recieves a
/// `StateUpdate` in return and performs updates to ECS Components from that.
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
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
        WriteStorage<'a, Loadout>,
        WriteStorage<'a, Controller>,
        ReadStorage<'a, Stats>,
        ReadStorage<'a, Body>,
        ReadStorage<'a, PhysicsState>,
        ReadStorage<'a, Attacking>,
        ReadStorage<'a, Uid>,
        ReadStorage<'a, Mounting>,
    );

    #[allow(clippy::while_let_on_iterator)] // TODO: Pending review in #587
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
            mut loadouts,
            mut controllers,
            stats,
            bodies,
            physics_states,
            attacking_storage,
            uids,
            mountings,
        ): Self::SystemData,
    ) {
        let mut server_emitter = server_bus.emitter();
        let mut local_emitter = local_bus.emitter();

        let mut join_iter = (
            &entities,
            &uids,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &mut energies,
            &mut loadouts,
            &mut controllers,
            &stats,
            &bodies,
            &physics_states,
            attacking_storage.maybe(),
        )
            .join();

        while let Some(mut tuple) = join_iter.next() {
            // Being dead overrides all other states
            if tuple.9.is_dead {
                // Do nothing
                continue;
            }
            // If mounted, character state is controlled by mount
            // TODO: Make mounting a state
            if let Some(Mounting(_)) = mountings.get(tuple.0) {
                *tuple.2 = CharacterState::Sit {};
                continue;
            }

            let actions = std::mem::replace(&mut tuple.8.actions, Vec::new());
            for action in actions {
                let j = JoinData::new(&tuple, &updater, &dt);
                let mut state_update = match j.character {
                    CharacterState::Idle => states::idle::Data.handle_event(&j, action),
                    CharacterState::Climb => states::climb::Data.handle_event(&j, action),
                    CharacterState::Glide => states::glide::Data.handle_event(&j, action),
                    CharacterState::GlideWield => {
                        states::glide_wield::Data.handle_event(&j, action)
                    },
                    CharacterState::Sit => {
                        states::sit::Data::handle_event(&states::sit::Data, &j, action)
                    },
                    CharacterState::Dance => {
                        states::dance::Data::handle_event(&states::dance::Data, &j, action)
                    },
                    CharacterState::BasicBlock => {
                        states::basic_block::Data.handle_event(&j, action)
                    },
                    CharacterState::Roll(data) => data.handle_event(&j, action),
                    CharacterState::Wielding => states::wielding::Data.handle_event(&j, action),
                    CharacterState::Equipping(data) => data.handle_event(&j, action),
                    CharacterState::TripleStrike(data) => data.handle_event(&j, action),
                    CharacterState::BasicMelee(data) => data.handle_event(&j, action),
                    CharacterState::BasicRanged(data) => data.handle_event(&j, action),
                    CharacterState::Boost(data) => data.handle_event(&j, action),
                    CharacterState::DashMelee(data) => data.handle_event(&j, action),
                    CharacterState::LeapMelee(data) => data.handle_event(&j, action),
                };
                local_emitter.append(&mut state_update.local_events);
                server_emitter.append(&mut state_update.server_events);
                incorporate_update(&mut tuple, state_update);
            }

            let j = JoinData::new(&tuple, &updater, &dt);

            let mut state_update = match j.character {
                CharacterState::Idle => states::idle::Data.behavior(&j),
                CharacterState::Climb => states::climb::Data.behavior(&j),
                CharacterState::Glide => states::glide::Data.behavior(&j),
                CharacterState::GlideWield => states::glide_wield::Data.behavior(&j),
                CharacterState::Sit => states::sit::Data::behavior(&states::sit::Data, &j),
                CharacterState::Dance => states::dance::Data::behavior(&states::dance::Data, &j),
                CharacterState::BasicBlock => states::basic_block::Data.behavior(&j),
                CharacterState::Roll(data) => data.behavior(&j),
                CharacterState::Wielding => states::wielding::Data.behavior(&j),
                CharacterState::Equipping(data) => data.behavior(&j),
                CharacterState::TripleStrike(data) => data.behavior(&j),
                CharacterState::BasicMelee(data) => data.behavior(&j),
                CharacterState::BasicRanged(data) => data.behavior(&j),
                CharacterState::Boost(data) => data.behavior(&j),
                CharacterState::DashMelee(data) => data.behavior(&j),
                CharacterState::LeapMelee(data) => data.behavior(&j),
            };

            local_emitter.append(&mut state_update.local_events);
            server_emitter.append(&mut state_update.server_events);
            incorporate_update(&mut tuple, state_update);
        }
    }
}
