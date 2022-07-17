use common::depot::{Depot, Id};
use hashbrown::{hash_map, HashMap};
use vek::*;

/// NOTE: Please don't add `Deserialize` without checking to make sure we
/// can guarantee the invariant that every entry in `area_names` points to a
/// valid id in `areas`.
#[derive(Default)]
pub struct BuildAreas {
    areas: Depot<Aabb<i32>>,
    area_names: HashMap<String, Id<Aabb<i32>>>,
}

pub enum BuildAreaError {
    /// This build area name is reserved by the system.
    Reserved,
    /// The build area name was not found.
    NotFound,
}

/// Build area names that can only be inserted, not removed.
const RESERVED_BUILD_AREA_NAMES: &[&str] = &["world"];

impl BuildAreas {
    pub fn areas(&self) -> &Depot<Aabb<i32>> { &self.areas }

    pub fn area_names(&self) -> &HashMap<String, Id<Aabb<i32>>> { &self.area_names }

    /// If the area_name is already in the map, returns Err(area_name).
    pub fn insert(&mut self, area_name: String, area: Aabb<i32>) -> Result<Id<Aabb<i32>>, String> {
        let area_name_entry = match self.area_names.entry(area_name) {
            hash_map::Entry::Occupied(o) => return Err(o.replace_key()),
            hash_map::Entry::Vacant(v) => v,
        };
        let bb_id = self.areas.insert(area.made_valid());
        area_name_entry.insert(bb_id);
        Ok(bb_id)
    }

    pub fn remove(&mut self, area_name: &str) -> Result<Aabb<i32>, BuildAreaError> {
        if RESERVED_BUILD_AREA_NAMES.contains(&area_name) {
            return Err(BuildAreaError::Reserved);
        }
        let bb_id = self
            .area_names
            .remove(area_name)
            .ok_or(BuildAreaError::NotFound)?;
        let area = self.areas.remove(bb_id).expect(
            "Entries in `areas` are added before entries in `area_names` in `insert`, and that is \
             the only exposed way to add elements to `area_names`.",
        );
        Ok(area)
    }
}
