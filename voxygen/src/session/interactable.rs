use std::{cmp::Reverse, collections::HashSet};

use specs::{Join, LendJoin, ReadStorage, WorldExt};
use vek::*;

use super::target::{self, Target};
use client::Client;
use common::{
    CachedSpatialGrid,
    comp::{
        self, Alignment, Collider, Content, pet, ship::figuredata::VOXEL_COLLIDER_MANIFEST,
        tool::ToolKind,
    },
    consts,
    link::Is,
    mounting::{Mount, Volume, VolumePos, VolumeRider},
    states::utils::can_perform_pet,
    terrain::{Block, TerrainGrid, UnlockKind},
    uid::{IdMaps, Uid},
};
use common_base::span;
use hashbrown::HashMap;

use crate::{
    game_input::GameInput,
    hud::CraftingTab,
    scene::{Scene, terrain::Interaction},
};

#[derive(Debug, Default)]
pub struct Interactables {
    /// `f32` is distance squared, currently only used to prioritize
    /// `Interactable`s when building `input_map`.
    pub input_map: HashMap<GameInput, (f32, Interactable)>,
    /// Set of all nearby interactable entities, stored separately for fast
    /// access in scene
    pub entities: HashSet<specs::Entity>,
}

#[derive(Clone, Debug)]
pub enum BlockInteraction {
    Collect { steal: bool },
    Unlock { kind: UnlockKind, steal: bool },
    Craft(CraftingTab),
    // TODO: mining blocks don't use the interaction key, so it might not be the best abstraction
    // to have them here, will see how things turn out
    Mine(ToolKind),
    Mount,
    Read(Content),
    LightToggle(bool),
}

#[derive(Debug, Clone)]
pub enum Interactable {
    Block {
        block: Block,
        volume_pos: VolumePos,
        interaction: BlockInteraction,
    },
    Entity {
        entity: specs::Entity,
        interaction: EntityInteraction,
    },
}

/// The type of interaction an entity has
#[derive(Debug, Clone, Copy, PartialEq, Eq, enum_map::Enum)]
pub enum EntityInteraction {
    HelpDowned,
    PickupItem,
    ActivatePortal,
    Pet,
    Talk,
    CampfireSit,
    Trade,
    StayFollow,
    Mount,
}

// TODO: push this block down below `get_interactables` in a separate MR so that
// it is consistent with placement of other impl blocks in this file.
impl BlockInteraction {
    fn from_block_pos(
        terrain: &TerrainGrid,
        id_maps: &IdMaps,
        colliders: &ReadStorage<Collider>,
        volume_pos: VolumePos,
        interaction: Interaction,
    ) -> Option<(Block, Self)> {
        let block = volume_pos.get_block(terrain, id_maps, colliders)?;
        let block_interaction = match interaction {
            Interaction::Collect => {
                // TODO: this check may no longer be necessary?!
                // Check if the block is not collectable
                if !block.is_directly_collectible() {
                    return None;
                };
                // Check if this is an unlockable sprite.
                let unlock = match volume_pos.kind {
                    Volume::Terrain => block.get_sprite().and_then(|sprite| {
                        let sprite_cfg = terrain.sprite_cfg_at(volume_pos.pos);
                        sprite.unlock_condition(sprite_cfg)
                    }),
                    Volume::Entity(_) => None,
                };

                if let Some(unlock) = unlock {
                    BlockInteraction::Unlock {
                        kind: unlock.into_owned(),
                        steal: block.is_owned(),
                    }
                } else if let Some(mine_tool) = block.mine_tool() {
                    BlockInteraction::Mine(mine_tool)
                } else {
                    BlockInteraction::Collect {
                        steal: block.is_owned(),
                    }
                }
            },
            Interaction::Read => match volume_pos.kind {
                common::mounting::Volume::Terrain => block.get_sprite().and_then(|sprite| {
                    let chunk = terrain.pos_chunk(volume_pos.pos)?;
                    let sprite_chunk_pos = TerrainGrid::chunk_offs(volume_pos.pos);
                    let sprite_cfg = chunk.meta().sprite_cfg_at(sprite_chunk_pos);
                    sprite
                        .content(sprite_cfg.cloned())
                        .map(BlockInteraction::Read)
                })?,
                // Signs on volume entities are not currently supported
                common::mounting::Volume::Entity(_) => return None,
            },
            Interaction::Craft(tab) => BlockInteraction::Craft(tab),
            Interaction::Mount => BlockInteraction::Mount,
            Interaction::LightToggle(enable) => BlockInteraction::LightToggle(enable),
        };
        Some((block, block_interaction))
    }

    fn game_input(&self) -> GameInput {
        match self {
            BlockInteraction::Collect { .. }
            | BlockInteraction::Read(_)
            | BlockInteraction::LightToggle(_)
            | BlockInteraction::Craft(_)
            | BlockInteraction::Unlock { .. } => GameInput::Interact,
            BlockInteraction::Mine(_) => GameInput::Primary,
            BlockInteraction::Mount => GameInput::Mount,
        }
    }

    fn range(&self) -> f32 {
        // Update `max_range` below when editing this.
        match self {
            BlockInteraction::Collect { .. }
            | BlockInteraction::Unlock { .. }
            | BlockInteraction::Mine(_)
            | BlockInteraction::Craft(_) => consts::MAX_PICKUP_RANGE,
            BlockInteraction::Mount => consts::MAX_SPRITE_MOUNT_RANGE,
            BlockInteraction::LightToggle(_) | BlockInteraction::Read(_) => {
                consts::MAX_INTERACT_RANGE
            },
        }
    }

    fn max_range() -> f32 {
        consts::MAX_PICKUP_RANGE
            .max(consts::MAX_MOUNT_RANGE)
            .max(consts::MAX_INTERACT_RANGE)
    }
}

#[derive(Debug)]
pub enum GetInteractablesError {
    ClientMissingPosition,
    ClientMissingUid,
}

/// Scan for any nearby interactables, including further ones if directly
/// targetted.
pub(super) fn get_interactables(
    client: &Client,
    collect_target: Option<Target<target::Collectable>>,
    entity_target: Option<Target<target::Entity>>,
    mine_target: Option<Target<target::Mine>>,
    terrain_target: Option<Target<target::Terrain>>,
    scene: &Scene,
) -> Result<HashMap<GameInput, (f32, Interactable)>, GetInteractablesError> {
    span!(_guard, "select_interactable");
    use common::{spiral::Spiral2d, terrain::TerrainChunk, vol::RectRasterableVol};

    let voxel_colliders_manifest = VOXEL_COLLIDER_MANIFEST.read();

    let ecs = client.state().ecs();
    let id_maps = ecs.read_resource::<IdMaps>();
    let scene_terrain = scene.terrain();
    let terrain = client.state().terrain();
    let player_entity = client.entity();
    let interpolated = ecs.read_storage::<crate::ecs::comp::Interpolated>();
    let player_pos = interpolated
        .get(player_entity)
        .ok_or(GetInteractablesError::ClientMissingPosition)?
        .pos;

    let uids = ecs.read_storage::<Uid>();
    let healths = ecs.read_storage::<comp::Health>();
    let colliders = ecs.read_storage::<comp::Collider>();
    let char_states = ecs.read_storage::<comp::CharacterState>();
    let is_mounts = ecs.read_storage::<Is<Mount>>();
    let bodies = ecs.read_storage::<comp::Body>();
    let masses = ecs.read_storage::<comp::Mass>();
    let items = ecs.read_storage::<comp::PickupItem>();
    let alignments = ecs.read_storage::<comp::Alignment>();
    let is_volume_rider = ecs.read_storage::<Is<VolumeRider>>();

    let player_chunk = player_pos.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
        (e.floor() as i32).div_euclid(sz as i32)
    });
    let player_body = bodies.get(player_entity);
    let player_mass = masses.get(player_entity);
    let Some(player_uid) = uids.get(player_entity).copied() else {
        tracing::error!("Client has no Uid component! Not scanning for any interactables.");
        return Err(GetInteractablesError::ClientMissingUid);
    };

    let spacial_grid = ecs.read_resource::<CachedSpatialGrid>();

    let entities = ecs.entities();
    let mut entity_data = (
        &entities,
        !&is_mounts,
        &uids,
        &interpolated,
        &bodies,
        &masses,
        char_states.maybe(),
        healths.maybe(),
        alignments.maybe(),
        items.mask().maybe(),
    )
        .lend_join();

    let interactable_entities = spacial_grid
        .0
        .in_circle_aabr(player_pos.xy(), EntityInteraction::max_range())
        .chain(entity_target.map(|t| t.kind.0))
        .filter(|&entity| entity != player_entity)
        .filter_map(|entity| entity_data.get(entity, &entities))
        .flat_map(
            |(entity, _, uid, interpolated, body, mass, char_state, health, alignment, has_item)| {
                // If an entity is downed, the only allowed interaction is HelpDowned
                let is_downed = comp::is_downed(health, char_state);

                // Interactions using [`GameInput::Interact`]
                let interaction = if is_downed {
                    Some(EntityInteraction::HelpDowned)
                } else if has_item.is_some() {
                    Some(EntityInteraction::PickupItem)
                } else if body.is_portal() {
                    Some(EntityInteraction::ActivatePortal)
                } else if alignment.is_some_and(|alignment| {
                    can_perform_pet(comp::Pos(player_pos), comp::Pos(interpolated.pos), *alignment)
                }) {
                    Some(EntityInteraction::Pet)
                } else if alignment.is_some_and(|alignment| matches!(alignment, Alignment::Npc)) {
                    Some(EntityInteraction::Talk)
                } else {
                    None
                };

                // Interaction using [`GameInput::Sit`]
                let sit =
                    (body.is_campfire() && !is_downed).then_some(EntityInteraction::CampfireSit);

                // Interaction using [`GameInput::Trade`]
                // TODO: Remove this once we have a better way do determine whether an entity
                // can be traded with not based on alignment.
                let trade = (!is_downed
                    && alignment.is_some_and(|alignment| match alignment {
                        Alignment::Npc => true,
                        Alignment::Owned(other_uid) => other_uid == uid || player_uid == *other_uid,
                        _ => false,
                    }))
                .then_some(EntityInteraction::Trade);

                // Interaction using [`GameInput::Mount`]
                let mount = (matches!(alignment, Some(Alignment::Owned(other_uid)) if player_uid == *other_uid)
                    && pet::is_mountable(body, mass, player_body, player_mass)
                    && !is_downed
                    && !client.is_riding())
                .then_some(EntityInteraction::Mount);

                // Interaction using [`GameInput::StayFollow`]
                let stayfollow = alignment
                    .filter(|alignment| {
                        matches!(alignment,
                            Alignment::Owned(other_uid) if player_uid == *other_uid)
                            && !is_downed
                    })
                    .map(|_| EntityInteraction::StayFollow);

                // Roughly filter out entities farther than interaction distance
                let distance_squared = player_pos.distance_squared(interpolated.pos);

                    [interaction, sit, trade, mount, stayfollow].into_iter().flatten()
                        .filter_map(move |interaction| {
                            (distance_squared <= interaction.range().powi(2)).then_some((
                                interaction,
                                entity,
                                distance_squared,
                            ))
                        })
            },
        );

    let volumes_data = (
        &entities,
        &ecs.read_storage::<Uid>(),
        &ecs.read_storage::<comp::Body>(),
        &ecs.read_storage::<crate::ecs::comp::Interpolated>(),
        &ecs.read_storage::<comp::Collider>(),
    );

    let mut volumes_data = volumes_data.lend_join();

    let volume_interactables = spacial_grid
        .0
        .in_circle_aabr(player_pos.xy(), BlockInteraction::max_range())
        .filter(|&e| e != player_entity)
        .filter_map(|e| volumes_data.get(e, &entities))
        .filter_map(|(entity, uid, body, interpolated, collider)| {
            let vol = collider.get_vol(&voxel_colliders_manifest)?;
            let (blocks_of_interest, offset) =
                scene
                    .figure_mgr()
                    .get_blocks_of_interest(entity, body, Some(collider))?;

            let mat = Mat4::from(interpolated.ori.to_quat()).translated_3d(interpolated.pos)
                * Mat4::translation_3d(offset);

            let p = mat.inverted().mul_point(player_pos);
            let aabb = Aabb {
                min: Vec3::zero(),
                max: vol.volume().sz.as_(),
            };

            if aabb.contains_point(p) || aabb.distance_to_point(p) < BlockInteraction::max_range() {
                Some(blocks_of_interest.interactables.iter().map(
                    move |(block_offset, interaction)| {
                        let wpos = mat.mul_point(block_offset.as_() + 0.5);
                        (wpos, VolumePos::entity(*block_offset, *uid), *interaction)
                    },
                ))
            } else {
                None
            }
        })
        .flatten();

    // TODO: this formula for the number to take was guessed
    // Note: assumes RECT_SIZE.x == RECT_SIZE.y
    let interactable_blocks = Spiral2d::new()
        .take(
            ((BlockInteraction::max_range() / TerrainChunk::RECT_SIZE.x as f32).ceil() as usize
                * 2
                + 1)
            .pow(2),
        )
        .flat_map(|offset| {
            let chunk_pos = player_chunk + offset;
            let chunk_voxel_pos =
                Vec3::<i32>::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));
            scene_terrain
                .get(chunk_pos)
                .map(|data| (data, chunk_voxel_pos))
        })
        .flat_map(|(chunk_data, chunk_pos)| {
            // TODO: maybe we could make this more efficient by putting the
            // interactables is some sort of spatial structure
            chunk_data
                .blocks_of_interest
                .interactables
                .iter()
                .map(move |(block_offset, interaction)| (chunk_pos + block_offset, interaction))
                .map(|(pos, interaction)| {
                    (
                        pos.as_::<f32>() + 0.5,
                        VolumePos::terrain(pos),
                        *interaction,
                    )
                })
        })
        .chain(volume_interactables)
        .chain(
            mine_target
                .map(|t| t.position_int())
                .into_iter()
                .chain(collect_target.map(|t| t.position_int()))
                .map(|pos| (pos.as_(), VolumePos::terrain(pos), Interaction::Collect)),
        )
        .filter_map(|(wpos, volume_pos, interaction)| {
            let distance_sq = wpos.distance_squared(player_pos);
            if distance_sq > BlockInteraction::max_range().powi(2) {
                return None;
            }

            let (block, interaction) = BlockInteraction::from_block_pos(
                &terrain,
                &id_maps,
                &colliders,
                volume_pos,
                interaction,
            )?;

            (distance_sq <= interaction.range().powi(2)).then_some((
                block,
                volume_pos,
                interaction,
                distance_sq,
            ))
        })
        .filter(|(_, volume_pos, interaction, _)| {
            // Additional checks for `BlockInteraction::Mount` after filtering by distance.
            if let BlockInteraction::Mount = interaction {
                !is_volume_rider.contains(player_entity)
                    // TODO: why does this check `is_entity`?
                    // TODO: Use shared volume riders component here
                    && (volume_pos.is_entity()
                        || !is_volume_rider
                            .join()
                            .any(|is_volume_rider| is_volume_rider.pos == *volume_pos))
            } else {
                true
            }
        });

    // Helper to check if an interactable is directly targetted by the player, and
    // should thus be prioritized over non-directly targetted ones.
    let is_direct_target = |interactable: &Interactable| match interactable {
        Interactable::Block {
            volume_pos,
            interaction,
            ..
        } => {
            matches!(
                (mine_target, volume_pos, interaction),
                (Some(target), VolumePos { kind: Volume::Terrain, pos }, BlockInteraction::Mine(_))
                    if target.position_int() == *pos)
                || matches!(
                        (collect_target, volume_pos, interaction),
                        (Some(target), VolumePos { kind: Volume::Terrain, pos }, BlockInteraction::Collect { .. } | BlockInteraction::Unlock { .. })
                            if target.position_int() == *pos)
                || matches!(
                    (terrain_target, volume_pos),
                    (Some(target), VolumePos { kind: Volume::Terrain, pos })
                        if target.position_int() == *pos)
        },
        Interactable::Entity { entity, .. } => {
            entity_target.is_some_and(|target| target.kind.0 == *entity)
        },
    };

    Ok(interactable_entities
        .map(|(interaction, entity, distance)| {
            (distance.powi(2), Interactable::Entity {
                entity,
                interaction,
            })
        })
        .chain(
            interactable_blocks.map(|(block, volume_pos, interaction, distance_squared)| {
                (distance_squared, Interactable::Block {
                    block,
                    volume_pos,
                    interaction,
                })
            }),
        )
        .fold(HashMap::new(), |mut map, (distance_sq, interaction)| {
            let input = interaction.game_input();

            if map
                .get(&input)
                .is_none_or(|(other_distance_sq, other_interaction)| {
                    (
                        // Prioritize direct targets
                        is_direct_target(other_interaction),
                        other_interaction.priority(),
                        Reverse(*other_distance_sq),
                    ) < (
                        is_direct_target(&interaction),
                        interaction.priority(),
                        Reverse(distance_sq),
                    )
                })
            {
                map.insert(input, (distance_sq, interaction));
            }

            map
        }))
}

impl Interactable {
    fn game_input(&self) -> GameInput {
        match self {
            Interactable::Block { interaction, .. } => interaction.game_input(),
            Interactable::Entity { interaction, .. } => interaction.game_input(),
        }
    }

    /// Priorities for different interactions. Note: Priorities are grouped by
    /// GameInput
    #[rustfmt::skip]
    fn priority(&self) -> usize {
        match self {
            // GameInput::Interact
            Self::Entity { interaction: EntityInteraction::ActivatePortal, .. }  => 4,
            Self::Entity { interaction: EntityInteraction::PickupItem, .. }      => 3,
            Self::Block  { interaction: BlockInteraction::Craft(_), .. }         => 3,
            Self::Block  { interaction: BlockInteraction::Collect { .. }, .. }   => 3,
            Self::Entity { interaction: EntityInteraction::HelpDowned, .. }      => 2,
            Self::Block  { interaction: BlockInteraction::Unlock { .. }, .. }    => 1,
            Self::Block  { interaction: BlockInteraction::Read(_), .. }          => 1,
            Self::Block  { interaction: BlockInteraction::LightToggle(_), .. }   => 1,
            Self::Entity { interaction: EntityInteraction::Pet, .. }             => 0,
            Self::Entity { interaction: EntityInteraction::Talk , .. }           => 0,

            // GameInput::Mount
            Self::Entity { interaction: EntityInteraction::Mount, .. }           => 1,
            Self::Block  { interaction: BlockInteraction::Mount, .. }            => 0,

            // These interactions have dedicated keybinds already, no need to prioritize them
            Self::Block  { interaction: BlockInteraction::Mine(_), .. }          => 0,
            Self::Entity { interaction: EntityInteraction::StayFollow, .. }      => 0,
            Self::Entity { interaction: EntityInteraction::Trade, .. }           => 0,
            Self::Entity { interaction: EntityInteraction::CampfireSit, .. }     => 0,
        }
    }
}

impl EntityInteraction {
    pub(crate) fn game_input(&self) -> GameInput {
        match self {
            EntityInteraction::HelpDowned
            | EntityInteraction::PickupItem
            | EntityInteraction::ActivatePortal
            | EntityInteraction::Pet
            | EntityInteraction::Talk => GameInput::Interact,
            EntityInteraction::StayFollow => GameInput::StayFollow,
            EntityInteraction::Trade => GameInput::Trade,
            EntityInteraction::Mount => GameInput::Mount,
            EntityInteraction::CampfireSit => GameInput::Sit,
        }
    }

    fn range(&self) -> f32 {
        // Update `max_range` below when editing this.
        match self {
            Self::Trade => consts::MAX_TRADE_RANGE,
            Self::Mount | Self::Pet => consts::MAX_MOUNT_RANGE,
            Self::PickupItem => consts::MAX_PICKUP_RANGE,
            Self::Talk => consts::MAX_NPCINTERACT_RANGE,
            Self::CampfireSit => consts::MAX_CAMPFIRE_RANGE,
            Self::ActivatePortal => consts::TELEPORTER_RADIUS,
            Self::HelpDowned | Self::StayFollow => consts::MAX_INTERACT_RANGE,
        }
    }

    fn max_range() -> f32 {
        consts::MAX_TRADE_RANGE
            .max(consts::MAX_MOUNT_RANGE)
            .max(consts::MAX_PICKUP_RANGE)
            .max(consts::MAX_NPCINTERACT_RANGE)
            .max(consts::MAX_CAMPFIRE_RANGE)
            .max(consts::MAX_INTERACT_RANGE)
    }
}

impl Interactables {
    /// Maps the interaction targets to all their available interactions
    pub fn inverted_map(
        &self,
    ) -> (
        HashMap<specs::Entity, Vec<EntityInteraction>>,
        HashMap<VolumePos, (Block, Vec<&BlockInteraction>)>,
    ) {
        let (mut entity_map, block_map) = self.input_map.iter().fold(
            (HashMap::new(), HashMap::new()),
            |(mut entity_map, mut block_map), (_input, (_, interactable))| {
                match interactable {
                    Interactable::Entity {
                        entity,
                        interaction,
                    } => {
                        entity_map
                            .entry(*entity)
                            .and_modify(|i: &mut Vec<_>| i.push(*interaction))
                            .or_insert_with(|| vec![*interaction]);
                    },
                    Interactable::Block {
                        block,
                        volume_pos,
                        interaction,
                    } => {
                        block_map
                            .entry(*volume_pos)
                            .and_modify(|(_, i): &mut (_, Vec<_>)| i.push(interaction))
                            .or_insert_with(|| (*block, vec![interaction]));
                    },
                }

                (entity_map, block_map)
            },
        );

        // Ensure interactions are ordered in a stable way
        // TODO: Once blocks can have more than one interaction, do the same for blocks
        // here too
        for v in entity_map.values_mut() {
            v.sort_unstable_by_key(|i| i.game_input())
        }

        (entity_map, block_map)
    }
}
