/// A match that returns an option and has a wildcard pattern for `None`.
///
/// # Example
/// ```
/// use veloren_common::match_some;
///
/// let x = 5;
/// let res = match_some!(x,
///    1 => true,
///    5 => false,
/// );
///
/// assert_eq!(res, Some(false));
/// ```
#[macro_export]
macro_rules! match_some {
    ($expr:expr $(, $pat:pat => $ret_expr:expr)+ $(,)?) => {
        match $expr {
            $($pat => Some($ret_expr),)+
            _ => None,
        }
    };
}
