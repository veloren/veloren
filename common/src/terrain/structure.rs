use super::{BlockKind, StructureSprite};
use crate::{
    assets::{self, AssetCache, AssetExt, AssetHandle, BoxedError, DotVox, Ron, SharedString},
    make_case_elim,
    vol::{BaseVol, ReadVol, SizedVol, WriteVol},
    volumes::dyna::{Dyna, DynaError},
};
use common_i18n::Content;
use dot_vox::DotVoxData;
use hashbrown::HashMap;
use serde::Deserialize;
use std::{num::NonZeroU8, sync::Arc};
use vek::*;

use crate::terrain::SpriteCfg;

make_case_elim!(
    structure_block,
    #[derive(Clone, PartialEq, Debug, Deserialize)]
    #[repr(u8)]
    pub enum StructureBlock {
        None = 0,
        Grass = 1,
        TemperateLeaves = 2,
        PineLeaves = 3,
        Acacia = 4,
        Mangrove = 5,
        PalmLeavesInner = 6,
        PalmLeavesOuter = 7,
        Water = 8,
        GreenSludge = 9,
        Fruit = 10,
        Coconut = 11,
        MaybeChest = 12,
        Hollow = 13,
        Liana = 14,
        Normal(color: Rgb<u8>) = 15,
        Log = 16,
        Filled(kind: BlockKind, color: Rgb<u8>) = 17,
        Sprite(sprite: StructureSprite) = 18,
        Chestnut = 19,
        Baobab = 20,
        BirchWood = 21,
        FrostpineLeaves = 22,
        // NOTE: When adding set it equal to `23`.
        // = 23,
        EntitySpawner(entitykind: String, spawn_chance: f32) = 24,
        // TODO: It seems like only Keyhole and KeyholeBars are used out of the keyhole variants?
        Keyhole(consumes: String) = 25,
        BoneKeyhole(consumes: String) = 26,
        GlassKeyhole(consumes: String) = 27,
        Sign(content: Content, ori: u8) = 28,
        KeyholeBars(consumes: String) = 29,
        HaniwaKeyhole(consumes: String) = 30,
        TerracottaKeyhole(consumes: String) = 31,
        SahaginKeyhole(consumes: String) = 32,
        VampireKeyhole(consumes: String) = 33,
        MyrmidonKeyhole(consumes: String) = 34,
        MinotaurKeyhole(consumes: String) = 35,
        MapleLeaves = 36,
        CherryLeaves = 37,
        AutumnLeaves = 38,
        RedwoodWood = 39,
        SpriteWithCfg(kind: StructureSprite, sprite_cfg: SpriteCfg) = 40,
        Choice(block_table: Vec<(f32, StructureBlock)>) = 41,
    }
);

// We can't derive this because of the `make_case_elim` macro.
#[expect(clippy::derivable_impls)]
impl Default for StructureBlock {
    fn default() -> Self { StructureBlock::None }
}

#[derive(Debug)]
pub enum StructureError {
    OutOfBounds,
}

#[derive(Clone, Debug)]
pub struct Structure {
    center: Vec3<i32>,
    base: Arc<BaseStructure<StructureBlock>>,
    custom_indices: [Option<StructureBlock>; 256],
}

#[derive(Debug)]
pub(crate) struct BaseStructure<B> {
    pub(crate) vol: Dyna<Option<NonZeroU8>, ()>,
    pub(crate) palette: [B; 256],
}

pub struct StructuresGroup(Vec<Structure>);

impl std::ops::Deref for StructuresGroup {
    type Target = [Structure];

    fn deref(&self) -> &[Structure] { &self.0 }
}

impl assets::Asset for StructuresGroup {
    fn load(cache: &AssetCache, specifier: &SharedString) -> Result<Self, BoxedError> {
        let specs = cache.load::<Ron<Vec<StructureSpec>>>(specifier)?.read();

        Ok(StructuresGroup(
            specs
                .0
                .iter()
                .map(|sp| {
                    let base = cache
                        .load::<Arc<BaseStructure<StructureBlock>>>(&sp.specifier)?
                        .cloned();
                    Ok(Structure {
                        center: Vec3::from(sp.center),
                        base,
                        custom_indices: {
                            let mut indices = std::array::from_fn(|_| None);
                            for (&idx, custom) in default_custom_indices()
                                .iter()
                                .chain(sp.custom_indices.iter())
                            {
                                indices[idx as usize] = Some(custom.clone());
                            }
                            indices
                        },
                    })
                })
                .collect::<Result<_, BoxedError>>()?,
        ))
    }
}

const STRUCTURE_MANIFESTS_DIR: &str = "world.manifests";
impl Structure {
    pub fn load_group(specifier: &str) -> AssetHandle<StructuresGroup> {
        StructuresGroup::load_expect(&format!("{STRUCTURE_MANIFESTS_DIR}.{specifier}"))
    }

    #[must_use]
    pub fn with_center(mut self, center: Vec3<i32>) -> Self {
        self.center = center;
        self
    }

    pub fn get_bounds(&self) -> Aabb<i32> {
        Aabb {
            min: -self.center,
            max: self.base.vol.size().map(|e| e as i32) - self.center,
        }
    }
}

impl BaseVol for Structure {
    type Error = StructureError;
    type Vox = StructureBlock;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, StructureError> {
        match self.base.vol.get(pos + self.center) {
            Ok(None) => Ok(&StructureBlock::None),
            Ok(Some(index)) => match &self.custom_indices[index.get() as usize] {
                Some(sb) => Ok(sb),
                None => Ok(&self.base.palette[index.get() as usize]),
            },
            Err(DynaError::OutOfBounds) => Err(StructureError::OutOfBounds),
        }
    }
}

pub(crate) fn load_base_structure<B: Default>(
    dot_vox_data: &DotVoxData,
    mut to_block: impl FnMut(Rgb<u8>) -> B,
) -> BaseStructure<B> {
    let mut palette = std::array::from_fn(|_| B::default());
    if let Some(model) = dot_vox_data.models.first() {
        for (i, col) in dot_vox_data
            .palette
            .iter()
            .map(|col| Rgb::new(col.r, col.g, col.b))
            .enumerate()
        {
            palette[(i + 1).min(255)] = to_block(col);
        }

        let mut vol = Dyna::filled(
            Vec3::new(model.size.x, model.size.y, model.size.z),
            None,
            (),
        );

        for voxel in &model.voxels {
            let _ = vol.set(
                Vec3::new(voxel.x, voxel.y, voxel.z).map(i32::from),
                Some(NonZeroU8::new(voxel.i + 1).unwrap()),
            );
        }

        BaseStructure { vol, palette }
    } else {
        BaseStructure {
            vol: Dyna::filled(Vec3::zero(), None, ()),
            palette,
        }
    }
}

impl assets::Asset for BaseStructure<StructureBlock> {
    fn load(cache: &AssetCache, specifier: &SharedString) -> Result<Self, BoxedError> {
        let dot_vox_data = cache.load::<DotVox>(specifier)?.read();
        let dot_vox_data = &dot_vox_data.0;

        Ok(load_base_structure(dot_vox_data, |col| {
            StructureBlock::Filled(BlockKind::Misc, col)
        }))
    }
}

#[derive(Clone, Deserialize)]
struct StructureSpec {
    specifier: String,
    center: [i32; 3],
    #[serde(default)]
    custom_indices: HashMap<u8, StructureBlock>,
}

fn default_custom_indices() -> HashMap<u8, StructureBlock> {
    let blocks: [_; 16] = [
        /* 1 */ Some(StructureBlock::TemperateLeaves),
        /* 2 */ Some(StructureBlock::PineLeaves),
        /* 3 */ None,
        /* 4 */ Some(StructureBlock::Water),
        /* 5 */ Some(StructureBlock::Acacia),
        /* 6 */ Some(StructureBlock::Mangrove),
        /* 7 */ Some(StructureBlock::GreenSludge),
        /* 8 */ Some(StructureBlock::Fruit),
        /* 9 */ Some(StructureBlock::Grass),
        /* 10 */ Some(StructureBlock::Liana),
        /* 11 */ Some(StructureBlock::MaybeChest),
        /* 12 */ Some(StructureBlock::Coconut),
        /* 13 */ None,
        /* 14 */ Some(StructureBlock::PalmLeavesOuter),
        /* 15 */ Some(StructureBlock::PalmLeavesInner),
        /* 16 */ Some(StructureBlock::Hollow),
    ];

    blocks
        .iter()
        .enumerate()
        .filter_map(|(i, sb)| sb.as_ref().map(|sb| (i as u8 + 1, sb.clone())))
        .collect()
}

#[test]
fn test_load_structures() {
    use crate::assets;
    let errors = assets::load_rec_dir::<Ron<Vec<StructureSpec>>>("world.manifests.site_structures")
        .expect("This should be able to load")
        .read()
        .ids()
        .chain(
            assets::load_rec_dir::<Ron<Vec<StructureSpec>>>("world.manifests.spots")
                .expect("This should be able to load")
                .read()
                .ids(),
        )
        .chain(
            assets::load_rec_dir::<Ron<Vec<StructureSpec>>>("world.manifests.spots_general")
                .expect("This should be able to load")
                .read()
                .ids(),
        )
        .chain(
            assets::load_rec_dir::<Ron<Vec<StructureSpec>>>("world.manifests.trees")
                .expect("This should be able to load")
                .read()
                .ids(),
        )
        .chain(
            assets::load_rec_dir::<Ron<Vec<StructureSpec>>>("world.manifests.shrubs")
                .expect("This should be able to load")
                .read()
                .ids(),
        )
        .filter_map(|id| {
            Ron::<Vec<StructureSpec>>::load(id)
                .err()
                .map(|err| (id, err))
        })
        .fold(None::<String>, |mut acc, (id, err)| {
            use std::fmt::Write;

            let s = acc.get_or_insert_default();
            _ = writeln!(s, "{id}: {err}");

            acc
        });

    if let Some(errors) = errors {
        panic!("Failed to load the following structures:\n{errors}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        assets,
        generation::tests::validate_entity_config,
        lottery::{LootSpec, tests::validate_loot_spec},
        terrain::Block,
    };
    use std::ops::Deref;

    pub fn validate_sprite_and_cfg(sprite: StructureSprite, sprite_cfg: &SpriteCfg) {
        let SpriteCfg {
            // TODO: write validation for UnlockKind?
            unlock: _,
            // TODO: requires access to i18n for validation
            content: _,
            loot_table,
        } = sprite_cfg;

        let sprite = sprite
            .get_block(Block::air)
            .get_sprite()
            .expect("This should have the sprite");

        if let Some(loot_table) = loot_table.clone() {
            if !sprite.is_defined_as_container() {
                panic!(
                    r"
Manifest contains a structure block with custom loot table for a sprite
that isn't defined as container, you probably don't want that.

If you want, add this sprite to `is_defined_as_container` list.
Sprite in question: {sprite:?}
"
                );
            }

            validate_loot_spec(&LootSpec::LootTable(loot_table))
        }
    }

    pub fn validate_choice_block(_chosen_block: &[(f32, StructureBlock)]) {
        // TODO
    }

    fn validate_structure_block(sb: &StructureBlock, id: &str) {
        match sb {
            StructureBlock::SpriteWithCfg(sprite, sprite_cfg) => {
                std::panic::catch_unwind(|| validate_sprite_and_cfg(*sprite, sprite_cfg))
                    .unwrap_or_else(|_| {
                        panic!("failed to load structure_block in: {id}\n{sb:?}");
                    })
            },
            StructureBlock::EntitySpawner(entity_kind, _spawn_chance) => {
                let config = &entity_kind;
                std::panic::catch_unwind(|| validate_entity_config(config)).unwrap_or_else(|_| {
                    panic!("failed to load structure_block in: {id}\n{sb:?}");
                })
            },
            StructureBlock::Choice(choice_block) => {
                std::panic::catch_unwind(|| validate_choice_block(choice_block)).unwrap_or_else(
                    |_| {
                        panic!("failed to load structure_block in: {id}\n{sb:?}");
                    },
                )
            },
            // These probably can't fail
            StructureBlock::None
            | StructureBlock::Grass
            | StructureBlock::TemperateLeaves
            | StructureBlock::PineLeaves
            | StructureBlock::Acacia
            | StructureBlock::Mangrove
            | StructureBlock::PalmLeavesInner
            | StructureBlock::PalmLeavesOuter
            | StructureBlock::Water
            | StructureBlock::GreenSludge
            | StructureBlock::Fruit
            | StructureBlock::Coconut
            | StructureBlock::MaybeChest
            | StructureBlock::Hollow
            | StructureBlock::Liana
            | StructureBlock::Normal { .. }
            | StructureBlock::Log
            | StructureBlock::Filled { .. }
            | StructureBlock::Sprite { .. }
            | StructureBlock::Chestnut
            | StructureBlock::Baobab
            | StructureBlock::BirchWood
            | StructureBlock::FrostpineLeaves
            | StructureBlock::MapleLeaves
            | StructureBlock::CherryLeaves
            | StructureBlock::RedwoodWood
            | StructureBlock::AutumnLeaves => {},
            // TODO: ideally this should be tested as well
            StructureBlock::Keyhole { .. }
            | StructureBlock::MyrmidonKeyhole { .. }
            | StructureBlock::MinotaurKeyhole { .. }
            | StructureBlock::SahaginKeyhole { .. }
            | StructureBlock::VampireKeyhole { .. }
            | StructureBlock::BoneKeyhole { .. }
            | StructureBlock::GlassKeyhole { .. }
            | StructureBlock::KeyholeBars { .. }
            | StructureBlock::HaniwaKeyhole { .. }
            | StructureBlock::TerracottaKeyhole { .. } => {},
            // TODO: requires access to i18n for validation
            StructureBlock::Sign { .. } => {},
        }
    }

    #[test]
    fn test_structure_manifests() {
        let specs =
            assets::load_rec_dir::<Ron<Vec<StructureSpec>>>(STRUCTURE_MANIFESTS_DIR).unwrap();
        for id in specs.read().ids() {
            // Ignore manifest file
            if id != "world.manifests.spots" {
                let group: Vec<StructureSpec> = Ron::load(id)
                    .unwrap_or_else(|e| {
                        panic!("failed to load: {id}\n{e:?}");
                    })
                    .read()
                    .deref()
                    .clone()
                    .into_inner();
                for StructureSpec {
                    specifier,
                    center: _center,
                    custom_indices,
                } in group
                {
                    BaseStructure::<StructureBlock>::load(&specifier).unwrap_or_else(|e| {
                        panic!("failed to load specifier for: {id}\n{e:?}");
                    });

                    for sb in custom_indices.values() {
                        validate_structure_block(sb, id);
                    }
                }
            }
        }
    }
}
