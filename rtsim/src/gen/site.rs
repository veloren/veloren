use crate::data::Site;
use common::store::Id;
use world::{
    site::Site as WorldSite,
    World,
    IndexRef,
};

impl Site {
    pub fn generate(world_site: Id<WorldSite>, world: &World, index: IndexRef) -> Self {
        // match &world_site.kind {
        //     SiteKind::Refactor(site2) => {
        //         let site = Site::generate(world_site_id, world, index);
        //         println!("Registering rtsim site at {:?}...", site.wpos);
        //         this.sites.create(site);
        //     }
        //     _ => {},
        // }

        Self {
            wpos: index.sites.get(world_site).get_origin(),
            world_site: Some(world_site),
        }
    }
}
