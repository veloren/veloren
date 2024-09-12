use std::cmp::Ordering;

use rand::Rng;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};

use crate::resources::Time;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum HeadState {
    Attached,
    Detached(Time),
}

impl HeadState {
    pub fn is_attached(&self) -> bool { matches!(self, Self::Attached) }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Heads {
    heads: Vec<HeadState>,
}

impl Heads {
    pub fn new(amount: usize) -> Self {
        Self {
            heads: vec![HeadState::Attached; amount],
        }
    }

    pub fn capacity(&self) -> usize { self.heads.len() }

    pub fn amount(&self) -> usize { self.heads.iter().filter(|h| h.is_attached()).count() }

    pub fn amount_missing(&self) -> usize { self.capacity() - self.amount() }

    pub fn remove_one(&mut self, rng: &mut impl Rng, time: Time) -> Option<usize> {
        if self.amount() == 0 {
            return None;
        }

        let mut h = rng.gen_range(0..self.amount());

        self.heads.iter_mut().position(|head| {
            if matches!(head, HeadState::Attached) {
                if h == 0 {
                    *head = HeadState::Detached(time);
                    true
                } else {
                    h -= 1;
                    false
                }
            } else {
                false
            }
        })
    }

    pub fn regrow_oldest(&mut self) -> bool {
        if self.amount_missing() == 0 {
            return false;
        }

        self.heads
            .iter_mut()
            .min_by(|a, b| match (a, b) {
                (HeadState::Attached, HeadState::Attached) => Ordering::Equal,
                (HeadState::Attached, HeadState::Detached(_)) => Ordering::Greater,
                (HeadState::Detached(_), HeadState::Attached) => Ordering::Less,
                (HeadState::Detached(a), HeadState::Detached(b)) => {
                    // Time should never be NaN, but no need to panic here.
                    a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal)
                },
            })
            .map(|head| {
                *head = HeadState::Attached;
            })
            .is_some()
    }

    pub fn reset(&mut self) {
        for head in self.heads.iter_mut() {
            *head = HeadState::Attached;
        }
    }

    /// For correctness don't change the variant for HeadState here.
    pub fn heads_mut(&mut self) -> &mut [HeadState] { &mut self.heads }

    pub fn heads(&self) -> &[HeadState] { &self.heads }
}

impl Component for Heads {
    type Storage = DerefFlaggedStorage<Self, specs::HashMapStorage<Self>>;
}
