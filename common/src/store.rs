use rayon::prelude::*;
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    fmt, hash,
    marker::PhantomData,
    ops::{Index, IndexMut},
};

// NOTE: We use u64 to make sure we are consistent across all machines.  We
// assume that usize fits into 8 bytes.
pub struct Id<T>(u64, PhantomData<T>);

impl<T> Id<T> {
    pub fn id(&self) -> u64 { self.0 }
}

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { Self(self.0, PhantomData) }
}
impl<T> Eq for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> Ordering { self.0.cmp(&(other.0)) }
}
impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Id<{}>({})", std::any::type_name::<T>(), self.0)
    }
}
impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, h: &mut H) { self.0.hash(h); }
}

pub struct Store<T> {
    items: Vec<T>,
}

impl<T> Default for Store<T> {
    fn default() -> Self { Self { items: Vec::new() } }
}

impl<T> Store<T> {
    pub fn get(&self, id: Id<T>) -> &T {
        // NOTE: Safe conversion, because it came from usize.
        self.items.get(id.0 as usize).unwrap()
    }

    pub fn get_mut(&mut self, id: Id<T>) -> &mut T {
        // NOTE: Safe conversion, because it came from usize.
        self.items.get_mut(id.0 as usize).unwrap()
    }

    pub fn ids(&self) -> impl Iterator<Item = Id<T>> {
        (0..self.items.len()).map(|i| Id(i as u64, PhantomData))
    }

    pub fn values(&self) -> impl ExactSizeIterator<Item = &T> { self.items.iter() }

    pub fn values_mut(&mut self) -> impl ExactSizeIterator<Item = &mut T> { self.items.iter_mut() }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &T)> { self.ids().zip(self.values()) }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id<T>, &mut T)> {
        self.ids().zip(self.values_mut())
    }

    pub fn insert(&mut self, item: T) -> Id<T> {
        // NOTE: Assumes usize fits into 8 bytes.
        let id = Id(self.items.len() as u64, PhantomData);
        self.items.push(item);
        id
    }

    pub fn recreate_id(&self, i: u64) -> Option<Id<T>> {
        if i as usize >= self.items.len() {
            None
        } else {
            Some(Id::<T>(i, PhantomData))
        }
    }
}

impl<T: Send + Sync> Store<T> {
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (Id<T>, &mut T)> {
        self.items
            .par_iter_mut()
            .enumerate()
            .map(|(idx, obj)| (Id(idx as u64, PhantomData), obj))
    }
}

impl<T> Index<Id<T>> for Store<T> {
    type Output = T;

    fn index(&self, id: Id<T>) -> &Self::Output { self.get(id) }
}

impl<T> IndexMut<Id<T>> for Store<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output { self.get_mut(id) }
}
