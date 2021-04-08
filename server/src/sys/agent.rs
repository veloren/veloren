use crate::rtsim::{Entity as RtSimData, RtSim};
use common::{
    comp::{
        self,
        agent::{AgentEvent, Tactic, Target, DEFAULT_INTERACTION_TIME, TRADE_INTERACTION_TIME},
        buff::{BuffKind, Buffs},
        compass::{Direction, Distance},
        dialogue::{MoodContext, MoodState, Subject},
        group,
        inventory::{item::ItemTag, slot::EquipSlot},
        invite::{InviteKind, InviteResponse},
        item::{
            tool::{ToolKind, UniqueKind},
            ItemDesc, ItemKind,
        },
        skills::{AxeSkill, BowSkill, HammerSkill, Skill, StaffSkill, SwordSkill},
        Agent, Alignment, BehaviorCapability, BehaviorState, Body, CharacterState, ControlAction,
        ControlEvent, Controller, Energy, Health, InputKind, Inventory, LightEmitter, MountState,
        Ori, PhysicsState, Pos, Scale, Stats, UnresolvedChatMsg, Vel,
    },
    event::{Emitter, EventBus, ServerEvent},
    path::TraversalConfig,
    resources::{DeltaTime, Time, TimeOfDay},
    rtsim::{Memory, MemoryItem, RtSimEntity, RtSimEvent},
    states::utils::StageSection,
    terrain::{Block, TerrainGrid},
    time::DayPeriod,
    trade::{TradeAction, TradePhase, TradeResult},
    uid::{Uid, UidAllocator},
    util::Dir,
    vol::ReadVol,
};
use common_base::prof_span;
use common_ecs::{Job, Origin, ParMode, Phase, System};
use rand::{thread_rng, Rng};
use rayon::iter::ParallelIterator;
use specs::{
    saveload::{Marker, MarkerAllocator},
    shred::ResourceId,
    Entities, Entity as EcsEntity, Join, ParJoin, Read, ReadExpect, ReadStorage, SystemData, World,
    Write, WriteExpect, WriteStorage,
};
use std::{f32::consts::PI, sync::Arc, time::Duration};
use vek::*;

struct AgentData<'a> {
    entity: &'a EcsEntity,
    rtsim_entity: Option<&'a RtSimData>,
    uid: &'a Uid,
    pos: &'a Pos,
    vel: &'a Vel,
    ori: &'a Ori,
    energy: &'a Energy,
    body: Option<&'a Body>,
    inventory: &'a Inventory,
    stats: &'a Stats,
    physics_state: &'a PhysicsState,
    alignment: Option<&'a Alignment>,
    traversal_config: TraversalConfig,
    scale: f32,
    flees: bool,
    damage: f32,
    light_emitter: Option<&'a LightEmitter>,
    glider_equipped: bool,
    is_gliding: bool,
    health: Option<&'a Health>,
    char_state: &'a CharacterState,
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    uid_allocator: Read<'a, UidAllocator>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    group_manager: Read<'a, group::GroupManager>,
    energies: ReadStorage<'a, Energy>,
    positions: ReadStorage<'a, Pos>,
    velocities: ReadStorage<'a, Vel>,
    orientations: ReadStorage<'a, Ori>,
    scales: ReadStorage<'a, Scale>,
    healths: ReadStorage<'a, Health>,
    inventories: ReadStorage<'a, Inventory>,
    stats: ReadStorage<'a, Stats>,
    physics_states: ReadStorage<'a, PhysicsState>,
    char_states: ReadStorage<'a, CharacterState>,
    uids: ReadStorage<'a, Uid>,
    groups: ReadStorage<'a, group::Group>,
    terrain: ReadExpect<'a, TerrainGrid>,
    alignments: ReadStorage<'a, Alignment>,
    bodies: ReadStorage<'a, Body>,
    mount_states: ReadStorage<'a, MountState>,
    time_of_day: Read<'a, TimeOfDay>,
    light_emitter: ReadStorage<'a, LightEmitter>,
    world: ReadExpect<'a, Arc<world::World>>,
    rtsim_entities: ReadStorage<'a, RtSimEntity>,
    buffs: ReadStorage<'a, Buffs>,
}

// This is 3.1 to last longer than the last damage timer (3.0 seconds)
const DAMAGE_MEMORY_DURATION: f64 = 0.1;
const FLEE_DURATION: f32 = 3.0;
const MAX_FOLLOW_DIST: f32 = 12.0;
const MAX_CHASE_DIST: f32 = 250.0;
const MAX_FLEE_DIST: f32 = 20.0;
const LISTEN_DIST: f32 = 16.0;
const SEARCH_DIST: f32 = 48.0;
const SIGHT_DIST: f32 = 80.0;
const SNEAK_COEFFICIENT: f32 = 0.25;
const AVG_FOLLOW_DIST: f32 = 6.0;
const RETARGETING_THRESHOLD_SECONDS: f64 = 10.0;

/// This system will allow NPCs to modify their controller
#[derive(Default)]
pub struct Sys;
impl<'a> System<'a> for Sys {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        ReadData<'a>,
        Write<'a, EventBus<ServerEvent>>,
        WriteStorage<'a, Agent>,
        WriteStorage<'a, Controller>,
        WriteExpect<'a, RtSim>,
    );

    const NAME: &'static str = "agent";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn run(
        job: &mut Job<Self>,
        (read_data, event_bus, mut agents, mut controllers, mut rtsim): Self::SystemData,
    ) {
        let rtsim = &mut *rtsim;
        job.cpu_stats.measure(ParMode::Rayon);
        (
            &read_data.entities,
            (&read_data.energies, read_data.healths.maybe()),
            (
                &read_data.positions,
                &read_data.velocities,
                &read_data.orientations,
            ),
            read_data.bodies.maybe(),
            &read_data.inventories,
            &read_data.stats,
            &read_data.physics_states,
            &read_data.uids,
            &mut agents,
            &mut controllers,
            read_data.light_emitter.maybe(),
            read_data.groups.maybe(),
            read_data.mount_states.maybe(),
            &read_data.char_states,
        )
            .par_join()
            .filter(|(_, _, _, _, _, _, _, _, _, _, _, _, mount_state, _)| {
                // Skip mounted entities
                mount_state
                    .map(|ms| *ms == MountState::Unmounted)
                    .unwrap_or(true)
            })
            .for_each_init(
                || {
                    prof_span!(guard, "agent rayon job");
                    guard
                },
                |_guard,
                 (
                    entity,
                    (energy, health),
                    (pos, vel, ori),
                    body,
                    inventory,
                    stats,
                    physics_state,
                    uid,
                    agent,
                    controller,
                    light_emitter,
                    groups,
                    _,
                    char_state,
                )| {
                    //// Hack, replace with better system when groups are more sophisticated
                    //// Override alignment if in a group unless entity is owned already
                    let alignment = if !matches!(
                        &read_data.alignments.get(entity),
                        &Some(Alignment::Owned(_))
                    ) {
                        groups
                            .and_then(|g| read_data.group_manager.group_info(*g))
                            .and_then(|info| read_data.uids.get(info.leader))
                            .copied()
                            .map(Alignment::Owned)
                            .or(read_data.alignments.get(entity).copied())
                    } else {
                        read_data.alignments.get(entity).copied()
                    };

                    controller.reset();
                    let mut event_emitter = event_bus.emitter();

                    // Default to looking in orientation direction (can be overridden below)
                    controller.inputs.look_dir = ori.look_dir();

                    let scale = read_data.scales.get(entity).map(|s| s.0).unwrap_or(1.0);

                    let glider_equipped = inventory
                        .equipped(EquipSlot::Glider)
                        .as_ref()
                        .map_or(false, |item| {
                            matches!(item.kind(), comp::item::ItemKind::Glider(_))
                        });
                    let is_gliding = matches!(
                        read_data.char_states.get(entity),
                        Some(CharacterState::GlideWield) | Some(CharacterState::Glide)
                    ) && !physics_state.on_ground;

                    // This controls how picky NPCs are about their pathfinding. Giants are larger
                    // and so can afford to be less precise when trying to move around
                    // the world (especially since they would otherwise get stuck on
                    // obstacles that smaller entities would not).
                    let node_tolerance = scale * 1.5;
                    let slow_factor = body.map(|b| b.base_accel() / 250.0).unwrap_or(0.0).min(1.0);
                    let traversal_config = TraversalConfig {
                        node_tolerance,
                        slow_factor,
                        on_ground: physics_state.on_ground,
                        in_liquid: physics_state.in_liquid.is_some(),
                        min_tgt_dist: 1.0,
                        can_climb: body.map(|b| b.can_climb()).unwrap_or(false),
                        can_fly: body.map(|b| b.can_fly().is_some()).unwrap_or(false),
                    };

                    if traversal_config.can_fly {
                        // hack (kinda): Never turn off flight for entities that can fly at all,
                        // since it results in stuttering and falling back to the ground.

                        // If we need to be able to have entities (dragons maybe?) both fly and
                        // run/jump, we probably need to refactor to avoid resetting the controller
                        // every frame.
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Fly));
                    }

                    let flees = alignment
                        .map(|a| !matches!(a, Alignment::Enemy | Alignment::Owned(_)))
                        .unwrap_or(true);
                    let damage = health.map_or(1.0, |h| h.current() as f32 / h.maximum() as f32);
                    let rtsim_entity = read_data
                        .rtsim_entities
                        .get(entity)
                        .and_then(|rtsim_ent| rtsim.get_entity(rtsim_ent.0));

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
                        stats,
                        physics_state,
                        alignment: alignment.as_ref(),
                        traversal_config,
                        scale,
                        flees,
                        damage,
                        light_emitter,
                        glider_equipped,
                        is_gliding,
                        health: read_data.healths.get(entity),
                        char_state,
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

                    // If falling fast and can glide, save yourself!
                    if data.glider_equipped && !data.physics_state.on_ground {
                        // toggle glider when vertical velocity is above some threshold (here ~
                        // glider fall vertical speed)
                        data.glider_fall(agent, controller, &read_data);
                    } else if let Some(Target {
                        target, hostile, ..
                    }) = agent.target
                    {
                        if let Some(tgt_health) = read_data.healths.get(target) {
                            // If the target is hostile (either based on alignment or if
                            // the target just attacked
                            if !tgt_health.is_dead {
                                if hostile {
                                    data.hostile_tree(
                                        agent,
                                        controller,
                                        &read_data,
                                        &mut event_emitter,
                                    );
                                // Target is something worth following methinks
                                } else if let Some(Alignment::Owned(_)) = data.alignment {
                                    if let Some(tgt_pos) = read_data.positions.get(target) {
                                        let dist_sqrd = pos.0.distance_squared(tgt_pos.0);
                                        // If really far away drop everything and follow
                                        if dist_sqrd > (2.0 * MAX_FOLLOW_DIST).powi(2) {
                                            agent.bearing = Vec2::zero();
                                            data.follow(
                                                agent,
                                                controller,
                                                &read_data.terrain,
                                                tgt_pos,
                                            );
                                        // Attack target's attacker
                                        } else if tgt_health.last_change.0 < 5.0
                                            && tgt_health.last_change.1.amount < 0
                                        {
                                            if let comp::HealthSource::Damage {
                                                by: Some(by), ..
                                            } = tgt_health.last_change.1.cause
                                            {
                                                if let Some(attacker) = read_data
                                                    .uid_allocator
                                                    .retrieve_entity_internal(by.id())
                                                {
                                                    agent.target = Some(Target {
                                                        target: attacker,
                                                        hostile: true,
                                                        selected_at: read_data.time.0,
                                                    });
                                                    if let Some(tgt_pos) =
                                                        read_data.positions.get(attacker)
                                                    {
                                                        if should_stop_attacking(
                                                            read_data.healths.get(attacker),
                                                            read_data.buffs.get(attacker),
                                                        ) {
                                                            agent.target = Some(Target {
                                                                target,
                                                                hostile: false,
                                                                selected_at: read_data.time.0,
                                                            });
                                                            data.idle(
                                                                agent, controller, &read_data,
                                                            );
                                                        } else {
                                                            data.attack(
                                                                agent,
                                                                controller,
                                                                &read_data.terrain,
                                                                tgt_pos,
                                                                read_data.bodies.get(attacker),
                                                                &read_data.dt,
                                                                &read_data,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        // Follow owner if too far away and not
                                        // fighting
                                        } else if dist_sqrd > MAX_FOLLOW_DIST.powi(2) {
                                            data.follow(
                                                agent,
                                                controller,
                                                &read_data.terrain,
                                                tgt_pos,
                                            );

                                        // Otherwise just idle
                                        } else {
                                            data.idle_tree(
                                                agent,
                                                controller,
                                                &read_data,
                                                &mut event_emitter,
                                            );
                                        }
                                    }
                                } else {
                                    data.idle_tree(
                                        agent,
                                        controller,
                                        &read_data,
                                        &mut event_emitter,
                                    );
                                }
                            } else {
                                agent.target = None;
                                data.idle_tree(agent, controller, &read_data, &mut event_emitter);
                            }
                        } else {
                            agent.target = None;
                            data.idle_tree(agent, controller, &read_data, &mut event_emitter);
                        }
                    } else {
                        // Target an entity that's attacking us if the attack was recent and we
                        // have a health component
                        match health {
                            Some(health) if health.last_change.0 < DAMAGE_MEMORY_DURATION => {
                                if let comp::HealthSource::Damage { by: Some(by), .. } =
                                    health.last_change.1.cause
                                {
                                    if let Some(attacker) =
                                        read_data.uid_allocator.retrieve_entity_internal(by.id())
                                    {
                                        if let Some(tgt_pos) = read_data.positions.get(attacker) {
                                            // If the target is dead or in a safezone, remove the
                                            // target
                                            // and idle.
                                            if should_stop_attacking(
                                                read_data.healths.get(attacker),
                                                read_data.buffs.get(attacker),
                                            ) {
                                                agent.target = None;
                                                data.idle_tree(
                                                    agent,
                                                    controller,
                                                    &read_data,
                                                    &mut event_emitter,
                                                );
                                            } else {
                                                agent.target = Some(Target {
                                                    target: attacker,
                                                    hostile: true,
                                                    selected_at: read_data.time.0,
                                                });
                                                data.attack(
                                                    agent,
                                                    controller,
                                                    &read_data.terrain,
                                                    tgt_pos,
                                                    read_data.bodies.get(attacker),
                                                    &read_data.dt,
                                                    &read_data,
                                                );
                                                // Remember this encounter if an RtSim entity
                                                if let Some(tgt_stats) =
                                                    read_data.stats.get(attacker)
                                                {
                                                    if data.rtsim_entity.is_some() {
                                                        agent.rtsim_controller.events.push(
                                                            RtSimEvent::AddMemory(Memory {
                                                                item: MemoryItem::CharacterFight {
                                                                    name: tgt_stats.name.clone(),
                                                                },
                                                                time_to_forget: read_data.time.0
                                                                    + 300.0,
                                                            }),
                                                        );
                                                    }
                                                }
                                            }
                                        } else {
                                            agent.target = None;
                                            data.idle_tree(
                                                agent,
                                                controller,
                                                &read_data,
                                                &mut event_emitter,
                                            );
                                        }
                                    }
                                } else {
                                    agent.target = None;
                                    data.idle_tree(
                                        agent,
                                        controller,
                                        &read_data,
                                        &mut event_emitter,
                                    );
                                }
                            },
                            _ => {
                                data.idle_tree(agent, controller, &read_data, &mut event_emitter);
                            },
                        }
                    }

                    debug_assert!(controller.inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                    debug_assert!(controller.inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
                },
            );
        for (agent, rtsim_entity) in (&mut agents, &read_data.rtsim_entities).join() {
            // Entity must be loaded in as it has an agent component :)
            // React to all events in the controller
            for event in core::mem::take(&mut agent.rtsim_controller.events) {
                match event {
                    RtSimEvent::AddMemory(memory) => {
                        rtsim.insert_entity_memory(rtsim_entity.0, memory.clone())
                    },
                    RtSimEvent::SetMood(memory) => {
                        rtsim.set_entity_mood(rtsim_entity.0, memory.clone())
                    },
                    _ => {},
                }
            }
        }
    }
}

impl<'a> AgentData<'a> {
    ////////////////////////////////////////
    // Subtrees
    ////////////////////////////////////////
    fn idle_tree(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        // Set owner if no target
        if agent.target.is_none() && thread_rng().gen_bool(0.1) {
            if let Some(Alignment::Owned(owner)) = self.alignment {
                if let Some(owner) = read_data.uid_allocator.retrieve_entity_internal(owner.id()) {
                    agent.target = Some(Target {
                        target: owner,
                        hostile: false,
                        selected_at: read_data.time.0,
                    });
                }
            }
        }
        // Interact if incoming messages
        if !agent.inbox.is_empty() {
            agent.action_timer = 0.1;
        }
        if agent.action_timer > 0.0 {
            if agent.action_timer
                < (if agent.behavior.is(BehaviorState::TRADING) {
                    TRADE_INTERACTION_TIME
                } else {
                    DEFAULT_INTERACTION_TIME
                })
            {
                self.interact(agent, controller, &read_data, event_emitter);
            } else {
                agent.action_timer = 0.0;
                agent.target = None;
                controller.actions.push(ControlAction::Stand);
                self.idle(agent, controller, &read_data);
            }
        } else if thread_rng().gen::<f32>() < 0.1 {
            self.choose_target(agent, controller, &read_data, event_emitter);
        } else {
            self.idle(agent, controller, &read_data);
        }
    }

    fn hostile_tree(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        if let Some(Target {
            target,
            selected_at,
            ..
        }) = agent.target
        {
            if let Some(tgt_pos) = read_data.positions.get(target) {
                let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
                // Should the agent flee?
                if 1.0 - agent.psyche.aggro > self.damage && self.flees {
                    if agent.action_timer == 0.0 && agent.behavior.can(BehaviorCapability::SPEAK) {
                        let msg = "npc.speech.villager_under_attack".to_string();
                        event_emitter
                            .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        agent.action_timer = 0.01;
                    } else if agent.action_timer < FLEE_DURATION || dist_sqrd < MAX_FLEE_DIST {
                        self.flee(
                            agent,
                            controller,
                            &read_data.terrain,
                            tgt_pos,
                            &read_data.dt,
                        );
                    } else {
                        agent.action_timer = 0.0;
                        agent.target = None;
                        self.idle(agent, controller, &read_data);
                    }

                // If not fleeing, attack the hostile
                // entity!
                } else {
                    // If the hostile entity is dead or in a safezone, return to idle
                    if should_stop_attacking(
                        read_data.healths.get(target),
                        read_data.buffs.get(target),
                    ) {
                        agent.target = None;
                        if agent.behavior.can(BehaviorCapability::SPEAK) {
                            let msg = "npc.speech.villager_enemy_killed".to_string();
                            event_emitter
                                .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        }
                    // Choose a new target every 10 seconds
                    // TODO: This should be more principled. Consider factoring
                    // health, combat rating, wielded
                    // weapon, etc, into the decision to change
                    // target.
                    } else if read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS {
                        self.choose_target(agent, controller, &read_data, event_emitter);
                    } else if dist_sqrd < SIGHT_DIST.powi(2) {
                        self.attack(
                            agent,
                            controller,
                            &read_data.terrain,
                            tgt_pos,
                            read_data.bodies.get(target),
                            &read_data.dt,
                            &read_data,
                        );
                    } else {
                        agent.target = None;
                        self.idle(agent, controller, &read_data);
                    }
                }
            }
        }
    }

    ////////////////////////////////////////
    // Action Nodes
    ////////////////////////////////////////

    fn glider_fall(&self, agent: &mut Agent, controller: &mut Controller, read_data: &ReadData) {
        if self.vel.0.z < -26.0 {
            controller.actions.push(ControlAction::GlideWield);
            if let Some(Target { target, .. }) = agent.target {
                if let Some(tgt_pos) = read_data.positions.get(target) {
                    controller.inputs.move_dir = (self.pos.0 - tgt_pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero);
                }
            }
        }
    }

    fn idle(&self, agent: &mut Agent, controller: &mut Controller, read_data: &ReadData) {
        // Light lanterns at night
        // TODO Add a method to turn on NPC lanterns underground
        let lantern_equipped = self
            .inventory
            .equipped(EquipSlot::Lantern)
            .as_ref()
            .map_or(false, |item| {
                matches!(item.kind(), comp::item::ItemKind::Lantern(_))
            });
        let lantern_turned_on = self.light_emitter.is_some();
        let day_period = DayPeriod::from(read_data.time_of_day.0);
        // Only emit event for agents that have a lantern equipped
        if lantern_equipped && thread_rng().gen_bool(0.001) {
            if day_period.is_dark() && !lantern_turned_on {
                // Agents with turned off lanterns turn them on randomly once it's
                // nighttime and keep them on
                // Only emit event for agents that sill need to
                // turn on their lantern
                controller.events.push(ControlEvent::EnableLantern)
            } else if lantern_turned_on && day_period.is_light() {
                // agents with turned on lanterns turn them off randomly once it's
                // daytime and keep them off
                controller.events.push(ControlEvent::DisableLantern)
            }
        };

        agent.action_timer = 0.0;
        if let Some((travel_to, _destination)) = &agent.rtsim_controller.travel_to {
            // if it has an rtsim destination and can fly then it should
            // if it is flying and bumps something above it then it should move down
            if self.traversal_config.can_fly
                && !read_data
                    .terrain
                    .ray(self.pos.0, self.pos.0 + (Vec3::unit_z() * 3.0))
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_some())
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Fly));
            } else {
                controller
                    .actions
                    .push(ControlAction::CancelInput(InputKind::Fly))
            }
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                *travel_to,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                        * speed.min(agent.rtsim_controller.speed_factor);
                self.jump_if(controller, bearing.z > 1.5 || self.traversal_config.can_fly);
                controller.inputs.climb = Some(comp::Climb::Up);
                //.filter(|_| bearing.z > 0.1 || self.physics_state.in_liquid.is_some());

                controller.inputs.move_z = bearing.z
                    + if self.traversal_config.can_fly {
                        let obstacle_ahead = read_data
                            .terrain
                            .ray(
                                self.pos.0 + Vec3::unit_z(),
                                self.pos.0
                                    + bearing.try_normalized().unwrap_or_else(Vec3::unit_y) * 80.0
                                    + Vec3::unit_z(),
                            )
                            .until(Block::is_solid)
                            .cast()
                            .1
                            .map_or(true, |b| b.is_some());
                        let mut ground_too_close = self
                            .body
                            .map(|body| {
                                let height_approx = self.pos.0.y
                                    - read_data
                                        .world
                                        .sim()
                                        .get_alt_approx(self.pos.0.xy().map(|x: f32| x as i32))
                                        .unwrap_or(0.0);

                                height_approx < body.flying_height()
                            })
                            .unwrap_or(false);

                        const NUM_RAYS: usize = 5;
                        for i in 0..=NUM_RAYS {
                            let magnitude = self.body.map_or(20.0, |b| b.flying_height());
                            // Lerp between a line straight ahead and straight down to detect a
                            // wedge of obstacles we might fly into (inclusive so that both vectors
                            // are sampled)
                            if let Some(dir) = Lerp::lerp(
                                -Vec3::unit_z(),
                                Vec3::new(bearing.x, bearing.y, 0.0),
                                i as f32 / NUM_RAYS as f32,
                            )
                            .try_normalized()
                            {
                                ground_too_close |= read_data
                                    .terrain
                                    .ray(self.pos.0, self.pos.0 + magnitude * dir)
                                    .until(|b: &Block| b.is_solid() || b.is_liquid())
                                    .cast()
                                    .1
                                    .map_or(false, |b| b.is_some())
                            }
                        }

                        if obstacle_ahead || ground_too_close {
                            1.0 //fly up when approaching obstacles
                        } else {
                            -0.1
                        } //flying things should slowly come down from the stratosphere
                    } else {
                        0.05 //normal land traveller offset
                    };

                // Put away weapon
                if thread_rng().gen_bool(0.1)
                    && matches!(
                        read_data.char_states.get(*self.entity),
                        Some(CharacterState::Wielding)
                    )
                {
                    controller.actions.push(ControlAction::Unwield);
                }
            }
        } else {
            agent.bearing += Vec2::new(
                thread_rng().gen::<f32>() - 0.5,
                thread_rng().gen::<f32>() - 0.5,
            ) * 0.1
                - agent.bearing * 0.003
                - agent.patrol_origin.map_or(Vec2::zero(), |patrol_origin| {
                    (self.pos.0 - patrol_origin).xy() * 0.0002
                });

            // Stop if we're too close to a wall
            agent.bearing *= 0.1
                + if read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0
                            + Vec3::from(agent.bearing)
                                .try_normalized()
                                .unwrap_or_else(Vec3::unit_y)
                                * 5.0
                            + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_none())
                {
                    0.9
                } else {
                    0.0
                };

            if agent.bearing.magnitude_squared() > 0.5f32.powi(2) {
                controller.inputs.move_dir = agent.bearing * 0.65;
            }

            // Put away weapon
            if thread_rng().gen_bool(0.1)
                && matches!(
                    read_data.char_states.get(*self.entity),
                    Some(CharacterState::Wielding)
                )
            {
                controller.actions.push(ControlAction::Unwield);
            }

            // Sit
            if thread_rng().gen::<f32>() < 0.0035 {
                controller.actions.push(ControlAction::Sit);
            }
        }
    }

    fn interact(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        // TODO: Process group invites
        // TODO: Add Group AgentEvent
        // let accept = false;  // set back to "matches!(alignment, Alignment::Npc)"
        // when we got better NPC recruitment mechanics if accept {
        //     // Clear agent comp
        //     //*agent = Agent::default();
        //     controller
        //         .events
        //         .push(ControlEvent::InviteResponse(InviteResponse::Accept));
        // } else {
        //     controller
        //         .events
        //         .push(ControlEvent::InviteResponse(InviteResponse::Decline));
        // }
        agent.action_timer += read_data.dt.0;
        let msg = agent.inbox.pop_back();
        match msg {
            Some(AgentEvent::Talk(by, subject)) => {
                if agent.behavior.can(BehaviorCapability::SPEAK) {
                    if let Some(target) = read_data.uid_allocator.retrieve_entity_internal(by.id())
                    {
                        agent.target = Some(Target {
                            target,
                            hostile: false,
                            selected_at: read_data.time.0,
                        });

                        if self.look_toward(controller, read_data, &target) {
                            controller.actions.push(ControlAction::Stand);
                            controller.actions.push(ControlAction::Talk);
                            match subject {
                                Subject::Regular => {
                                    if let (
                                        Some((_travel_to, destination_name)),
                                        Some(rtsim_entity),
                                    ) = (&agent.rtsim_controller.travel_to, &self.rtsim_entity)
                                    {
                                        let msg =
                                            if let Some(tgt_stats) = read_data.stats.get(target) {
                                                agent.rtsim_controller.events.push(
                                                    RtSimEvent::AddMemory(Memory {
                                                        item: MemoryItem::CharacterInteraction {
                                                            name: tgt_stats.name.clone(),
                                                        },
                                                        time_to_forget: read_data.time.0 + 600.0,
                                                    }),
                                                );
                                                if rtsim_entity
                                                    .brain
                                                    .remembers_character(&tgt_stats.name)
                                                {
                                                    format!(
                                                        "Greetings fair {}! It has been far too \
                                                         long since last I saw you. I'm going to \
                                                         {} right now.",
                                                        &tgt_stats.name, destination_name
                                                    )
                                                } else {
                                                    format!(
                                                        "I'm heading to {}! Want to come along?",
                                                        destination_name
                                                    )
                                                }
                                            } else {
                                                format!(
                                                    "I'm heading to {}! Want to come along?",
                                                    destination_name
                                                )
                                            };
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(*self.uid, msg),
                                        ));
                                    } else if agent.behavior.can_trade() {
                                        let msg = "npc.speech.merchant_advertisement".to_string();
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(*self.uid, msg),
                                        ));
                                    } else {
                                        let msg = "npc.speech.villager".to_string();
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(*self.uid, msg),
                                        ));
                                    }
                                },
                                Subject::Trade => {
                                    if agent.behavior.can_trade() {
                                        if !agent.behavior.is(BehaviorState::TRADING) {
                                            controller.events.push(ControlEvent::InitiateInvite(
                                                by,
                                                InviteKind::Trade,
                                            ));
                                            let msg =
                                                "npc.speech.merchant_advertisement".to_string();
                                            event_emitter.emit(ServerEvent::Chat(
                                                UnresolvedChatMsg::npc(*self.uid, msg),
                                            ));
                                        } else {
                                            event_emitter.emit(ServerEvent::Chat(
                                                UnresolvedChatMsg::npc(
                                                    *self.uid,
                                                    "npc.speech.merchant_busy".to_string(),
                                                ),
                                            ));
                                        }
                                    } else {
                                        // TODO: maybe make some travellers willing to trade with
                                        // simpler goods like potions
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(
                                                *self.uid,
                                                "npc.speech.villager_decline_trade".to_string(),
                                            ),
                                        ));
                                    }
                                },
                                Subject::Mood => {
                                    if let Some(rtsim_entity) = self.rtsim_entity {
                                        if !rtsim_entity.brain.remembers_mood() {
                                            // TODO: the following code will need a rework to
                                            // implement more mood contexts
                                            // This require that town NPCs becomes rtsim_entities to
                                            // work fully.
                                            match rand::random::<u32>() % 3 {
                                                0 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Good(
                                                                MoodContext::GoodWeather,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 21200.0,
                                                    }),
                                                ),
                                                1 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Neutral(
                                                                MoodContext::EverydayLife,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 21200.0,
                                                    }),
                                                ),
                                                2 => agent.rtsim_controller.events.push(
                                                    RtSimEvent::SetMood(Memory {
                                                        item: MemoryItem::Mood {
                                                            state: MoodState::Bad(
                                                                MoodContext::GoodWeather,
                                                            ),
                                                        },
                                                        time_to_forget: read_data.time.0 + 86400.0,
                                                    }),
                                                ),
                                                _ => {}, // will never happen
                                            }
                                        }
                                        if let Some(memory) = rtsim_entity.brain.get_mood() {
                                            let msg = match &memory.item {
                                                MemoryItem::Mood { state } => state.describe(),
                                                _ => "".to_string(),
                                            };
                                            event_emitter.emit(ServerEvent::Chat(
                                                UnresolvedChatMsg::npc(*self.uid, msg),
                                            ));
                                        }
                                    }
                                },
                                Subject::Location(location) => {
                                    if let Some(tgt_pos) = read_data.positions.get(target) {
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(
                                                *self.uid,
                                                format!(
                                                    "{} ? I think it's {} {} from here!",
                                                    location.name,
                                                    Distance::from_dir(
                                                        location.origin.as_::<f32>()
                                                            - tgt_pos.0.xy()
                                                    )
                                                    .name(),
                                                    Direction::from_dir(
                                                        location.origin.as_::<f32>()
                                                            - tgt_pos.0.xy()
                                                    )
                                                    .name()
                                                ),
                                            ),
                                        ));
                                    }
                                },
                                Subject::Person(person) => {
                                    if let Some(src_pos) = read_data.positions.get(target) {
                                        let msg = if let Some(person_pos) = person.origin {
                                            let distance = Distance::from_dir(
                                                person_pos.xy() - src_pos.0.xy(),
                                            );
                                            match distance {
                                                Distance::NextTo | Distance::Near => {
                                                    format!(
                                                        "{} ? I think he's {} {} from here!",
                                                        person.name(),
                                                        distance.name(),
                                                        Direction::from_dir(
                                                            person_pos.xy() - src_pos.0.xy(),
                                                        )
                                                        .name()
                                                    )
                                                },
                                                _ => {
                                                    format!(
                                                        "{} ? I think he's gone visiting another \
                                                         town. Come back later!",
                                                        person.name()
                                                    )
                                                },
                                            }
                                        } else {
                                            format!(
                                                "{} ? Sorry, I don't know where you can find him.",
                                                person.name()
                                            )
                                        };
                                        event_emitter.emit(ServerEvent::Chat(
                                            UnresolvedChatMsg::npc(*self.uid, msg),
                                        ));
                                    }
                                },
                                Subject::Work => {},
                            }
                        }
                    }
                }
            },
            Some(AgentEvent::TradeInvite(with)) => {
                if agent.behavior.can_trade() {
                    if !agent.behavior.is(BehaviorState::TRADING) {
                        // stand still and looking towards the trading player
                        controller.actions.push(ControlAction::Stand);
                        controller.actions.push(ControlAction::Talk);
                        if let Some(target) =
                            read_data.uid_allocator.retrieve_entity_internal(with.id())
                        {
                            agent.target = Some(Target {
                                target,
                                hostile: false,
                                selected_at: read_data.time.0,
                            });
                        }
                        controller
                            .events
                            .push(ControlEvent::InviteResponse(InviteResponse::Accept));
                        agent.behavior.unset(BehaviorState::TRADING_ISSUER);
                        agent.behavior.set(BehaviorState::TRADING);
                    } else {
                        controller
                            .events
                            .push(ControlEvent::InviteResponse(InviteResponse::Decline));
                        if agent.behavior.can(BehaviorCapability::SPEAK) {
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                                *self.uid,
                                "npc.speech.merchant_busy".to_string(),
                            )));
                        }
                    }
                } else {
                    // TODO: Provide a hint where to find the closest merchant?
                    controller
                        .events
                        .push(ControlEvent::InviteResponse(InviteResponse::Decline));
                    if agent.behavior.can(BehaviorCapability::SPEAK) {
                        event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                            *self.uid,
                            "npc.speech.villager_decline_trade".to_string(),
                        )));
                    }
                }
            },
            Some(AgentEvent::TradeAccepted(with)) => {
                if !agent.behavior.is(BehaviorState::TRADING) {
                    if let Some(target) =
                        read_data.uid_allocator.retrieve_entity_internal(with.id())
                    {
                        agent.target = Some(Target {
                            target,
                            hostile: false,
                            selected_at: read_data.time.0,
                        });
                    }
                    agent.behavior.set(BehaviorState::TRADING);
                    agent.behavior.set(BehaviorState::TRADING_ISSUER);
                }
            },
            Some(AgentEvent::FinishedTrade(result)) => {
                if agent.behavior.is(BehaviorState::TRADING) {
                    match result {
                        TradeResult::Completed => {
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                                *self.uid,
                                "npc.speech.merchant_trade_successful".to_string(),
                            )))
                        },
                        _ => event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                            *self.uid,
                            "npc.speech.merchant_trade_declined".to_string(),
                        ))),
                    }
                    agent.behavior.unset(BehaviorState::TRADING);
                }
            },
            Some(AgentEvent::UpdatePendingTrade(boxval)) => {
                let (tradeid, pending, prices, inventories) = *boxval;
                if agent.behavior.is(BehaviorState::TRADING) {
                    let who: usize = if agent.behavior.is(BehaviorState::TRADING_ISSUER) {
                        0
                    } else {
                        1
                    };
                    let balance0: f32 =
                        prices.balance(&pending.offers, &inventories, 1 - who, true);
                    let balance1: f32 = prices.balance(&pending.offers, &inventories, who, false);
                    tracing::debug!("UpdatePendingTrade({}, {})", balance0, balance1);
                    if balance0 >= balance1 {
                        // If the trade is favourable to us, only send an accept message if we're
                        // not already accepting (since otherwise, spamclicking the accept button
                        // results in lagging and moving to the review phase of an unfavorable trade
                        // (although since the phase is included in the message, this shouldn't
                        // result in fully accepting an unfavourable trade))
                        if !pending.accept_flags[who] {
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *self.entity,
                                tradeid,
                                TradeAction::Accept(pending.phase),
                            ));
                        }
                    } else {
                        if balance1 > 0.0 {
                            let msg = format!(
                                "That only covers {:.1}% of my costs!",
                                balance0 / balance1 * 100.0
                            );
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                                *self.uid, msg,
                            )));
                        }
                        if pending.phase != TradePhase::Mutate {
                            // we got into the review phase but without balanced goods, decline
                            agent.behavior.unset(BehaviorState::TRADING);
                            event_emitter.emit(ServerEvent::ProcessTradeAction(
                                *self.entity,
                                tradeid,
                                TradeAction::Decline,
                            ));
                        }
                    }
                }
            },
            None => {
                if agent.behavior.can(BehaviorCapability::SPEAK) {
                    // no new events, continue looking towards the last interacting player for some
                    // time
                    if let Some(Target { target, .. }) = &agent.target {
                        self.look_toward(controller, read_data, target);
                    } else {
                        agent.action_timer = 0.0;
                    }
                }
            },
        }
    }

    fn look_toward(
        &self,
        controller: &mut Controller,
        read_data: &ReadData,
        target: &EcsEntity,
    ) -> bool {
        if let Some(tgt_pos) = read_data.positions.get(*target) {
            let eye_offset = self.body.map_or(0.0, |b| b.eye_height());
            let tgt_eye_offset = read_data
                .bodies
                .get(*target)
                .map_or(0.0, |b| b.eye_height());
            if let Some(dir) = Dir::from_unnormalized(
                Vec3::new(tgt_pos.0.x, tgt_pos.0.y, tgt_pos.0.z + tgt_eye_offset)
                    - Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
            ) {
                controller.inputs.look_dir = dir;
            }
            true
        } else {
            false
        }
    }

    fn flee(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
        dt: &DeltaTime,
    ) {
        if let Some(body) = self.body {
            if body.can_strafe() && !self.is_gliding {
                controller.actions.push(ControlAction::Unwield);
            }
        }
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*terrain,
            self.pos.0,
            self.vel.0,
            // Away from the target (ironically)
            self.pos.0
                + (self.pos.0 - tgt_pos.0)
                    .try_normalized()
                    .unwrap_or_else(Vec3::unit_y)
                    * 50.0,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config
            },
        ) {
            controller.inputs.move_dir =
                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
        }
        agent.action_timer += dt.0;
    }

    fn choose_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        agent.action_timer = 0.0;

        // Search for new targets (this looks expensive, but it's only run occasionally)
        // TODO: Replace this with a better system that doesn't consider *all* entities
        let target = (&read_data.entities, &read_data.positions, &read_data.healths, &read_data.stats, &read_data.inventories, read_data.alignments.maybe(), read_data.char_states.maybe())
            .join()
            .filter(|(e, e_pos, e_health, e_stats, e_inventory, e_alignment, char_state)| {
                let mut search_dist = SEARCH_DIST;
                let mut listen_dist = LISTEN_DIST;
                if char_state.map_or(false, |c_s| c_s.is_stealthy()) {
                    // TODO: make sneak more effective based on a stat like e_stats.fitness
                    search_dist *= SNEAK_COEFFICIENT;
                    listen_dist *= SNEAK_COEFFICIENT;
                }
                ((e_pos.0.distance_squared(self.pos.0) < search_dist.powi(2) &&
                    // Within our view
                    (e_pos.0 - self.pos.0).try_normalized().map(|v| v.dot(*controller.inputs.look_dir) > 0.15).unwrap_or(true))
                        // Within listen distance
                        || e_pos.0.distance_squared(self.pos.0) < listen_dist.powi(2)) // TODO implement proper sound system for agents
                    && e != self.entity
                    && !e_health.is_dead
                    && (try_owner_alignment(self.alignment, &read_data).and_then(|a| try_owner_alignment(*e_alignment, &read_data).map(|b| a.hostile_towards(*b))).unwrap_or(false) || (
                            if let Some(rtsim_entity) = &self.rtsim_entity {
                                if rtsim_entity.brain.remembers_fight_with_character(&e_stats.name) {
                                    agent.rtsim_controller.events.push(
                                        RtSimEvent::AddMemory(Memory {
                                            item: MemoryItem::CharacterFight { name: e_stats.name.clone() },
                                            time_to_forget: read_data.time.0 + 300.0,
                                        })
                                    );
                                    let msg = format!("{}! How dare you cross me again!", e_stats.name.clone());
                                    event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        ) ||
                        (
                            self.alignment.map_or(false, |alignment| {
                                if matches!(alignment, Alignment::Npc) && e_inventory.equipped_items().filter(|item| item.tags().contains(&ItemTag::Cultist)).count() > 2 {
                                    if agent.behavior.can(BehaviorCapability::SPEAK) {
                                        if self.rtsim_entity.is_some() {
                                            agent.rtsim_controller.events.push(
                                                RtSimEvent::AddMemory(Memory {
                                                    item: MemoryItem::CharacterFight { name: e_stats.name.clone() },
                                                    time_to_forget: read_data.time.0 + 300.0,
                                                })
                                            );
                                        }
                                        let msg = "npc.speech.villager_cultist_alarm".to_string();
                                        event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                                    }
                                    true
                                } else {
                                    false
                                }
                            })
                        ))

            })
            // Can we even see them?
            .filter(|(_, e_pos, _, _, _, _, _)| read_data.terrain
                .ray(self.pos.0 + Vec3::unit_z(), e_pos.0 + Vec3::unit_z())
                .until(Block::is_opaque)
                .cast()
                .0 >= e_pos.0.distance(self.pos.0))
            .min_by_key(|(_, e_pos, _, _, _, _, _)| (e_pos.0.distance_squared(self.pos.0) * 100.0) as i32) // TODO choose target by more than just distance
            .map(|(e, _, _, _, _, _, _)| e);
        if let Some(target) = target {
            agent.target = Some(Target {
                target,
                hostile: true,
                selected_at: read_data.time.0,
            })
        } else {
            agent.target = None;
        }
    }

    fn jump_if(&self, controller: &mut Controller, condition: bool) {
        if condition {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Jump));
        } else {
            controller
                .actions
                .push(ControlAction::CancelInput(InputKind::Jump))
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
        tgt_body: Option<&Body>,
        dt: &DeltaTime,
        read_data: &ReadData,
    ) {
        let min_attack_dist = self.body.map_or(3.0, |b| b.radius() * self.scale + 2.0);
        let tactic = match self
            .inventory
            .equipped(EquipSlot::Mainhand)
            .as_ref()
            .and_then(|item| {
                if let ItemKind::Tool(tool) = &item.kind() {
                    Some(&tool.kind)
                } else {
                    None
                }
            }) {
            Some(ToolKind::Bow) | Some(ToolKind::BowSimple) => Tactic::Bow,
            Some(ToolKind::Staff) | Some(ToolKind::StaffSimple) => Tactic::Staff,
            Some(ToolKind::Hammer) => Tactic::Hammer,
            Some(ToolKind::Sword)
            | Some(ToolKind::Spear)
            | Some(ToolKind::SwordSimple)
            | Some(ToolKind::AxeSimple) => Tactic::Sword,
            Some(ToolKind::Axe) => Tactic::Axe,
            Some(ToolKind::Unique(UniqueKind::StoneGolemFist)) => Tactic::StoneGolemBoss,
            Some(ToolKind::Unique(UniqueKind::QuadMedQuick)) => Tactic::CircleCharge {
                radius: 3,
                circle_time: 2,
            },
            Some(ToolKind::Unique(UniqueKind::QuadMedCharge)) => Tactic::CircleCharge {
                radius: 12,
                circle_time: 1,
            },
            Some(ToolKind::Unique(UniqueKind::TheropodCharge)) => Tactic::CircleCharge {
                radius: 6,
                circle_time: 1,
            },

            Some(ToolKind::Unique(UniqueKind::QuadMedJump)) => Tactic::QuadMedJump,
            Some(ToolKind::Unique(UniqueKind::QuadMedBasic)) => Tactic::QuadMedBasic,
            Some(ToolKind::Unique(UniqueKind::QuadLowRanged)) => Tactic::QuadLowRanged,
            Some(ToolKind::Unique(UniqueKind::QuadLowTail)) => Tactic::TailSlap,
            Some(ToolKind::Unique(UniqueKind::QuadLowQuick)) => Tactic::QuadLowQuick,
            Some(ToolKind::Unique(UniqueKind::QuadLowBasic)) => Tactic::QuadLowBasic,
            Some(ToolKind::Unique(UniqueKind::QuadLowBreathe))
            | Some(ToolKind::Unique(UniqueKind::QuadLowBeam)) => Tactic::Lavadrake,
            Some(ToolKind::Unique(UniqueKind::TheropodBasic)) => Tactic::Theropod,
            Some(ToolKind::Unique(UniqueKind::TheropodBird)) => Tactic::Theropod,
            Some(ToolKind::Unique(UniqueKind::ObjectTurret)) => Tactic::Turret,
            Some(ToolKind::Unique(UniqueKind::MindflayerStaff)) => Tactic::Mindflayer,
            _ => Tactic::Melee,
        };

        // Wield the weapon as running towards the target
        controller.actions.push(ControlAction::Wield);

        let eye_offset = self.body.map_or(0.0, |b| b.eye_height());

        let tgt_eye_offset = tgt_body.map_or(0.0, |b| b.eye_height()) +
                   // Special case for jumping attacks to jump at the body
                   // of the target and not the ground around the target
                   // For the ranged it is to shoot at the feet and not
                   // the head to get splash damage
                   if tactic == Tactic::QuadMedJump {
                       1.0
                   } else if matches!(tactic, Tactic::QuadLowRanged) {
                       -1.0
                   } else {
                       0.0
                   };

        // Hacky distance offset for ranged weapons. This is
        // intentionally hacky for now before we make ranged
        // NPCs lead targets and implement varying aiming
        // skill
        let distance_offset = match tactic {
            Tactic::Bow => {
                0.0004 /* Yay magic numbers */ * self.pos.0.distance_squared(tgt_pos.0)
            },
            Tactic::Staff => {
                0.0015 /* Yay magic numbers */ * self.pos.0.distance_squared(tgt_pos.0)
            },
            Tactic::QuadLowRanged => {
                0.03 /* Yay magic numbers */ * self.pos.0.distance_squared(tgt_pos.0)
            },
            _ => 0.0,
        };

        // Apply the distance and eye offsets to make the
        // look_dir the vector from projectile launch to
        // target point
        if let Some(dir) = Dir::from_unnormalized(
            Vec3::new(
                tgt_pos.0.x,
                tgt_pos.0.y,
                tgt_pos.0.z + tgt_eye_offset + distance_offset,
            ) - Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
        ) {
            controller.inputs.look_dir = dir;
        }

        let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);

        // Match on tactic. Each tactic has different controls
        // depending on the distance from the agent to the target
        match tactic {
            Tactic::Melee => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                    controller.inputs.move_dir = Vec2::zero();
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }

                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Axe => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 6.0 {
                        controller
                            .actions
                            .push(ControlAction::CancelInput(InputKind::Secondary));
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 4.0 && self.energy.current() > 10 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Axe(AxeSkill::UnlockLeap))
                        && self.energy.current() > 800
                        && thread_rng().gen_bool(0.5)
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                        agent.action_timer += dt.0;
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Hammer => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 4.0 {
                        controller
                            .actions
                            .push(ControlAction::CancelInput(InputKind::Secondary));
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 2.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                        && self.energy.current() > 700
                        && thread_rng().gen_bool(0.9)
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                        agent.action_timer += dt.0;
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            if self
                                .stats
                                .skill_set
                                .has_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                                && agent.action_timer > 5.0
                            {
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Sword => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Sword(SwordSkill::UnlockSpin))
                        && agent.action_timer < 2.0
                        && self.energy.current() > 600
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                        agent.action_timer += dt.0;
                    } else if agent.action_timer > 2.0 {
                        agent.action_timer = 0.0;
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            if agent.action_timer > 4.0 {
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Secondary));
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Bow => {
                if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                    && dist_sqrd < (2.0 * min_attack_dist).powi(2)
                {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Roll));
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller.inputs.move_dir = bearing
                                .xy()
                                .rotated_z(thread_rng().gen_range(0.5..1.57))
                                .try_normalized()
                                .unwrap_or_else(Vec2::zero)
                                * speed;
                            if agent.action_timer > 4.0 {
                                controller
                                    .actions
                                    .push(ControlAction::CancelInput(InputKind::Secondary));
                                agent.action_timer = 0.0;
                            } else if agent.action_timer > 2.0 && self.energy.current() > 300 {
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Secondary));
                                agent.action_timer += dt.0;
                            } else if self
                                .stats
                                .skill_set
                                .has_skill(Skill::Bow(BowSkill::UnlockRepeater))
                                && self.energy.current() > 400
                                && thread_rng().gen_bool(0.8)
                            {
                                controller
                                    .actions
                                    .push(ControlAction::CancelInput(InputKind::Secondary));
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                                agent.action_timer += dt.0;
                            } else {
                                controller
                                    .actions
                                    .push(ControlAction::CancelInput(InputKind::Secondary));
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Primary));
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Staff => {
                if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                    && dist_sqrd < min_attack_dist.powi(2)
                {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Roll));
                } else if dist_sqrd < (5.0 * min_attack_dist).powi(2) {
                    if agent.action_timer < 1.5 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 3.0 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else {
                        agent.action_timer = 0.0;
                    }
                    if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Staff(StaffSkill::UnlockShockwave))
                        && self.energy.current() > 800
                        && thread_rng().gen::<f32>() > 0.8
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                    } else if self.energy.current() > 10 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller.inputs.move_dir = bearing
                                .xy()
                                .rotated_z(thread_rng().gen_range(-1.57..-0.5))
                                .try_normalized()
                                .unwrap_or_else(Vec2::zero)
                                * speed;
                            controller
                                .actions
                                .push(ControlAction::basic_input(InputKind::Primary));
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Roll));
                    }
                } else if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::StoneGolemBoss => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    // 2.0 is temporary correction factor to allow them to melee with their
                    // large hitbox
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                    //controller.inputs.primary.set_state(true);
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if self.vel.0.is_approx_zero() {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                    }
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            if agent.action_timer > 5.0 {
                                controller
                                    .actions
                                    .push(ControlAction::basic_input(InputKind::Secondary));
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::CircleCharge {
                radius,
                circle_time,
            } => {
                if dist_sqrd < min_attack_dist.powi(2) && thread_rng().gen_bool(0.5) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else if dist_sqrd < (radius as f32 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = (self.pos.0 - tgt_pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y);
                } else if dist_sqrd < ((radius as f32 + 1.0) * min_attack_dist).powi(2)
                    && dist_sqrd > (radius as f32 * min_attack_dist).powi(2)
                {
                    if agent.action_timer < circle_time as f32 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < circle_time as f32 + 0.5 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 2.0 * circle_time as f32 + 0.5 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 2.0 * circle_time as f32 + 1.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else {
                        agent.action_timer = 0.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowRanged => {
                if dist_sqrd < (3.0 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y);
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            if agent.action_timer > 5.0 {
                                agent.action_timer = 0.0;
                            } else if agent.action_timer > 2.5 {
                                controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                                    .xy()
                                    .rotated_z(1.75 * PI)
                                    .try_normalized()
                                    .unwrap_or_else(Vec2::zero)
                                    * speed;
                                agent.action_timer += dt.0;
                            } else {
                                controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                                    .xy()
                                    .rotated_z(0.25 * PI)
                                    .try_normalized()
                                    .unwrap_or_else(Vec2::zero)
                                    * speed;
                                agent.action_timer += dt.0;
                            }
                            controller
                                .actions
                                .push(ControlAction::basic_input(InputKind::Secondary));
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    } else {
                        agent.target = None;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::TailSlap => {
                if dist_sqrd < (1.5 * min_attack_dist).powi(2) {
                    if agent.action_timer > 4.0 {
                        controller
                            .actions
                            .push(ControlAction::CancelInput(InputKind::Primary));
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 1.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    }
                    controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y)
                        * 0.1;
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowQuick => {
                if dist_sqrd < (1.5 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                } else if dist_sqrd < (3.0 * min_attack_dist).powi(2)
                    && dist_sqrd > (2.0 * min_attack_dist).powi(2)
                {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                    controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                        .xy()
                        .rotated_z(-0.47 * PI)
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y);
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowBasic => {
                if dist_sqrd < (1.5 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 5.0 {
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 2.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadMedJump => {
                if dist_sqrd < (1.5 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                } else if dist_sqrd < (5.0 * min_attack_dist).powi(2) {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Ability(0)));
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                            controller
                                .actions
                                .push(ControlAction::basic_input(InputKind::Primary));
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            self.jump_if(controller, bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadMedBasic => {
                if dist_sqrd < min_attack_dist.powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer < 2.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 3.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    } else {
                        agent.action_timer = 0.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Lavadrake | Tactic::QuadLowBeam => {
                if dist_sqrd < (2.5 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                } else if dist_sqrd < (7.0 * min_attack_dist).powi(2) {
                    if agent.action_timer < 2.0 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 4.0 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 6.0 {
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Ability(0)));
                        agent.action_timer += dt.0;
                    } else {
                        agent.action_timer = 0.0;
                    }
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Theropod => {
                if dist_sqrd < (2.0 * min_attack_dist).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if let Some((bearing, speed)) = agent.chaser.chase(
                        &*terrain,
                        self.pos.0,
                        self.vel.0,
                        tgt_pos.0,
                        TraversalConfig {
                            min_tgt_dist: 1.25,
                            ..self.traversal_config
                        },
                    ) {
                        controller.inputs.move_dir =
                            bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        self.jump_if(controller, bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Turret => {
                if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else {
                    agent.target = None;
                }
            },
            Tactic::FixedTurret => {
                controller.inputs.look_dir = self.ori.look_dir();
                if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else {
                    agent.target = None;
                }
            },
            Tactic::RotatingTurret => {
                controller.inputs.look_dir = Dir::new(
                    Quaternion::from_xyzw(self.ori.look_dir().x, self.ori.look_dir().y, 0.0, 0.0)
                        .rotated_z(6.0 * dt.0 as f32)
                        .into_vec3()
                        .try_normalized()
                        .unwrap_or_default(),
                );
                if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else {
                    agent.target = None;
                }
            },
            Tactic::Mindflayer => {
                const MINDFLAYER_ATTACK_DIST: f32 = 16.0;
                const MINION_SUMMON_THRESHOLD: f32 = 0.20;
                let health_fraction = self.health.map_or(0.5, |h| h.fraction());
                // Extreme hack to set action_timer at start of combat
                if agent.action_timer < MINION_SUMMON_THRESHOLD
                    && health_fraction > MINION_SUMMON_THRESHOLD
                {
                    agent.action_timer = health_fraction - MINION_SUMMON_THRESHOLD;
                }
                let mindflayer_is_far = dist_sqrd > MINDFLAYER_ATTACK_DIST.powi(2);
                if agent.action_timer > health_fraction {
                    // Summon minions at particular thresholds of health
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Ability(1)));
                    if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
                    {
                        agent.action_timer -= MINION_SUMMON_THRESHOLD;
                    }
                } else if mindflayer_is_far {
                    // If too far from target, blink to them.
                    controller.actions.push(ControlAction::StartInput {
                        input: InputKind::Ability(0),
                        target_entity: agent
                            .target
                            .as_ref()
                            .and_then(|t| read_data.uids.get(t.target))
                            .copied(),
                        select_pos: None,
                    });
                } else {
                    // If close to target, use either primary or secondary ability
                    if matches!(self.char_state, CharacterState::BasicBeam(c) if c.timer < Duration::from_secs(10) && !matches!(c.stage_section, StageSection::Recover))
                    {
                        // If already using primary, keep using primary until 10 consecutive seconds
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                    } else if matches!(self.char_state, CharacterState::SpinMelee(c) if c.consecutive_spins < 50 && !matches!(c.stage_section, StageSection::Recover))
                    {
                        // If already using secondary, keep using secondary until 10 consecutive
                        // seconds
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                    } else if thread_rng().gen_bool(health_fraction.into()) {
                        // Else if at high health, use primary
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Primary));
                    } else {
                        // Else use secondary
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Secondary));
                    }
                }

                // Move towards target
                if let Some((bearing, speed)) = agent.chaser.chase(
                    &*terrain,
                    self.pos.0,
                    self.vel.0,
                    tgt_pos.0,
                    TraversalConfig {
                        min_tgt_dist: 1.25,
                        ..self.traversal_config
                    },
                ) {
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(controller, bearing.z > 1.5);
                    controller.inputs.move_z = bearing.z;
                }
            },
        }
    }

    fn follow(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
    ) {
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*terrain,
            self.pos.0,
            self.vel.0,
            tgt_pos.0,
            TraversalConfig {
                min_tgt_dist: AVG_FOLLOW_DIST,
                ..self.traversal_config
            },
        ) {
            let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
            controller.inputs.move_dir = bearing.xy().try_normalized().unwrap_or_else(Vec2::zero)
                * speed.min(0.2 + (dist_sqrd - AVG_FOLLOW_DIST.powi(2)) / 8.0);
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
        }
    }
}

fn can_see_tgt(terrain: &TerrainGrid, pos: &Pos, tgt_pos: &Pos, dist_sqrd: f32) -> bool {
    terrain
        .ray(pos.0 + Vec3::unit_z(), tgt_pos.0 + Vec3::unit_z())
        .until(Block::is_opaque)
        .cast()
        .0
        .powi(2)
        >= dist_sqrd
}

// If target is dead or has invulnerability buff, returns true
fn should_stop_attacking(health: Option<&Health>, buffs: Option<&Buffs>) -> bool {
    health.map_or(true, |a| a.is_dead)
        || buffs.map_or(false, |b| b.kinds.contains_key(&BuffKind::Invulnerability))
}

/// Attempts to get alignment of owner if entity has Owned alignment
fn try_owner_alignment<'a>(
    alignment: Option<&'a Alignment>,
    read_data: &'a ReadData,
) -> Option<&'a Alignment> {
    if let Some(Alignment::Owned(owner_uid)) = alignment {
        if let Some(owner) = read_data
            .uid_allocator
            .retrieve_entity_internal(owner_uid.id())
        {
            return read_data.alignments.get(owner);
        }
    }
    alignment
}
