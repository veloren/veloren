use crate::data::{AgentData, ReadData};
use common::{
    comp::{
        agent::Psyche, buff::BuffKind, inventory::item::ItemTag, item::ItemDesc, Alignment, Body,
        Pos,
    },
    consts::GRAVITY,
    terrain::Block,
    util::Dir,
    vol::ReadVol,
};
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
