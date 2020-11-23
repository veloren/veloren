use crate::sim::WorldSim;
use common::path::Path;
//use hashbrown::hash_map::DefaultHashBuilder;
use vek::*;

#[allow(dead_code)]
pub struct SearchCfg {
    // 0.0 = no discount, 1.0 = free travel
    path_discount: f32,
    // Cost per metre altitude change per metre horizontal
    // 0.0 = no cost, 1.0 = same cost vertical as horizontal
    gradient_aversion: f32,
}

#[allow(dead_code)]
pub struct Searcher<'a> {
    land: &'a WorldSim,
    pub cfg: SearchCfg,
}

#[allow(dead_code)]
impl<'a> Searcher<'a> {
    /// Attempt to find a path between two chunks on the map.
    pub fn search(self, _a: Vec2<i32>, _b: Vec2<i32>) -> Option<Path<i32>> {
        // TODO: implement this function

        //let heuristic = |pos: &Vec2<i32>| (pos - b).map(|e| e as f32).magnitude();
        // Astar::new(
        //     100_000,
        //     a,
        //     heuristc,
        //     DefaultHashBuilder::default(),
        // );

        None
    }
}
