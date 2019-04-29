pub mod meta;
mod lod;
use std::sync::Arc;

use crate::{
    regionmanager::meta::RegionId,
    job::JobManager,
    lodstore::LodLayer,
    lodstore::Layer,
};
use lod::terrain::Terrain;

#[derive(Debug, Clone)]
pub struct Region {
    id: RegionId,
    jobmanager: Arc<JobManager>,

    pub block: LodLayer<Terrain>,
    temp: LodLayer<Terrain>,
    light: LodLayer<Terrain>,
    evil: LodLayer<Terrain>,
    civ: LodLayer<Terrain>,
}

impl Region {
    pub fn new(id: RegionId, jobmanager: Arc<JobManager>) -> Self {
        Self {
            id,
            jobmanager,
            block: Terrain::new(),
            temp: Terrain::new(),
            light: Terrain::new(),
            evil: Terrain::new(),
            civ: Terrain::new(),
        }
    }
}

/*

pub type aaa = LodLayer<e::Terain>;

fn example() {
    let own = e::Terain::new();
    let t8 = own.get(Vec3::new(1,1,1));
    //let tn = own.get2((1,2,3))
}

*/

#[cfg(test)]
mod tests {
    use crate::{
        regionmanager::meta::RegionId,
        job::JobManager,
        lodstore::LodLayer,
        lodstore::Layer,
        region::lod::terrain::Terrain,
        region::Region,
    };
    use vek::*;
    use std::sync::Arc;
    use std::{thread, time};
/*
    #[test]
    fn createRegion() {
        let mut r = Region::new((0,0), Arc::new(JobManager::new()));
        r.block.make_at_least(Vec3::new(0,0,0), Vec3::new(65535,65535,65535), 9);
    }*/

    #[test]
    fn createRegionToBlock() {
        // one region fully blown needs around 80 GB, 1/8 of a region needs 10GB for full block level
        let mut r = Region::new((0,0), Arc::new(JobManager::new()));
        r.block.make_at_least(Vec3::new(0,0,0), Vec3::new(65535/2,65535/2,65535/2), 0);
        r.block.make_at_least(Vec3::new(0,0,0), Vec3::new(65535/2,65535/2,65535/2), 0);

        thread::sleep(time::Duration::from_secs(100));

    }
/*
    #[test]
    fn createRegionToSubBlock() {
        let mut r = Region::new((0,0), Arc::new(JobManager::new()));
        r.block.make_at_least(Vec3::new(0,0,0), Vec3::new(65535,65535,65535), -4);
    }*/
}