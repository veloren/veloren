use specs::{
    shred::ResourceId, Entities, Join, LazyUpdate, Read, ReadExpect, ReadStorage, SystemData,
    World, Write, WriteStorage,
};

use common::{
    comp::{
        self, inventory::item::MaterialStatManifest, Beam, Body, CharacterState, Combo, Controller,
        Density, Energy, Health, Inventory, InventoryManip, Mass, Melee, Mounting, Ori,
        PhysicsState, Poise, PoiseState, Pos, SkillSet, StateUpdate, Stats, Vel,
    },
    event::{Emitter, EventBus, LocalEvent, ServerEvent},
    outcome::Outcome,
    resources::DeltaTime,
    states::behavior::{JoinData, JoinStruct},
    terrain::TerrainGrid,
    uid::Uid,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, Phase, System};
use std::time::Duration;

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    local_bus: Read<'a, EventBus<LocalEvent>>,
    dt: Read<'a, DeltaTime>,
    lazy_update: Read<'a, LazyUpdate>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    masses: ReadStorage<'a, Mass>,
    physics_states: ReadStorage<'a, PhysicsState>,
    melee_attacks: ReadStorage<'a, Melee>,
    beams: ReadStorage<'a, Beam>,
    uids: ReadStorage<'a, Uid>,
    mountings: ReadStorage<'a, Mounting>,
    stats: ReadStorage<'a, Stats>,
    skill_sets: ReadStorage<'a, SkillSet>,
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

        for (
            entity,
            uid,
            mut char_state,
            mut pos,
            mut vel,
            mut ori,
            mass,
            mut density,
            energy,
            inventory,
            mut controller,
            health,
            body,
            physics,
            (stat, skill_set),
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
            (&read_data.stats, &read_data.skill_sets),
            &read_data.combos,
        )
            .join()
        {
            prof_span!("entity");
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
                match poise_state {
                    PoiseState::Normal => {},
                    PoiseState::Interrupted => {
                        poise.reset();
                        *char_state = CharacterState::Stunned(common::states::stunned::Data {
                            static_data: common::states::stunned::StaticData {
                                buildup_duration: Duration::from_millis(125),
                                recover_duration: Duration::from_millis(125),
                                movement_speed: 0.80,
                                poise_state,
                            },
                            timer: Duration::default(),
                            stage_section: common::states::utils::StageSection::Buildup,
                            was_wielded,
                        });
                        outcomes.push(Outcome::PoiseChange {
                            pos,
                            state: PoiseState::Interrupted,
                        });
                    },
                    PoiseState::Stunned => {
                        poise.reset();
                        *char_state = CharacterState::Stunned(common::states::stunned::Data {
                            static_data: common::states::stunned::StaticData {
                                buildup_duration: Duration::from_millis(300),
                                recover_duration: Duration::from_millis(300),
                                movement_speed: 0.65,
                                poise_state,
                            },
                            timer: Duration::default(),
                            stage_section: common::states::utils::StageSection::Buildup,
                            was_wielded,
                        });
                        outcomes.push(Outcome::PoiseChange {
                            pos,
                            state: PoiseState::Stunned,
                        });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 5.0 * *poise.knockback(),
                        });
                    },
                    PoiseState::Dazed => {
                        poise.reset();
                        *char_state = CharacterState::Stunned(common::states::stunned::Data {
                            static_data: common::states::stunned::StaticData {
                                buildup_duration: Duration::from_millis(600),
                                recover_duration: Duration::from_millis(250),
                                movement_speed: 0.45,
                                poise_state,
                            },
                            timer: Duration::default(),
                            stage_section: common::states::utils::StageSection::Buildup,
                            was_wielded,
                        });
                        outcomes.push(Outcome::PoiseChange {
                            pos,
                            state: PoiseState::Dazed,
                        });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 10.0 * *poise.knockback(),
                        });
                    },
                    PoiseState::KnockedDown => {
                        poise.reset();
                        *char_state = CharacterState::Stunned(common::states::stunned::Data {
                            static_data: common::states::stunned::StaticData {
                                buildup_duration: Duration::from_millis(750),
                                recover_duration: Duration::from_millis(500),
                                movement_speed: 0.4,
                                poise_state,
                            },
                            timer: Duration::default(),
                            stage_section: common::states::utils::StageSection::Buildup,
                            was_wielded,
                        });
                        outcomes.push(Outcome::PoiseChange {
                            pos,
                            state: PoiseState::KnockedDown,
                        });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 10.0 * *poise.knockback(),
                        });
                    },
                }
            }

            // Controller actions
            let actions = std::mem::take(&mut controller.actions);

            let mut join_struct = JoinStruct {
                entity,
                uid,
                char_state,
                pos: &mut pos,
                vel: &mut vel,
                ori: &mut ori,
                mass,
                density: &mut density,
                energy,
                inventory,
                controller: &mut controller,
                health,
                body,
                physics,
                melee_attack: read_data.melee_attacks.get(entity),
                beam: read_data.beams.get(entity),
                stat,
                skill_set,
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
                let state_update = j.character.handle_event(&j, action);
                Self::publish_state_update(
                    &mut join_struct,
                    state_update,
                    &mut local_emitter,
                    &mut server_emitter,
                );
            }

            // Mounted occurs after control actions have been handled
            // If mounted, character state is controlled by mount
            if let Some(Mounting(_)) = read_data.mountings.get(entity) {
                let idle_state = CharacterState::Idle {};
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

            let state_update = j.character.behavior(&j);
            Self::publish_state_update(
                &mut join_struct,
                state_update,
                &mut local_emitter,
                &mut server_emitter,
            );
        }
    }
}

impl Sys {
    fn publish_state_update(
        join: &mut JoinStruct,
        mut state_update: StateUpdate,
        local_emitter: &mut Emitter<LocalEvent>,
        server_emitter: &mut Emitter<ServerEvent>,
    ) {
        local_emitter.append(&mut state_update.local_events);
        server_emitter.append(&mut state_update.server_events);

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
            server_emitter.emit(ServerEvent::InventoryManip(
                join.entity,
                InventoryManip::SwapEquippedWeapons,
            ));
        }
    }
}
