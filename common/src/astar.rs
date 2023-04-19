use crate::path::Path;
use core::{
    cmp::Ordering::{self, Equal},
    fmt,
    hash::{BuildHasher, Hash},
};
use hashbrown::HashMap;
use std::collections::BinaryHeap;

#[derive(Copy, Clone, Debug)]
pub struct PathEntry<S> {
    // cost so far + heursitic
    cost_estimate: f32,
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
        other
            .cost_estimate
            .partial_cmp(&self.cost_estimate)
            .unwrap_or(Equal)
    }
}

impl<S: Eq> PartialOrd for PathEntry<S> {
    fn partial_cmp(&self, other: &PathEntry<S>) -> Option<Ordering> { Some(self.cmp(other)) }

    // This is particularily hot in `BinaryHeap::pop`, so we provide this
    // implementation.
    //
    // NOTE: This probably doesn't handle edge cases like `NaNs` in a consistent
    // manner with `Ord`, but I don't think we need to care about that here(?)
    //
    // See note about reverse ordering above.
    fn le(&self, other: &PathEntry<S>) -> bool { other.cost_estimate <= self.cost_estimate }
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

    pub fn map<U>(self, f: impl FnOnce(Path<T>) -> Path<U>) -> PathResult<U> {
        match self {
            PathResult::None(p) => PathResult::None(f(p)),
            PathResult::Exhausted(p) => PathResult::Exhausted(f(p)),
            PathResult::Path(p) => PathResult::Path(f(p)),
            PathResult::Pending => PathResult::Pending,
        }
    }
}

// If node entry exists, this was visited!
#[derive(Clone, Debug)]
struct NodeEntry<S> {
    // if came_from == self this is the start node!
    came_from: S,
    cheapest_score: f32,
}

#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    potential_nodes: BinaryHeap<PathEntry<S>>, // cost, node pairs
    visited_nodes: HashMap<S, NodeEntry<S>, Hasher>,
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
            .field("visited_nodes", &self.visited_nodes)
            .field("cheapest_node", &self.cheapest_node)
            .field("cheapest_cost", &self.cheapest_cost)
            .finish()
    }
}

impl<S: Clone + Eq + Hash, H: BuildHasher + Clone> Astar<S, H> {
    pub fn new(max_iters: usize, start: S, hasher: H) -> Self {
        Self {
            max_iters,
            iter: 0,
            potential_nodes: core::iter::once(PathEntry {
                cost_estimate: 0.0,
                node: start.clone(),
            })
            .collect(),
            visited_nodes: {
                let mut s = HashMap::with_capacity_and_hasher(1, hasher);
                s.extend(core::iter::once((start.clone(), NodeEntry {
                    came_from: start,
                    cheapest_score: 0.0,
                })));
                s
            },
            cheapest_node: None,
            cheapest_cost: None,
        }
    }

    pub fn poll<I>(
        &mut self,
        iters: usize,
        // Estimate how far we are from the target? but we are given two nodes...
        // (current, previous)
        mut heuristic: impl FnMut(&S, &S) -> f32,
        // get neighboring nodes
        mut neighbors: impl FnMut(&S) -> I,
        // have we reached target?
        mut satisfied: impl FnMut(&S) -> bool,
    ) -> PathResult<S>
    where
        I: Iterator<Item = (S, f32)>, // (node, transition cost)
    {
        let iter_limit = self.max_iters.min(self.iter + iters);
        while self.iter < iter_limit {
            if let Some(PathEntry { node, .. }) = self.potential_nodes.pop() {
                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node));
                } else {
                    let (node_cheapest, came_from) = self
                        .visited_nodes
                        .get(&node)
                        .map(|n| (n.cheapest_score, n.came_from.clone()))
                        .unwrap();
                    for (neighbor, transition) in neighbors(&node) {
                        if neighbor == came_from {
                            continue;
                        }
                        let neighbor_cheapest = self
                            .visited_nodes
                            .get(&neighbor)
                            .map_or(f32::MAX, |n| n.cheapest_score);

                        // compute cost to traverse to each neighbor
                        let cost = node_cheapest + transition;

                        if cost < neighbor_cheapest {
                            let previously_visited = self
                                .visited_nodes
                                .insert(neighbor.clone(), NodeEntry {
                                    came_from: node.clone(),
                                    cheapest_score: cost,
                                })
                                .is_some();
                            let h = heuristic(&neighbor, &node);
                            // note that cheapest_scores does not include the heuristic
                            // priority queue does include heuristic
                            let cost_estimate = cost + h;

                            if self.cheapest_cost.map(|cc| h < cc).unwrap_or(true) {
                                self.cheapest_node = Some(node.clone());
                                self.cheapest_cost = Some(h);
                            };

                            // TODO: I think the if here should be removed
                            // if we hadn't already visted, add this to potential nodes, what about
                            // its neighbors, wouldn't they need to be revisted???
                            if !previously_visited {
                                self.potential_nodes.push(PathEntry {
                                    cost_estimate,
                                    node: neighbor,
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
        while let Some(node) = self
            .visited_nodes
            .get(cnode)
            .map(|n| &n.came_from)
            .filter(|n| *n != cnode)
        {
            path.push(node.clone());
            cnode = node;
        }
        path.into_iter().rev().collect()
    }
}
