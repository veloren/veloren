use crate::path::Path;
use core::{
    cmp::Ordering::{self, Equal},
    f32, fmt,
    hash::{BuildHasher, Hash},
};
use hashbrown::{HashMap, HashSet};
use std::collections::BinaryHeap;

#[derive(Copy, Clone, Debug)]
pub struct PathEntry<S> {
    cost: f32,
    node: S,
}

impl<S: Eq> PartialEq for PathEntry<S> {
    fn eq(&self, other: &PathEntry<S>) -> bool { self.node.eq(&other.node) }
}

impl<S: Eq> Eq for PathEntry<S> {}

impl<S: Eq> Ord for PathEntry<S> {
    // This method implements reverse ordering, so that the lowest cost
    // will be ordered first
    fn cmp(&self, other: &PathEntry<S>) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap_or(Equal)
    }
}

impl<S: Eq> PartialOrd for PathEntry<S> {
    fn partial_cmp(&self, other: &PathEntry<S>) -> Option<Ordering> { Some(self.cmp(other)) }
}

pub enum PathResult<T> {
    None(Path<T>),
    Exhausted(Path<T>),
    Path(Path<T>),
    Pending,
}

impl<T> PathResult<T> {
    pub fn into_path(self) -> Option<Path<T>> {
        match self {
            PathResult::Path(path) => Some(path),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    potential_nodes: BinaryHeap<PathEntry<S>>,
    came_from: HashMap<S, S, Hasher>,
    cheapest_scores: HashMap<S, f32, Hasher>,
    final_scores: HashMap<S, f32, Hasher>,
    visited: HashSet<S, Hasher>,
    cheapest_node: Option<S>,
    cheapest_cost: Option<f32>,
}

/// NOTE: Must manually derive since Hasher doesn't implement it.
impl<S: Clone + Eq + Hash + fmt::Debug, H: BuildHasher> fmt::Debug for Astar<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Astar")
            .field("iter", &self.iter)
            .field("max_iters", &self.max_iters)
            .field("potential_nodes", &self.potential_nodes)
            .field("came_from", &self.came_from)
            .field("cheapest_scores", &self.cheapest_scores)
            .field("final_scores", &self.final_scores)
            .field("visited", &self.visited)
            .field("cheapest_node", &self.cheapest_node)
            .field("cheapest_cost", &self.cheapest_cost)
            .finish()
    }
}

impl<S: Clone + Eq + Hash, H: BuildHasher + Clone> Astar<S, H> {
    pub fn new(max_iters: usize, start: S, heuristic: impl FnOnce(&S) -> f32, hasher: H) -> Self {
        Self {
            max_iters,
            iter: 0,
            potential_nodes: core::iter::once(PathEntry {
                cost: 0.0,
                node: start.clone(),
            })
            .collect(),
            came_from: HashMap::with_hasher(hasher.clone()),
            cheapest_scores: {
                let mut h = HashMap::with_capacity_and_hasher(1, hasher.clone());
                h.extend(core::iter::once((start.clone(), 0.0)));
                h
            },
            final_scores: {
                let mut h = HashMap::with_capacity_and_hasher(1, hasher.clone());
                h.extend(core::iter::once((start.clone(), heuristic(&start))));
                h
            },
            visited: {
                let mut s = HashSet::with_capacity_and_hasher(1, hasher);
                s.extend(core::iter::once(start));
                s
            },
            cheapest_node: None,
            cheapest_cost: None,
        }
    }

    pub fn poll<I>(
        &mut self,
        iters: usize,
        mut heuristic: impl FnMut(&S) -> f32,
        mut neighbors: impl FnMut(&S) -> I,
        mut transition: impl FnMut(&S, &S) -> f32,
        mut satisfied: impl FnMut(&S) -> bool,
    ) -> PathResult<S>
    where
        I: Iterator<Item = S>,
    {
        let iter_limit = self.max_iters.min(self.iter + iters);
        while self.iter < iter_limit {
            if let Some(PathEntry { node, cost }) = self.potential_nodes.pop() {
                self.cheapest_cost = Some(cost);
                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node));
                } else {
                    self.cheapest_node = Some(node.clone());
                    for neighbor in neighbors(&node) {
                        let node_cheapest = self.cheapest_scores.get(&node).unwrap_or(&f32::MAX);
                        let neighbor_cheapest =
                            self.cheapest_scores.get(&neighbor).unwrap_or(&f32::MAX);

                        let cost = node_cheapest + transition(&node, &neighbor);
                        if cost < *neighbor_cheapest {
                            self.came_from.insert(neighbor.clone(), node.clone());
                            self.cheapest_scores.insert(neighbor.clone(), cost);
                            let neighbor_cost = cost + heuristic(&neighbor);
                            self.final_scores.insert(neighbor.clone(), neighbor_cost);

                            if self.visited.insert(neighbor.clone()) {
                                self.potential_nodes.push(PathEntry {
                                    node: neighbor.clone(),
                                    cost: neighbor_cost,
                                });
                            }
                        }
                    }
                }
            } else {
                return PathResult::None(
                    self.cheapest_node
                        .clone()
                        .map(|lc| self.reconstruct_path_to(lc))
                        .unwrap_or_default(),
                );
            }

            self.iter += 1
        }

        if self.iter >= self.max_iters {
            PathResult::Exhausted(
                self.cheapest_node
                    .clone()
                    .map(|lc| self.reconstruct_path_to(lc))
                    .unwrap_or_default(),
            )
        } else {
            PathResult::Pending
        }
    }

    pub fn get_cheapest_cost(&self) -> Option<f32> { self.cheapest_cost }

    fn reconstruct_path_to(&mut self, end: S) -> Path<S> {
        let mut path = vec![end.clone()];
        let mut cnode = &end;
        while let Some(node) = self.came_from.get(cnode) {
            path.push(node.clone());
            cnode = node;
        }
        path.into_iter().rev().collect()
    }
}
