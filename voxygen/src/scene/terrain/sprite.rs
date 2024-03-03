use std::ops::Range;

use common::{
    assets,
    terrain::{sprite, Block, SpriteKind},
};
use hashbrown::HashMap;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
/// Configuration data for an individual sprite model.
pub struct SpriteModelConfig<Model> {
    /// Data for the .vox model associated with this sprite.
    pub model: Model,
    /// Sprite model center (as an offset from 0 in the .vox file).
    pub offset: (f32, f32, f32),
    /// LOD axes (how LOD gets applied along each axis, when we switch
    /// to an LOD model).
    pub lod_axes: (f32, f32, f32),
}

#[derive(Deserialize, Debug)]
/// Configuration data for a group of sprites (currently associated with a
/// particular SpriteKind).
pub struct SpriteConfig<Model> {
    /// All possible model variations for this sprite.
    // NOTE: Could make constant per sprite type, but eliminating this indirection and
    // allocation is probably not that important considering how sprites are used.
    pub variations: Vec<SpriteModelConfig<Model>>,
    /// The extent to which the sprite sways in the window.
    ///
    /// 0.0 is normal.
    pub wind_sway: f32,
}

// TODO: reduce llvm IR lines from this
/// Configuration data for all sprite models.
///
/// NOTE: Model is an asset path to the appropriate sprite .vox model.
#[derive(Deserialize)]
pub struct SpriteSpec(HashMap<(SpriteKind, SpriteAttributeFilters), Option<SpriteConfig<String>>>);

macro_rules! impl_sprite_attribute_filter {
    (
        $(#[$meta:meta])*
        $vis:vis struct $n:ident {
            $($attr:ident $field_name:ident = |$filter_arg:ident: $filter_ty:ty, $value_arg:ident| $filter:block),+
        }
    ) => {
        $(#[$meta])*
        $vis struct $n {
            $(
                pub $field_name: Option<$filter_ty>,
            )+
        }

        impl $n {
            fn sprite_attribute_score(&self, block: &Block) -> Option<usize> {
                if $(
                    self.$field_name.as_ref().map_or(true, |$filter_arg| {
                        block
                            .get_attr::<sprite::$attr>()
                            .map_or(false, |$value_arg| $filter)
                    })
                )&&+ {
                    Some(
                        [$(self.$field_name.is_some()),+]
                        .into_iter()
                        .filter(|o| *o)
                        .count(),
                    )
                } else {
                    None
                }
            }

            #[cfg(test)]
            fn is_valid_for_category(&self, category: sprite::Category) -> Result<(), &'static str> {
                $(if self.$field_name.is_some() && !category.has_attr::<sprite::$attr>() {
                    return Err(::std::any::type_name::<sprite::$attr>());
                })*
                Ok(())
            }
        }
    };
}

impl_sprite_attribute_filter!(
    #[derive(Debug, Clone, Deserialize, Default, PartialEq, Eq, Hash)]
    #[serde(default)]
    pub struct SpriteAttributeFilters {
        Growth growth_stage = |filter: Range<u8>, growth| { filter.contains(&growth.0) },
        LightEnabled light_enabled = |filter: bool, light_enabled| { *filter == light_enabled.0 }
    }
);

impl assets::Asset for SpriteSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl SpriteSpec {
    pub fn get(
        &self,
        kind: SpriteKind,
    ) -> impl Iterator<Item = (&SpriteConfig<String>, &SpriteAttributeFilters)> + '_ {
        self.0
            .iter()
            .filter_map(move |((sprite_kind, filters), v)| {
                (*sprite_kind == kind).then_some((v.as_ref()?, filters))
            })
    }

    pub fn get_for_block(&self, block: &Block) -> Option<(usize, &SpriteConfig<String>)> {
        let sprite = block.get_sprite()?;

        self.get(sprite)
            .enumerate()
            .filter_map(|(cfg_i, (cfg, filter))| {
                Some((cfg_i, cfg, filter.sprite_attribute_score(block)?))
            })
            .max_by_key(|(_, _, score)| *score)
            .map(|(cfg_i, cfg, _)| (cfg_i, cfg))
    }
}

#[cfg(test)]
mod test {
    use common_assets::AssetExt;

    use super::SpriteSpec;

    #[test]
    fn test_sprite_spec_valid() {
        let spec = SpriteSpec::load_expect("voxygen.voxel.sprite_manifest").read();

        for (sprite, filter) in spec.0.keys() {
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
    }
}
