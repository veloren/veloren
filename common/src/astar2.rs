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
    priority: f32,
    node: S,
    //cost: f32,
}

impl<S: Eq> PartialEq for PathEntry<S> {
    fn eq(&self, other: &PathEntry<S>) -> bool { self.node.eq(&other.node) }
}

impl<S: Eq> Eq for PathEntry<S> {}

impl<S: Eq> Ord for PathEntry<S> {
    // This method implements reverse ordering, so that the lowest cost
    // will be ordered first
    fn cmp(&self, other: &PathEntry<S>) -> Ordering {
        other.priority.partial_cmp(&self.priority).unwrap_or(Equal)
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
    fn le(&self, other: &PathEntry<S>) -> bool { other.priority <= self.priority }
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
    // TODO: we could use `(S, u8)` here?
    // idea: if we bake in the gridness we could just store a direction
    came_from: [Option<S>; 256],
    cheapest_score: [f32; 256],
}

// ideas:
// * merge hashmaps
// * "chunked" exploration
// * things we put on priority queue don't need to point into a hashmap (i.e. we
//   only need a hashmap to map from new/unknown nodes to whatever
//   datastructure)
#[derive(Clone)]
pub struct Astar<S, Hasher> {
    iter: usize,
    max_iters: usize,
    potential_nodes: BinaryHeap<PathEntry<S>>, // cost, node pairs
    // converting to single hash structure: 11349 ms -> 10462 ms / 10612 ms
    // with two hash structures (came_from and cheapest_scores): 10861 ms
    visited_nodes: HashMap<S, NodeEntry<S>, Hasher>,
    // -> 25055 ms -> 15771 ms with Box -> fixed bugs 10731 ms, hmmm
    clusters: HashMap<S, Box<Cluster<S>>, Hasher>, // TODO: Box cluster?
    //came_from: HashMap<S, S, Hasher>,
    //cheapest_scores: HashMap<S, f32, Hasher>,
    //final_scores: HashMap<S, f32, Hasher>,
    //visited: HashSet<S, Hasher>,
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
                priority: 0.0,
                //cost: 0.0,
                node: start.clone(),
            })
            .collect(),
            /*
            came_from: HashMap::with_hasher(hasher.clone()),
            cheapest_scores: {
                let mut h = HashMap::with_capacity_and_hasher(1, hasher.clone());
                h.extend(core::iter::once((start.clone(), 0.0)));
                h
            },
            final_scores: {
                let mut h = HashMap::with_capacity_and_hasher(1, hasher.clone());
                h.extend(core::iter::once((start.clone(), 0.0)));
                h
            },
            visited: {
                let mut s = HashSet::with_capacity_and_hasher(1, hasher);
                s.extend(core::iter::once(start));
                s
            },
            */
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
        // estimate how far we are from the target? but we are given two nodes... (current,
        // previous)
        mut heuristic: impl FnMut(&S, &S) -> f32,
        // get neighboring nodes
        mut neighbors: impl FnMut(&S) -> I,
        // cost of edge between these two nodes
        // I assume this is (source, destination)?
        mut transition: impl FnMut(&S, &S) -> f32,
        // have we reached a/the target?
        mut satisfied: impl FnMut(&S) -> bool,
        // this function clusters nodes together for cache locality purposes
        // output (cluster base, offset in cluster)
        cluster: impl Fn(&S) -> (S, u8),
    ) -> PathResult<S>
    where
        // Combining transition into this: 9913 ms -> 8204 ms (~1.7 out of ~6.5 seconds)
        I: Iterator<Item = (S, f32)>,
    {
        /*
         */
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
        let iter_limit = self.max_iters.min(self.iter + iters);
        while self.iter < iter_limit {
            // pop highest priority node
            if let Some(PathEntry { node, .. }) = self.potential_nodes.pop() {
                // if this is the destination, we return
                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node, cluster));
                } else {
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
                    // regression
                    //if node_cheapest < cost {
                    // we already processed it
                    //    continue;
                    //}
                    // 10700 ms -> 10477 ms (moving this out of the loop)
                    // we have to fetch this even though it was put into the priority queu
                    /*
                    let node_cheapest = self
                        .visited_nodes
                        .get(&node)
                        .map_or(f32::MAX, |n| n.cheapest_score);
                    */
                    // otherwise we iterate neighbors
                    // TODO: try for_each here
                    // 6879 ms -> 6989 ms (regression using for_each)
                    //neighbors(&node).for_each(|(neighbor, transition)| {
                    for (neighbor, transition) in neighbors(&node) {
                        // skipping here: 10694 ms -> 9913 ms (almost whole second out of 7 taken
                        // for this, this is because the `transition` call is fairly expensive)
                        if neighbor == came_from {
                            continue;
                            //return;
                        }
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
                        /*
                        let neighbor_cheapest = self
                            .visited_nodes
                            .get(&neighbor)
                            .map_or(f32::MAX, |n| n.cheapest_score);
                         */
                        // 10573 ms -> 11546 ms (with entry api appears to be regression)
                        /*
                        let mut previously_visited = true;
                        let neighbor_entry = self
                            .visited_nodes
                            .entry(neighbor.clone())
                            .or_insert_with(|| {
                                previously_visited = false;
                                NodeEntry {
                                    came_from: node.clone(),
                                    cheapest_score: f32::MAX,
                                }
                            });
                        let neighbor_cheapest = neighbor_entry.cheapest_score;
                         */
                        /*
                        let node_cheapest = *self.cheapest_scores.get(&node).unwrap_or(&f32::MAX);
                        let neighbor_cheapest =
                            *self.cheapest_scores.get(&neighbor).unwrap_or(&f32::MAX);
                        */

                        // TODO: have caller provide transition cost with neighbors iterator (so
                        // that duplicate costs in `transition` can be avoided?)
                        // compute cost to traverse to each neighbor
                        let cost = node_cheapest + transition; //transition(&node, &neighbor);
                        // if this is cheaper than existing cost for that neighbor (or neighbor
                        // hasn't been visited)
                        // can we convince ourselves that this is always true if node was not
                        // visited?
                        if cost < neighbor_cheapest {
                            //neighbor_entry.cheapest_score = cost;
                            /*
                            // note: unconditional insert, same cost as overwriting if it already
                            // exists
                            let previously_visited = self
                                .came_from
                                .insert(neighbor.clone(), node.clone())
                                .is_some();
                            self.cheapest_scores.insert(neighbor.clone(), cost);
                            */
                            /*
                            let previously_visited = self
                                .visited_nodes
                                .insert(neighbor.clone(), NodeEntry {
                                    came_from: node.clone(),
                                    cheapest_score: cost,
                                })
                                .is_some();
                             */
                            let cluster_mut =
                                self.clusters.entry(cluster_key).or_insert_with(|| {
                                    Box::new(Cluster {
                                        came_from: std::array::from_fn(|_| None),
                                        cheapest_score: [0.0; 256],
                                    })
                                });
                            cluster_mut.came_from[usize::from(index)] = Some(node.clone());
                            cluster_mut.cheapest_score[usize::from(index)] = cost;

                            let h = heuristic(&neighbor, &node);
                            // note that cheapest_scores does not include the heuristic
                            // this is what final_scores does, priority queue does include
                            // heuristic
                            let priority = cost + h;
                            // note this is literally unused, removing saves ~350 ms out of 11349
                            // (note this is all of startup time)
                            //self.final_scores.insert(neighbor.clone(), neighbor_cost);

                            if self.cheapest_cost.map(|cc| h < cc).unwrap_or(true) {
                                self.cheapest_node = Some(node.clone());
                                self.cheapest_cost = Some(h);
                            };

                            // commenting out if here: 11349 ms -> 12498 ms (but may give better
                            // paths?) (about 1 extra second or +10% time)
                            // with single hashmap change this has much more impact:
                            // 3473 ms -> 11981 ms

                            // if we hadn't already visted, add this to potential nodes, what about
                            // its neighbors, wouldn't they need to be revisted???
                            if !previously_visited {
                                self.potential_nodes.push(PathEntry {
                                    priority,
                                    //cost,
                                    node: neighbor,
                                });
                            }
                        }
                    }
                    //});
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

    // At least in world site pathfinding this is super cheap compared to actually
    // finding the path!
    fn reconstruct_path_to(&mut self, end: S, cluster: impl Fn(&S) -> (S, u8)) -> Path<S> {
        let mut path = vec![end.clone()];
        let mut cnode = &end;
        let (mut ckey, mut ci) = cluster(cnode);
        while let Some(node) = self
            .clusters
            .get(&ckey)
            .and_then(|c| c.came_from[usize::from(ci)].as_ref())
            .filter(|n| *n != cnode)
        /*
        self
            .visited_nodes
            .get(cnode)
            .map(|n| &n.came_from)
            .filter(|n| *n != cnode)
        */
        //self.came_from.get(cnode)
        {
            path.push(node.clone());
            cnode = node;
            (ckey, ci) = cluster(cnode);
        }
        path.into_iter().rev().collect()
    }
}
