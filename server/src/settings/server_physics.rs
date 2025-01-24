//! List of players which are not allowed to use client side physics, to punish
//! abuse

use super::{EditableSetting, SERVER_PHYSICS_FORCE_FILENAME as FILENAME, editable::Version};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
pub use v0::*;

#[derive(Deserialize, Serialize)]
pub enum ServerPhysicsForceListRaw {
    V0(ServerPhysicsForceList),
}

impl TryFrom<ServerPhysicsForceListRaw> for (Version, ServerPhysicsForceList) {
    type Error = <ServerPhysicsForceList as EditableSetting>::Error;

    fn try_from(value: ServerPhysicsForceListRaw) -> Result<Self, Self::Error> {
        use ServerPhysicsForceListRaw::*;
        Ok(match value {
            V0(mut value) => (value.validate()?, value),
        })
    }
}

impl From<ServerPhysicsForceList> for ServerPhysicsForceListRaw {
    fn from(value: ServerPhysicsForceList) -> Self { Self::V0(value) }
}

impl EditableSetting for ServerPhysicsForceList {
    type Error = Infallible;
    type Legacy = ServerPhysicsForceList;
    type Setting = ServerPhysicsForceListRaw;

    const FILENAME: &'static str = FILENAME;
}

type Latest = ServerPhysicsForceList;

mod v0 {
    use super::Latest;
    use authc::Uuid;
    use serde::{Deserialize, Serialize};
    use std::{
        collections::HashMap,
        ops::{Deref, DerefMut},
    };

    use crate::settings::{EditableSetting, editable::Version};

    #[derive(Clone, Deserialize, Serialize, Debug)]
    pub struct ServerPhysicsForceRecord {
        /// Moderator/Admin who forced the player to server authoritative
        /// physics, none if applied via the server (currently not possible)
        pub by: Option<(Uuid, String)>,
        pub reason: Option<String>,
    }

    #[derive(Clone, Deserialize, Serialize, Default)]
    pub struct ServerPhysicsForceList(HashMap<Uuid, ServerPhysicsForceRecord>);

    impl Deref for ServerPhysicsForceList {
        type Target = HashMap<Uuid, ServerPhysicsForceRecord>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ServerPhysicsForceList {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl ServerPhysicsForceList {
        pub(super) fn validate(&mut self) -> Result<Version, <Latest as EditableSetting>::Error> {
            Ok(Version::Latest)
        }
    }
}
