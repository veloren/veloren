use ordered_float::OrderedFloat;
use specs::{Join, WorldExt};
use vek::*;

use super::{
    find_shortest_distance,
    target::{self, Target, MAX_TARGET_RANGE},
};
use client::Client;
use common::{
    comp,
    comp::tool::ToolKind,
    consts::MAX_PICKUP_RANGE,
    link::Is,
    mounting::Mount,
    terrain::{Block, TerrainGrid, UnlockKind},
    util::find_dist::{Cube, Cylinder, FindDist},
    vol::ReadVol,
};
use common_base::span;

use crate::{
    hud::CraftingTab,
    scene::{terrain::Interaction, Scene},
};

#[derive(Clone, Debug)]
pub enum BlockInteraction {
    Collect,
    Unlock(UnlockKind),
    Craft(CraftingTab),
    // TODO: mining blocks don't use the interaction key, so it might not be the best abstraction
    // to have them here, will see how things turn out
    Mine(ToolKind),
}

#[derive(Clone, Debug)]
pub enum Interactable {
    Block(Block, Vec3<i32>, BlockInteraction),
    Entity(specs::Entity),
}

impl Interactable {
    pub fn entity(&self) -> Option<specs::Entity> {
        match self {
            Self::Entity(e) => Some(*e),
            Self::Block(_, _, _) => None,
        }
    }

    fn from_block_pos(
        terrain: &TerrainGrid,
        pos: Vec3<i32>,
        interaction: Interaction,
    ) -> Option<Self> {
        let Ok(&block) = terrain.get(pos) else { return None };
        let block_interaction = match interaction {
            Interaction::Collect => {
                // Check if this is an unlockable sprite
                let unlock = block.get_sprite().and_then(|sprite| {
                    let Some(chunk) = terrain.pos_chunk(pos) else { return None };
                    let sprite_chunk_pos = TerrainGrid::chunk_offs(pos);
                    let sprite_cfg = chunk.meta().sprite_cfg_at(sprite_chunk_pos);
                    let unlock_condition = sprite.unlock_condition(sprite_cfg.cloned());
                    // HACK: No other way to distinguish between things that should be unlockable
                    // and regular sprites with the current unlock_condition method so we hack
                    // around that by saying that it is a regular collectible sprite if
                    // `unlock_condition` returns UnlockKind::Free and the cfg was `None`.
                    if sprite_cfg.is_some() || !matches!(&unlock_condition, UnlockKind::Free) {
                        Some(unlock_condition)
                    } else {
                        None
                    }
                });

                if let Some(unlock) = unlock {
                    BlockInteraction::Unlock(unlock)
                } else if let Some(mine_tool) = block.mine_tool() {
                    BlockInteraction::Mine(mine_tool)
                } else {
                    BlockInteraction::Collect
                }
            },
            Interaction::Craft(tab) => BlockInteraction::Craft(tab),
        };
        Some(Self::Block(block, pos, block_interaction))
    }
}

/// Select interactable to highlight, display interaction text for, and to
/// interact with if the interact key is pressed
/// Selected in the following order:
/// 1) Targeted items, in order of nearest under cursor:
///     (a) entity (if within range)
///     (b) collectable
///     (c) can be mined, and is a mine sprite (Air) not a weak rock.
/// 2) outside of targeted cam ray
///     -> closest of nearest interactable entity/block
pub(super) fn select_interactable(
    client: &Client,
    collect_target: Option<Target<target::Collectable>>,
    entity_target: Option<Target<target::Entity>>,
    mine_target: Option<Target<target::Mine>>,
    scene: &Scene,
) -> Option<Interactable> {
    span!(_guard, "select_interactable");
    use common::{spiral::Spiral2d, terrain::TerrainChunk, vol::RectRasterableVol};

    let nearest_dist = find_shortest_distance(&[
        mine_target.map(|t| t.distance),
        entity_target.map(|t| t.distance),
        collect_target.map(|t| t.distance),
    ]);

    let terrain = client.state().terrain();

    if let Some(interactable) = entity_target
        .and_then(|t| {
            if t.distance < MAX_TARGET_RANGE && Some(t.distance) == nearest_dist {
                let entity = t.kind.0;
                Some(Interactable::Entity(entity))
            } else {
                None
            }
        })
        .or_else(|| {
            collect_target.and_then(|t| {
                if Some(t.distance) == nearest_dist {
                    terrain.get(t.position_int()).ok().map(|&b| {
                        Interactable::Block(b, t.position_int(), BlockInteraction::Collect)
                    })
                } else {
                    None
                }
            })
        })
        .or_else(|| {
            mine_target.and_then(|t| {
                if Some(t.distance) == nearest_dist {
                    terrain.get(t.position_int()).ok().and_then(|&b| {
                        // Handling edge detection. sometimes the casting (in Target mod) returns a
                        // position which is actually empty, which we do not want labeled as an
                        // interactable. We are only returning the mineable air
                        // elements (e.g. minerals). The mineable weakrock are used
                        // in the terrain selected_pos, but is not an interactable.
                        if let Some(mine_tool) = b.mine_tool() && b.is_air() {
                            Some(Interactable::Block(
                                b,
                                t.position_int(),
                                BlockInteraction::Mine(mine_tool),
                            ))
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
        })
    {
        Some(interactable)
    } else {
        // If there are no directly targeted interactables select the closest one if any
        // are in range
        let ecs = client.state().ecs();
        let player_entity = client.entity();
        let positions = ecs.read_storage::<comp::Pos>();
        let player_pos = positions.get(player_entity)?.0;

        let scales = ecs.read_storage::<comp::Scale>();
        let colliders = ecs.read_storage::<comp::Collider>();
        let char_states = ecs.read_storage::<comp::CharacterState>();
        let is_mount = ecs.read_storage::<Is<Mount>>();
        let bodies = ecs.read_storage::<comp::Body>();
        let items = ecs.read_storage::<comp::Item>();
        let stats = ecs.read_storage::<comp::Stats>();

        let player_cylinder = Cylinder::from_components(
            player_pos,
            scales.get(player_entity).copied(),
            colliders.get(player_entity),
            char_states.get(player_entity),
        );

        let closest_interactable_entity = (
            &ecs.entities(),
            &positions,
            &bodies,
            scales.maybe(),
            colliders.maybe(),
            char_states.maybe(),
            !&is_mount,
            (stats.mask() | items.mask()).maybe(),
        )
            .join()
            .filter(|&(e, _, _, _, _, _, _, _)| e != player_entity) // skip the player's entity 
            .filter_map(|(e, p, b, s, c, cs, _, has_stats_or_item)| {
                // Note, if this becomes expensive to compute do it after the distance check!
                //
                // The entities that can be interacted with:
                // * Sitting at campfires (Body::is_campfire)
                // * Talking/trading with npcs (note hud code uses Alignment but I can't bring
                //   myself to write more code on that depends on having this on the client so
                //   we just check for presence of Stats component for now, it is okay to have
                //   some false positives here as long as it doesn't frequently prevent us from
                //   interacting with actual interactable entities that are closer by)
                // * Dropped items that can be picked up (Item component)
                let is_interactable = b.is_campfire() || has_stats_or_item.is_some();
                if !is_interactable {
                    return None;
                };

                let cylinder = Cylinder::from_components(p.0, s.copied(), c, cs);
                // Roughly filter out entities farther than interaction distance
                if player_cylinder.approx_in_range(cylinder, MAX_PICKUP_RANGE) {
                    Some((e, player_cylinder.min_distance(cylinder)))
                } else {
                    None
                }
            })
            .min_by_key(|(_, dist)| OrderedFloat(*dist));

        // Only search as far as closest interactable entity
        let search_dist = closest_interactable_entity.map_or(MAX_PICKUP_RANGE, |(_, dist)| dist);
        let player_chunk = player_pos.xy().map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        });
        let scene_terrain = scene.terrain();

        // Find closest interactable block
        // TODO: consider doing this one first?
        let closest_interactable_block_pos = Spiral2d::new()
            // TODO: this formula for the number to take was guessed
            // Note: assumes RECT_SIZE.x == RECT_SIZE.y
            .take(((search_dist / TerrainChunk::RECT_SIZE.x as f32).ceil() as usize * 2 + 1).pow(2))
            .flat_map(|offset| {
                let chunk_pos = player_chunk + offset;
                let chunk_voxel_pos =
                        Vec3::<i32>::from(chunk_pos * TerrainChunk::RECT_SIZE.map(|e| e as i32));
                scene_terrain.get(chunk_pos).map(|data| (data, chunk_voxel_pos))
            })
            // TODO: maybe we could make this more efficient by putting the
            // interactables is some sort of spatial structure
            .flat_map(|(chunk_data, chunk_pos)| {
                chunk_data
                    .blocks_of_interest
                    .interactables
                    .iter()
                    .map(move |(block_offset, interaction)| (chunk_pos + block_offset, interaction))
            })
            .map(|(block_pos, interaction)| (
                block_pos,
                block_pos.map(|e| e as f32 + 0.5)
                    .distance_squared(player_pos),
                interaction,
            ))
            .min_by_key(|(_, dist_sqr, _)| OrderedFloat(*dist_sqr))
            .map(|(block_pos, _, interaction)| (block_pos, interaction));

        // Return the closest of the 2 closest
        closest_interactable_block_pos
            .filter(|(block_pos, _)| {
                player_cylinder.min_distance(Cube {
                    min: block_pos.as_(),
                    side_length: 1.0,
                }) < search_dist
            })
            .and_then(|(block_pos, interaction)| {
                Interactable::from_block_pos(&terrain, block_pos, *interaction)
            })
            .or_else(|| closest_interactable_entity.map(|(e, _)| Interactable::Entity(e)))
    }
}
