use common::{
    comp::{
        self,
        agent::{AgentEvent, Tactic, Target, DEFAULT_INTERACTION_TIME, TRADE_INTERACTION_TIME},
        group,
        inventory::{slot::EquipSlot, trade_pricing::TradePricing},
        invite::InviteResponse,
        item::{
            tool::{ToolKind, UniqueKind},
            ItemKind,
        },
        skills::{AxeSkill, BowSkill, HammerSkill, Skill, StaffSkill, SwordSkill},
        Agent, Alignment, Body, CharacterState, ControlAction, ControlEvent, Controller, Energy,
        Health, InputKind, Inventory, LightEmitter, MountState, Ori, PhysicsState, Pos, Scale,
        Stats, UnresolvedChatMsg, Vel,
    },
    event::{Emitter, EventBus, ServerEvent},
    path::TraversalConfig,
    resources::{DeltaTime, TimeOfDay},
    terrain::{Block, TerrainGrid},
    time::DayPeriod,
    trade::{Good, TradeAction, TradePhase, TradeResult},
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
    Write, WriteStorage,
};
use std::f32::consts::PI;
use vek::*;

struct AgentData<'a> {
    entity: &'a EcsEntity,
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
}

#[derive(SystemData)]
pub struct ReadData<'a> {
    entities: Entities<'a>,
    uid_allocator: Read<'a, UidAllocator>,
    dt: Read<'a, DeltaTime>,
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
    );

    const NAME: &'static str = "agent";
    const ORIGIN: Origin = Origin::Server;
    const PHASE: Phase = Phase::Create;

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    fn run(
        job: &mut Job<Self>,
        (read_data, event_bus, mut agents, mut controllers): Self::SystemData,
    ) {
        job.cpu_stats.measure(ParMode::Rayon);
        (
            &read_data.entities,
            (&read_data.energies, &read_data.healths),
            &read_data.positions,
            &read_data.velocities,
            &read_data.orientations,
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
        )
            .par_join()
            .filter(|(_, _, _, _, _, _, _, _, _, _, _, _, _, _, mount_state)| {
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
                    pos,
                    vel,
                    ori,
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
                        can_fly: body.map(|b| b.can_fly()).unwrap_or(false),
                    };

                    let flees = alignment
                        .map(|a| !matches!(a, Alignment::Enemy | Alignment::Owned(_)))
                        .unwrap_or(true);

                    let damage = health.current() as f32 / health.maximum() as f32;

                    // Package all this agent's data into a convenient struct
                    let data = AgentData {
                        entity: &entity,
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
                    };

                    ///////////////////////////////////////////////////////////
                    // Behavior tree
                    ///////////////////////////////////////////////////////////

                    // If falling fast and can glide, save yourself!
                    if data.fall_glide() {
                        //toggle glider when vertical velocity is above some threshold (here ~
                        // glider fall vertical speed)
                        if vel.0.z < -26.0 {
                            controller.actions.push(ControlAction::GlideWield);
                            if let Some(Target { target, hostile: _ }) = agent.target {
                                if let Some(tgt_pos) = read_data.positions.get(target) {
                                    controller.inputs.move_dir = (pos.0 - tgt_pos.0)
                                        .xy()
                                        .try_normalized()
                                        .unwrap_or_else(Vec2::zero);
                                }
                            }
                        }
                    } else if let Some(Target { target, hostile }) = agent.target {
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
                                                    });
                                                    if let (Some(tgt_pos), Some(tgt_health)) = (
                                                        read_data.positions.get(attacker),
                                                        read_data.healths.get(attacker),
                                                    ) {
                                                        if tgt_health.is_dead
                                                            || dist_sqrd > MAX_CHASE_DIST.powi(2)
                                                        {
                                                            agent.target = Some(Target {
                                                                target,
                                                                hostile: false,
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
                        // Target an entity that's attacking us if the attack was recent
                        if health.last_change.0 < DAMAGE_MEMORY_DURATION {
                            if let comp::HealthSource::Damage { by: Some(by), .. } =
                                health.last_change.1.cause
                            {
                                if let Some(attacker) =
                                    read_data.uid_allocator.retrieve_entity_internal(by.id())
                                {
                                    if let Some(tgt_pos) = read_data.positions.get(attacker) {
                                        // If the target is dead, remove the target and idle.
                                        if read_data
                                            .healths
                                            .get(attacker)
                                            .map_or(true, |a| a.is_dead)
                                        {
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
                                            });
                                            data.attack(
                                                agent,
                                                controller,
                                                &read_data.terrain,
                                                tgt_pos,
                                                read_data.bodies.get(attacker),
                                                &read_data.dt,
                                            );
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
                                data.idle_tree(agent, controller, &read_data, &mut event_emitter);
                            }
                        } else {
                            data.idle_tree(agent, controller, &read_data, &mut event_emitter);
                        }
                    }

                    debug_assert!(controller.inputs.move_dir.map(|e| !e.is_nan()).reduce_and());
                    debug_assert!(controller.inputs.look_dir.map(|e| !e.is_nan()).reduce_and());
                },
            );
    }
}

impl<'a> AgentData<'a> {
    fn fall_glide(&self) -> bool { self.glider_equipped && !self.physics_state.on_ground }

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
                < (if agent.trading {
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
            self.choose_target(agent, controller, &read_data);
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
        if let Some(Target { target, .. }) = agent.target {
            if let (Some(tgt_pos), Some(tgt_health)) = (
                read_data.positions.get(target),
                read_data.healths.get(target),
            ) {
                let dist_sqrd = self.pos.0.distance_squared(tgt_pos.0);
                // Should the agent flee?
                if 1.0 - agent.psyche.aggro > self.damage && self.flees {
                    if agent.action_timer == 0.0 && agent.can_speak {
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
                    // If the hostile entity is dead, return to idle
                    if tgt_health.is_dead {
                        agent.target = None;
                        if agent.can_speak {
                            let msg = "I have destroyed my enemy!".to_string();
                            event_emitter
                                .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        }
                    } else if dist_sqrd < SIGHT_DIST.powi(2) {
                        self.attack(
                            agent,
                            controller,
                            &read_data.terrain,
                            tgt_pos,
                            read_data.bodies.get(target),
                            &read_data.dt,
                        );
                    } else {
                        agent.target = None;
                        self.idle(agent, controller, &read_data);
                    }
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
            controller.inputs.fly.set_state(
                self.traversal_config.can_fly
                    && !read_data
                        .terrain
                        .ray(self.pos.0, self.pos.0 + (Vec3::unit_z() * 3.0))
                        .until(Block::is_solid)
                        .cast()
                        .1
                        .map_or(true, |b| b.is_some()),
            );
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
                controller.inputs.jump.set_state(
                    bearing.z > 1.5
                        || self.traversal_config.can_fly && self.traversal_config.on_ground,
                );
                controller.inputs.climb = Some(comp::Climb::Up);
                //.filter(|_| bearing.z > 0.1 || self.physics_state.in_liquid.is_some());

                controller.inputs.move_z = bearing.z
                    + if self.traversal_config.can_fly {
                        if read_data
                            .terrain
                            .ray(
                                self.pos.0 + Vec3::unit_z(),
                                self.pos.0
                                    + bearing.try_normalized().unwrap_or_else(Vec3::unit_y) * 60.0
                                    + Vec3::unit_z(),
                            )
                            .until(Block::is_solid)
                            .cast()
                            .1
                            .map_or(true, |b| b.is_some())
                        {
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
            Some(AgentEvent::Talk(by)) => {
                if agent.can_speak {
                    if let Some(target) = read_data.uid_allocator.retrieve_entity_internal(by.id())
                    {
                        agent.target = Some(Target {
                            target,
                            hostile: false,
                        });
                        if let Some(tgt_pos) = read_data.positions.get(target) {
                            let eye_offset = self.body.map_or(0.0, |b| b.eye_height());
                            let tgt_eye_offset =
                                read_data.bodies.get(target).map_or(0.0, |b| b.eye_height());
                            if let Some(dir) = Dir::from_unnormalized(
                                Vec3::new(tgt_pos.0.x, tgt_pos.0.y, tgt_pos.0.z + tgt_eye_offset)
                                    - Vec3::new(
                                        self.pos.0.x,
                                        self.pos.0.y,
                                        self.pos.0.z + eye_offset,
                                    ),
                            ) {
                                controller.inputs.look_dir = dir;
                            }
                            controller.actions.push(ControlAction::Stand);
                            controller.actions.push(ControlAction::Talk);
                            if let Some((_travel_to, destination_name)) =
                                &agent.rtsim_controller.travel_to
                            {
                                let msg = format!(
                                    "I'm heading to {}! Want to come along?",
                                    destination_name
                                );
                                event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                                    *self.uid, msg,
                                )));
                            } else {
                                let msg = "npc.speech.villager".to_string();
                                event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                                    *self.uid, msg,
                                )));
                            }
                        }
                    }
                }
            },
            Some(AgentEvent::TradeInvite(_with)) => {
                if agent.trade_for_site.is_some() && !agent.trading {
                    // stand still and looking towards the trading player
                    controller.actions.push(ControlAction::Stand);
                    controller.actions.push(ControlAction::Talk);
                    controller
                        .events
                        .push(ControlEvent::InviteResponse(InviteResponse::Accept));
                    agent.trading = true;
                } else {
                    // TODO: Provide a hint where to find the closest merchant?
                    controller
                        .events
                        .push(ControlEvent::InviteResponse(InviteResponse::Decline));
                }
            },
            Some(AgentEvent::FinishedTrade(result)) => {
                if agent.trading {
                    match result {
                        TradeResult::Completed => {
                            event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                                *self.uid,
                                "Thank you for trading with me!".to_string(),
                            )))
                        },
                        _ => event_emitter.emit(ServerEvent::Chat(UnresolvedChatMsg::npc(
                            *self.uid,
                            "Maybe another time, have a good day!".to_string(),
                        ))),
                    }
                    agent.trading = false;
                }
            },
            Some(AgentEvent::UpdatePendingTrade(boxval)) => {
                let (tradeid, pending, prices, inventories) = *boxval;
                if agent.trading {
                    // I assume player is [0], agent is [1]
                    fn trade_margin(g: Good) -> f32 {
                        match g {
                            Good::Tools | Good::Armor => 0.5,
                            Good::Food | Good::Potions | Good::Ingredients => 0.75,
                            Good::Coin => 1.0,
                            _ => 0.0, // what is this?
                        }
                    }
                    let balance0: f32 = pending.offers[0]
                        .iter()
                        .map(|(slot, amount)| {
                            inventories[0]
                                .as_ref()
                                .map(|ri| {
                                    ri.inventory.get(slot).map(|item| {
                                        let (material, factor) =
                                            TradePricing::get_material(&item.name);
                                        prices.values.get(&material).cloned().unwrap_or_default()
                                            * factor
                                            * (*amount as f32)
                                            * trade_margin(material)
                                    })
                                })
                                .flatten()
                                .unwrap_or_default()
                        })
                        .sum();
                    let balance1: f32 = pending.offers[1]
                        .iter()
                        .map(|(slot, amount)| {
                            inventories[1]
                                .as_ref()
                                .map(|ri| {
                                    ri.inventory.get(slot).map(|item| {
                                        let (material, factor) =
                                            TradePricing::get_material(&item.name);
                                        prices.values.get(&material).cloned().unwrap_or_default()
                                            * factor
                                            * (*amount as f32)
                                    })
                                })
                                .flatten()
                                .unwrap_or_default()
                        })
                        .sum();
                    tracing::debug!("UpdatePendingTrade({}, {})", balance0, balance1);
                    if balance0 >= balance1 {
                        event_emitter.emit(ServerEvent::ProcessTradeAction(
                            *self.entity,
                            tradeid,
                            TradeAction::Accept(pending.phase),
                        ));
                    } else {
                        if balance1 > 0.0 {
                            let msg = format!(
                                "That only covers {:.1}% of my costs!",
                                balance0 / balance1 * 100.0
                            );
                            event_emitter
                                .emit(ServerEvent::Chat(UnresolvedChatMsg::npc(*self.uid, msg)));
                        }
                        if pending.phase != TradePhase::Mutate {
                            // we got into the review phase but without balanced goods, decline
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
                if agent.can_speak {
                    // no new events, continue looking towards the last interacting player for some
                    // time
                    if let Some(Target { target, .. }) = &agent.target {
                        if let Some(tgt_pos) = read_data.positions.get(*target) {
                            let eye_offset = self.body.map_or(0.0, |b| b.eye_height());
                            let tgt_eye_offset = read_data
                                .bodies
                                .get(*target)
                                .map_or(0.0, |b| b.eye_height());
                            if let Some(dir) = Dir::from_unnormalized(
                                Vec3::new(tgt_pos.0.x, tgt_pos.0.y, tgt_pos.0.z + tgt_eye_offset)
                                    - Vec3::new(
                                        self.pos.0.x,
                                        self.pos.0.y,
                                        self.pos.0.z + eye_offset,
                                    ),
                            ) {
                                controller.inputs.look_dir = dir;
                            }
                        }
                    } else {
                        agent.action_timer = 0.0;
                    }
                }
            },
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
            controller.inputs.jump.set_state(bearing.z > 1.5);
            controller.inputs.move_z = bearing.z;
        }
        agent.action_timer += dt.0;
    }

    fn choose_target(&self, agent: &mut Agent, controller: &mut Controller, read_data: &ReadData) {
        agent.action_timer = 0.0;

        // Search for new targets (this looks expensive, but it's only run occasionally)
        // TODO: Replace this with a better system that doesn't consider *all* entities
        let target = (&read_data.entities, &read_data.positions, &read_data.healths, read_data.alignments.maybe(), read_data.char_states.maybe())
            .join()
            .filter(|(e, e_pos, e_health, e_alignment, char_state)| {
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
                    && self.alignment.and_then(|a| e_alignment.map(|b| a.hostile_towards(*b))).unwrap_or(false)
            })
            // Can we even see them?
            .filter(|(_, e_pos, _, _, _)| read_data.terrain
                .ray(self.pos.0 + Vec3::unit_z(), e_pos.0 + Vec3::unit_z())
                .until(Block::is_opaque)
                .cast()
                .0 >= e_pos.0.distance(self.pos.0))
            .min_by_key(|(_, e_pos, _, _, _)| (e_pos.0.distance_squared(self.pos.0) * 100.0) as i32) // TODO choose target by more than just distance
            .map(|(e, _, _, _, _)| e);
        if let Some(target) = target {
            agent.target = Some(Target {
                target,
                hostile: true,
            })
        } else {
            agent.target = None;
        }
    }

    fn attack(
        &self,
        agent: &mut Agent,
        controller: &mut Controller,
        terrain: &TerrainGrid,
        tgt_pos: &Pos,
        tgt_body: Option<&Body>,
        dt: &DeltaTime,
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
                radius: 15,
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
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }

                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Axe => {
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 6.0 {
                        controller.inputs.secondary.set_state(false);
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 4.0 && self.energy.current() > 10 {
                        controller.inputs.secondary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Axe(AxeSkill::UnlockLeap))
                        && self.energy.current() > 800
                        && thread_rng().gen_bool(0.5)
                    {
                        controller.inputs.ability3.set_state(true);
                        agent.action_timer += dt.0;
                    } else {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Hammer => {
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 4.0 {
                        controller.inputs.secondary.set_state(false);
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 2.0 {
                        controller.inputs.secondary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Hammer(HammerSkill::UnlockLeap))
                        && self.energy.current() > 700
                        && thread_rng().gen_bool(0.9)
                    {
                        controller.inputs.ability3.set_state(true);
                        agent.action_timer += dt.0;
                    } else {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                                controller.inputs.ability3.set_state(true);
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Sword => {
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if self
                        .stats
                        .skill_set
                        .has_skill(Skill::Sword(SwordSkill::UnlockSpin))
                        && agent.action_timer < 2.0
                        && self.energy.current() > 600
                    {
                        controller.inputs.ability3.set_state(true);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer > 2.0 {
                        agent.action_timer = 0.0;
                    } else {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                                controller.inputs.secondary.set_state(true);
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Bow => {
                if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                    && dist_sqrd < (2.0 * min_attack_dist * self.scale).powi(2)
                {
                    controller.inputs.roll.set_state(true);
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
                                controller.inputs.secondary.set_state(false);
                                agent.action_timer = 0.0;
                            } else if agent.action_timer > 2.0 && self.energy.current() > 300 {
                                controller.inputs.secondary.set_state(true);
                                agent.action_timer += dt.0;
                            } else if self
                                .stats
                                .skill_set
                                .has_skill(Skill::Bow(BowSkill::UnlockRepeater))
                                && self.energy.current() > 400
                                && thread_rng().gen_bool(0.8)
                            {
                                controller.inputs.secondary.set_state(false);
                                controller.inputs.ability3.set_state(true);
                                agent.action_timer += dt.0;
                            } else {
                                controller.inputs.secondary.set_state(false);
                                controller.actions.push(ControlAction::StartInput {
                                    ability: InputKind::Primary,
                                    target: None,
                                });
                                //controller.inputs.primary.set_state(true);
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Staff => {
                if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                    && dist_sqrd < (min_attack_dist * self.scale).powi(2)
                {
                    controller.inputs.roll.set_state(true);
                } else if dist_sqrd < (5.0 * min_attack_dist * self.scale).powi(2) {
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
                        controller.inputs.ability3.set_state(true);
                    } else if self.energy.current() > 10 {
                        controller.inputs.secondary.set_state(true);
                    } else {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                            controller.actions.push(ControlAction::StartInput {
                                ability: InputKind::Primary,
                                target: None,
                            });
                            //controller.inputs.primary.set_state(true);
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                    if self.body.map(|b| b.is_humanoid()).unwrap_or(false)
                        && dist_sqrd < 16.0f32.powi(2)
                        && thread_rng().gen::<f32>() < 0.02
                    {
                        controller.inputs.roll.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::StoneGolemBoss => {
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    // 2.0 is temporary correction factor to allow them to melee with their
                    // large hitbox
                    controller.inputs.move_dir = Vec2::zero();
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
                } else if dist_sqrd < MAX_CHASE_DIST.powi(2) {
                    if self.vel.0.is_approx_zero() {
                        controller.inputs.ability3.set_state(true);
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
                                controller.inputs.secondary.set_state(true);
                                agent.action_timer = 0.0;
                            } else {
                                agent.action_timer += dt.0;
                            }
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
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
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) && thread_rng().gen_bool(0.5)
                {
                    controller.inputs.move_dir = Vec2::zero();
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
                } else if dist_sqrd < (radius as f32 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = (self.pos.0 - tgt_pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y);
                } else if dist_sqrd < ((radius as f32 + 1.0) * min_attack_dist * self.scale).powi(2)
                    && dist_sqrd > (radius as f32 * min_attack_dist * self.scale).powi(2)
                {
                    if agent.action_timer < circle_time as f32 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < circle_time as f32 + 0.5 {
                        controller.inputs.secondary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 2.0 * circle_time as f32 + 0.5 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 2.0 * circle_time as f32 + 1.0 {
                        controller.inputs.secondary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowRanged => {
                if dist_sqrd < (3.0 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                        .xy()
                        .try_normalized()
                        .unwrap_or_else(Vec2::unit_y);
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
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
                            controller.inputs.secondary.set_state(true);
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
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
                if dist_sqrd < (1.5 * min_attack_dist * self.scale).powi(2) {
                    if agent.action_timer > 4.0 {
                        controller.actions.push(ControlAction::CancelInput);
                        //controller.inputs.primary.set_state(false);
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 1.0 {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
                        agent.action_timer += dt.0;
                    } else {
                        controller.inputs.secondary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowQuick => {
                if dist_sqrd < (1.5 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller.inputs.secondary.set_state(true);
                } else if dist_sqrd < (3.0 * min_attack_dist * self.scale).powi(2)
                    && dist_sqrd > (2.0 * min_attack_dist * self.scale).powi(2)
                {
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadLowBasic => {
                if dist_sqrd < (1.5 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer > 5.0 {
                        agent.action_timer = 0.0;
                    } else if agent.action_timer > 2.0 {
                        controller.inputs.secondary.set_state(true);
                        agent.action_timer += dt.0;
                    } else {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadMedJump => {
                if dist_sqrd < (1.5 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller.inputs.secondary.set_state(true);
                } else if dist_sqrd < (5.0 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.ability3.set_state(true);
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
                            controller.actions.push(ControlAction::StartInput {
                                ability: InputKind::Primary,
                                target: None,
                            });
                            //controller.inputs.primary.set_state(true);
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                        } else {
                            controller.inputs.move_dir =
                                bearing.xy().try_normalized().unwrap_or_else(Vec2::zero) * speed;
                            controller.inputs.jump.set_state(bearing.z > 1.5);
                            controller.inputs.move_z = bearing.z;
                        }
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::QuadMedBasic => {
                if dist_sqrd < (min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    if agent.action_timer < 2.0 {
                        controller.inputs.secondary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 3.0 {
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Lavadrake | Tactic::QuadLowBeam => {
                if dist_sqrd < (2.5 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller.inputs.secondary.set_state(true);
                } else if dist_sqrd < (7.0 * min_attack_dist * self.scale).powi(2) {
                    if agent.action_timer < 2.0 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 4.0 {
                        controller.inputs.move_dir = (tgt_pos.0 - self.pos.0)
                            .xy()
                            .rotated_z(-0.47 * PI)
                            .try_normalized()
                            .unwrap_or_else(Vec2::unit_y);
                        controller.actions.push(ControlAction::StartInput {
                            ability: InputKind::Primary,
                            target: None,
                        });
                        //controller.inputs.primary.set_state(true);
                        agent.action_timer += dt.0;
                    } else if agent.action_timer < 6.0 {
                        controller.inputs.ability3.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Theropod => {
                if dist_sqrd < (2.0 * min_attack_dist * self.scale).powi(2) {
                    controller.inputs.move_dir = Vec2::zero();
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
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
                        controller.inputs.jump.set_state(bearing.z > 1.5);
                        controller.inputs.move_z = bearing.z;
                    }
                } else {
                    agent.target = None;
                }
            },
            Tactic::Turret => {
                if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
                } else {
                    agent.target = None;
                }
            },
            Tactic::FixedTurret => {
                controller.inputs.look_dir = self.ori.look_dir();
                if can_see_tgt(&*terrain, self.pos, tgt_pos, dist_sqrd) {
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
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
                    controller.actions.push(ControlAction::StartInput {
                        ability: InputKind::Primary,
                        target: None,
                    });
                    //controller.inputs.primary.set_state(true);
                } else {
                    agent.target = None;
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
            controller.inputs.jump.set_state(bearing.z > 1.5);
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
