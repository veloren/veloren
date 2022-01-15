use specs::{
    shred::ResourceId, Entities, Join, LazyUpdate, Read, ReadExpect, ReadStorage, SystemData,
    World, Write, WriteStorage,
};

use common::{
    comp::{
        self, character_state::OutputEvents, inventory::item::MaterialStatManifest,
        ActiveAbilities, Beam, Body, CharacterState, Combo, Controller, Density, Energy, Health,
        Inventory, InventoryManip, Mass, Melee, Ori, PhysicsState, Poise, Pos, SkillSet,
        StateUpdate, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    outcome::Outcome,
    resources::{DeltaTime, Time},
    states::{
        behavior::{JoinData, JoinStruct},
        idle,
    },
    terrain::TerrainGrid,
    uid::Uid,
    mounting::Rider,
    link::Is,
};
use common_ecs::{Job, Origin, Phase, System};

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    local_bus: Read<'a, EventBus<LocalEvent>>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    lazy_update: Read<'a, LazyUpdate>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    masses: ReadStorage<'a, Mass>,
    physics_states: ReadStorage<'a, PhysicsState>,
    melee_attacks: ReadStorage<'a, Melee>,
    beams: ReadStorage<'a, Beam>,
    uids: ReadStorage<'a, Uid>,
    is_riders: ReadStorage<'a, Is<Rider>>,
    stats: ReadStorage<'a, Stats>,
    skill_sets: ReadStorage<'a, SkillSet>,
    active_abilities: ReadStorage<'a, ActiveAbilities>,
    msm: Read<'a, MaterialStatManifest>,
    combos: ReadStorage<'a, Combo>,
    alignments: ReadStorage<'a, comp::Alignment>,
    terrain: ReadExpect<'a, TerrainGrid>,
    inventories: ReadStorage<'a, Inventory>,
}

/// ## Character Behavior System
/// Passes `JoinData` to `CharacterState`'s `behavior` handler fn's. Receives a
/// `StateUpdate` in return and performs updates to ECS Components from that.
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Density>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, Poise>,
        Write<'a, Vec<Outcome>>,
    );

    const NAME: &'static str = "character_behavior";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            read_data,
            mut character_states,
            mut positions,
            mut velocities,
            mut orientations,
            mut densities,
            mut energies,
            mut controllers,
            mut poises,
            mut outcomes,
        ): Self::SystemData,
    ) {
        let mut server_emitter = read_data.server_bus.emitter();
        let mut local_emitter = read_data.local_bus.emitter();

        let mut local_events = Vec::new();
        let mut server_events = Vec::new();
        let mut output_events = OutputEvents::new(&mut local_events, &mut server_events);

        for (
            entity,
            uid,
            mut char_state,
            pos,
            vel,
            ori,
            mass,
            mut density,
            energy,
            inventory,
            controller,
            health,
            body,
            physics,
            (stat, skill_set, active_abilities, is_rider),
            combo,
        ) in (
            &read_data.entities,
            &read_data.uids,
            &mut character_states,
            &mut positions,
            &mut velocities,
            &mut orientations,
            &read_data.masses,
            &mut densities,
            &mut energies,
            read_data.inventories.maybe(),
            &mut controllers,
            read_data.healths.maybe(),
            &read_data.bodies,
            &read_data.physics_states,
            (
                &read_data.stats,
                &read_data.skill_sets,
                &read_data.active_abilities,
                read_data.is_riders.maybe(),
            ),
            &read_data.combos,
        )
            .join()
        {
            // Being dead overrides all other states
            if health.map_or(false, |h| h.is_dead) {
                // Do nothing
                continue;
            }

            // Enter stunned state if poise damage is enough
            if let Some(mut poise) = poises.get_mut(entity) {
                let was_wielded = char_state.is_wield();
                let poise_state = poise.poise_state();
                let pos = pos.0;
                if let (Some((stunned_state, stunned_duration)), impulse_strength) =
                    poise_state.poise_effect(was_wielded)
                {
                    // Reset poise if there is some stunned state to apply
                    poise.reset(*read_data.time, stunned_duration);
                    *char_state = stunned_state;
                    outcomes.push(Outcome::PoiseChange {
                        pos,
                        state: poise_state,
                    });
                    if let Some(impulse_strength) = impulse_strength {
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: impulse_strength * *poise.knockback(),
                        });
                    }
                }
            }

            // Controller actions
            let actions = std::mem::take(&mut controller.actions);

            let mut join_struct = JoinStruct {
                entity,
                uid,
                char_state,
                pos,
                vel,
                ori,
                mass,
                density: &mut density,
                energy,
                inventory,
                controller,
                health,
                body,
                physics,
                melee_attack: read_data.melee_attacks.get(entity),
                beam: read_data.beams.get(entity),
                stat,
                skill_set,
                active_abilities,
                combo,
                alignment: read_data.alignments.get(entity),
                terrain: &read_data.terrain,
            };

            for action in actions {
                let j = JoinData::new(
                    &join_struct,
                    &read_data.lazy_update,
                    &read_data.dt,
                    &read_data.msm,
                );
                let state_update = j.character.handle_event(&j, &mut output_events, action);
                Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
            }

            // Mounted occurs after control actions have been handled
            // If mounted, character state is controlled by mount
            if is_rider.is_some() {
                let idle_state = CharacterState::Idle(idle::Data { is_sneaking: false });
                if *join_struct.char_state != idle_state {
                    *join_struct.char_state = idle_state;
                }
                continue;
            }

            let j = JoinData::new(
                &join_struct,
                &read_data.lazy_update,
                &read_data.dt,
                &read_data.msm,
            );

            let state_update = j.character.behavior(&j, &mut output_events);
            Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
        }

        local_emitter.append_vec(local_events);
        server_emitter.append_vec(server_events);
    }
}

impl Sys {
    fn publish_state_update(
        join: &mut JoinStruct,
        mut state_update: StateUpdate,
        output_events: &mut OutputEvents,
    ) {
        // TODO: if checking equality is expensive use optional field in StateUpdate
        if *join.char_state != state_update.character {
            *join.char_state = state_update.character
        };
        *join.pos = state_update.pos;
        *join.vel = state_update.vel;
        *join.ori = state_update.ori;
        *join.density = state_update.density;
        *join.energy = state_update.energy;
        join.controller
            .queued_inputs
            .append(&mut state_update.queued_inputs);
        for input in state_update.removed_inputs {
            join.controller.queued_inputs.remove(&input);
        }
        if state_update.swap_equipped_weapons {
            output_events.emit_server(ServerEvent::InventoryManip(
                join.entity,
                InventoryManip::SwapEquippedWeapons,
            ));
        }
    }
}
