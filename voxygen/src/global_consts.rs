// Library
use gfx::{
    // Urgh
    gfx_defines,
    gfx_constant_struct_meta,
    gfx_impl_struct_meta,
};

gfx_defines! {
    constant GlobalConsts {
        view_mat: [[f32; 4]; 4] = "view_mat",
        proj_mat: [[f32; 4]; 4] = "proj_mat",
        cam_origin: [f32; 4] = "cam_origin",
        play_origin: [f32; 4] = "play_origin",
        view_distance: [f32; 4] = "view_distance",
        time: [f32; 4] = "time",
    }
}
