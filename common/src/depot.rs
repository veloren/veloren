use serde::{Deserialize, Serialize};
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    fmt, hash,
    marker::PhantomData,
};

/// Type safe index into Depot
#[derive(Deserialize, Serialize)]
pub struct Id<T> {
    idx: u32,
    gen: u32,
    phantom: PhantomData<T>,
}

impl<T> Id<T> {
    pub fn id(&self) -> u64 { self.idx as u64 | ((self.gen as u64) << 32) }
}

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            gen: self.gen,
            phantom: PhantomData,
        }
    }
}
impl<T> Eq for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.idx == other.idx && self.gen == other.gen }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering { (self.idx, self.gen).cmp(&(other.idx, other.gen)) }
}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Id<{}>({}, {})",
            std::any::type_name::<T>(),
            self.idx,
            self.gen
        )
    }
}
impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, h: &mut H) {
        self.idx.hash(h);
        self.gen.hash(h);
    }
}

struct Entry<T> {
    gen: u32,
    item: Option<T>,
}

/// A general-purpose high performance allocator, basically Vec with type safe
/// indices (Id)
pub struct Depot<T> {
    entries: Vec<Entry<T>>,
    len: usize,
}

impl<T> Default for Depot<T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            len: 0,
        }
    }
}

impl<T> Depot<T> {
    pub fn is_empty(&self) -> bool { self.len == 0 }

    pub fn len(&self) -> usize { self.len }

    pub fn contains(&self, id: Id<T>) -> bool {
        self.entries
            .get(id.idx as usize)
            .map(|e| e.gen == id.gen && e.item.is_some())
            .unwrap_or(false)
    }

    pub fn get(&self, id: Id<T>) -> Option<&T> {
        if let Some(entry) = self.entries.get(id.idx as usize) {
            if entry.gen == id.gen {
                entry.item.as_ref()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, id: Id<T>) -> Option<&mut T> {
        if let Some(entry) = self.entries.get_mut(id.idx as usize) {
            if entry.gen == id.gen {
                entry.item.as_mut()
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn ids(&self) -> impl Iterator<Item = Id<T>> + '_ { self.iter().map(|(id, _)| id) }

    pub fn values(&self) -> impl Iterator<Item = &T> + '_ { self.iter().map(|(_, item)| item) }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.iter_mut().map(|(_, item)| item)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .filter_map(move |(idx, entry)| {
                Some(Id {
                    idx: idx as u32,
                    gen: entry.gen,
                    phantom: PhantomData,
                })
                .zip(entry.item.as_ref())
            })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id<T>, &mut T)> + '_ {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(move |(idx, entry)| {
                Some(Id {
                    idx: idx as u32,
                    gen: entry.gen,
                    phantom: PhantomData,
                })
                .zip(entry.item.as_mut())
            })
    }

    pub fn insert(&mut self, item: T) -> Id<T> {
        if self.len < self.entries.len() {
            // TODO: Make this more efficient with a lookahead system
            let (idx, entry) = self
                .entries
                .iter_mut()
                .enumerate()
                .find(|(_, e)| e.item.is_none())
                .unwrap();
            entry.item = Some(item);
            assert!(entry.gen < u32::MAX);
            entry.gen += 1;
            self.len += 1;
            Id {
                idx: idx as u32,
                gen: entry.gen,
                phantom: PhantomData,
            }
        } else {
            assert!(self.entries.len() < (u32::MAX - 1) as usize);
            let id = Id {
                idx: self.entries.len() as u32,
                gen: 0,
                phantom: PhantomData,
            };
            self.entries.push(Entry {
                gen: 0,
                item: Some(item),
            });
            self.len += 1;
            id
        }
    }

    pub fn remove(&mut self, id: Id<T>) -> Option<T> {
        if let Some(item) = self
            .entries
            .get_mut(id.idx as usize)
            .and_then(|e| if e.gen == id.gen { e.item.take() } else { None })
        {
            self.len -= 1;
            Some(item)
        } else {
            None
        }
    }

    pub fn recreate_id(&self, i: u64) -> Option<Id<T>> {
        if i as usize >= self.entries.len() {
            None
        } else {
            Some(Id {
                idx: i as u32,
                gen: self
                    .entries
                    .get(i as usize)
                    .map(|e| e.gen)
                    .unwrap_or_default(),
                phantom: PhantomData,
            })
        }
    }
}
