use crate::trade::SiteId;
use common_i18n::Content;
use serde::{Deserialize, Serialize};
use std::{any::Any, hash::Hash};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Marker {
    id: Option<u64>, /* Arbitrary value that identifies the thing the marker is pointing to.
                      * Usually a hash. */
    pub site: Option<SiteId>,
    pub kind: MarkerKind,
    pub wpos: Vec2<f32>,
    pub label: Option<Content>,
}

impl Marker {
    pub fn at(wpos: Vec2<f32>) -> Self {
        Self {
            id: None,
            site: None,
            kind: MarkerKind::Unknown,
            wpos,
            label: None,
        }
    }

    /// Generate a deterministic marker ID from the given identifying data.
    ///
    /// IDs are used to correlate marker identities by frontends (i.e: to
    /// deduplicate them). They are not, in themselves, meaningful.
    pub fn with_id<T: Any + Hash>(mut self, id: T) -> Self {
        self.id = Some(ahash::RandomState::with_seed(0).hash_one((id.type_id(), id)));
        self
    }

    pub fn with_kind(mut self, kind: MarkerKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_label(mut self, label: impl Into<Option<Content>>) -> Self {
        self.label = label.into();
        self
    }

    pub fn with_site_id(mut self, site: impl Into<Option<SiteId>>) -> Self {
        self.site = site.into();
        self
    }

    pub fn is_same(&self, other: &Self) -> bool { self.id.is_some_and(|id| Some(id) == other.id) }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[repr(u8)]
pub enum MarkerKind {
    Town,
    Castle,
    Cave,
    Tree,
    Gnarling,
    GliderCourse,
    ChapelSite,
    Terracotta,
    Bridge,
    Adlet,
    Haniwa,
    DwarvenMine,
    Cultist,
    Sahagin,
    VampireCastle,
    Myrmidon,
    Character,
    Unknown,
}
