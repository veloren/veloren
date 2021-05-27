use crate::rtsim::{Entity as RtSimData, RtSim};
use common::{
    comp::{
        self,
        agent::{
            AgentEvent, Target, DEFAULT_INTERACTION_TIME, MAX_LISTEN_DIST, TRADE_INTERACTION_TIME,
        },
        buff::{BuffKind, Buffs},
        compass::{Direction, Distance},
        dialogue::{MoodContext, MoodState, Subject},
        group,
        inventory::{item::ItemTag, slot::EquipSlot},
        invite::{InviteKind, InviteResponse},
        item::{
            tool::{AbilitySpec, ToolKind},
            Item, ItemDesc, ItemKind,
        },
        skills::{AxeSkill, BowSkill, HammerSkill, Skill, StaffSkill, SwordSkill},
        Agent, Alignment, BehaviorCapability, BehaviorState, Body, CharacterAbility,
        CharacterState, ControlAction, ControlEvent, Controller, Energy, Health, HealthChange,
        InputKind, Inventory, InventoryAction, LightEmitter, MountState, Ori, PhysicsState, Pos,
        Scale, SkillSet, Stats, UnresolvedChatMsg, Vel,
    },
    consts::GRAVITY,
    effect::{BuffEffect, Effect},
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
    skill_set: &'a SkillSet,
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
    cached_spatial_grid: &'a common::CachedSpatialGrid,
}

struct TargetData<'a> {
    pos: &'a Pos,
    body: Option<&'a Body>,
    scale: Option<&'a Scale>,
}

struct AttackData {
    min_attack_dist: f32,
    dist_sqrd: f32,
    angle: f32,
}

#[derive(Eq, PartialEq)]
pub enum Tactic {
    Melee,
    Axe,
    Hammer,
    Sword,
    Bow,
    Staff,
    StoneGolem,
    CircleCharge { radius: u32, circle_time: u32 },
    QuadLowRanged,
    TailSlap,
    QuadLowQuick,
    QuadLowBasic,
    QuadLowBeam,
    QuadMedJump,
    QuadMedBasic,
    Theropod,
    Turret,
    FixedTurret,
    RotatingTurret,
    Mindflayer,
    BirdLargeBreathe,
    BirdLargeFire,
    Minotaur,
    ClayGolem,
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    uid_allocator: Read<'a, UidAllocator>,
    dt: Read<'a, DeltaTime>,
    time: Read<'a, Time>,
    cached_spatial_grid: Read<'a, common::CachedSpatialGrid>,
    group_manager: Read<'a, group::GroupManager>,
    energies: ReadStorage<'a, Energy>,
    positions: ReadStorage<'a, Pos>,
    velocities: ReadStorage<'a, Vel>,
    orientations: ReadStorage<'a, Ori>,
    scales: ReadStorage<'a, Scale>,
    healths: ReadStorage<'a, Health>,
    inventories: ReadStorage<'a, Inventory>,
    stats: ReadStorage<'a, Stats>,
    skill_set: ReadStorage<'a, SkillSet>,
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
    #[cfg(feature = "worldgen")]
    world: ReadExpect<'a, Arc<world::World>>,
    rtsim_entities: ReadStorage<'a, RtSimEntity>,
    buffs: ReadStorage<'a, Buffs>,
}

// This is 3.1 to last longer than the last damage timer (3.0 seconds)
const DAMAGE_MEMORY_DURATION: f64 = 3.1;
const FLEE_DURATION: f32 = 3.0;
const MAX_FOLLOW_DIST: f32 = 12.0;
const MAX_PATH_DIST: f32 = 170.0;
const PARTIAL_PATH_DIST: f32 = 50.0;
const MAX_FLEE_DIST: f32 = 20.0;
const SEARCH_DIST: f32 = 48.0;
const SNEAK_COEFFICIENT: f32 = 0.25;
const AVG_FOLLOW_DIST: f32 = 6.0;
const RETARGETING_THRESHOLD_SECONDS: f64 = 10.0;
const HEALING_ITEM_THRESHOLD: f32 = 0.5;
const DEFAULT_ATTACK_RANGE: f32 = 2.0;
const AWARENESS_INVESTIGATE_THRESHOLD: f32 = 1.0;
const AWARENESS_DECREMENT_CONSTANT: f32 = 0.07;
const SECONDS_BEFORE_FORGET_SOUNDS: f64 = 180.0;

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
            &read_data.skill_set,
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
                    skill_set,
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
                        Some(CharacterState::GlideWield) | Some(CharacterState::Glide(_))
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
                        in_liquid: physics_state.in_liquid().is_some(),
                        min_tgt_dist: 1.0,
                        can_climb: body.map(|b| b.can_climb()).unwrap_or(false),
                        can_fly: body.map(|b| b.fly_thrust().is_some()).unwrap_or(false),
                    };

                    let flees = alignment
                        .map(|a| !matches!(a, Alignment::Enemy | Alignment::Owned(_)))
                        .unwrap_or(true);
                    let damage = health.map_or(1.0, |h| h.current() as f32 / h.maximum() as f32);
                    let rtsim_entity = read_data
                        .rtsim_entities
                        .get(entity)
                        .and_then(|rtsim_ent| rtsim.get_entity(rtsim_ent.0));

                    if traversal_config.can_fly && rtsim_entity.is_some() {
                        // hack (kinda): Never turn off flight for rtsim entities that can fly
                        // at all, since it results in stuttering and falling back to the ground.
                        controller
                            .actions
                            .push(ControlAction::basic_input(InputKind::Fly));
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
                        flees,
                        damage,
                        light_emitter,
                        glider_equipped,
                        is_gliding,
                        health: read_data.healths.get(entity),
                        char_state,
                        cached_spatial_grid: &read_data.cached_spatial_grid,
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
                                                            let target_data = TargetData {
                                                                pos: tgt_pos,
                                                                body: read_data
                                                                    .bodies
                                                                    .get(attacker),
                                                                scale: read_data
                                                                    .scales
                                                                    .get(attacker),
                                                            };
                                                            data.attack(
                                                                agent,
                                                                controller,
                                                                &target_data,
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
                                            // target and idle.
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
                                                let target_data = TargetData {
                                                    pos: tgt_pos,
                                                    body: read_data.bodies.get(attacker),
                                                    scale: read_data.scales.get(attacker),
                                                };
                                                data.attack(
                                                    agent,
                                                    controller,
                                                    &target_data,
                                                    &read_data,
                                                );
                                                // Remember this encounter if an RtSim entity
                                                if let Some(tgt_stats) = data
                                                    .rtsim_entity
                                                    .and_then(|_| read_data.stats.get(attacker))
                                                {
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
        decrement_awareness(agent);
        forget_old_sounds(agent, read_data);

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
            if !matches!(agent.inbox.front(), Some(AgentEvent::ServerSound(_))) {
                agent.action_state.timer = 0.1;
            } else if let Some(AgentEvent::ServerSound(sound)) = agent.inbox.pop_front() {
                agent.sounds_heard.push(sound);
                agent.awareness += sound.vol;
            }
        }
        if agent.action_state.timer > 0.0 {
            if agent.action_state.timer
                < (if agent.behavior.is(BehaviorState::TRADING) {
                    TRADE_INTERACTION_TIME
                } else {
                    DEFAULT_INTERACTION_TIME
                })
            {
                self.interact(agent, controller, &read_data, event_emitter);
            } else {
                agent.action_state.timer = 0.0;
                agent.target = None;
                controller.actions.push(ControlAction::Stand);
                self.idle(agent, controller, &read_data);
            }
        } else if thread_rng().gen::<f32>() < 0.1 {
            self.choose_target(agent, controller, &read_data, event_emitter);
        } else if agent.awareness > AWARENESS_INVESTIGATE_THRESHOLD {
            self.handle_elevated_awareness(agent, controller, read_data);
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
        if self.damage < HEALING_ITEM_THRESHOLD && self.heal_self(agent, controller) {
            agent.action_state.timer = 0.01;
            return;
        }

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
                    if agent.action_state.timer == 0.0
                        && agent.behavior.can(BehaviorCapability::SPEAK)
                    {
                        let msg = "npc.speech.villager_under_attack".to_string();
                        event_emitter
                            .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        agent.action_state.timer = 0.01;
                    } else if agent.action_state.timer < FLEE_DURATION || dist_sqrd < MAX_FLEE_DIST
                    {
                        self.flee(agent, controller, &read_data.terrain, tgt_pos);
                        agent.action_state.timer += read_data.dt.0;
                    } else {
                        agent.action_state.timer = 0.0;
                        agent.target = None;
                        self.idle(agent, controller, &read_data);
                    }

                // If not fleeing, attack the hostile entity!
                } else {
                    // If the hostile entity is dead or has an invulnerability buff (eg, those
                    // applied in safezones), return to idle
                    if should_stop_attacking(
                        read_data.healths.get(target),
                        read_data.buffs.get(target),
                    ) {
                        if agent.behavior.can(BehaviorCapability::SPEAK) {
                            let msg = "npc.speech.villager_enemy_killed".to_string();
                            event_emitter
                                .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        }
                        agent.target = None;
                    // Choose a new target every 10 seconds, but only for
                    // enemies
                    // TODO: This should be more principled. Consider factoring
                    // health, combat rating, wielded weapon, etc, into the
                    // decision to change target.
                    } else if read_data.time.0 - selected_at > RETARGETING_THRESHOLD_SECONDS
                        && matches!(self.alignment, Some(Alignment::Enemy))
                    {
                        self.choose_target(agent, controller, &read_data, event_emitter);
                    } else {
                        // TODO Add utility for attacking vs leaving target alone
                        let target_data = TargetData {
                            pos: tgt_pos,
                            body: read_data.bodies.get(target),
                            scale: read_data.scales.get(target),
                        };
                        self.attack(agent, controller, &target_data, &read_data);
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

        if self.damage < HEALING_ITEM_THRESHOLD && self.heal_self(agent, controller) {
            agent.action_state.timer = 0.01;
            return;
        }

        agent.action_state.timer = 0.0;
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
                //.filter(|_| bearing.z > 0.1 || self.physics_state.in_liquid().is_some());

                controller.inputs.move_z = bearing.z
                    + if self.traversal_config.can_fly {
                        // NOTE: costs 4 us (imbris)
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
                                #[cfg(feature = "worldgen")]
                                let height_approx = self.pos.0.y
                                    - read_data
                                        .world
                                        .sim()
                                        .get_alt_approx(self.pos.0.xy().map(|x: f32| x as i32))
                                        .unwrap_or(0.0);
                                #[cfg(not(feature = "worldgen"))]
                                let height_approx = self.pos.0.y;

                                height_approx < body.flying_height()
                            })
                            .unwrap_or(false);

                        const NUM_RAYS: usize = 5;

                        // NOTE: costs 15-20 us (imbris)
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
            // NOTE: costs 1 us (imbris)
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
        agent.action_state.timer += read_data.dt.0;

        let msg = agent.inbox.pop_front();
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
                            tracing::trace!(?tradeid, ?balance0, ?balance1, "Accept Pending Trade");
                        }
                    } else {
                        if balance1 > 0.0 {
                            let msg = format!(
                                "That only covers {:.1}% of my costs!",
                                balance0 / balance1 * 100.0
                            );
                            if let Some(tgt_data) = &agent.target {
                                if let Some(with) = read_data.uids.get(tgt_data.target) {
                                    event_emitter.emit(ServerEvent::Chat(
                                        UnresolvedChatMsg::npc_tell(*self.uid, *with, msg),
                                    ));
                                } else {
                                    event_emitter.emit(ServerEvent::Chat(
                                        UnresolvedChatMsg::npc_say(*self.uid, msg),
                                    ));
                                }
                            } else {
                                event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc_say(
                                    *self.uid, msg,
                                )));
                            }
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
            _ => {
                if agent.behavior.can(BehaviorCapability::SPEAK) {
                    // No new events, continue looking towards the last interacting player for some
                    // time
                    if let Some(Target { target, .. }) = &agent.target {
                        self.look_toward(controller, read_data, target);
                    } else {
                        agent.action_state.timer = 0.0;
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
    }

    /// Attempt to consume a healing item, and return whether any healing items
    /// were queued. Callers should use this to implement a delay so that
    /// the healing isn't interrupted.
    fn heal_self(&self, _agent: &mut Agent, controller: &mut Controller) -> bool {
        let healing_value = |item: &Item| {
            let mut value = 0;
            #[allow(clippy::single_match)]
            match item.kind() {
                ItemKind::Consumable { effect, .. } => {
                    for e in effect.iter() {
                        use BuffKind::*;
                        match e {
                            Effect::Health(HealthChange { amount, .. }) => {
                                value += *amount;
                            },
                            Effect::Buff(BuffEffect { kind, data, .. })
                                if matches!(kind, Regeneration | Saturation | Potion) =>
                            {
                                value += (data.strength
                                    * data.duration.map_or(0.0, |d| d.as_secs() as f32))
                                    as i32;
                            }
                            _ => {},
                        }
                    }
                },
                _ => {},
            }
            value
        };

        let mut consumables: Vec<_> = self
            .inventory
            .slots_with_id()
            .filter_map(|(id, slot)| match slot {
                Some(item) if healing_value(item) > 0 => Some((id, item)),
                _ => None,
            })
            .collect();

        consumables.sort_by_key(|(_, item)| healing_value(item));

        if let Some((id, _)) = consumables.last() {
            use comp::inventory::slot::Slot;
            controller
                .actions
                .push(ControlAction::InventoryAction(InventoryAction::Use(
                    Slot::Inventory(*id),
                )));
            true
        } else {
            false
        }
    }

    fn choose_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
        event_emitter: &mut Emitter<'_, ServerEvent>,
    ) {
        agent.action_state.timer = 0.0;

        // Search area
        let target = self.cached_spatial_grid.0
            .in_circle_aabr(self.pos.0.xy(), SEARCH_DIST)
            .filter_map(|entity| {
                read_data.positions
                    .get(entity)
                    .and_then(|l| read_data.healths.get(entity).map(|r| (l, r)))
                    .and_then(|l| read_data.stats.get(entity).map(|r| (l, r)))
                    .and_then(|l| read_data.inventories.get(entity).map(|r| (l, r)))
                    .map(|(((pos, health), stats), inventory)| {
                        (entity, pos, health, stats, inventory, read_data.alignments.get(entity), read_data.char_states.get(entity))
                    })
            })
            .filter(|(e, e_pos, e_health, e_stats, e_inventory, e_alignment, char_state)| {
                let mut search_dist = SEARCH_DIST;
                let mut listen_dist = MAX_LISTEN_DIST;
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
                    && !invulnerability_is_in_buffs(read_data.buffs.get(*e))
                    && (try_owner_alignment(self.alignment, &read_data).and_then(|a| try_owner_alignment(*e_alignment, &read_data).map(|b| a.hostile_towards(*b))).unwrap_or(false) || (
                            if let Some(rtsim_entity) = &self.rtsim_entity {
                                if agent.behavior.can(BehaviorCapability::SPEAK) {
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

        agent.target = target.map(|target| Target {
            target,
            hostile: true,
            selected_at: read_data.time.0,
        });
    }

    fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        let tool_tactic = |tool_kind| match tool_kind {
            ToolKind::Bow => Tactic::Bow,
            ToolKind::Staff => Tactic::Staff,
            ToolKind::Hammer => Tactic::Hammer,
            ToolKind::Sword | ToolKind::Spear => Tactic::Sword,
            ToolKind::Axe => Tactic::Axe,
            _ => Tactic::Melee,
        };

        let tactic = self
            .inventory
            .equipped(EquipSlot::ActiveMainhand)
            .as_ref()
            .map(|item| {
                if let Some(ability_spec) = item.ability_spec() {
                    match ability_spec {
                        AbilitySpec::Custom(spec) => match spec.as_str() {
                            "Axe Simple" | "Sword Simple" => Tactic::Sword,
                            "Staff Simple" => Tactic::Staff,
                            "Bow Simple" => Tactic::Bow,
                            "Stone Golem" => Tactic::StoneGolem,
                            "Quad Med Quick" => Tactic::CircleCharge {
                                radius: 3,
                                circle_time: 2,
                            },
                            "Quad Med Jump" => Tactic::QuadMedJump,
                            "Quad Med Charge" => Tactic::CircleCharge {
                                radius: 12,
                                circle_time: 1,
                            },
                            "Quad Med Basic" => Tactic::QuadMedBasic,
                            "Quad Low Ranged" => Tactic::QuadLowRanged,
                            "Quad Low Breathe" | "Quad Low Beam" => Tactic::QuadLowBeam,
                            "Quad Low Tail" => Tactic::TailSlap,
                            "Quad Low Quick" => Tactic::QuadLowQuick,
                            "Quad Low Basic" => Tactic::QuadLowBasic,
                            "Theropod Basic" | "Theropod Bird" => Tactic::Theropod,
                            "Theropod Charge" => Tactic::CircleCharge {
                                radius: 6,
                                circle_time: 1,
                            },
                            "Turret" => Tactic::Turret,
                            "Haniwa Sentry" => Tactic::RotatingTurret,
                            "Bird Large Breathe" => Tactic::BirdLargeBreathe,
                            "Bird Large Fire" => Tactic::BirdLargeFire,
                            "Mindflayer" => Tactic::Mindflayer,
                            "Minotaur" => Tactic::Minotaur,
                            "Clay Golem" => Tactic::ClayGolem,
                            _ => Tactic::Melee,
                        },
                        AbilitySpec::Tool(tool_kind) => tool_tactic(*tool_kind),
                    }
                } else if let ItemKind::Tool(tool) = &item.kind() {
                    tool_tactic(tool.kind)
                } else {
                    Tactic::Melee
                }
            })
            .unwrap_or(Tactic::Melee);

        // Wield the weapon as running towards the target
        controller.actions.push(ControlAction::Wield);

        let min_attack_dist = (self.body.map_or(0.5, |b| b.radius()) + DEFAULT_ATTACK_RANGE)
            * self.scale
            + tgt_data.body.map_or(0.5, |b| b.radius()) * tgt_data.scale.map_or(1.0, |s| s.0);
        let dist_sqrd = self.pos.0.distance_squared(tgt_data.pos.0);
        let angle = self
            .ori
            .look_vec()
            .angle_between(tgt_data.pos.0 - self.pos.0)
            .to_degrees();

        let eye_offset = self.body.map_or(0.0, |b| b.eye_height());

        let tgt_eye_offset = tgt_data.body.map_or(0.0, |b| b.eye_height()) +
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

        // FIXME: Retrieve actual projectile speed!
        // We have to assume projectiles are faster than base speed because there are
        // skills that increase it, and in most cases this will cause agents to
        // overshoot
        if let Some(dir) = match tactic {
            Tactic::Bow
            | Tactic::FixedTurret
            | Tactic::QuadLowRanged
            | Tactic::QuadMedJump
            | Tactic::RotatingTurret
            | Tactic::Staff
            | Tactic::Turret
                if dist_sqrd > 0.0 =>
            {
                aim_projectile(
                    90.0, // + self.vel.0.magnitude(),
                    Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            }
            Tactic::ClayGolem if matches!(self.char_state, CharacterState::BasicRanged(_)) => {
                const ROCKET_SPEED: f32 = 30.0;
                aim_projectile(
                    ROCKET_SPEED,
                    Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
                    Vec3::new(
                        tgt_data.pos.0.x,
                        tgt_data.pos.0.y,
                        tgt_data.pos.0.z + tgt_eye_offset,
                    ),
                )
            },
            _ => Dir::from_unnormalized(
                Vec3::new(
                    tgt_data.pos.0.x,
                    tgt_data.pos.0.y,
                    tgt_data.pos.0.z + tgt_eye_offset,
                ) - Vec3::new(self.pos.0.x, self.pos.0.y, self.pos.0.z + eye_offset),
            ),
        } {
            controller.inputs.look_dir = dir;
        }

        let attack_data = AttackData {
            min_attack_dist,
            dist_sqrd,
            angle,
        };

        // Match on tactic. Each tactic has different controls
        // depending on the distance from the agent to the target
        match tactic {
            Tactic::Melee => {
                self.handle_melee_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Axe => {
                self.handle_axe_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Hammer => {
                self.handle_hammer_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Sword => {
                self.handle_sword_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Bow => {
                self.handle_bow_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Staff => {
                self.handle_staff_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::StoneGolem => self.handle_stone_golem_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::CircleCharge {
                radius,
                circle_time,
            } => self.handle_circle_charge_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
                radius,
                circle_time,
            ),
            Tactic::QuadLowRanged => self.handle_quadlow_ranged_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::TailSlap => {
                self.handle_tail_slap_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::QuadLowQuick => self.handle_quadlow_quick_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::QuadLowBasic => self.handle_quadlow_basic_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::QuadMedJump => self.handle_quadmed_jump_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::QuadMedBasic => self.handle_quadmed_basic_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::QuadLowBeam => self.handle_quadlow_beam_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::Theropod => {
                self.handle_theropod_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::Turret => {
                self.handle_turret_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::FixedTurret => self.handle_fixed_turret_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::RotatingTurret => self.handle_rotating_turret_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::Mindflayer => self.handle_mindflayer_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::BirdLargeFire => self.handle_birdlarge_fire_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            // Mostly identical to BirdLargeFire but tweaked for flamethrower instead of shockwave
            Tactic::BirdLargeBreathe => self.handle_birdlarge_breathe_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
            Tactic::Minotaur => {
                self.handle_minotaur_attack(agent, controller, &attack_data, &tgt_data, &read_data)
            },
            Tactic::ClayGolem => self.handle_clay_golem_attack(
                agent,
                controller,
                &attack_data,
                &tgt_data,
                &read_data,
            ),
        }
    }

    fn handle_melee_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && attack_data.angle < 45.0 {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
            controller.inputs.move_dir = Vec2::zero();
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);

            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_axe_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && attack_data.angle < 45.0 {
            controller.inputs.move_dir = Vec2::zero();
            if agent.action_state.timer > 6.0 {
                controller
                    .actions
                    .push(ControlAction::CancelInput(InputKind::Secondary));
                agent.action_state.timer = 0.0;
            } else if agent.action_state.timer > 4.0 && self.energy.current() > 10 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else if self.skill_set.has_skill(Skill::Axe(AxeSkill::UnlockLeap))
                && self.energy.current() > 800
                && thread_rng().gen_bool(0.5)
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                agent.action_state.timer += read_data.dt.0;
            } else {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_hammer_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && attack_data.angle < 45.0 {
            controller.inputs.move_dir = Vec2::zero();
            if agent.action_state.timer > 4.0 {
                controller
                    .actions
                    .push(ControlAction::CancelInput(InputKind::Secondary));
                agent.action_state.timer = 0.0;
            } else if agent.action_state.timer > 2.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else if self
                .skill_set
                .has_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                && self.energy.current() > 700
                && thread_rng().gen_bool(0.9)
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                agent.action_state.timer += read_data.dt.0;
            } else {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self.path_toward_target(agent, controller, tgt_data, read_data, true, None)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
                && attack_data.angle < 45.0
            {
                if self
                    .skill_set
                    .has_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                    && agent.action_state.timer > 5.0
                {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Ability(0)));
                    agent.action_state.timer = 0.0;
                } else {
                    agent.action_state.timer += read_data.dt.0;
                }
            }
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_sword_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && attack_data.angle < 45.0 {
            controller.inputs.move_dir = Vec2::zero();
            if self
                .skill_set
                .has_skill(Skill::Sword(SwordSkill::UnlockSpin))
                && agent.action_state.timer < 2.0
                && self.energy.current() > 600
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer > 2.0 {
                agent.action_state.timer = 0.0;
            } else {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self.path_toward_target(agent, controller, tgt_data, read_data, true, None)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
            {
                if agent.action_state.timer > 4.0 && attack_data.angle < 45.0 {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                    agent.action_state.timer = 0.0;
                } else {
                    agent.action_state.timer += read_data.dt.0;
                }
            }
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_bow_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MIN_CHARGE_FRAC: f32 = 0.5;
        const OPTIMAL_TARGET_VELOCITY: f32 = 5.0;
        const DESIRED_ENERGY_LEVEL: u32 = 500;
        // Logic to use abilities
        if let CharacterState::ChargedRanged(c) = self.char_state {
            if !matches!(c.stage_section, StageSection::Recover) {
                // Don't even bother with this logic if in recover
                let target_speed_sqd = agent
                    .target
                    .as_ref()
                    .map(|t| t.target)
                    .and_then(|e| read_data.velocities.get(e))
                    .map_or(0.0, |v| v.0.magnitude_squared());
                if c.charge_frac() < MIN_CHARGE_FRAC
                    || (target_speed_sqd > OPTIMAL_TARGET_VELOCITY.powi(2) && c.charge_frac() < 1.0)
                {
                    // If haven't charged to desired level, or target is moving too fast and haven't
                    // fully charged, keep charging
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                }
                // Else don't send primary input to release the shot
            }
        } else if matches!(self.char_state, CharacterState::RepeaterRanged(c) if self.energy.current() > 50 && !matches!(c.stage_section, StageSection::Recover))
        {
            // If in repeater ranged, have enough energy, and aren't in recovery, try to
            // keep firing
            if attack_data.dist_sqrd > attack_data.min_attack_dist.powi(2)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
            {
                // Only keep firing if not in melee range or if can see target
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
            }
        } else if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            if self
                .skill_set
                .has_skill(Skill::Bow(BowSkill::UnlockShotgun))
                && self.energy.current() > 450
                && thread_rng().gen_bool(0.5)
            {
                // Use shotgun if target close and have sufficient energy
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
            } else if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && self.energy.current() > CharacterAbility::default_roll().get_energy_cost()
            {
                // Else roll away if can roll and have enough energy
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            } else {
                self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
                if attack_data.angle < 15.0 {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                }
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if can_see_tgt(
                &*read_data.terrain,
                self.pos,
                tgt_data.pos,
                attack_data.dist_sqrd,
            ) {
                // If not really far, and can see target, attempt to shoot bow
                if self.energy.current() < DESIRED_ENERGY_LEVEL {
                    // If low on energy, use primary to attempt to regen energy
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Primary));
                } else {
                    // Else we have enough energy, use repeater
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                }
            }
        }
        // Logic to move. Intentionally kept separate from ability logic so duplicated
        // work is less necessary.
        if attack_data.dist_sqrd < (2.0 * attack_data.min_attack_dist).powi(2) {
            // Attempt to move away from target if too close
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    -bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // Else attempt to circle target if neither too close nor too far
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                ) && attack_data.angle < 45.0
                {
                    controller.inputs.move_dir = bearing
                        .xy()
                        .rotated_z(thread_rng().gen_range(0.5..1.57))
                        .try_normalized()
                        .unwrap_or_else(Vec2::zero)
                        * speed;
                } else {
                    // Unless cannot see target, then move towards them
                    controller.inputs.move_dir =
                        bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                    self.jump_if(controller, bearing.z > 1.5);
                    controller.inputs.move_z = bearing.z;
                }
            }
            // Sometimes try to roll
            if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            // If too far, move towards target
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_staff_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
            && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2)
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Roll));
        } else if attack_data.dist_sqrd < (5.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            if agent.action_state.timer < 1.5 {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 3.0 {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(-0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                agent.action_state.timer += read_data.dt.0;
            } else {
                agent.action_state.timer = 0.0;
            }
            if self
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
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                ) && attack_data.angle < 15.0
                {
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
                && attack_data.dist_sqrd < 16.0f32.powi(2)
                && thread_rng().gen::<f32>() < 0.02
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Roll));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_stone_golem_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && attack_data.angle < 90.0 {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
            //controller.inputs.primary.set_state(true);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self.vel.0.is_approx_zero() {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
            }
            if self.path_toward_target(agent, controller, tgt_data, read_data, true, None)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
                && attack_data.angle < 90.0
            {
                if agent.action_state.timer > 5.0 {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Secondary));
                    agent.action_state.timer = 0.0;
                } else {
                    agent.action_state.timer += read_data.dt.0;
                }
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_circle_charge_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
        radius: u32,
        circle_time: u32,
    ) {
        if attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) && thread_rng().gen_bool(0.5)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else if attack_data.dist_sqrd < (radius as f32 * attack_data.min_attack_dist).powi(2) {
            controller.inputs.move_dir = (self.pos.0 - tgt_data.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y);
        } else if attack_data.dist_sqrd
            < ((radius as f32 + 1.0) * attack_data.min_attack_dist).powi(2)
            && attack_data.dist_sqrd > (radius as f32 * attack_data.min_attack_dist).powi(2)
        {
            if agent.action_state.timer < circle_time as f32 {
                let move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                let obstacle_left = read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0 + move_dir.with_z(0.0) * 2.0 + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_some());
                if obstacle_left {
                    agent.action_state.timer = circle_time as f32;
                }
                controller.inputs.move_dir = move_dir;
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < circle_time as f32 + 0.5 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 2.0 * circle_time as f32 + 0.5 {
                let move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(-0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                let obstacle_right = read_data
                    .terrain
                    .ray(
                        self.pos.0 + Vec3::unit_z(),
                        self.pos.0 + move_dir.with_z(0.0) * 2.0 + Vec3::unit_z(),
                    )
                    .until(Block::is_solid)
                    .cast()
                    .1
                    .map_or(true, |b| b.is_some());
                if obstacle_right {
                    agent.action_state.timer = 2.0 * circle_time as f32 + 0.5;
                }
                controller.inputs.move_dir = move_dir;
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 2.0 * circle_time as f32 + 1.0 {
                if agent.action_state.timer < 2.0 * circle_time as f32 {
                    agent.action_state.timer = 2.0 * circle_time as f32;
                }
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else {
                agent.action_state.timer = 0.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadlow_ranged_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 90.0
        {
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y);
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                if attack_data.angle < 15.0
                    && can_see_tgt(
                        &*read_data.terrain,
                        self.pos,
                        tgt_data.pos,
                        attack_data.dist_sqrd,
                    )
                {
                    if agent.action_state.timer > 5.0 {
                        agent.action_state.timer = 0.0;
                    } else if agent.action_state.timer > 2.5 {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(1.75 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.action_state.timer += read_data.dt.0;
                    } else {
                        controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.25 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::zero)
                            * speed;
                        agent.action_state.timer += read_data.dt.0;
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
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_tail_slap_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            if agent.action_state.timer > 4.0 {
                controller
                    .actions
                    .push(ControlAction::CancelInput(InputKind::Primary));
                agent.action_state.timer = 0.0;
            } else if agent.action_state.timer > 1.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            } else {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            }
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y)
                * 0.1;
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadlow_quick_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Secondary));
        } else if attack_data.dist_sqrd < (3.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.dist_sqrd > (2.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 90.0
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
            controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                .xy()
                .rotated_z(-0.47 * PI)
                .try_normalized()
                .unwrap_or_else(Vec2::unit_y);
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadlow_basic_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 70.0
            && attack_data.dist_sqrd < (1.3 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            if agent.action_state.timer > 5.0 {
                agent.action_state.timer = 0.0;
            } else if agent.action_state.timer > 2.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadmed_jump_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (1.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Secondary));
        } else if attack_data.angle < 15.0
            && attack_data.dist_sqrd < (5.0 * attack_data.min_attack_dist).powi(2)
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(0)));
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            if self.path_toward_target(agent, controller, tgt_data, read_data, true, None)
                && attack_data.angle < 15.0
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
            }
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadmed_basic_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0 && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) {
            controller.inputs.move_dir = Vec2::zero();
            if agent.action_state.timer < 2.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 3.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            } else {
                agent.action_state.timer = 0.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_quadlow_beam_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0
            && attack_data.dist_sqrd < (2.5 * attack_data.min_attack_dist).powi(2)
        {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Secondary));
        } else if attack_data.dist_sqrd < (7.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            if agent.action_state.timer < 2.0 {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 4.0 && attack_data.angle < 15.0 {
                controller.inputs.move_dir = (tgt_data.pos.0 - self.pos.0)
                    .xy()
                    .rotated_z(-0.47 * PI)
                    .try_normalized()
                    .unwrap_or_else(Vec2::unit_y);
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.timer += read_data.dt.0;
            } else if agent.action_state.timer < 6.0 && attack_data.angle < 15.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
                agent.action_state.timer += read_data.dt.0;
            } else {
                agent.action_state.timer = 0.0;
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_theropod_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.angle < 90.0 && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2) {
            controller.inputs.move_dir = Vec2::zero();
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if can_see_tgt(
            &*read_data.terrain,
            self.pos,
            tgt_data.pos,
            attack_data.dist_sqrd,
        ) && attack_data.angle < 15.0
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else {
            agent.target = None;
        }
    }

    fn handle_fixed_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        controller.inputs.look_dir = self.ori.look_dir();
        if can_see_tgt(
            &*read_data.terrain,
            self.pos,
            tgt_data.pos,
            attack_data.dist_sqrd,
        ) && attack_data.angle < 15.0
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else {
            agent.target = None;
        }
    }

    fn handle_rotating_turret_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        controller.inputs.look_dir = Dir::new(
            Quaternion::from_xyzw(self.ori.look_dir().x, self.ori.look_dir().y, 0.0, 0.0)
                .rotated_z(6.0 * read_data.dt.0 as f32)
                .into_vec3()
                .try_normalized()
                .unwrap_or_default(),
        );
        if can_see_tgt(
            &*read_data.terrain,
            self.pos,
            tgt_data.pos,
            attack_data.dist_sqrd,
        ) {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else {
            agent.target = None;
        }
    }

    fn handle_mindflayer_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MINDFLAYER_ATTACK_DIST: f32 = 16.0;
        const MINION_SUMMON_THRESHOLD: f32 = 0.20;
        let health_fraction = self.health.map_or(0.5, |h| h.fraction());
        // Sets counter at start of combat, using `condition` to keep track of whether
        // it was already intitialized
        if !agent.action_state.condition {
            agent.action_state.counter = 1.0 - MINION_SUMMON_THRESHOLD;
            agent.action_state.condition = true;
        }

        if agent.action_state.counter > health_fraction {
            // Summon minions at particular thresholds of health
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(2)));

            if matches!(self.char_state, CharacterState::BasicSummon(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.action_state.counter -= MINION_SUMMON_THRESHOLD;
            }
        } else if attack_data.dist_sqrd < MINDFLAYER_ATTACK_DIST.powi(2) {
            if can_see_tgt(
                &*read_data.terrain,
                self.pos,
                tgt_data.pos,
                attack_data.dist_sqrd,
            ) {
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
            } else {
                self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
            }
        } else if attack_data.dist_sqrd < MAX_PATH_DIST.powi(2) {
            // If too far from target, throw a random number of necrotic spheres at them and
            // then blink to them.
            let num_fireballs = &mut agent.action_state.int_counter;
            if *num_fireballs == 0 {
                controller.actions.push(ControlAction::StartInput {
                    input: InputKind::Ability(0),
                    target_entity: agent
                        .target
                        .as_ref()
                        .and_then(|t| read_data.uids.get(t.target))
                        .copied(),
                    select_pos: None,
                });
                if matches!(self.char_state, CharacterState::Blink(_)) {
                    *num_fireballs = rand::random::<u8>() % 4;
                }
            } else if matches!(self.char_state, CharacterState::Wielding) {
                *num_fireballs -= 1;
                controller.actions.push(ControlAction::StartInput {
                    input: InputKind::Ability(1),
                    target_entity: agent
                        .target
                        .as_ref()
                        .and_then(|t| read_data.uids.get(t.target))
                        .copied(),
                    select_pos: None,
                });
            }
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else {
            self.path_toward_target(agent, controller, tgt_data, read_data, false, None);
        }
    }

    fn handle_birdlarge_fire_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        if attack_data.dist_sqrd > 30.0_f32.powi(2) {
            // If random chance and can see target
            if thread_rng().gen_bool(0.05)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
                && attack_data.angle < 15.0
            {
                // Fireball
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
            }
            // If some target
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                // Walk to target
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                // If less than 20 blocks higher than target
                if (self.pos.0.z - tgt_data.pos.0.z) < 20.0 {
                    // Fly upward
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Fly));
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Jump));
                    controller.inputs.move_z = 1.0;
                } else {
                    // Jump
                    self.jump_if(controller, bearing.z > 1.5);
                    controller.inputs.move_z = bearing.z;
                }
            }
        }
        // If higher than 2 blocks
        else if !read_data
            .terrain
            .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 2.0))
            .until(Block::is_solid)
            .cast()
            .1
            .map_or(true, |b| b.is_some())
        {
            // Do not increment the timer during this movement
            // The next stage shouldn't trigger until the entity
            // is on the ground
            // Fly to target
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Fly));
            let move_dir = tgt_data.pos.0 - self.pos.0;
            controller.inputs.move_dir =
                move_dir.xy().try_normalized().unwrap_or_else(Vec2::zero) * 2.0;
            controller.inputs.move_z = move_dir.z - 0.5;
            // If further than 4 blocks and random chance
            if thread_rng().gen_bool(0.05)
                && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
                && attack_data.angle < 15.0
            {
                // Fireball
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
            }
        }
        // If further than 4 blocks and random chance
        else if thread_rng().gen_bool(0.05)
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            // Fireball
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        }
        // If random chance and less than 20 blocks higher than target and further than 4
        // blocks
        else if thread_rng().gen_bool(0.5)
            && (self.pos.0.z - tgt_data.pos.0.z) < 15.0
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Fly));
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Jump));
            controller.inputs.move_z = 1.0;
        }
        // If further than 2.5 blocks and random chance
        else if attack_data.dist_sqrd > (2.5 * attack_data.min_attack_dist).powi(2) {
            // Walk to target
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        }
        // If energy higher than 600 and random chance
        else if self.energy.current() > 600 && thread_rng().gen_bool(0.4) {
            // Shockwave
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(0)));
        } else if attack_data.angle < 90.0 {
            // Triple strike
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Secondary));
        } else {
            // Target is behind us. Turn around and chase target
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        }
    }

    fn handle_birdlarge_breathe_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        // Set fly to false
        controller
            .actions
            .push(ControlAction::CancelInput(InputKind::Fly));
        if attack_data.dist_sqrd > 30.0_f32.powi(2) {
            if thread_rng().gen_bool(0.05)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
                && attack_data.angle < 15.0
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
            }
            if let Some((bearing, speed)) = agent.chaser.chase(
                &*read_data.terrain,
                self.pos.0,
                self.vel.0,
                tgt_data.pos.0,
                TraversalConfig {
                    min_tgt_dist: 1.25,
                    ..self.traversal_config
                },
            ) {
                controller.inputs.move_dir =
                    bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                if (self.pos.0.z - tgt_data.pos.0.z) < 20.0 {
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Fly));
                    controller
                        .actions
                        .push(ControlAction::basic_input(InputKind::Jump));
                    controller.inputs.move_z = 1.0;
                } else {
                    self.jump_if(controller, bearing.z > 1.5);
                    controller.inputs.move_z = bearing.z;
                }
            }
        } else if !read_data
            .terrain
            .ray(self.pos.0, self.pos.0 - (Vec3::unit_z() * 2.0))
            .until(Block::is_solid)
            .cast()
            .1
            .map_or(true, |b| b.is_some())
        {
            // Do not increment the timer during this movement
            // The next stage shouldn't trigger until the entity
            // is on the ground
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Fly));
            let move_dir = tgt_data.pos.0 - self.pos.0;
            controller.inputs.move_dir =
                move_dir.xy().try_normalized().unwrap_or_else(Vec2::zero) * 2.0;
            controller.inputs.move_z = move_dir.z - 0.5;
            if thread_rng().gen_bool(0.05)
                && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
                && attack_data.angle < 15.0
            {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
            }
        } else if thread_rng().gen_bool(0.05)
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
            && attack_data.angle < 15.0
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else if thread_rng().gen_bool(0.5)
            && (self.pos.0.z - tgt_data.pos.0.z) < 15.0
            && attack_data.dist_sqrd > (4.0 * attack_data.min_attack_dist).powi(2)
        {
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Fly));
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Jump));
            controller.inputs.move_z = 1.0;
        } else if attack_data.dist_sqrd > (3.0 * attack_data.min_attack_dist).powi(2) {
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        } else if self.energy.current() > 600
            && agent.action_state.timer < 3.0
            && attack_data.angle < 15.0
        {
            // Fire breath attack
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(0)));
            // Move towards the target slowly
            self.path_toward_target(agent, controller, tgt_data, read_data, true, Some(0.5));
            agent.action_state.timer += read_data.dt.0;
        } else if agent.action_state.timer < 6.0
            && attack_data.angle < 90.0
            && attack_data.dist_sqrd < attack_data.min_attack_dist.powi(2)
        {
            // Triplestrike
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Secondary));
            agent.action_state.timer += read_data.dt.0;
        } else {
            // Reset timer
            agent.action_state.timer = 0.0;
            // Target is behind us or the timer needs to be reset. Chase target
            self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
        }
    }

    fn handle_minotaur_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const MINOTAUR_FRENZY_THRESHOLD: f32 = 0.5;
        const MINOTAUR_ATTACK_RANGE: f32 = 5.0;
        const MINOTAUR_CHARGE_DISTANCE: f32 = 15.0;
        let minotaur_attack_distance =
            self.body.map_or(0.0, |b| b.radius()) + MINOTAUR_ATTACK_RANGE;
        let health_fraction = self.health.map_or(1.0, |h| h.fraction());
        // Sets action counter at start of combat
        if agent.action_state.counter < MINOTAUR_FRENZY_THRESHOLD
            && health_fraction > MINOTAUR_FRENZY_THRESHOLD
        {
            agent.action_state.counter = MINOTAUR_FRENZY_THRESHOLD;
        }
        if health_fraction < agent.action_state.counter {
            // Makes minotaur buff itself with frenzy
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(1)));
            if matches!(self.char_state, CharacterState::SelfBuff(c) if matches!(c.stage_section, StageSection::Recover))
            {
                agent.action_state.counter = 0.0;
            }
        } else if matches!(self.char_state, CharacterState::DashMelee(c) if !matches!(c.stage_section, StageSection::Recover))
        {
            // If already charging, keep charging if not in recover
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Ability(0)));
        } else if matches!(self.char_state, CharacterState::ChargedMelee(c) if matches!(c.stage_section, StageSection::Charge) && c.timer < c.static_data.charge_duration)
        {
            // If already charging a melee attack, keep charging it if charging
            controller
                .actions
                .push(ControlAction::basic_input(InputKind::Primary));
        } else if attack_data.dist_sqrd > MINOTAUR_CHARGE_DISTANCE.powi(2) {
            // Charges at target if they are far enough away
            if attack_data.angle < 60.0 {
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
            }
        } else if attack_data.dist_sqrd < minotaur_attack_distance.powi(2) {
            if agent.action_state.condition && !self.char_state.is_attack() {
                // Cripple target if not just used cripple
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
                agent.action_state.condition = false;
            } else if !self.char_state.is_attack() {
                // Cleave target if not just used cleave
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.condition = true;
            }
        }
        // Make minotaur move towards target
        self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
    }

    fn handle_clay_golem_attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        attack_data: &AttackData,
        tgt_data: &TargetData,
        read_data: &ReadData,
    ) {
        const GOLEM_MELEE_RANGE: f32 = 4.0;
        const GOLEM_LASER_RANGE: f32 = 30.0;
        const GOLEM_LONG_RANGE: f32 = 50.0;
        const GOLEM_TARGET_SPEED: f32 = 8.0;
        let golem_melee_range = self.body.map_or(0.0, |b| b.radius()) + GOLEM_MELEE_RANGE;
        // Magnitude squared of cross product of target velocity with golem orientation
        let target_speed_cross_sqd = agent
            .target
            .as_ref()
            .map(|t| t.target)
            .and_then(|e| read_data.velocities.get(e))
            .map_or(0.0, |v| v.0.cross(self.ori.look_vec()).magnitude_squared());
        if attack_data.dist_sqrd < golem_melee_range.powi(2) {
            if agent.action_state.counter < 7.5 {
                // If target is close, whack them
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Primary));
                agent.action_state.counter += read_data.dt.0;
            } else {
                // If whacked for too long, nuke them
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(1)));
                if matches!(self.char_state, CharacterState::BasicRanged(c) if matches!(c.stage_section, StageSection::Recover))
                {
                    agent.action_state.counter = 0.0;
                }
            }
        } else if attack_data.dist_sqrd < GOLEM_LASER_RANGE.powi(2) {
            if matches!(self.char_state, CharacterState::BasicBeam(c) if c.timer < Duration::from_secs(10))
                || target_speed_cross_sqd < GOLEM_TARGET_SPEED.powi(2)
                    && can_see_tgt(
                        &*read_data.terrain,
                        self.pos,
                        tgt_data.pos,
                        attack_data.dist_sqrd,
                    )
                    && attack_data.angle < 45.0
            {
                // If target in range threshold and haven't been lasering for more than 10
                // seconds already or if target is moving slow-ish, laser them
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Secondary));
            } else {
                // Else target moving too fast for laser, shockwave time
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
            }
        } else if attack_data.dist_sqrd < GOLEM_LONG_RANGE.powi(2) {
            if target_speed_cross_sqd < GOLEM_TARGET_SPEED.powi(2)
                && can_see_tgt(
                    &*read_data.terrain,
                    self.pos,
                    tgt_data.pos,
                    attack_data.dist_sqrd,
                )
            {
                // If target is far-ish and moving slow-ish, rocket them
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(1)));
            } else {
                // Else target moving too fast for laser, shockwave time
                controller
                    .actions
                    .push(ControlAction::basic_input(InputKind::Ability(0)));
            }
        }
        // Make clay golem move towards target
        self.path_toward_target(agent, controller, tgt_data, read_data, true, None);
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

    fn handle_elevated_awareness(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        read_data: &ReadData,
    ) {
        // Currently this means that we are in a safezone
        if invulnerability_is_in_buffs(read_data.buffs.get(*self.entity)) {
            self.idle(agent, controller, &read_data);
            return;
        }

        let is_enemy = matches!(self.alignment, Some(Alignment::Enemy));

        if let Some(sound) = agent.sounds_heard.last() {
            let sound_pos = Pos(sound.pos);
            let dist_sqrd = self.pos.0.distance_squared(sound_pos.0);

            if is_enemy {
                let far_enough = dist_sqrd > 10.0_f32.powi(2);

                if far_enough {
                    self.follow(agent, controller, &read_data.terrain, &sound_pos);
                } else {
                    // TODO: Change this to a search action instead of idle
                    self.idle(agent, controller, &read_data);
                }
            } else if self.flees {
                let aggro = agent.psyche.aggro;
                let close_enough = dist_sqrd < 35.0_f32.powi(2);
                let loud_sound = sound.vol >= 10.0;

                if close_enough && (aggro <= 0.5 || (aggro <= 0.7 && loud_sound)) {
                    self.flee(agent, controller, &read_data.terrain, &sound_pos);
                } else {
                    self.idle(agent, controller, &read_data);
                }
            } else {
                self.idle(agent, controller, &read_data);
            }
        }
    }

    /// Directs the entity to path and move toward the target
    /// If full_path is false, the entity will path to a location 50 units along
    /// the vector between the entity and the target. The speed multiplier
    /// multiplies the movement speed by a value less than 1.0.
    /// A `None` value implies a multiplier of 1.0.
    /// Returns `false` if the pathfinding algorithm fails to return a path
    fn path_toward_target(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        tgt_data: &TargetData,
        read_data: &ReadData,
        full_path: bool,
        speed_multiplier: Option<f32>,
    ) -> bool {
        let pathing_pos = if full_path {
            tgt_data.pos.0
        } else {
            self.pos.0
                + PARTIAL_PATH_DIST
                    * (tgt_data.pos.0 - self.pos.0)
                        .try_normalized()
                        .unwrap_or_else(Vec3::zero)
        };
        let speed_multiplier = speed_multiplier.unwrap_or(1.0).min(1.0);
        if let Some((bearing, speed)) = agent.chaser.chase(
            &*read_data.terrain,
            self.pos.0,
            self.vel.0,
            pathing_pos,
            TraversalConfig {
                min_tgt_dist: 1.25,
                ..self.traversal_config
            },
        ) {
            controller.inputs.move_dir =
                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed * speed_multiplier;
            self.jump_if(controller, bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
            true
        } else {
            false
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
    health.map_or(true, |a| a.is_dead) || invulnerability_is_in_buffs(buffs)
}

// FIXME: The logic that is used in this function and throughout the code
// shouldn't be used to mean that a character is in a safezone.
fn invulnerability_is_in_buffs(buffs: Option<&Buffs>) -> bool {
    buffs.map_or(false, |b| b.kinds.contains_key(&BuffKind::Invulnerability))
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

/// Projectile motion: Returns the direction to aim for the projectile to reach
/// target position. Does not take any forces but gravity into account.
fn aim_projectile(speed: f32, pos: Vec3<f32>, tgt: Vec3<f32>) -> Option<Dir> {
    let mut to_tgt = tgt - pos;
    let dist_sqrd = to_tgt.xy().magnitude_squared();
    let u_sqrd = speed.powi(2);
    to_tgt.z = (u_sqrd
        - (u_sqrd.powi(2) - GRAVITY * (GRAVITY * dist_sqrd + 2.0 * to_tgt.z * u_sqrd))
            .sqrt()
            .max(0.0))
        / GRAVITY;

    Dir::from_unnormalized(to_tgt)
}

fn forget_old_sounds(agent: &mut Agent, read_data: &ReadData) {
    if !agent.sounds_heard.is_empty() {
        // Keep (retain) only newer sounds
        agent
            .sounds_heard
            .retain(|&sound| read_data.time.0 - sound.time <= SECONDS_BEFORE_FORGET_SOUNDS);
    }
}

fn decrement_awareness(agent: &mut Agent) {
    let mut decrement = AWARENESS_DECREMENT_CONSTANT;
    let awareness = agent.awareness;

    let too_high = awareness >= 100.0;
    let high = awareness >= 50.0;
    let medium = awareness >= 30.0;
    let low = awareness > 15.0;
    let positive = awareness >= 0.0;
    let negative = awareness < 0.0;

    if too_high {
        decrement *= 3.0;
    } else if high {
        decrement *= 1.0;
    } else if medium {
        decrement *= 2.5;
    } else if low {
        decrement *= 0.70;
    } else if positive {
        decrement *= 0.5;
    } else if negative {
        return;
    }

    agent.awareness -= decrement;
}
