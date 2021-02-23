use specs::{
    shred::ResourceId, Entities, Join, LazyUpdate, Read, ReadExpect, ReadStorage, System,
    SystemData, World, WriteStorage,
};

use common::{
    comp::{
        inventory::{
            item::MaterialStatManifest,
            slot::{EquipSlot, Slot},
        },
        Beam, Body, CharacterState, Controller, Energy, Health, Inventory, Melee, Mounting, Ori,
        PhysicsState, Poise, PoiseState, Pos, StateUpdate, Stats, Vel,
    },
    event::{EventBus, LocalEvent, ServerEvent},
    metrics::SysMetrics,
    resources::DeltaTime,
    span,
    states::{
        self,
        behavior::{CharacterBehavior, JoinData, JoinStruct},
    },
    uid::Uid,
};
use std::time::Duration;

fn incorporate_update(join: &mut JoinStruct, state_update: StateUpdate) {
    // TODO: if checking equality is expensive use optional field in StateUpdate
    if join.char_state.get_unchecked() != &state_update.character {
        *join.char_state.get_mut_unchecked() = state_update.character
    };
    *join.pos = state_update.pos;
    *join.vel = state_update.vel;
    *join.ori = state_update.ori;
    // Note: might be changed every tick by timer anyway
    if join.energy.get_unchecked() != &state_update.energy {
        *join.energy.get_mut_unchecked() = state_update.energy
    };
    if state_update.swap_equipped_weapons {
        let mut inventory = join.inventory.get_mut_unchecked();
        let inventory = &mut *inventory;
        inventory
            .swap(
                Slot::Equip(EquipSlot::Mainhand),
                Slot::Equip(EquipSlot::Offhand),
            )
            .first()
            .unwrap_none(); // Swapping main and offhand never results in leftover items
    }
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    server_bus: Read<'a, EventBus<ServerEvent>>,
    local_bus: Read<'a, EventBus<LocalEvent>>,
    dt: Read<'a, DeltaTime>,
    lazy_update: Read<'a, LazyUpdate>,
    metrics: ReadExpect<'a, SysMetrics>,
    healths: ReadStorage<'a, Health>,
    bodies: ReadStorage<'a, Body>,
    physics_states: ReadStorage<'a, PhysicsState>,
    melee_attacks: ReadStorage<'a, Melee>,
    beams: ReadStorage<'a, Beam>,
    uids: ReadStorage<'a, Uid>,
    mountings: ReadStorage<'a, Mounting>,
    stats: ReadStorage<'a, Stats>,
    msm: Read<'a, MaterialStatManifest>,
}

/// ## Character Behavior System
/// Passes `JoinData` to `CharacterState`'s `behavior` handler fn's. Receives a
/// `StateUpdate` in return and performs updates to ECS Components from that.
pub struct Sys;

impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        WriteStorage<'a, CharacterState>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Vel>,
        WriteStorage<'a, Ori>,
        WriteStorage<'a, Energy>,
        WriteStorage<'a, Inventory>,
        WriteStorage<'a, Controller>,
        WriteStorage<'a, Poise>,
    );

    #[allow(clippy::while_let_on_iterator)] // TODO: Pending review in #587
    fn run(
        &mut self,
        (
            read_data,
            mut character_states,
            mut positions,
            mut velocities,
            mut orientations,
            mut energies,
            mut inventories,
            mut controllers,
            mut poises,
        ): Self::SystemData,
    ) {
        let start_time = std::time::Instant::now();
        span!(_guard, "run", "character_behavior::Sys::run");
        let mut server_emitter = read_data.server_bus.emitter();
        let mut local_emitter = read_data.local_bus.emitter();

        for (
            entity,
            uid,
            mut char_state,
            mut pos,
            mut vel,
            mut ori,
            energy,
            inventory,
            mut controller,
            health,
            body,
            physics,
            stat,
        ) in (
            &read_data.entities,
            &read_data.uids,
            &mut character_states.restrict_mut(),
            &mut positions,
            &mut velocities,
            &mut orientations,
            &mut energies.restrict_mut(),
            &mut inventories.restrict_mut(),
            &mut controllers,
            &read_data.healths,
            &read_data.bodies,
            &read_data.physics_states,
            &read_data.stats,
        )
            .join()
        {
            // Being dead overrides all other states
            if health.is_dead {
                // Do nothing
                continue;
            }
            // If mounted, character state is controlled by mount
            // TODO: Make mounting a state
            if let Some(Mounting(_)) = read_data.mountings.get(entity) {
                let sit_state = CharacterState::Sit {};
                if char_state.get_unchecked() != &sit_state {
                    *char_state.get_mut_unchecked() = sit_state;
                }
                continue;
            }

            // Enter stunned state if poise damage is enough
            if let Some(mut poise) = poises.get_mut(entity) {
                let was_wielded = char_state.get_unchecked().is_wield();
                let poise_state = poise.poise_state();
                match poise_state {
                    PoiseState::Normal => {},
                    PoiseState::Interrupted => {
                        poise.reset();
                        *char_state.get_mut_unchecked() =
                            CharacterState::Stunned(common::states::stunned::Data {
                                static_data: common::states::stunned::StaticData {
                                    buildup_duration: Duration::from_millis(150),
                                    recover_duration: Duration::from_millis(150),
                                    movement_speed: 0.4,
                                    poise_state,
                                },
                                timer: Duration::default(),
                                stage_section: common::states::utils::StageSection::Buildup,
                                was_wielded,
                            });
                    },
                    PoiseState::Stunned => {
                        poise.reset();
                        *char_state.get_mut_unchecked() =
                            CharacterState::Stunned(common::states::stunned::Data {
                                static_data: common::states::stunned::StaticData {
                                    buildup_duration: Duration::from_millis(500),
                                    recover_duration: Duration::from_millis(300),
                                    movement_speed: 0.1,
                                    poise_state,
                                },
                                timer: Duration::default(),
                                stage_section: common::states::utils::StageSection::Buildup,
                                was_wielded,
                            });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 5.0 * poise.knockback(),
                        });
                    },
                    PoiseState::Dazed => {
                        poise.reset();
                        *char_state.get_mut_unchecked() =
                            CharacterState::Stunned(common::states::stunned::Data {
                                static_data: common::states::stunned::StaticData {
                                    buildup_duration: Duration::from_millis(800),
                                    recover_duration: Duration::from_millis(250),
                                    movement_speed: 0.0,
                                    poise_state,
                                },
                                timer: Duration::default(),
                                stage_section: common::states::utils::StageSection::Buildup,
                                was_wielded,
                            });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 10.0 * poise.knockback(),
                        });
                    },
                    PoiseState::KnockedDown => {
                        poise.reset();
                        *char_state.get_mut_unchecked() =
                            CharacterState::Stunned(common::states::stunned::Data {
                                static_data: common::states::stunned::StaticData {
                                    buildup_duration: Duration::from_millis(1000),
                                    recover_duration: Duration::from_millis(750),
                                    movement_speed: 0.0,
                                    poise_state,
                                },
                                timer: Duration::default(),
                                stage_section: common::states::utils::StageSection::Buildup,
                                was_wielded,
                            });
                        server_emitter.emit(ServerEvent::Knockback {
                            entity,
                            impulse: 10.0 * poise.knockback(),
                        });
                    },
                }
            }

            // Controller actions
            let actions = std::mem::replace(&mut controller.actions, Vec::new());

            let mut join_struct = JoinStruct {
                entity,
                uid: &uid,
                char_state,
                pos: &mut pos,
                vel: &mut vel,
                ori: &mut ori,
                energy,
                inventory,
                controller: &mut controller,
                health: &health,
                body: &body,
                physics: &physics,
                melee_attack: read_data.melee_attacks.get(entity),
                beam: read_data.beams.get(entity),
                stat: &stat,
            };

            for action in actions {
                let j = JoinData::new(
                    &join_struct,
                    &read_data.lazy_update,
                    &read_data.dt,
                    &read_data.msm,
                );
                let mut state_update = match j.character {
                    CharacterState::Idle => states::idle::Data.handle_event(&j, action),
                    CharacterState::Talk => states::talk::Data.handle_event(&j, action),
                    CharacterState::Climb => states::climb::Data.handle_event(&j, action),
                    CharacterState::Glide => states::glide::Data.handle_event(&j, action),
                    CharacterState::GlideWield => {
                        states::glide_wield::Data.handle_event(&j, action)
                    },
                    CharacterState::Stunned(data) => data.handle_event(&j, action),
                    CharacterState::Sit => {
                        states::sit::Data::handle_event(&states::sit::Data, &j, action)
                    },
                    CharacterState::Dance => {
                        states::dance::Data::handle_event(&states::dance::Data, &j, action)
                    },
                    CharacterState::Sneak => {
                        states::sneak::Data::handle_event(&states::sneak::Data, &j, action)
                    },
                    CharacterState::BasicBlock => {
                        states::basic_block::Data.handle_event(&j, action)
                    },
                    CharacterState::Roll(data) => data.handle_event(&j, action),
                    CharacterState::Wielding => states::wielding::Data.handle_event(&j, action),
                    CharacterState::Equipping(data) => data.handle_event(&j, action),
                    CharacterState::ComboMelee(data) => data.handle_event(&j, action),
                    CharacterState::BasicMelee(data) => data.handle_event(&j, action),
                    CharacterState::BasicRanged(data) => data.handle_event(&j, action),
                    CharacterState::Boost(data) => data.handle_event(&j, action),
                    CharacterState::DashMelee(data) => data.handle_event(&j, action),
                    CharacterState::LeapMelee(data) => data.handle_event(&j, action),
                    CharacterState::SpinMelee(data) => data.handle_event(&j, action),
                    CharacterState::ChargedMelee(data) => data.handle_event(&j, action),
                    CharacterState::ChargedRanged(data) => data.handle_event(&j, action),
                    CharacterState::RepeaterRanged(data) => data.handle_event(&j, action),
                    CharacterState::Shockwave(data) => data.handle_event(&j, action),
                    CharacterState::BasicBeam(data) => data.handle_event(&j, action),
                };
                local_emitter.append(&mut state_update.local_events);
                server_emitter.append(&mut state_update.server_events);
                incorporate_update(&mut join_struct, state_update);
            }

            let j = JoinData::new(
                &join_struct,
                &read_data.lazy_update,
                &read_data.dt,
                &read_data.msm,
            );

            let mut state_update = match j.character {
                CharacterState::Idle => states::idle::Data.behavior(&j),
                CharacterState::Talk => states::talk::Data.behavior(&j),
                CharacterState::Climb => states::climb::Data.behavior(&j),
                CharacterState::Glide => states::glide::Data.behavior(&j),
                CharacterState::GlideWield => states::glide_wield::Data.behavior(&j),
                CharacterState::Stunned(data) => data.behavior(&j),
                CharacterState::Sit => states::sit::Data::behavior(&states::sit::Data, &j),
                CharacterState::Dance => states::dance::Data::behavior(&states::dance::Data, &j),
                CharacterState::Sneak => states::sneak::Data::behavior(&states::sneak::Data, &j),
                CharacterState::BasicBlock => states::basic_block::Data.behavior(&j),
                CharacterState::Roll(data) => data.behavior(&j),
                CharacterState::Wielding => states::wielding::Data.behavior(&j),
                CharacterState::Equipping(data) => data.behavior(&j),
                CharacterState::ComboMelee(data) => data.behavior(&j),
                CharacterState::BasicMelee(data) => data.behavior(&j),
                CharacterState::BasicRanged(data) => data.behavior(&j),
                CharacterState::Boost(data) => data.behavior(&j),
                CharacterState::DashMelee(data) => data.behavior(&j),
                CharacterState::LeapMelee(data) => data.behavior(&j),
                CharacterState::SpinMelee(data) => data.behavior(&j),
                CharacterState::ChargedMelee(data) => data.behavior(&j),
                CharacterState::ChargedRanged(data) => data.behavior(&j),
                CharacterState::RepeaterRanged(data) => data.behavior(&j),
                CharacterState::Shockwave(data) => data.behavior(&j),
                CharacterState::BasicBeam(data) => data.behavior(&j),
            };

            local_emitter.append(&mut state_update.local_events);
            server_emitter.append(&mut state_update.server_events);
            incorporate_update(&mut join_struct, state_update);
        }
        read_data.metrics.character_behavior_ns.store(
            start_time.elapsed().as_nanos() as u64,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}
