use common::assets;
use serde::Deserialize;
use std::path::PathBuf;

// NOTE: we are free to split the manifest asset format and the format processed
// for display into separate structs but they happen to be identical for now

// See best practices for attribution: https://wiki.creativecommons.org/wiki/Best_practices_for_attribution

#[derive(Clone, Deserialize)]
pub struct Art {
    /// Name of the art.
    pub name: String,
    /// Link if the asset is from or derived from an external source that can be
    /// linked.
    #[serde(default)]
    pub source_link: String,
    /// List of authors for the credited art, field can be omitted if there are
    /// no authors to list.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Relative path to the asset from the top level asset folder.
    /// Used so we can keep track of the actual files, but not currently used in
    /// the credits screen to display anything.
    pub asset_path: PathBuf,
    /// License that the art is under, can be omitted, if not present assumed to
    /// be GPL3.
    #[serde(default)]
    pub license: String,
    /// Link to the license if one is available.
    #[serde(default)]
    pub license_link: String,
    /// Notes on any modifications that were made if the original work was
    /// modified by us.
    #[serde(default)]
    pub modifications: String,
    /// Any additional attribution notes that may be desired and/or required by
    /// the respective license that can't be conveyed or would be awkward to
    /// convey with the other provided fields.
    #[serde(default)]
    pub notes: String,
}

#[derive(Clone, Deserialize)]
pub struct Contributor {
    pub name: String,
    /// Short note or description of the contributions
    /// Optional, can be left empty/omitted
    #[serde(default)]
    pub contributions: String,
}

/// Credits manifest processed into format for display in the UI
#[derive(Clone, Deserialize)]
pub struct Credits {
    pub music: Vec<Art>,
    pub fonts: Vec<Art>,
    pub other_art: Vec<Art>,
    pub contributors: Vec<Contributor>,
    // TODO: include credits for dependencies where the license requires attribution?
}

impl assets::Asset for Credits {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_art_asset_paths_exists() {
        use assets::AssetExt;
        let credits = Credits::load_expect_cloned("common.credits");

        credits
            .music
            .into_iter()
            .chain(credits.fonts)
            .chain(credits.other_art)
            .for_each(|art| {
                assert!(
                    assets::ASSETS_PATH.join(&art.asset_path).exists(),
                    "assets/{} does not exist!",
                    art.asset_path.display(),
                );
            });
    }
}
