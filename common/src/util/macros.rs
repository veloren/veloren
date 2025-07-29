#[macro_export]
macro_rules! match_some {
    ($expr:expr $(, $pat:pat => $ret_expr:expr)+ $(,)?) => {
        match $expr {
            $($pat => Some($ret_expr),)+
            _ => None,
        }
    };
}
