use specs::{
    shred, Entities, LazyUpdate, LendJoin, Read, ReadExpect, ReadStorage, SystemData, WriteStorage,
};

use common::{
    comp::{
        self,
        character_state::{CharacterStateEvents, OutputEvents},
        inventory::item::{tool::AbilityMap, MaterialStatManifest},
        ActiveAbilities, Beam, Body, CharacterActivity, CharacterState, Combo, Controller, Density,
        Energy, Health, Inventory, InventoryManip, Mass, Melee, Ori, PhysicsState, Poise, Pos,
        Scale, SkillSet, Stance, StateUpdate, Stats, Vel,
    },
    event::{self, EventBus, KnockbackEvent, LocalEvent},
    link::Is,
    mounting::{Rider, VolumeRider},
    outcome::Outcome,
    resources::{DeltaTime, Time},
    states::{
        behavior::{JoinData, JoinStruct},
        idle,
    },
    terrain::TerrainGrid,
    uid::Uid,
};
use common_ecs::{Job, Origin, Phase, System};

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    events: CharacterStateEvents<'a>,
    local_bus: Read<'a, EventBus<LocalEvent>>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    lazy_update: Read<'a, LazyUpdate>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    masses: ReadStorage<'a, Mass>,
    scales: ReadStorage<'a, Scale>,
    physics_states: ReadStorage<'a, PhysicsState>,
    melee_attacks: ReadStorage<'a, Melee>,
    beams: ReadStorage<'a, Beam>,
    uids: ReadStorage<'a, Uid>,
    is_riders: ReadStorage<'a, Is<Rider>>,
    is_volume_riders: ReadStorage<'a, Is<VolumeRider>>,
    stats: ReadStorage<'a, Stats>,
    skill_sets: ReadStorage<'a, SkillSet>,
    active_abilities: ReadStorage<'a, ActiveAbilities>,
    msm: ReadExpect<'a, MaterialStatManifest>,
    ability_map: ReadExpect<'a, AbilityMap>,
    combos: ReadStorage<'a, Combo>,
    alignments: ReadStorage<'a, comp::Alignment>,
    terrain: ReadExpect<'a, TerrainGrid>,
    inventories: ReadStorage<'a, Inventory>,
    stances: ReadStorage<'a, Stance>,
}

/// ## Character Behavior System
/// Passes `JoinData` to `CharacterState`'s `behavior` handler fn's. Receives a
/// `StateUpdate` in return and performs updates to ECS Components from that.
#[derive(Default)]
pub struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, CharacterActivity>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Density>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, Poise>,
        Read<'a, EventBus<Outcome>>,
    );

    const NAME: &'static str = "character_behavior";
    const ORIGIN: Origin = Origin::Common;
    const PHASE: Phase = Phase::Create;

    fn run(
        _job: &mut Job<Self>,
        (
            read_data,
            mut character_states,
            mut character_activities,
            mut positions,
            mut velocities,
            mut orientations,
            mut densities,
            mut energies,
            mut controllers,
            mut poises,
            outcomes,
        ): Self::SystemData,
    ) {
        let mut local_emitter = read_data.local_bus.emitter();
        let mut outcomes_emitter = outcomes.emitter();
        let mut emitters = read_data.events.get_emitters();

        let mut local_events = Vec::new();
        let mut output_events = OutputEvents::new(&mut local_events, &mut emitters);

        let join = (
            &read_data.entities,
            &read_data.uids,
            &mut character_states,
            &mut character_activities,
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
            (
                &read_data.physics_states,
                read_data.scales.maybe(),
                &read_data.stats,
                &read_data.skill_sets,
                read_data.active_abilities.maybe(),
                read_data.is_riders.maybe(),
            ),
            read_data.combos.maybe(),
        )
            .lend_join();
        join.for_each(|comps| {
            let (
                entity,
                uid,
                mut char_state,
                character_activity,
                pos,
                vel,
                ori,
                mass,
                density,
                energy,
                inventory,
                controller,
                health,
                body,
                (physics, scale, stat, skill_set, active_abilities, is_rider),
                combo,
            ) = comps;
            // Being dead overrides all other states
            if health.map_or(false, |h| h.is_dead) {
                // Do nothing
                return;
            }

            // Remove components that entity should not have if not in relevant char state
            if !char_state.is_melee_attack() {
                read_data.lazy_update.remove::<Melee>(entity);
            }
            if !char_state.is_beam_attack() {
                read_data.lazy_update.remove::<Beam>(entity);
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
                    outcomes_emitter.emit(Outcome::PoiseChange {
                        pos,
                        state: poise_state,
                    });
                    if let Some(impulse_strength) = impulse_strength {
                        output_events.emit_server(KnockbackEvent {
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
                character_activity,
                pos,
                vel,
                ori,
                scale,
                mass,
                density,
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
                mount_data: read_data.is_riders.get(entity),
                volume_mount_data: read_data.is_volume_riders.get(entity),
                stance: read_data.stances.get(entity),
            };

            for action in actions {
                let j = JoinData::new(
                    &join_struct,
                    &read_data.lazy_update,
                    &read_data.dt,
                    &read_data.time,
                    &read_data.msm,
                    &read_data.ability_map,
                );
                let state_update = j.character.handle_event(&j, &mut output_events, action);
                Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
            }

            // Mounted occurs after control actions have been handled
            // If mounted, character state is controlled by mount
            if is_rider.is_some() && !join_struct.char_state.can_perform_mounted() {
                // TODO: A better way to swap between mount inputs and rider inputs
                *join_struct.char_state = CharacterState::Idle(idle::Data::default());
                return;
            }

            let j = JoinData::new(
                &join_struct,
                &read_data.lazy_update,
                &read_data.dt,
                &read_data.time,
                &read_data.msm,
                &read_data.ability_map,
            );

            let state_update = j.character.behavior(&j, &mut output_events);
            Self::publish_state_update(&mut join_struct, state_update, &mut output_events);
        });

        local_emitter.append_vec(local_events);
    }
}

impl Sys {
    fn publish_state_update(
        join: &mut JoinStruct,
        state_update: StateUpdate,
        output_events: &mut OutputEvents,
    ) {
        // Here we check for equality with the previous value of these components before
        // updating them so that the modification detection will not be
        // triggered unnecessarily. This is important for minimizing updates
        // sent to the clients (and thus keeping bandwidth usage down).
        //
        // TODO: if checking equality is expensive for char_state use optional field in
        // StateUpdate
        if *join.char_state != state_update.character {
            *join.char_state = state_update.character
        }
        if *join.character_activity != state_update.character_activity {
            *join.character_activity = state_update.character_activity
        }
        if *join.density != state_update.density {
            *join.density = state_update.density
        }
        if *join.energy != state_update.energy {
            *join.energy = state_update.energy;
        };

        // These components use a different type of change detection.
        *join.pos = state_update.pos;
        *join.vel = state_update.vel;
        *join.ori = state_update.ori;

        for (input, attr) in state_update.queued_inputs {
            join.controller.queued_inputs.insert(input, attr);
        }
        for input in state_update.removed_inputs {
            join.controller.queued_inputs.remove(&input);
        }
        if state_update.swap_equipped_weapons {
            output_events.emit_server(event::InventoryManipEvent(
                join.entity,
                InventoryManip::SwapEquippedWeapons,
            ));
        }
    }
}
