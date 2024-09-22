use std::ops::Range;

use super::SPRITE_LOD_LEVELS;
use common::{
    assets,
    terrain::{sprite, Block, SpriteKind},
};
use hashbrown::HashMap;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize, Debug)]
/// Configuration data for an individual sprite model.
pub(super) struct SpriteModelConfig {
    /// Data for the .vox model associated with this sprite.
    pub model: String,
    /// Sprite model center (as an offset from 0 in the .vox file).
    pub offset: (f32, f32, f32),
    /// LOD axes (how LOD gets applied along each axis, when we switch
    /// to an LOD model).
    pub lod_axes: (f32, f32, f32),
}

macro_rules! impl_sprite_attribute_filter {
    (
        $($attr:ident $field_name:ident = |$filter_arg:ident: $filter_ty:ty, $value_arg:ident| $filter:block),+ $(,)?
    ) => {
        // TODO: depending on what types of filters we end up with an enum may end up being more suitable.
        #[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq, Hash)]
        #[serde(default)]
        pub struct SpriteAttributeFilters {
            $(
                pub $field_name: Option<$filter_ty>,
            )+
        }

        impl SpriteAttributeFilters {
            fn matches_filter(&self, block: &Block) -> bool {
                $(
                    self.$field_name.as_ref().map_or(true, |$filter_arg| {
                        block
                            .get_attr::<sprite::$attr>()
                            .map_or(false, |$value_arg| $filter)
                    })
                )&&+
            }

            #[cfg(test)]
            fn is_valid_for_category(&self, category: sprite::Category) -> Result<(), &'static str> {
                $(if self.$field_name.is_some() && !category.has_attr::<sprite::$attr>() {
                    return Err(::std::any::type_name::<sprite::$attr>());
                })*
                Ok(())
            }

            fn no_filters(&self) -> bool {
                true $(&& self.$field_name.is_none())+
            }
        }
    };
}

impl_sprite_attribute_filter!(
    Growth growth_stage = |filter: Range<u8>, growth| { filter.contains(&growth.0) },
    LightEnabled light_enabled = |filter: bool, light_enabled| { *filter == light_enabled.0 },
);

/// Configuration data for a group of sprites (currently associated with a
/// particular SpriteKind).
#[derive(Deserialize, Debug)]
struct SpriteConfig {
    /// Filter for selecting what config to use based on sprite attributes.
    #[serde(default)]
    filter: SpriteAttributeFilters,
    /// All possible model variations for this sprite.
    // NOTE: Could make constant per sprite type, but eliminating this indirection and
    // allocation is probably not that important considering how sprites are used.
    #[serde(default)]
    variations: Vec<SpriteModelConfig>,
    /// The extent to which the sprite sways in the wind.
    ///
    /// 0.0 is normal.
    #[serde(default)]
    wind_sway: f32,
}

#[serde_with::serde_as]
#[derive(Deserialize)]
struct SpriteSpecRaw(
    #[serde_as(as = "serde_with::MapPreventDuplicates<_, _>")]
    HashMap<SpriteKind, Vec<SpriteConfig>>,
);

/// Configuration data for all sprite models.
///
/// NOTE: Model is an asset path to the appropriate sprite .vox model.
#[derive(Deserialize)]
#[serde(try_from = "SpriteSpecRaw")]
pub struct SpriteSpec(HashMap<SpriteKind, Vec<SpriteConfig>>);

/// Conversion of [`SpriteSpec`] from a hashmap failed because some sprite kinds
/// were missing.
struct SpritesMissing(Vec<SpriteKind>);

impl core::fmt::Display for SpritesMissing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "Missing entries in the sprite manifest for these sprites: {:?}",
            &self.0,
        )
    }
}

impl TryFrom<SpriteSpecRaw> for SpriteSpec {
    type Error = SpritesMissing;

    fn try_from(SpriteSpecRaw(map): SpriteSpecRaw) -> Result<Self, Self::Error> {
        let sprites_missing = SpriteKind::all()
            .iter()
            .copied()
            .filter(|kind| !map.contains_key(kind))
            .collect::<Vec<_>>();

        if sprites_missing.is_empty() {
            Ok(Self(map))
        } else {
            Err(SpritesMissing(sprites_missing))
        }
    }
}

impl assets::Asset for SpriteSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl SpriteSpec {
    pub fn map_to_data(
        &self,
        mut map_variation: impl FnMut(&SpriteModelConfig) -> [SpriteModelData; super::SPRITE_LOD_LEVELS],
    ) -> HashMap<SpriteKind, FilteredSpriteData> {
        let mut to_sprite_data = |config: &SpriteConfig| SpriteData {
            variations: config.variations.iter().map(&mut map_variation).collect(),
            wind_sway: config.wind_sway,
        };

        // Note, the returned datastructure can potentially be optimized further from a
        // HashMap, a phf could be used or if we can rely on the sprite kind
        // discriminants in each sprite category being packed fairly densely, we
        // could just have an offset per sprite catagory used to
        // convert a sprite kind into a flat index.
        self.0
            .iter()
            .map(|(kind, config)| {
                let filtered_data = match config.as_slice() {
                    [config] if config.filter.no_filters() => {
                        FilteredSpriteData::Unfiltered(to_sprite_data(config))
                    },
                    // Note, we have a test that checks if this is completely empty. That should be
                    // represented by an entry with no variantions instead of having an empty
                    // top-level list.
                    filtered_configs => {
                        let list = filtered_configs
                            .iter()
                            .map(|config| (config.filter.clone(), to_sprite_data(config)))
                            .collect::<Box<[_]>>();
                        FilteredSpriteData::Filtered(list)
                    },
                };
                (*kind, filtered_data)
            })
            .collect()
    }
}

pub(in crate::scene) struct SpriteModelData {
    // Sprite vert page ranges that need to be drawn
    pub vert_pages: core::ops::Range<u32>,
    // Scale
    pub scale: Vec3<f32>,
    // Offset
    pub offset: Vec3<f32>,
}

pub(in crate::scene) struct SpriteData {
    pub variations: Box<[[SpriteModelData; SPRITE_LOD_LEVELS]]>,
    /// See [`SpriteConfig::wind_sway`].
    pub wind_sway: f32,
}

pub(in crate::scene) enum FilteredSpriteData {
    // Special case when there is only one entry with the an empty filter since this is most
    // cases, and it will reduce indirection.
    Unfiltered(SpriteData),
    Filtered(Box<[(SpriteAttributeFilters, SpriteData)]>),
}

impl FilteredSpriteData {
    /// Gets sprite data for the filter that matches the provided block.
    ///
    /// This only returns `None` if no filters matches the provided block (i.e.
    /// the set of filters does not cover all values). A "missing"
    /// placeholder model can be displayed in this case in this case.
    pub fn for_block(&self, block: &Block) -> Option<&SpriteData> {
        match self {
            Self::Unfiltered(data) => Some(data),
            Self::Filtered(multiple) => multiple
                .iter()
                .find_map(|(filter, data)| filter.matches_filter(block).then_some(data)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::SpriteSpec;
    use common_assets::AssetExt;

    #[test]
    fn test_sprite_spec_valid() {
        let spec = SpriteSpec::load_expect("voxygen.voxel.sprite_manifest").read();

        // Test that filters are relevant for the particular sprite kind.
        for (sprite, filter) in spec.0.iter().flat_map(|(&sprite, configs)| {
            configs.iter().map(move |config| (sprite, &config.filter))
        }) {
            if let Err(invalid_attribute) = filter.is_valid_for_category(sprite.category()) {
                panic!(
                    "Sprite category '{:?}' does not have attribute '{}' (in sprite config for \
                     {:?})",
                    sprite.category(),
                    invalid_attribute,
                    sprite,
                );
            }
        }

        // Test that there is at least one entry per sprite. An empty variations list in
        // an entry is used to represent a sprite that doesn't have a model.
        let mut empty_config = Vec::new();
        for (kind, configs) in &spec.0 {
            if configs.is_empty() {
                empty_config.push(kind)
            }
        }
        assert!(
            empty_config.is_empty(),
            "Sprite config(s) with no entries, if these sprite(s) are intended to have no models \
             use an explicit entry with an empty `variations` list instead: {empty_config:?}",
        );
    }
}
