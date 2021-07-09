use crate::util::DHashMap;
use std::hash::Hash;

/** A static table of known values where any key always maps to a single value.

It's not really intended to be a "collection" in the same way that, say, HashMap or Vec is
Since it's not intended to have a way of expressing that a value is not present (hence the default behaviour)
It's really quite specifically tailored to its application in the economy code where it wouldn't make sense to not have certain entries
Store is a bit different in that it is the one to generate an index, and so it can hold as many things as you like
Whereas with MapVec, we always know the index ahead of time.
**/

#[derive(Clone, Debug)]
pub struct MapVec<K, T> {
    /// We use this hasher (FxHasher32) because
    /// (1) we don't care about DDOS attacks (ruling out SipHash);
    /// (2) we care about determinism across computers (ruling out AAHash);
    /// (3) we have 1-byte keys (for which FxHash is supposedly fastest).
    entries: DHashMap<K, T>,
    default: T,
}

/// Need manual implementation of Default since K doesn't need that bound.
impl<K, T: Default> Default for MapVec<K, T> {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            default: Default::default(),
        }
    }
}

impl<K: Copy + Eq + Hash, T: Clone> MapVec<K, T> {
    pub fn from_list<'a>(i: impl IntoIterator<Item = &'a (K, T)>, default: T) -> Self
    where
        K: 'a,
        T: 'a,
    {
        Self {
            entries: i.into_iter().cloned().collect(),
            default,
        }
    }

    pub fn from_iter(i: impl Iterator<Item = (K, T)>, default: T) -> Self {
        Self {
            entries: i.collect(),
            default,
        }
    }

    pub fn from_default(default: T) -> Self {
        Self {
            entries: DHashMap::default(),
            default,
        }
    }

    pub fn get_mut(&mut self, entry: K) -> &mut T {
        let default = &self.default;
        self.entries.entry(entry).or_insert_with(|| default.clone())
    }

    pub fn get(&self, entry: K) -> &T { self.entries.get(&entry).unwrap_or(&self.default) }

    pub fn map<U: Default>(self, mut f: impl FnMut(K, T) -> U) -> MapVec<K, U> {
        MapVec {
            entries: self
                .entries
                .into_iter()
                .map(|(s, v)| (s, f(s, v)))
                .collect(),
            default: U::default(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &T)> + '_ {
        self.entries.iter().map(|(s, v)| (*s, v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &mut T)> + '_ {
        self.entries.iter_mut().map(|(s, v)| (*s, v))
    }
}

impl<K: Copy + Eq + Hash, T: Clone> std::ops::Index<K> for MapVec<K, T> {
    type Output = T;

    fn index(&self, entry: K) -> &Self::Output { self.get(entry) }
}

impl<K: Copy + Eq + Hash, T: Clone> std::ops::IndexMut<K> for MapVec<K, T> {
    fn index_mut(&mut self, entry: K) -> &mut Self::Output { self.get_mut(entry) }
}
