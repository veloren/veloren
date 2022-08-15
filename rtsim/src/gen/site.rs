use crate::data::{Site, FactionId};
use common::store::Id;
use vek::*;
use world::{
    site::Site as WorldSite,
    World,
    IndexRef,
};

impl Site {
    pub fn generate(world_site: Id<WorldSite>, world: &World, index: IndexRef, nearby_factions: &[(Vec2<i32>, FactionId)]) -> Self {
        let wpos = index.sites.get(world_site).get_origin();

        Self {
            wpos,
            world_site: Some(world_site),
            faction: nearby_factions
                .iter()
                .min_by_key(|(faction_wpos, _)| faction_wpos.distance_squared(wpos))
                .map(|(_, faction)| *faction),
        }
    }
}
