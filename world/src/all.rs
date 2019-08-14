use serde_derive::{Deserialize, Serialize};

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum ForestKind {
    Palm,
    Savannah,
    Oak,
    Pine,
    SnowPine,
    Mangrove,
}
