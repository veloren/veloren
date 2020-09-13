pub fn either_with<T, F>(opt1: Option<T>, opt2: Option<T>, f: F) -> Option<T>
where
    F: FnOnce(T, T) -> T,
{
    match (opt1, opt2) {
        (Some(v1), Some(v2)) => Some(f(v1, v2)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    }
}
