use common::uid::Uid;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum RetrieveError {
    EcsAccessError(EcsAccessError),
    OtherError(String),
    DataReadError,
    BincodeError(String),
    InvalidType,
}

impl core::fmt::Display for RetrieveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RetrieveError::EcsAccessError(e) => {
                write!(f, "RetrieveError: {}", e)
            },
            RetrieveError::OtherError(e) => {
                write!(f, "RetrieveError: Unknown error: {}", e)
            },
            RetrieveError::DataReadError => {
                write!(
                    f,
                    "RetrieveError: Can't pass data through WASM FFI: WASM Memory is corrupted"
                )
            },
            RetrieveError::BincodeError(e) => {
                write!(f, "RetrieveError: Bincode error: {}", e)
            },
            RetrieveError::InvalidType => {
                write!(
                    f,
                    "RetrieveError: This type wasn't expected as the result for this Retrieve"
                )
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EcsAccessError {
    EcsPointerNotAvailable,
    EcsComponentNotFound(Uid, String),
    EcsResourceNotFound(String),
    EcsEntityNotFound(Uid),
}

impl core::fmt::Display for EcsAccessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EcsAccessError::EcsPointerNotAvailable => {
                write!(f, "EcsAccessError can't read the ECS pointer")
            },
            EcsAccessError::EcsComponentNotFound(a, b) => {
                write!(
                    f,
                    "EcsAccessError can't find component {} for entity from UID {}",
                    b, a
                )
            },
            EcsAccessError::EcsResourceNotFound(a) => {
                write!(f, "EcsAccessError can't find resource {}", a)
            },
            EcsAccessError::EcsEntityNotFound(a) => {
                write!(f, "EcsAccessError can't find entity from UID {}", a)
            },
        }
    }
}
