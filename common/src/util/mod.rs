mod color;
pub mod dir;
pub mod find_dist;
mod option;
pub mod plane;
pub mod projection;
/// Contains [`SpatialGrid`] which is useful for accelerating queries of nearby
/// entities
mod spatial_grid;

pub const GIT_VERSION_BUILD: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));
pub const GIT_TAG_BUILD: &str = include_str!(concat!(env!("OUT_DIR"), "/gittag"));
pub const VELOREN_VERSION_STAGE: &str = "Pre-Alpha";

lazy_static::lazy_static! {
    pub static ref GIT_VERSION: String = if GIT_VERSION_BUILD.is_empty() {
        std::env::var("VELOREN_GIT_VERSION").expect("failed to get git version")
    } else {
        GIT_VERSION_BUILD.to_string()
    };
    pub static ref GIT_TAG: String = if GIT_TAG_BUILD.is_empty() {
        std::env::var("VELOREN_GIT_TAG").expect("failed to get git tag")
    } else {
        GIT_TAG_BUILD.to_string()
    };
    pub static ref GIT_HASH: &'static str = GIT_VERSION.split('/').next().expect("failed to retrieve git_hash!");
    static ref GIT_DATETIME: &'static str = GIT_VERSION.split('/').nth(1).expect("failed to retrieve git_datetime!");
    pub static ref GIT_DATE: String = GIT_DATETIME.split('-').take(3).collect::<Vec<&str>>().join("-");
    pub static ref GIT_TIME: &'static str = GIT_DATETIME.split('-').nth(3).expect("failed to retrieve git_time!");
    pub static ref DISPLAY_VERSION: String = if GIT_TAG.is_empty() {
        format!("{}-{}", VELOREN_VERSION_STAGE, *GIT_DATE)
    } else {
        format!("{}-{}", VELOREN_VERSION_STAGE, GIT_TAG)
    };
    pub static ref DISPLAY_VERSION_LONG: String = if GIT_TAG.is_empty() {
        format!("{} ({})", DISPLAY_VERSION.as_str(), *GIT_HASH)
    } else {
        format!("{} ({})", DISPLAY_VERSION.as_str(), GIT_VERSION)
    };
}

pub use color::*;
pub use dir::*;
pub use option::either_with;
pub use plane::Plane;
pub use projection::Projection;
pub use spatial_grid::SpatialGrid;
