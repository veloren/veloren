use crate::path::Path;
use core::cmp::Ordering::Equal;
use hashbrown::{HashMap, HashSet};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::f32;
use std::hash::Hash;

#[derive(Copy, Clone, Debug)]
pub struct PathEntry<S> {
    cost: f32,
    node: S,
}

impl<S: Eq> PartialEq for PathEntry<S> {
    fn eq(&self, other: &PathEntry<S>) -> bool {
        self.node.eq(&other.node)
    }
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
    fn partial_cmp(&self, other: &PathEntry<S>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn reconstruct_path<S>(came_from: &HashMap<S, S>, target: &S) -> Vec<S>
where
    S: Clone + Eq + Hash,
{
    let mut path = Vec::new();
    path.push(target.to_owned());
    let mut cur_node = target;
    while let Some(node) = came_from.get(cur_node) {
        path.push(node.to_owned());
        cur_node = node;
    }
    path
}

pub fn astar<S, I>(
    initial: S,
    target: S,
    mut heuristic: impl FnMut(&S, &S) -> f32,
    mut neighbors: impl FnMut(&S) -> I,
    mut transition_cost: impl FnMut(&S, &S) -> f32,
) -> Option<Vec<S>>
where
    S: Clone + Eq + Hash,
    I: IntoIterator<Item = S>,
{
    // Set of discovered nodes so far
    let mut potential_nodes = BinaryHeap::new();
    potential_nodes.push(PathEntry {
        cost: 0.0f32,
        node: initial.clone(),
    });

    // For entry e, contains the cheapest node preceding it on the known path from start to e
    let mut came_from = HashMap::new();

    // Contains cheapest cost from 'initial' to the current entry
    let mut cheapest_scores = HashMap::new();
    cheapest_scores.insert(initial.clone(), 0.0f32);

    // Contains cheapest score to get to node + heuristic to the end, for an entry
    let mut final_scores = HashMap::new();
    final_scores.insert(initial.clone(), heuristic(&initial, &target));

    // Set of nodes we have already visited
    let mut visited = HashSet::new();
    visited.insert(initial.clone());

    let mut iters = 0;
    while let Some(PathEntry { node: current, .. }) = potential_nodes.pop() {
        if current == target {
            return Some(reconstruct_path(&came_from, &current));
        }

        let current_neighbors = neighbors(&current);
        for neighbor in current_neighbors {
            let current_cheapest_score = cheapest_scores.get(&current).unwrap_or(&f32::MAX);
            let neighbor_cheapest_score = cheapest_scores.get(&neighbor).unwrap_or(&f32::MAX);
            let score = current_cheapest_score + transition_cost(&current, &neighbor);
            if score < *neighbor_cheapest_score {
                // Path to the neighbor is better than anything yet recorded
                came_from.insert(neighbor.to_owned(), current.to_owned());
                cheapest_scores.insert(neighbor.clone(), score);
                let neighbor_score = score + heuristic(&neighbor, &target);
                final_scores.insert(neighbor.clone(), neighbor_score);

                if visited.insert(neighbor.clone()) {
                    potential_nodes.push(PathEntry {
                        node: neighbor.clone(),
                        cost: neighbor_score,
                    });
                }
            }
        }

        iters += 1;
        if iters >= 10000 {
            println!("Ran out of turns!");
            break;
        }
    }

    None
}

pub enum PathResult<T> {
    None(Path<T>),
    Exhausted(Path<T>),
    Path(Path<T>),
    Pending,
}

#[derive(Clone, Debug)]
pub struct Astar<S: Clone + Eq + Hash> {
    iter: usize,
    max_iters: usize,
    potential_nodes: BinaryHeap<PathEntry<S>>,
    came_from: HashMap<S, S>,
    cheapest_scores: HashMap<S, f32>,
    final_scores: HashMap<S, f32>,
    visited: HashSet<S>,
    lowest_cost: Option<S>,
}

impl<S: Clone + Eq + Hash> Astar<S> {
    pub fn new(max_iters: usize, start: S, heuristic: impl FnOnce(&S) -> f32) -> Self {
        Self {
            max_iters,
            iter: 0,
            potential_nodes: std::iter::once(PathEntry {
                cost: 0.0,
                node: start.clone(),
            })
            .collect(),
            came_from: HashMap::default(),
            cheapest_scores: std::iter::once((start.clone(), 0.0)).collect(),
            final_scores: std::iter::once((start.clone(), heuristic(&start))).collect(),
            visited: std::iter::once(start).collect(),
            lowest_cost: None,
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
        while self.iter < self.max_iters.min(self.iter + iters) {
            if let Some(PathEntry { node, .. }) = self.potential_nodes.pop() {
                if satisfied(&node) {
                    return PathResult::Path(self.reconstruct_path_to(node));
                } else {
                    self.lowest_cost = Some(node.clone());
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
                    self.lowest_cost
                        .clone()
                        .map(|lc| self.reconstruct_path_to(lc))
                        .unwrap_or_default(),
                );
            }

            self.iter += 1
        }

        if self.iter >= self.max_iters {
            PathResult::Exhausted(
                self.lowest_cost
                    .clone()
                    .map(|lc| self.reconstruct_path_to(lc))
                    .unwrap_or_default(),
            )
        } else {
            PathResult::Pending
        }
    }

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
