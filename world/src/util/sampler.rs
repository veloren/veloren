pub trait Sampler: Sized {
    type Index;
    type Sample;

    fn get(&mut self, index: Self::Index) -> Self::Sample;
}
