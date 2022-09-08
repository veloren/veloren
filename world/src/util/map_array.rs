use std::cmp::PartialEq;

pub trait GenericIndex<V: Clone, const N: usize> {
    const LENGTH: usize = N;
    const VALUES: [V; N];

    fn from_usize(n: usize) -> Self;
    fn into_usize(self) -> usize;
}

#[derive(Debug)]
pub struct NotFound();

pub fn index_from_enum<E: Clone + PartialEq, I: GenericIndex<E, N>, const N: usize>(
    val: E,
) -> Result<I, NotFound> {
    I::VALUES
        .iter()
        .position(|v| val == *v)
        .ok_or(NotFound {})
        .map(I::from_usize)
}

pub fn enum_from_index<E: Clone, I: GenericIndex<E, N>, const N: usize>(idx: I) -> E {
    I::VALUES[idx.into_usize()].clone()
}

#[cfg(test)]
mod tests {
    use crate::util::map_array::{enum_from_index, index_from_enum, GenericIndex, NotFound};
    use std::{
        convert::{TryFrom, TryInto},
        ops::{Index, IndexMut},
    };

    // the Values we want to generate an Index for
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MyEnum0 {
        A,
        B,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum MyEnum {
        C(MyEnum0),
        D,
    }

    // the opaque index type into the "map"
    struct MyIndex(usize);

    impl GenericIndex<MyEnum, 3> for MyIndex {
        const VALUES: [MyEnum; MyIndex::LENGTH] =
            [MyEnum::C(MyEnum0::B), MyEnum::C(MyEnum0::A), MyEnum::D];

        fn from_usize(n: usize) -> Self { Self(n) }

        fn into_usize(self) -> usize { self.0 }
    }

    impl TryFrom<MyEnum> for MyIndex {
        type Error = NotFound;

        fn try_from(e: MyEnum) -> Result<MyIndex, NotFound> { index_from_enum(e) }
    }

    impl From<MyIndex> for MyEnum {
        fn from(idx: MyIndex) -> MyEnum { enum_from_index(idx) }
    }

    // the "map" itself
    struct MyMap<V>([V; MyIndex::LENGTH]);

    impl<V: Default + Copy> Default for MyMap<V> {
        fn default() -> Self { MyMap([V::default(); MyIndex::LENGTH]) }
    }

    impl<V> Index<MyIndex> for MyMap<V> {
        type Output = V;

        fn index(&self, index: MyIndex) -> &Self::Output { &self.0[index.0] }
    }

    impl<V> IndexMut<MyIndex> for MyMap<V> {
        fn index_mut(&mut self, index: MyIndex) -> &mut Self::Output { &mut self.0[index.0] }
    }

    impl<V> MyMap<V> {
        pub fn iter(&self) -> impl Iterator<Item = (MyIndex, &V)> + '_ {
            self.0.iter().enumerate().map(|(i, v)| (MyIndex(i), v))
        }
    }

    // test: create a map, set some values and output it
    // Output: m[C(B)]=19 m[C(A)]=42 m[D]=0
    #[test]
    fn test_map_array() {
        let mut m = MyMap::default();
        if let Ok(i) = MyEnum::C(MyEnum0::A).try_into() {
            m[i] = 42.0;
        }
        if let Ok(i) = MyEnum::C(MyEnum0::B).try_into() {
            m[i] = 19.0;
        }
        for (k, v) in m.iter() {
            let k2: MyEnum = k.into();
            println!("m[{:?}]={}", k2, *v);
        }
    }
}
