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
    #[allow(clippy::unconditional_recursion)] // false positive as we use .node
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
    /// No reachable nodes were satisfactory.
    ///
    /// Contains path to node with the lowest heuristic value (out of the
    /// explored nodes).
    None(Path<T>),
    /// Either max_iters or max_cost was reached.
    ///
    /// Contains path to node with the lowest heuristic value (out of the
    /// explored nodes).
    Exhausted(Path<T>),
    /// Path succefully found.
    ///
    /// Second field is cost.
    Path(Path<T>, f32),
    Pending,
}

impl<T> PathResult<T> {
    /// Returns `Some((path, cost))` if a path reaching the target was
    /// successfully found.
    pub fn into_path(self) -> Option<(Path<T>, f32)> {
        match self {
            PathResult::Path(path, cost) => Some((path, cost)),
            _ => None,
        }
    }

    pub fn map<U>(self, f: impl FnOnce(Path<T>) -> Path<U>) -> PathResult<U> {
        match self {
            PathResult::None(p) => PathResult::None(f(p)),
            PathResult::Exhausted(p) => PathResult::Exhausted(f(p)),
            PathResult::Path(p, cost) => PathResult::Path(f(p), cost),
            PathResult::Pending => PathResult::Pending,
        }
    }
}

// If node entry exists, this was visited!
#[derive(Clone, Debug)]
struct NodeEntry<S> {
    /// Previous node in the cheapest path (known so far) that goes from the
    /// start to this node.
    ///
    /// If `came_from == self` this is the start node! (to avoid inflating the
    /// size with `Option`)
    came_from: S,
    /// Cost to reach this node from the start by following the cheapest path
    /// known so far. This is the sum of the transition costs between all the
    /// nodes on this path.
    cost: f32,
}

#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    max_cost: f32,
    potential_nodes: BinaryHeap<PathEntry<S>>, // cost, node pairs
    visited_nodes: HashMap<S, NodeEntry<S>, Hasher>,
    /// Node with the lowest heuristic value so far.
    ///
    /// (node, heuristic value)
    closest_node: Option<(S, f32)>,
}

/// NOTE: Must manually derive since Hasher doesn't implement it.
impl<S: Clone + Eq + Hash + fmt::Debug, H: BuildHasher> fmt::Debug for Astar<S, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Astar")
            .field("iter", &self.iter)
            .field("max_iters", &self.max_iters)
            .field("potential_nodes", &self.potential_nodes)
            .field("visited_nodes", &self.visited_nodes)
            .field("closest_node", &self.closest_node)
            .finish()
    }
}

impl<S: Clone + Eq + Hash, H: BuildHasher + Clone> Astar<S, H> {
    pub fn new(max_iters: usize, start: S, hasher: H) -> Self {
        Self {
            max_iters,
            max_cost: f32::MAX,
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
                    cost: 0.0,
                })));
                s
            },
            closest_node: None,
        }
    }

    pub fn with_max_cost(mut self, max_cost: f32) -> Self {
        self.max_cost = max_cost;
        self
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
            if let Some(PathEntry {
                node,
                cost_estimate,
            }) = self.potential_nodes.pop()
            {
                let (node_cost, came_from) = self
                    .visited_nodes
                    .get(&node)
                    .map(|n| (n.cost, n.came_from.clone()))
                    .expect("All nodes in the queue should be included in visisted_nodes");

                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node), node_cost);
                // Note, we assume that cost_estimate isn't an overestimation
                // (i.e. that `heuristic` doesn't overestimate).
                } else if cost_estimate > self.max_cost {
                    return PathResult::Exhausted(
                        self.closest_node
                            .clone()
                            .map(|(lc, _)| self.reconstruct_path_to(lc))
                            .unwrap_or_default(),
                    );
                } else {
                    for (neighbor, transition_cost) in neighbors(&node) {
                        if neighbor == came_from {
                            continue;
                        }
                        let neighbor_cost = self
                            .visited_nodes
                            .get(&neighbor)
                            .map_or(f32::MAX, |n| n.cost);

                        // compute cost to traverse to each neighbor
                        let cost = node_cost + transition_cost;

                        if cost < neighbor_cost {
                            let previously_visited = self
                                .visited_nodes
                                .insert(neighbor.clone(), NodeEntry {
                                    came_from: node.clone(),
                                    cost,
                                })
                                .is_some();
                            let h = heuristic(&neighbor, &node);
                            // note that `cost` field does not include the heuristic
                            // priority queue does include heuristic
                            let cost_estimate = cost + h;

                            if self
                                .closest_node
                                .as_ref()
                                .map(|&(_, ch)| h < ch)
                                .unwrap_or(true)
                            {
                                self.closest_node = Some((node.clone(), h));
                            };

                            // TODO: I think the if here should be removed
                            // if we hadn't already visited, add this to potential nodes, what about
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
                    self.closest_node
                        .clone()
                        .map(|(lc, _)| self.reconstruct_path_to(lc))
                        .unwrap_or_default(),
                );
            }

            self.iter += 1
        }

        if self.iter >= self.max_iters {
            PathResult::Exhausted(
                self.closest_node
                    .clone()
                    .map(|(lc, _)| self.reconstruct_path_to(lc))
                    .unwrap_or_default(),
            )
        } else {
            PathResult::Pending
        }
    }

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
