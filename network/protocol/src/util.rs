/// Used for storing Buffers in a QUIC
#[derive(Debug)]
pub struct SortedVec<K, V> {
    pub data: Vec<(K, V)>,
}

impl<K, V> Default for SortedVec<K, V> {
    fn default() -> Self { Self { data: vec![] } }
}

impl<K, V> SortedVec<K, V>
where
    K: Ord + Copy,
{
    pub fn insert(&mut self, k: K, v: V) {
        self.data.push((k, v));
        self.data.sort_by_key(|&(k, _)| k);
    }

    pub fn delete(&mut self, k: &K) -> Option<V> {
        if let Ok(i) = self.data.binary_search_by_key(k, |&(k, _)| k) {
            Some(self.data.remove(i).1)
        } else {
            None
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        if let Ok(i) = self.data.binary_search_by_key(k, |&(k, _)| k) {
            Some(&self.data[i].1)
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        if let Ok(i) = self.data.binary_search_by_key(k, |&(k, _)| k) {
            Some(&mut self.data[i].1)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorted_vec() {
        let mut vec = SortedVec::default();
        vec.insert(10, "Hello");
        println!("{:?}", vec.data);
        vec.insert(30, "World");
        println!("{:?}", vec.data);
        vec.insert(20, " ");
        println!("{:?}", vec.data);
        assert_eq!(vec.data[0].1, "Hello");
        assert_eq!(vec.data[1].1, " ");
        assert_eq!(vec.data[2].1, "World");
        assert_eq!(vec.get(&30), Some(&"World"));
        assert_eq!(vec.get_mut(&20), Some(&mut " "));
        assert_eq!(vec.get(&10), Some(&"Hello"));
        assert_eq!(vec.delete(&40), None);
        assert_eq!(vec.delete(&10), Some("Hello"));
        assert_eq!(vec.delete(&10), None);
        assert_eq!(vec.get(&30), Some(&"World"));
        assert_eq!(vec.get_mut(&20), Some(&mut " "));
        assert_eq!(vec.get(&10), None);
    }
}
