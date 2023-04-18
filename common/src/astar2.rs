#![allow(dead_code, unused_mut, unused_variables)]
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

#[derive(Clone, Debug)]
struct Cluster<S> {
    // idea: if we store pointers to adjacent clusters we could avoid the hashmap entirely when
    // accessing neighboring nodes, actually I'm not even sure what case we would need a hashmap
    // for in this scenario if we store cluster pointer in the priority queue (or index to cluster
    // pointer if saving space here is worth it)!
    // TODO: we could use `(S, u8)` here?
    // idea: if we bake in the gridness we could just store a direction (note: actually some things
    // like bridges are not adjacent)
    // idea: if we can gain something by making them smaller, we could allocate clusters in a Vec,
    // this amoritizes allocation costs some, if we point to neighbors we would need to use
    // indices although we could take advantage of that to make them smaller than pointer size
    // (alternatively a bump allocator would work very well here)
    // idea
    came_from: [Option<S>; 256],
    cheapest_score: [f32; 256],
}

#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    potential_nodes: BinaryHeap<PathEntry<S>>, // cost, node pairs
    visited_nodes: HashMap<S, NodeEntry<S>, Hasher>,
    clusters: HashMap<S, Box<Cluster<S>>, Hasher>,
    start_node: S,
    cheapest_node: Option<S>,
    cheapest_cost: Option<f32>,
}

/// NOTE: Must manually derive since Hasher doesn't implement it.
impl<S: Clone + Eq + Hash + fmt::Debug, H: BuildHasher> fmt::Debug for Astar<S, H> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() }
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
                let mut s = HashMap::with_capacity_and_hasher(1, hasher.clone());
                s.extend(core::iter::once((start.clone(), NodeEntry {
                    came_from: start.clone(),
                    cheapest_score: 0.0,
                })));
                s
            },
            clusters: HashMap::with_hasher(hasher),
            start_node: start,
            cheapest_node: None,
            cheapest_cost: None,
        }
    }

    pub fn poll<I>(
        &mut self,
        iters: usize,
        // Estimate how far we are from the target? but we are given two nodes... (current,
        // previous)
        mut heuristic: impl FnMut(&S, &S) -> f32,
        // get neighboring nodes
        mut neighbors: impl FnMut(&S) -> I,
        // have we reached target?
        mut satisfied: impl FnMut(&S) -> bool,
        // this function clusters nodes together for cache locality purposes
        // output (cluster base, offset in cluster)
        cluster: impl Fn(&S) -> (S, u8),
    ) -> PathResult<S>
    where
        I: Iterator<Item = (S, f32)>, // (node, transition cost)
    {
        /*
        if self.clusters.is_empty() {
            let (key, index) = cluster(&self.start_node);
            let mut came_from = std::array::from_fn(|_| None);
            came_from[usize::from(index)] = Some(self.start_node.clone());
            self.clusters.insert(
                key,
                Box::new(Cluster {
                    came_from,
                    cheapest_score: [0.0; 256],
                }),
            );
        }
         */
        let iter_limit = self.max_iters.min(self.iter + iters);
        while self.iter < iter_limit {
            if let Some(PathEntry { node, .. }) = self.potential_nodes.pop() {
                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node, cluster));
                } else {
                    /*
                    let (cluster_key, index) = cluster(&node);
                    let (node_cheapest, came_from) = self
                        .clusters
                        .get(&cluster_key)
                        .map(|c| {
                            (
                                c.cheapest_score[usize::from(index)],
                                c.came_from[usize::from(index)].clone().unwrap(),
                            )
                        })
                        .unwrap();
                    */
                    // we have to fetch this even though it was put into the priority queue
                    let (node_cheapest, came_from) = self
                        .visited_nodes
                        .get(&node)
                        .map(|n| (n.cheapest_score, n.came_from.clone()))
                        .unwrap();
                    for (neighbor, transition) in neighbors(&node) {
                        if neighbor == came_from {
                            continue;
                        }
                        /*
                        let (cluster_key, index) = cluster(&neighbor);
                        let mut previously_visited = false;
                        let neighbor_cheapest = self
                            .clusters
                            .get(&cluster_key)
                            .and_then(|c| {
                                previously_visited = c.came_from[usize::from(index)].is_some();

                                previously_visited.then(|| c.cheapest_score[usize::from(index)])
                            })
                            .unwrap_or(f32::MAX);
                        */
                        let neighbor_cheapest = self
                            .visited_nodes
                            .get(&neighbor)
                            .map_or(f32::MAX, |n| n.cheapest_score);

                        // compute cost to traverse to each neighbor
                        let cost = node_cheapest + transition;

                        if cost < neighbor_cheapest {
                            /*
                            neighbor_entry.cheapest_score = cost;
                            // note: unconditional insert, same cost as overwriting if it already
                            // exists
                            let previously_visited = self
                                .came_from
                                .insert(neighbor.clone(), node.clone())
                                .is_some();
                            self.cheapest_scores.insert(neighbor.clone(), cost);
                            */
                            let previously_visited = self
                                .visited_nodes
                                .insert(neighbor.clone(), NodeEntry {
                                    came_from: node.clone(),
                                    cheapest_score: cost,
                                })
                                .is_some();
                            /*
                            let cluster_mut =
                                self.clusters.entry(cluster_key).or_insert_with(|| {
                                    Box::new(Cluster {
                                        came_from: std::array::from_fn(|_| None),
                                        cheapest_score: [0.0; 256],
                                    })
                                });
                            cluster_mut.came_from[usize::from(index)] = Some(node.clone());
                            cluster_mut.cheapest_score[usize::from(index)] = cost;

                            */
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
                        .map(|lc| self.reconstruct_path_to(lc, cluster))
                        .unwrap_or_default(),
                );
            }

            self.iter += 1
        }

        if self.iter >= self.max_iters {
            PathResult::Exhausted(
                self.cheapest_node
                    .clone()
                    .map(|lc| self.reconstruct_path_to(lc, cluster))
                    .unwrap_or_default(),
            )
        } else {
            PathResult::Pending
        }
    }

    pub fn get_cheapest_cost(&self) -> Option<f32> { self.cheapest_cost }

    fn reconstruct_path_to(&mut self, end: S, cluster: impl Fn(&S) -> (S, u8)) -> Path<S> {
        let mut path = vec![end.clone()];
        let mut cnode = &end;
        /*
        let (mut ckey, mut ci) = cluster(cnode);
        while let Some(node) =
            self
                .clusters
                .get(&ckey)
                .and_then(|c| c.came_from[usize::from(ci)].as_ref())
                .filter(|n| *n != cnode)
             */
        /*
         */
        while let Some(node) = self
            .visited_nodes
            .get(cnode)
            .map(|n| &n.came_from)
            .filter(|n| *n != cnode)
        {
            path.push(node.clone());
            cnode = node;
            //(ckey, ci) = cluster(cnode);
        }
        path.into_iter().rev().collect()
    }
}
