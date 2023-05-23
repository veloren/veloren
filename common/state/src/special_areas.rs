use common::depot::{Depot, Id};
use hashbrown::{hash_map, HashMap};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use vek::*;

#[derive(Default)]
pub struct AreasContainer<Kind>(Areas, PhantomData<Kind>);

#[derive(Default)]
pub struct BuildArea;

#[derive(Default)]
pub struct NoDurabilityArea;

/// NOTE: Please don't add `Deserialize` without checking to make sure we
/// can guarantee the invariant that every entry in `area_names` points to a
/// valid id in `areas`.
#[derive(Default)]
pub struct Areas {
    areas: Depot<Aabb<i32>>,
    area_names: HashMap<String, Id<Aabb<i32>>>,
}

pub enum SpecialAreaError {
    /// This build area name is reserved by the system.
    Reserved,
    /// The build area name was not found.
    NotFound,
}

/// Build area names that can only be inserted, not removed.
const RESERVED_BUILD_AREA_NAMES: &[&str] = &["world"];

impl Areas {
    pub fn areas(&self) -> &Depot<Aabb<i32>> { &self.areas }

    pub fn area_metas(&self) -> &HashMap<String, Id<Aabb<i32>>> { &self.area_names }

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

    pub fn remove(&mut self, area_name: &str) -> Result<Aabb<i32>, SpecialAreaError> {
        if RESERVED_BUILD_AREA_NAMES.contains(&area_name) {
            return Err(SpecialAreaError::Reserved);
        }
        let bb_id = self
            .area_names
            .remove(area_name)
            .ok_or(SpecialAreaError::NotFound)?;
        let area = self.areas.remove(bb_id).expect(
            "Entries in `areas` are added before entries in `area_names` in `insert`, and that is \
             the only exposed way to add elements to `area_names`.",
        );
        Ok(area)
    }
}

impl<Kind> Deref for AreasContainer<Kind>
where
    Kind: AreaKind,
{
    type Target = Areas;

    fn deref(&self) -> &Self::Target { &self.0 }
}

impl<Kind> DerefMut for AreasContainer<Kind>
where
    Kind: AreaKind,
{
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

pub trait AreaKind {
    fn display() -> &'static str;
}

impl AreaKind for BuildArea {
    fn display() -> &'static str { "build" }
}

impl AreaKind for NoDurabilityArea {
    fn display() -> &'static str { "durability free" }
}
