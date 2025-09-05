use crate::{
    Fonts, LanguageMetadata,
    assets::{
        Asset, AssetCache, BoxedError, DirLoadable, FileAsset, Ron, SharedString, Source,
        source::DirEntry,
    },
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Localization metadata from manifest file
/// See `Language` for more info on each attributes
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub(crate) struct Manifest {
    pub(crate) fonts: Fonts,
    pub(crate) metadata: LanguageMetadata,
}

impl Asset for Manifest {
    fn load(cache: &AssetCache, specifier: &SharedString) -> Result<Self, BoxedError> {
        Ok(cache
            .load::<Ron<Self>>(specifier)
            .map(|v| v.read().clone().into_inner())?)
    }
}

impl DirLoadable for Manifest {
    fn select_ids(
        cache: &AssetCache,
        specifier: &SharedString,
    ) -> std::io::Result<Vec<SharedString>> {
        let mut specifiers = Vec::new();

        let source = cache.source();
        source.read_dir(specifier, &mut |entry| {
            if let DirEntry::Directory(spec) = entry {
                let manifest_spec = [spec, ".", "_manifest"].concat();

                if source.exists(DirEntry::File(&manifest_spec, "ron")) {
                    specifiers.push(manifest_spec.into());
                }
            }
        })?;

        Ok(specifiers)
    }
}

// Newtype wrapper representing fluent resource.
//
// NOTE:
// We store String, that later converted to FluentResource.
// We can't do it at load time, because we might want to do utf8 to ascii
// conversion and we know it only after we've loaded language manifest.
//
// Alternative solution is to make it hold Rc/Arc around FluentResource,
// implement methods that give us mutable control around resource entries,
// but doing it to eliminate Clone that happens N per programm life seems as
// overengineering.
//
// N is time of fluent files, so about 20 for English and the same for target
// localisation.
#[derive(Clone)]
pub(crate) struct Resource {
    pub(crate) src: String,
}

impl FileAsset for Resource {
    const EXTENSION: &'static str = "ftl";

    fn from_bytes(bytes: Cow<[u8]>) -> Result<Self, BoxedError> {
        Ok(Self {
            src: String::from_utf8(bytes.into())?,
        })
    }
}
