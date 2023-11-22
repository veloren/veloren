pub mod behavior_tree;
use server_agent::data::AgentEvents;
pub use server_agent::{action_nodes, attack, consts, data, util};
use vek::Vec3;

use crate::sys::agent::{
    behavior_tree::{BehaviorData, BehaviorTree},
    data::{AgentData, ReadData},
};
use common::{
    comp::{
        self, inventory::slot::EquipSlot, item::ItemDesc, Agent, Alignment, Body, CharacterState,
        Controller, Health, InputKind, Scale,
    },
    mounting::Volume,
    path::TraversalConfig,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rand::thread_rng;
use rayon::iter::ParallelIterator;
use specs::{LendJoin, ParJoin, WriteStorage};

/// This system will allow NPCs to modify their controller
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    type SystemData = (
        ReadData<'a>,
        AgentEvents<'a>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
    );

    const NAME: &'static str = "agent";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    fn run(
        job: &mut Job<Self>,
        (read_data, events, mut agents, mut controllers): Self::SystemData,
    ) {
        job.cpu_stats.measure(ParMode::Rayon);

        (
            &read_data.entities,
            (
                &read_data.energies,
                read_data.healths.maybe(),
                read_data.combos.maybe(),
            ),
            (
                &read_data.positions,
                &read_data.velocities,
                &read_data.orientations,
            ),
            read_data.bodies.maybe(),
            &read_data.inventories,
            (
                &read_data.char_states,
                &read_data.skill_set,
                &read_data.active_abilities,
            ),
            &read_data.physics_states,
            &read_data.uids,
            &mut agents,
            &mut controllers,
            read_data.light_emitter.maybe(),
            read_data.groups.maybe(),
            read_data.rtsim_entities.maybe(),
            (
                !&read_data.is_mounts,
                read_data.is_riders.maybe(),
                read_data.is_volume_riders.maybe(),
            ),
        )
            .par_join()
            .for_each_init(
                || {
                    prof_span!(guard, "agent rayon job");
                    guard
                },
                |_guard,
                 (
                    entity,
                    (energy, health, combo),
                    (pos, vel, ori),
                    body,
                    inventory,
                    (char_state, skill_set, active_abilities),
                    physics_state,
                    uid,
                    agent,
                    controller,
                    light_emitter,
                    group,
                    rtsim_entity,
                    (_, is_rider, is_volume_rider),
                )| {
                    let mut emitters = events.get_emitters();
                    let mut rng = thread_rng();

                    // The entity that is moving, if riding it's the mount, otherwise it's itself
                    let moving_entity = is_rider
                        .and_then(|is_rider| read_data.id_maps.uid_entity(is_rider.mount))
                        .or_else(|| {
                            is_volume_rider.and_then(|is_volume_rider| {
                                match is_volume_rider.pos.kind {
                                    Volume::Terrain => None,
                                    Volume::Entity(uid) => read_data.id_maps.uid_entity(uid),
                                }
                            })
                        })
                        .unwrap_or(entity);

                    let moving_body = read_data.bodies.get(moving_entity);
                    let physics_state = read_data
                        .physics_states
                        .get(moving_entity)
                        .unwrap_or(physics_state);

                    // Hack, replace with better system when groups are more sophisticated
                    // Override alignment if in a group unless entity is owned already
                    let alignment = if matches!(
                        &read_data.alignments.get(entity),
                        &Some(Alignment::Owned(_))
                    ) {
                        read_data.alignments.get(entity).copied()
                    } else {
                        group
                            .and_then(|g| read_data.group_manager.group_info(*g))
                            .and_then(|info| read_data.uids.get(info.leader))
                            .copied()
                            .map_or_else(
                                || read_data.alignments.get(entity).copied(),
                                |uid| Some(Alignment::Owned(uid)),
                            )
                    };

                    if !matches!(char_state, CharacterState::LeapMelee(_)) {
                        // Default to looking in orientation direction
                        // (can be overridden below)
                        //
                        // This definitely breaks LeapMelee and
                        // probably not only that, do we really need this at all?
                        controller.reset();
                        controller.inputs.look_dir = ori.look_dir();
                    }

                    let scale = read_data
                        .scales
                        .get(moving_entity)
                        .map_or(1.0, |Scale(s)| *s);

                    let glider_equipped = inventory
                        .equipped(EquipSlot::Glider)
                        .as_ref()
                        .map_or(false, |item| {
                            matches!(&*item.kind(), comp::item::ItemKind::Glider)
                        });

                    let is_gliding = matches!(
                        read_data.char_states.get(entity),
                        Some(CharacterState::GlideWield(_) | CharacterState::Glide(_))
                    ) && physics_state.on_ground.is_none();

                    if let Some((kp, ki, kd)) = moving_body.and_then(comp::agent::pid_coefficients)
                    {
                        if agent
                            .position_pid_controller
                            .as_ref()
                            .map_or(false, |pid| (pid.kp, pid.ki, pid.kd) != (kp, ki, kd))
                        {
                            agent.position_pid_controller = None;
                        }
                        let pid = agent.position_pid_controller.get_or_insert_with(|| {
                            fn pure_z(sp: Vec3<f32>, pv: Vec3<f32>) -> f32 { (sp - pv).z }
                            comp::PidController::new(kp, ki, kd, pos.0, 0.0, pure_z)
                        });
                        pid.add_measurement(read_data.time.0, pos.0);
                    } else {
                        agent.position_pid_controller = None;
                    }

                    // This controls how picky NPCs are about their pathfinding.
                    // Giants are larger and so can afford to be less precise
                    // when trying to move around the world
                    // (especially since they would otherwise get stuck on
                    // obstacles that smaller entities would not).
                    let node_tolerance = scale * 1.5;
                    let slow_factor = moving_body.map_or(0.0, |b| b.base_accel() / 250.0).min(1.0);
                    let traversal_config = TraversalConfig {
                        node_tolerance,
                        slow_factor,
                        on_ground: physics_state.on_ground.is_some(),
                        in_liquid: physics_state.in_liquid().is_some(),
                        min_tgt_dist: scale * moving_body.map_or(1.0, |body| body.max_radius()),
                        can_climb: moving_body.map_or(false, Body::can_climb),
                        can_fly: moving_body.map_or(false, |b| b.fly_thrust().is_some()),
                    };
                    let health_fraction = health.map_or(1.0, Health::fraction);

                    if traversal_config.can_fly && matches!(moving_body, Some(Body::Ship(_))) {
                        // hack (kinda): Never turn off flight airships
                        // since it results in stuttering and falling back to the ground.
                        //
                        // TODO: look into `controller.reset()` line above
                        // and see if it fixes it
                        controller.push_basic_input(InputKind::Fly);
                    }

                    // Package all this agent's data into a convenient struct
                    let data = AgentData {
                        entity: &entity,
                        rtsim_entity,
                        uid,
                        pos,
                        vel,
                        ori,
                        energy,
                        body,
                        inventory,
                        skill_set,
                        physics_state,
                        alignment: alignment.as_ref(),
                        traversal_config,
                        scale,
                        damage: health_fraction,
                        light_emitter,
                        glider_equipped,
                        is_gliding,
                        health: read_data.healths.get(entity),
                        char_state,
                        active_abilities,
                        combo,
                        buffs: read_data.buffs.get(entity),
                        stats: read_data.stats.get(entity),
                        cached_spatial_grid: &read_data.cached_spatial_grid,
                        msm: &read_data.msm,
                        poise: read_data.poises.get(entity),
                        stance: read_data.stances.get(entity),
                    };

                    ///////////////////////////////////////////////////////////
                    // Behavior tree
                    ///////////////////////////////////////////////////////////
                    // The behavior tree is meant to make decisions for agents
                    // *but should not* mutate any data (only action nodes
                    // should do that). Each path should lead to one (and only
                    // one) action node. This makes bugfinding much easier and
                    // debugging way easier. If you don't think so, try
                    // debugging the agent code before this MR
                    // (https://gitlab.com/veloren/veloren/-/merge_requests/1801).
                    // Each tick should arrive at one (1) action node which
                    // then determines what the agent does. If this makes you
                    // uncomfortable, consider dt the response time of the
                    // NPC. To make the tree easier to read, subtrees can be
                    // created as methods on `AgentData`. Action nodes are
                    // also methods on the `AgentData` struct. Action nodes
                    // are the only parts of this tree that should provide
                    // inputs.
                    let mut behavior_data = BehaviorData {
                        agent,
                        agent_data: data,
                        read_data: &read_data,
                        emitters: &mut emitters,
                        controller,
                        rng: &mut rng,
                    };

                    BehaviorTree::root().run(&mut behavior_data);

                    debug_assert!(controller.inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                    debug_assert!(controller.inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
                },
            );
    }
}
