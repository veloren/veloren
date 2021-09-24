use common::assets;
use serde::Deserialize;

// NOTE: we are free to split the manifest asset format and the format processed
// for display into separate structs but they happen to be identical for now

// TODO: add serde attribs to certain fields

#[derive(Clone, Deserialize)]
pub struct Art {
    pub name: String,
    // Include asset path as a field?
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: String,
    // Include optional license file path and/or web link?
}

#[derive(Clone, Deserialize)]
pub struct Contributor {
    pub name: String,
    /// Short note or description of the contributions
    /// Optional, can be left empty/ommitted
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
