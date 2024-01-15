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
pub use self::v2::*;

/// Versioned settings files, one per version (v0 is only here as an example; we
/// do not expect to see any actual v0 settings files).
#[derive(Deserialize, Serialize)]
pub enum ServerDescriptionRaw {
    V0(v0::ServerDescription),
    V1(v1::ServerDescription),
    V2(ServerDescriptions),
}

impl From<ServerDescriptions> for ServerDescriptionRaw {
    fn from(value: ServerDescriptions) -> Self {
        // Replace variant with that of current latest version.
        Self::V2(value)
    }
}

impl TryFrom<ServerDescriptionRaw> for (Version, ServerDescriptions) {
    type Error = <ServerDescriptions as EditableSetting>::Error;

    fn try_from(
        value: ServerDescriptionRaw,
    ) -> Result<Self, <ServerDescriptions as EditableSetting>::Error> {
        use ServerDescriptionRaw::*;
        Ok(match value {
            // Old versions
            V0(value) => (Version::Old, value.try_into()?),
            V1(value) => (Version::Old, value.try_into()?),
            // Latest version (move to old section using the pattern of other old version when it
            // is no longer latest).
            V2(mut value) => (value.validate()?, value),
        })
    }
}

type Final = ServerDescriptions;

impl EditableSetting for ServerDescriptions {
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

    use super::v2 as next;
    impl TryFrom<ServerDescription> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: ServerDescription) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::ServerDescriptions::migrate(value))
        }
    }
}

mod v2 {
    use std::collections::HashMap;

    use super::{v1 as prev, Final};
    use crate::settings::editable::{EditableSetting, Version};
    use serde::{Deserialize, Serialize};

    /// Map of all localized [`ServerDescription`]s
    #[derive(Clone, Deserialize, Serialize)]
    pub struct ServerDescriptions {
        pub default_locale: String,
        pub descriptions: HashMap<String, ServerDescription>,
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct ServerDescription {
        pub motd: String,
        pub rules: Option<String>,
    }

    impl Default for ServerDescriptions {
        fn default() -> Self {
            Self {
                default_locale: "en".to_string(),
                descriptions: HashMap::from([("en".to_string(), ServerDescription::default())]),
            }
        }
    }

    impl Default for ServerDescription {
        fn default() -> Self {
            Self {
                motd: "This is the best Veloren server".into(),
                rules: None,
            }
        }
    }

    impl ServerDescriptions {
        fn unwrap_locale_or_default<'a, 'b: 'a>(&'b self, locale: Option<&'a str>) -> &'a str {
            locale.map_or(&self.default_locale, |locale| {
                if self.descriptions.contains_key(locale) {
                    locale
                } else {
                    &self.default_locale
                }
            })
        }

        pub fn get(&self, locale: Option<&str>) -> Option<&ServerDescription> {
            self.descriptions.get(self.unwrap_locale_or_default(locale))
        }

        /// Attempts to get the rules in the specified locale, falls back to
        /// `default_locale` if no rules were specified in this locale
        pub fn get_rules(&self, locale: Option<&str>) -> Option<&str> {
            self.descriptions
                .get(self.unwrap_locale_or_default(locale))
                .and_then(|d| d.rules.as_deref())
                .or_else(|| {
                    self.descriptions
                        .get(&self.default_locale)?
                        .rules
                        .as_deref()
                })
        }
    }

    impl ServerDescriptions {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::ServerDescription) -> Self {
            Self {
                default_locale: "en".to_string(),
                descriptions: HashMap::from([("en".to_string(), ServerDescription {
                    motd: prev.0,
                    rules: None,
                })]),
            }
        }

        /// Perform any needed validation on this server description that can't
        /// be done using parsing.
        ///
        /// The returned version being "Old" indicates the loaded setting has
        /// been modified during validation (this is why validate takes
        /// `&mut self`).
        pub(super) fn validate(&mut self) -> Result<Version, <Final as EditableSetting>::Error> {
            if self.descriptions.is_empty() {
                *self = Self::default();
                Ok(Version::Old)
            } else if !self.descriptions.contains_key(&self.default_locale) {
                // default locale not present, select the a random one (as ordering in hashmaps
                // isn't predictable)
                self.default_locale = self
                    .descriptions
                    .keys()
                    .next()
                    .expect("We know descriptions isn't empty")
                    .to_string();
                Ok(Version::Old)
            } else {
                Ok(Version::Latest)
            }
        }
    }

    // NOTE: Whenever there is a version upgrade, copy this note as well as the
    // commented-out code below to the next version, then uncomment the code
    // for this version.
    /*
    use super::{v3 as next, MIGRATION_UPGRADE_GUARANTEE};
    impl TryFrom<ServerDescription> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: ServerDescription) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::ServerDescription::migrate(value).try_into().expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    } */
}
