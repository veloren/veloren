//! Versioned server description settings files.

// NOTE: Needed to allow the second-to-last migration to call try_into().

use super::{MIGRATION_UPGRADE_GUARANTEE, SERVER_DESCRIPTION_FILENAME as FILENAME};
use crate::settings::editable::{EditableSetting, Version};
use core::convert::{Infallible, TryFrom, TryInto};
use serde::{Deserialize, Serialize};

/// NOTE: Always replace this with the latest server description version. Then
/// update the ServerDescriptionRaw, the TryFrom<ServerDescriptionRaw> for
/// ServerDescription, the previously most recent module, and add a new module
/// for the latest version!  Please respect the migration upgrade guarantee
/// found in the parent module with any upgrade.
pub use self::v1::*;

/// Versioned settings files, one per version (v0 is only here as an example; we
/// do not expect to see any actual v0 settings files).
#[derive(Deserialize, Serialize)]
pub enum ServerDescriptionRaw {
    V0(v0::ServerDescription),
    V1(ServerDescription),
}

impl From<ServerDescription> for ServerDescriptionRaw {
    fn from(value: ServerDescription) -> Self {
        // Replace variant with that of current latest version.
        Self::V1(value)
    }
}

impl TryFrom<ServerDescriptionRaw> for (Version, ServerDescription) {
    type Error = <ServerDescription as EditableSetting>::Error;

    fn try_from(
        value: ServerDescriptionRaw,
    ) -> Result<Self, <ServerDescription as EditableSetting>::Error> {
        use ServerDescriptionRaw::*;
        Ok(match value {
            // Old versions
            V0(value) => (Version::Old, value.try_into()?),
            // Latest version (move to old section using the pattern of other old version when it
            // is no longer latest).
            V1(mut value) => (value.validate()?, value),
        })
    }
}

type Final = ServerDescription;

impl EditableSetting for ServerDescription {
    type Error = Infallible;
    type Legacy = legacy::ServerDescription;
    type Setting = ServerDescriptionRaw;

    const FILENAME: &'static str = FILENAME;
}

mod legacy {
    use super::{v0 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use core::convert::TryInto;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct ServerDescription(pub(super) String);

    impl From<ServerDescription> for Final {
        /// Legacy migrations can be migrated to the latest version through the
        /// process of "chaining" migrations, starting from
        /// `next::ServerDescription`.
        ///
        /// Note that legacy files are always valid, which is why we implement
        /// From rather than TryFrom.
        fn from(value: ServerDescription) -> Self {
            next::ServerDescription::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE)
        }
    }
}

/// This module represents a server description version that isn't actually
/// used.  It is here and part of the migration process to provide an example
/// for how to perform a migration for an old version; please use this as a
/// reference when constructing new migrations.
mod v0 {
    use super::{legacy as prev, v1 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use crate::settings::editable::{EditableSetting, Version};
    use core::convert::{TryFrom, TryInto};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct ServerDescription(pub(super) String);

    impl ServerDescription {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::ServerDescription) -> Self { ServerDescription(prev.0) }

        /// Perform any needed validation on this server description that can't
        /// be done using parsing.
        ///
        /// The returned version being "Old" indicates the loaded setting has
        /// been modified during validation (this is why validate takes
        /// `&mut self`).
        pub(super) fn validate(&mut self) -> Result<Version, <Final as EditableSetting>::Error> {
            Ok(Version::Latest)
        }
    }

    /// Pretty much every TryFrom implementation except that of the very last
    /// version should look exactly like this.
    impl TryFrom<ServerDescription> for Final {
        type Error = <Final as EditableSetting>::Error;

        #[allow(clippy::useless_conversion)]
        fn try_from(mut value: ServerDescription) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::ServerDescription::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    }
}

mod v1 {
    use super::{v0 as prev, Final};
    use crate::settings::editable::{EditableSetting, Version};
    use core::ops::{Deref, DerefMut};
    use serde::{Deserialize, Serialize};
    /* use super::v2 as next; */

    #[derive(Clone, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct ServerDescription(pub(super) String);

    impl Default for ServerDescription {
        fn default() -> Self { Self("This is the best Veloren server".into()) }
    }

    impl Deref for ServerDescription {
        type Target = String;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ServerDescription {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl ServerDescription {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::ServerDescription) -> Self { ServerDescription(prev.0) }

        /// Perform any needed validation on this server description that can't
        /// be done using parsing.
        ///
        /// The returned version being "Old" indicates the loaded setting has
        /// been modified during validation (this is why validate takes
        /// `&mut self`).
        pub(super) fn validate(&mut self) -> Result<Version, <Final as EditableSetting>::Error> {
            Ok(Version::Latest)
        }
    }

    // NOTE: Whenever there is a version upgrade, copy this note as well as the
    // commented-out code below to the next version, then uncomment the code
    // for this version.
    /* impl TryFrom<ServerDescription> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: ServerDescription) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::ServerDescription::migrate(value).try_into().expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    } */
}
