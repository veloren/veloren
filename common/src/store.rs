use std::{
    fmt,
    hash,
    ops::{Index, IndexMut},
    cmp::{PartialEq, Eq},
    marker::PhantomData,
};

pub struct Id<T>(usize, PhantomData<T>);

impl<T> Id<T> {
    pub fn id(&self) -> usize { self.0 }
}

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { Self(self.0, PhantomData) }
}
impl<T> Eq for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}
impl<T> fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Id<{}>({})", std::any::type_name::<T>(), self.0)
    }
}
impl<T> hash::Hash for Id<T> {
    fn hash<H: hash::Hasher>(&self, h: &mut H) {
        self.0.hash(h);
    }
}

pub struct Store<T> {
    items: Vec<T>,
}

impl<T> Default for Store<T> {
    fn default() -> Self { Self { items: Vec::new() } }
}

impl<T> Store<T> {
    pub fn get(&self, id: Id<T>) -> &T { self.items.get(id.0).unwrap() }

    pub fn get_mut(&mut self, id: Id<T>) -> &mut T { self.items.get_mut(id.0).unwrap() }

    pub fn ids(&self) -> impl Iterator<Item = Id<T>> {
        (0..self.items.len()).map(|i| Id(i, PhantomData))
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> { self.items.iter() }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> { self.items.iter_mut() }

    pub fn iter_ids(&self) -> impl Iterator<Item = (Id<T>, &T)> {
        self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| (Id(i, PhantomData), item))
    }

    pub fn insert(&mut self, item: T) -> Id<T> {
        let id = Id(self.items.len(), PhantomData);
        self.items.push(item);
        id
    }
}

impl<T> Index<Id<T>> for Store<T> {
    type Output = T;
    fn index(&self, id: Id<T>) -> &Self::Output { self.get(id) }
}

impl<T> IndexMut<Id<T>> for Store<T> {
    fn index_mut(&mut self, id: Id<T>) -> &mut Self::Output { self.get_mut(id) }
}
