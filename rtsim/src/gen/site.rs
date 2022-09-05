use crate::data::{FactionId, Site};
use common::store::Id;
use vek::*;
use world::{
    site::{Site as WorldSite, SiteKind},
    IndexRef, World,
};

impl Site {
    pub fn generate(
        world_site_id: Id<WorldSite>,
        world: &World,
        index: IndexRef,
        nearby_factions: &[(Vec2<i32>, FactionId)],
    ) -> Self {
        let world_site = index.sites.get(world_site_id);
        let wpos = world_site.get_origin();

        Self {
            wpos,
            world_site: Some(world_site_id),
            faction: if matches!(
                &world_site.kind,
                SiteKind::Refactor(_) | SiteKind::CliffTown(_) | SiteKind::DesertCity(_)
            ) {
                nearby_factions
                    .iter()
                    .min_by_key(|(faction_wpos, _)| faction_wpos.distance_squared(wpos))
                    .map(|(_, faction)| *faction)
            } else {
                None
            },
        }
    }
}
