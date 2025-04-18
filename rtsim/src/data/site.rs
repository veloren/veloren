use crate::data::{ReportId, Reports};
pub use common::rtsim::SiteId;
use common::{
    rtsim::{FactionId, NpcId},
    store::Id,
};
use hashbrown::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::ops::{Deref, DerefMut};
use vek::*;
use world::site::Site as WorldSite;

#[derive(Clone, Serialize, Deserialize)]
pub struct Site {
    pub seed: u32,
    pub wpos: Vec2<i32>,
    pub faction: Option<FactionId>,

    /// The [`Report`]s that the site tracks (you can imagine them being on a
    /// noticeboard or something).
    pub known_reports: HashSet<ReportId>,

    /// How many chunks this site is loaded in.
    #[serde(skip)]
    pub count_loaded_chunks: usize,

    /// The site generated during initial worldgen that this site corresponds
    /// to.
    ///
    /// Eventually, rtsim should replace initial worldgen's site system and this
    /// will not be necessary.
    ///
    /// When setting up rtsim state, we try to 'link' these two definitions of a
    /// site: but if initial worldgen has changed, this might not be
    /// possible. We try to delete sites that no longer exist during setup, but
    /// this is an inherent fallible process. If linking fails, we try to
    /// delete the site in rtsim2 in order to avoid an 'orphaned' site.
    /// (TODO: create new sites for new initial worldgen sites that come into
    /// being too).
    #[serde(skip_serializing, skip_deserializing)]
    pub world_site: Option<Id<WorldSite>>,

    // Note: there's currently no guarantee that site populations are non-intersecting
    #[serde(skip_serializing, skip_deserializing)]
    pub population: HashSet<NpcId>,

    /// A list of the nearby sites where each elements is both further and
    /// larger (currently based on number of plots) than the next.
    /// Effectively, this is a list of nearby sites that might be deemed
    /// 'important' to the current one
    #[serde(skip_serializing, skip_deserializing)]
    pub nearby_sites_by_size: Vec<SiteId>,
}

impl Site {
    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }

    pub fn cleanup(&mut self, reports: &Reports) {
        // Clear reports that have been forgotten
        self.known_reports
            .retain(|report| reports.contains_key(*report));
        // TODO: Limit number of reports
    }

    pub fn is_loaded(&self) -> bool { self.count_loaded_chunks > 0 }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Sites {
    pub sites: HopSlotMap<SiteId, Site>,

    #[serde(skip_serializing, skip_deserializing)]
    pub world_site_map: HashMap<Id<WorldSite>, SiteId>,
}

impl Sites {
    pub fn create(&mut self, site: Site) -> SiteId {
        let world_site = site.world_site;
        let key = self.sites.insert(site);
        if let Some(world_site) = world_site {
            self.world_site_map.insert(world_site, key);
        }
        key
    }
}

impl Deref for Sites {
    type Target = HopSlotMap<SiteId, Site>;

    fn deref(&self) -> &Self::Target { &self.sites }
}

impl DerefMut for Sites {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.sites }
}
