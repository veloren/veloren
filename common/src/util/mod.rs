mod color;
pub mod dir;
pub mod find_dist;
mod grid_hasher;
pub mod lines;
mod macros;
mod option;
pub mod plane;
pub mod projection;
mod ron_recover;
/// Contains [`SpatialGrid`] which is useful for accelerating queries of nearby
/// entities
mod spatial_grid;

pub const VELOREN_GIT_VERSION_BUILD: &str = env!("VELOREN_GIT_VERSION");
pub const VELOREN_VERSION_STAGE: &str = "Pre-Alpha";

lazy_static::lazy_static! {
    pub static ref VELOREN_GIT_VERSION: String =
        std::env::var("VELOREN_GIT_VERSION").unwrap_or_else(|_| VELOREN_GIT_VERSION_BUILD.to_string());
    pub static ref GIT_TAG: &'static str = VELOREN_GIT_VERSION.split('/').next().expect("failed to retrieve git_tag!");
    pub static ref GIT_HASH: &'static str = VELOREN_GIT_VERSION.split('/').nth(1).expect("failed to retrieve git_hash!");
    static ref GIT_DATETIME: &'static str = VELOREN_GIT_VERSION.split('/').nth(2).expect("failed to retrieve git_datetime!");
    pub static ref GIT_DATE: String = GIT_DATETIME.split('-').take(3).collect::<Vec<&str>>().join("-");
    pub static ref GIT_TIME: &'static str = GIT_DATETIME.split('-').nth(3).expect("failed to retrieve git_time!");
    pub static ref GIT_DATE_TIMESTAMP: i64 =
        NaiveDateTime::parse_from_str(*GIT_DATETIME, "%Y-%m-%d-%H:%M")
            .expect("Invalid date")
            .and_utc().timestamp();
    pub static ref DISPLAY_VERSION: String = if GIT_TAG.is_empty() {
        format!("{}-{}", VELOREN_VERSION_STAGE, *GIT_DATE)
    } else {
        format!("{}-{}", VELOREN_VERSION_STAGE, *GIT_TAG)
    };
    pub static ref DISPLAY_VERSION_LONG: String = format!("{} ({})", DISPLAY_VERSION.as_str(), *GIT_HASH);
}

use chrono::NaiveDateTime;
pub use color::*;
pub use dir::*;
pub use grid_hasher::GridHasher;
pub use option::either_with;
pub use plane::Plane;
pub use projection::Projection;
pub use ron_recover::ron_from_path_recoverable;
pub use spatial_grid::SpatialGrid;
