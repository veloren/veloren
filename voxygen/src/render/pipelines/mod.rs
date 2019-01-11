pub mod character;
pub mod skybox;

// Library
use gfx::{
    self,
    // Macros
    gfx_defines,
    gfx_constant_struct_meta,
    gfx_impl_struct_meta,
};
use vek::*;

gfx_defines! {
    constant Globals {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        cam_pos: [f32; 4] = "cam_pos",
        focus_pos: [f32; 4] = "focus_pos",
        view_distance: [f32; 4] = "view_distance",
        tod: [f32; 4] = "tod",
        time: [f32; 4] = "time",
    }
}

impl Globals {
    pub fn new() -> Self {
        // TODO: Get rid of this ugliness
        #[rustfmt::skip]
        fn f32_arr_to_mat(arr: [f32; 16]) -> [[f32; 4]; 4] {
            [
                [arr[ 0], arr[ 1], arr[ 2], arr[ 3]],
                [arr[ 4], arr[ 5], arr[ 6], arr[ 7]],
                [arr[ 8], arr[ 9], arr[10], arr[11]],
                [arr[12], arr[13], arr[14], arr[15]],
            ]
        }

        Self {
            view_mat: f32_arr_to_mat(Mat4::identity().into_col_array()),
            proj_mat: f32_arr_to_mat(Mat4::identity().into_col_array()),
            cam_pos: [0.0; 4],
            focus_pos: [0.0; 4],
            view_distance: [0.0; 4],
            tod: [0.0; 4],
            time: [0.0; 4],
        }
    }
}
