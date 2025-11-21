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

pub const VELOREN_VERSION_STAGE: &str = "Pre-Alpha";
const VELOREN_GIT_VERSION_BUILD: &str = env!("VELOREN_GIT_VERSION");

use std::str::FromStr;
lazy_static::lazy_static! {
    static ref VELOREN_GIT_VERSION: String =
        std::env::var("VELOREN_GIT_VERSION").unwrap_or_else(|_| VELOREN_GIT_VERSION_BUILD.to_string());
    pub static ref GIT_TAG: &'static str = VELOREN_GIT_VERSION.split('/').next().expect("failed to retrieve git_tag!");
    pub static ref GIT_HASH: u32 = u32::from_str_radix(VELOREN_GIT_VERSION.split('/').nth(1).expect("failed to retrieve git_hash!"), 16).expect("invalid git_hash!");
    pub static ref GIT_TIMESTAMP: i64 = i64::from_str(VELOREN_GIT_VERSION.split('/').nth(2).expect("failed to retrieve git_timestamp!")).expect("invalid git_timestamp!");
    pub static ref DISPLAY_VERSION: String = if GIT_TAG.is_empty() {
        make_display_version(*GIT_HASH, *GIT_TIMESTAMP)
    } else {
        append_date(*GIT_TAG, *GIT_TIMESTAMP)
    };
}

pub fn make_display_version(hash: u32, timestamp: i64) -> String {
    append_date(&format!("{:x}", hash), timestamp)
}

fn append_date(version: &str, timestamp: i64) -> String {
    use chrono::DateTime;
    if let Some(datetime) = DateTime::from_timestamp_secs(timestamp) {
        format!("{} [{}]", version, datetime.format("%F"))
    } else {
        version.to_owned()
    }
}

pub use color::*;
pub use dir::*;
pub use grid_hasher::GridHasher;
pub use option::either_with;
pub use plane::Plane;
pub use projection::Projection;
pub use ron_recover::ron_from_path_recoverable;
pub use spatial_grid::SpatialGrid;
