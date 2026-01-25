/// Divides two floats or returns `None` if the denominator is zero or if any
/// inputs are `NaN`. Rust provides `checked_div`, but not for floats, so this
/// function is needed.
///
/// Here is an example:
///
/// ```
/// use veloren_common::util::div::checked_div;
///
/// assert!(checked_div(50.0_f32, 0.0_f32).is_none());
/// ```
pub fn checked_div<T>(num: T, den: T) -> Option<T>
where
    T: num_traits::Float,
{
    if den.is_zero() || den.is_nan() || num.is_nan() {
        None
    } else {
        Some(num / den)
    }
}
