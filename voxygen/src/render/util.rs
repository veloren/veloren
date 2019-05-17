// TODO: Get rid of this ugliness.
#[rustfmt::skip]
pub fn arr_to_mat(arr: [f32; 16]) -> [[f32; 4]; 4] {
    [
        [arr[ 0], arr[ 1], arr[ 2], arr[ 3]],
        [arr[ 4], arr[ 5], arr[ 6], arr[ 7]],
        [arr[ 8], arr[ 9], arr[10], arr[11]],
        [arr[12], arr[13], arr[14], arr[15]],
    ]
}
