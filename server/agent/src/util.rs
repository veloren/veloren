use crate::data::{ActionMode, AgentData, AttackData, Path, ReadData, TargetData};
use common::{
    comp::{
        agent::Psyche, buff::BuffKind, inventory::item::ItemTag, item::ItemDesc, Agent, Alignment,
        Body, Controller, InputKind, Pos,
    },
    consts::GRAVITY,
    terrain::Block,
    util::Dir,
    vol::ReadVol,
};
use core::f32::consts::PI;
use rand::Rng;
use specs::{
    saveload::{Marker, MarkerAllocator},
    Entity as EcsEntity,
};
use vek::*;

pub fn is_dead_or_invulnerable(entity: EcsEntity, read_data: &ReadData) -> bool {
    is_dead(entity, read_data) || is_invulnerable(entity, read_data)
}

pub fn is_dead(entity: EcsEntity, read_data: &ReadData) -> bool {
    let health = read_data.healths.get(entity);
    health.map_or(false, |a| a.is_dead)
}

// FIXME: The logic that is used in this function and throughout the code
// shouldn't be used to mean that a character is in a safezone.
pub fn is_invulnerable(entity: EcsEntity, read_data: &ReadData) -> bool {
    let buffs = read_data.buffs.get(entity);

    buffs.map_or(false, |b| b.kinds.contains_key(&BuffKind::Invulnerability))
}

/// Gets alignment of owner if alignment given is `Owned`.
/// Returns original alignment if not owned.
pub fn try_owner_alignment<'a>(
    alignment: Option<&'a Alignment>,
    read_data: &'a ReadData,
) -> Option<&'a Alignment> {
    if let Some(Alignment::Owned(owner_uid)) = alignment {
        if let Some(owner) = get_entity_by_id(owner_uid.id(), read_data) {
            return read_data.alignments.get(owner);
        }
    }
    alignment
}

/// Projectile motion: Returns the direction to aim for the projectile to reach
/// target position. Does not take any forces but gravity into account.
pub fn aim_projectile(speed: f32, pos: Vec3<f32>, tgt: Vec3<f32>) -> Option<Dir> {
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

pub fn get_entity_by_id(id: u64, read_data: &ReadData) -> Option<EcsEntity> {
    read_data.uid_allocator.retrieve_entity_internal(id)
}

/// Calculates whether the agent should continue chase or let the target escape.
///
/// Will return true when score of letting target escape is higher then the
/// score of continuing the pursue, false otherwise.
pub fn stop_pursuing(
    dist_to_target_sqrd: f32,
    dist_to_home_sqrd: f32,
    own_health_fraction: f32,
    target_health_fraction: f32,
    dur_since_last_attacked: f64,
    psyche: &Psyche,
) -> bool {
    should_let_target_escape(
        dist_to_home_sqrd,
        dur_since_last_attacked,
        own_health_fraction,
    ) > should_continue_to_pursue(dist_to_target_sqrd, psyche, target_health_fraction)
}

/// Scores the benefit of continuing the pursue in value from 0 to infinity.
fn should_continue_to_pursue(
    dist_to_target_sqrd: f32,
    psyche: &Psyche,
    target_health_fraction: f32,
) -> f32 {
    let aggression_score = (1.0 / psyche.flee_health.max(0.25))
        * psyche.aggro_dist.unwrap_or(psyche.sight_dist)
        * psyche.sight_dist;

    (100.0 * aggression_score) / (dist_to_target_sqrd * target_health_fraction)
}

/// Scores the benefit of letting the target escape in a value from 0 to
/// infinity.
fn should_let_target_escape(
    dist_to_home_sqrd: f32,
    dur_since_last_attacked: f64,
    own_health_fraction: f32,
) -> f32 {
    (dist_to_home_sqrd / own_health_fraction) * dur_since_last_attacked as f32 * 0.005
}

pub fn entity_looks_like_cultist(entity: EcsEntity, read_data: &ReadData) -> bool {
    let number_of_cultist_items_equipped = read_data.inventories.get(entity).map_or(0, |inv| {
        inv.equipped_items()
            .filter(|item| item.tags().contains(&ItemTag::Cultist))
            .count()
    });

    number_of_cultist_items_equipped > 2
}

// FIXME: `Alignment::Npc` doesn't necessarily mean villager.
pub fn is_villager(alignment: Option<&Alignment>) -> bool {
    alignment.map_or(false, |alignment| matches!(alignment, Alignment::Npc))
}

pub fn is_village_guard(entity: EcsEntity, read_data: &ReadData) -> bool {
    read_data
        .stats
        .get(entity)
        .map_or(false, |stats| stats.name == "Guard")
}

pub fn are_our_owners_hostile(
    our_alignment: Option<&Alignment>,
    their_alignment: Option<&Alignment>,
    read_data: &ReadData,
) -> bool {
    try_owner_alignment(our_alignment, read_data).map_or(false, |our_owners_alignment| {
        try_owner_alignment(their_alignment, read_data).map_or(false, |their_owners_alignment| {
            our_owners_alignment.hostile_towards(*their_owners_alignment)
        })
    })
}

pub fn entities_have_line_of_sight(
    pos: &Pos,
    body: Option<&Body>,
    other_pos: &Pos,
    other_body: Option<&Body>,
    read_data: &ReadData,
) -> bool {
    let get_eye_pos = |pos: &Pos, body: Option<&Body>| {
        let eye_offset = body.map_or(0.0, |b| b.eye_height());

        Pos(pos.0.with_z(pos.0.z + eye_offset))
    };
    let eye_pos = get_eye_pos(pos, body);
    let other_eye_pos = get_eye_pos(other_pos, other_body);

    positions_have_line_of_sight(&eye_pos, &other_eye_pos, read_data)
}

pub fn positions_have_line_of_sight(pos_a: &Pos, pos_b: &Pos, read_data: &ReadData) -> bool {
    let dist_sqrd = pos_b.0.distance_squared(pos_a.0);

    read_data
        .terrain
        .ray(pos_a.0, pos_b.0)
        .until(Block::is_opaque)
        .cast()
        .0
        .powi(2)
        >= dist_sqrd
}

pub fn is_dressed_as_cultist(entity: EcsEntity, read_data: &ReadData) -> bool {
    read_data
        .inventories
        .get(entity)
        .map_or(false, |inventory| {
            inventory
                .equipped_items()
                .filter(|item| item.tags().contains(&ItemTag::Cultist))
                .count()
                > 2
        })
}

pub fn get_attacker(entity: EcsEntity, read_data: &ReadData) -> Option<EcsEntity> {
    read_data
        .healths
        .get(entity)
        .filter(|health| health.last_change.amount < 0.0)
        .and_then(|health| health.last_change.damage_by())
        .and_then(|damage_contributor| get_entity_by_id(damage_contributor.uid().0, read_data))
}

impl<'a> AgentData<'a> {
    pub fn has_buff(&self, read_data: &ReadData, buff: BuffKind) -> bool {
        read_data
            .buffs
            .get(*self.entity)
            .map_or(false, |b| b.kinds.contains_key(&buff))
    }
}

// Probably works best for melee (or maybe only for melee considering its
// reliance on blocking?)
/// Handles whether an agent should attack and how the agent moves around.
/// Returns whether the agent should attack (so that individual tactics can
/// determine what specific attack to use)
pub fn handle_attack_aggression(
    agent_data: &AgentData,
    agent: &mut Agent,
    controller: &mut Controller,
    attack_data: &AttackData,
    tgt_data: &TargetData,
    read_data: &ReadData,
    rng: &mut impl Rng,
    timer_pos_timeout_index: usize,
    timer_guarded_cycle_index: usize,
    fcounter_guarded_timer_index: usize,
    icounter_action_mode_index: usize,
    condition_guarded_defend_index: usize,
    condition_rolling_breakthrough_index: usize,
    position_guarded_cover_index: usize,
    position_flee_index: usize,
) -> bool {
    if let Some(health) = agent_data.health {
        agent.action_state.int_counters[icounter_action_mode_index] = if health.fraction() < 0.1 {
            agent.action_state.positions[position_guarded_cover_index] = None;
            ActionMode::Fleeing as u8
        } else if health.fraction() < 0.9 {
            agent.action_state.positions[position_flee_index] = None;
            ActionMode::Guarded as u8
        } else {
            agent.action_state.positions[position_guarded_cover_index] = None;
            agent.action_state.positions[position_flee_index] = None;
            ActionMode::Reckless as u8
        };
    }

    // If agent has not moved, assume agent was unable to move and reset attempted
    // path positions if occurs for too long
    if agent_data.vel.0.magnitude_squared() < 1_f32.powi(2) {
        agent.action_state.timers[timer_pos_timeout_index] += read_data.dt.0;
    } else {
        agent.action_state.timers[timer_pos_timeout_index] = 0.0;
    }

    if agent.action_state.timers[timer_pos_timeout_index] > 2.0 {
        agent.action_state.positions[position_guarded_cover_index] = None;
        agent.action_state.positions[position_flee_index] = None;
        agent.action_state.timers[timer_pos_timeout_index] = 0.0;
    }

    match ActionMode::from_u8(agent.action_state.int_counters[icounter_action_mode_index]) {
        ActionMode::Reckless => true,
        ActionMode::Guarded => {
            agent.action_state.timers[timer_guarded_cycle_index] += read_data.dt.0;
            if agent.action_state.timers[timer_guarded_cycle_index]
                > agent.action_state.counters[fcounter_guarded_timer_index]
            {
                agent.action_state.timers[timer_guarded_cycle_index] = 0.0;
                agent.action_state.conditions[condition_guarded_defend_index] ^= true;
                agent.action_state.counters[fcounter_guarded_timer_index] =
                    if agent.action_state.conditions[condition_guarded_defend_index] {
                        rng.gen_range(3.0..6.0)
                    } else {
                        rng.gen_range(6.0..10.0)
                    };
            }
            if let Some(pos) = agent.action_state.positions[position_guarded_cover_index] {
                if pos.distance_squared(agent_data.pos.0) < 3_f32.powi(2) {
                    agent.action_state.positions[position_guarded_cover_index] = None;
                }
            }
            if !agent.action_state.conditions[condition_guarded_defend_index] {
                agent.action_state.positions[position_guarded_cover_index] = None;
                true
            } else {
                if attack_data.dist_sqrd > 10_f32.powi(2) {
                    // Choose random point to either side when looking at target and move
                    // towards it
                    if let Some(pos) = agent.action_state.positions[position_guarded_cover_index] {
                        if pos.distance_squared(agent_data.pos.0) < 5_f32.powi(2) {
                            agent.action_state.positions[position_guarded_cover_index] = None;
                        }
                        agent_data.path_toward_target(
                            agent,
                            controller,
                            pos,
                            read_data,
                            Path::Separate,
                            None,
                        );
                    } else {
                        agent.action_state.positions[position_guarded_cover_index] = {
                            let rand_dir = {
                                let dir = (tgt_data.pos.0 - agent_data.pos.0)
                                    .try_normalized()
                                    .unwrap_or(Vec3::unit_x())
                                    .xy();
                                if rng.gen_bool(0.5) {
                                    dir.rotated_z(PI / 2.0 + rng.gen_range(-0.75..0.0))
                                } else {
                                    dir.rotated_z(-PI / 2.0 + rng.gen_range(-0.0..0.75))
                                }
                            };
                            let attempted_dist = rng.gen_range(6.0..16.0);
                            let actual_dist = read_data
                                .terrain
                                .ray(
                                    agent_data.pos.0 + Vec3::unit_z() * 0.5,
                                    agent_data.pos.0
                                        + Vec3::unit_z() * 0.5
                                        + rand_dir * attempted_dist,
                                )
                                .until(Block::is_solid)
                                .cast()
                                .0
                                - 1.0;
                            Some(agent_data.pos.0 + rand_dir * actual_dist)
                        };
                    }
                } else if let Some(pos) = agent.action_state.positions[position_guarded_cover_index]
                {
                    agent_data.path_toward_target(
                        agent,
                        controller,
                        pos,
                        read_data,
                        Path::Separate,
                        None,
                    );
                    if agent.action_state.conditions[condition_rolling_breakthrough_index] {
                        controller.push_basic_input(InputKind::Roll);
                        agent.action_state.conditions[condition_rolling_breakthrough_index] = false;
                    }
                    if tgt_data.char_state.map_or(false, |cs| cs.is_melee_attack()) {
                        controller.push_basic_input(InputKind::Block);
                    }
                } else {
                    agent.action_state.positions[position_guarded_cover_index] = {
                        let backwards = (agent_data.pos.0 - tgt_data.pos.0)
                            .try_normalized()
                            .unwrap_or(Vec3::unit_x())
                            .xy();
                        let pos = if read_data
                            .terrain
                            .ray(
                                agent_data.pos.0 + Vec3::unit_z() * 0.5,
                                agent_data.pos.0 + Vec3::unit_z() * 0.5 + backwards * 6.0,
                            )
                            .until(Block::is_solid)
                            .cast()
                            .0
                            > 5.0
                        {
                            agent_data.pos.0 + backwards * 5.0
                        } else {
                            agent.action_state.conditions[condition_rolling_breakthrough_index] =
                                true;
                            agent_data.pos.0
                                - backwards
                                    * read_data
                                        .terrain
                                        .ray(
                                            agent_data.pos.0 + Vec3::unit_z() * 0.5,
                                            agent_data.pos.0 + Vec3::unit_z() * 0.5
                                                - backwards * 10.0,
                                        )
                                        .until(Block::is_solid)
                                        .cast()
                                        .0
                                - 1.0
                        };
                        Some(pos)
                    }
                }
                false
            }
        },
        ActionMode::Fleeing => {
            if agent.action_state.conditions[condition_rolling_breakthrough_index] {
                controller.push_basic_input(InputKind::Roll);
                agent.action_state.conditions[condition_rolling_breakthrough_index] = false;
            }
            if let Some(pos) = agent.action_state.positions[position_flee_index] {
                if let Some(dir) = Dir::from_unnormalized(pos - agent_data.pos.0) {
                    controller.inputs.look_dir = dir;
                }
                if pos.distance_squared(agent_data.pos.0) < 5_f32.powi(2) {
                    agent.action_state.positions[position_flee_index] = None;
                }
                agent_data.path_toward_target(
                    agent,
                    controller,
                    pos,
                    read_data,
                    Path::Separate,
                    None,
                );
            } else {
                agent.action_state.positions[position_flee_index] = {
                    let rand_dir = {
                        let dir = (agent_data.pos.0 - tgt_data.pos.0)
                            .try_normalized()
                            .unwrap_or(Vec3::unit_x())
                            .xy();
                        dir.rotated_z(rng.gen_range(-0.75..0.75))
                    };
                    let attempted_dist = rng.gen_range(16.0..26.0);
                    let actual_dist = read_data
                        .terrain
                        .ray(
                            agent_data.pos.0 + Vec3::unit_z() * 0.5,
                            agent_data.pos.0 + Vec3::unit_z() * 0.5 + rand_dir * attempted_dist,
                        )
                        .until(Block::is_solid)
                        .cast()
                        .0
                        - 1.0;
                    if actual_dist < 10.0 {
                        let dist = read_data
                            .terrain
                            .ray(
                                agent_data.pos.0 + Vec3::unit_z() * 0.5,
                                agent_data.pos.0 + Vec3::unit_z() * 0.5 - rand_dir * attempted_dist,
                            )
                            .until(Block::is_solid)
                            .cast()
                            .0
                            - 1.0;
                        agent.action_state.conditions[condition_rolling_breakthrough_index] = true;
                        Some(agent_data.pos.0 - rand_dir * dist)
                    } else {
                        Some(agent_data.pos.0 + rand_dir * actual_dist)
                    }
                };
            }
            false
        },
    }
}
