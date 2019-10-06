use serde::Serialize;
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};

#[derive(Serialize)]
pub struct BlockManifest {
    pub id: String,
    pub block_type: String,
    pub asset_dir: String,
    pub map: BTreeMap<u8, String>,
    pub sfx_dir: String,
    pub hash_val: u64,
}

impl Hash for BlockManifest {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash_val.hash(state);
    }
}

pub fn calc_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
