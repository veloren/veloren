//! Versioned whitelist settings files.

// NOTE: Needed to allow the second-to-last migration to call try_into().

use super::{MIGRATION_UPGRADE_GUARANTEE, WHITELIST_FILENAME as FILENAME};
use crate::settings::editable::{EditableSetting, Version};
use core::convert::{Infallible, TryFrom, TryInto};
use serde::{Deserialize, Serialize};

/// NOTE: Always replace this with the latest whitelist version. Then update the
/// WhitelistRaw, the TryFrom<WhitelistRaw> for Whitelist, the previously most
/// recent module, and add a new module for the latest version!  Please respect
/// the migration upgrade guarantee found in the parent module with any upgrade.
pub use self::v1::*;

/// Versioned settings files, one per version (v0 is only here as an example; we
/// do not expect to see any actual v0 settings files).
#[derive(Deserialize, Serialize)]
pub enum WhitelistRaw {
    V0(v0::Whitelist),
    V1(Whitelist),
}

impl From<Whitelist> for WhitelistRaw {
    fn from(value: Whitelist) -> Self {
        // Replace variant with that of current latest version.
        Self::V1(value)
    }
}

impl TryFrom<WhitelistRaw> for (Version, Whitelist) {
    type Error = <Whitelist as EditableSetting>::Error;

    fn try_from(value: WhitelistRaw) -> Result<Self, <Whitelist as EditableSetting>::Error> {
        use WhitelistRaw::*;
        Ok(match value {
            // Old versions
            V0(value) => (Version::Old, value.try_into()?),
            // Latest version (move to old section using the pattern of other old version when it
            // is no longer latest).
            V1(mut value) => (value.validate()?, value),
        })
    }
}

type Final = Whitelist;

impl EditableSetting for Whitelist {
    type Error = Infallible;
    type Legacy = legacy::Whitelist;
    type Setting = WhitelistRaw;

    const FILENAME: &'static str = FILENAME;
}

mod legacy {
    use super::{v0 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use authc::Uuid;
    use core::convert::TryInto;
    use hashbrown::HashSet;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Whitelist(pub(super) HashSet<Uuid>);

    impl From<Whitelist> for Final {
        /// Legacy migrations can be migrated to the latest version through the
        /// process of "chaining" migrations, starting from
        /// `next::Whitelist`.
        ///
        /// Note that legacy files are always valid, which is why we implement
        /// From rather than TryFrom.
        fn from(value: Whitelist) -> Self {
            next::Whitelist::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE)
        }
    }
}

/// This module represents a whitelist version that isn't actually used.  It is
/// here and part of the migration process to provide an example for how to
/// perform a migration for an old version; please use this as a reference when
/// constructing new migrations.
mod v0 {
    use super::{legacy as prev, v1 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use crate::settings::editable::{EditableSetting, Version};
    use authc::Uuid;
    use core::convert::{TryFrom, TryInto};
    use hashbrown::HashSet;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Whitelist(pub(super) HashSet<Uuid>);

    impl Whitelist {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Whitelist) -> Self { Whitelist(prev.0) }

        /// Perform any needed validation on this whitelist that can't be done
        /// using parsing.
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
    impl TryFrom<Whitelist> for Final {
        type Error = <Final as EditableSetting>::Error;

        #[allow(clippy::useless_conversion)]
        fn try_from(mut value: Whitelist) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Whitelist::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    }
}

mod v1 {
    use super::{v0 as prev, Final};
    use crate::settings::editable::{EditableSetting, Version};
    use authc::Uuid;
    use chrono::{prelude::*, Utc};
    use common::comp::AdminRole;
    use core::ops::{Deref, DerefMut};
    use hashbrown::HashMap;
    use serde::{Deserialize, Serialize};
    /* use super::v2 as next; */

    /// Important: even if the role we are storing here appears to be identical
    /// to one used in another versioned store (like admin::Role), we *must*
    /// have our own versioned copy!  This ensures that if there's an update
    /// to the role somewhere else, the conversion function between them
    /// will break, letting people make an intelligent decision.
    ///
    /// In particular, *never remove variants from this enum* (or any other enum
    /// in a versioned settings file) without bumping the version and
    /// writing a migration that understands how to properly deal with
    /// existing instances of the old variant (you can delete From instances
    /// for the old variants at this point).  Otherwise, we will lose
    /// compatibility with old settings files, since we won't be able to
    /// deserialize them!
    #[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    pub enum Role {
        Moderator = 0,
        Admin = 1,
    }

    impl From<AdminRole> for Role {
        fn from(value: AdminRole) -> Self {
            match value {
                AdminRole::Moderator => Self::Moderator,
                AdminRole::Admin => Self::Admin,
            }
        }
    }

    impl From<Role> for AdminRole {
        fn from(value: Role) -> Self {
            match value {
                Role::Moderator => Self::Moderator,
                Role::Admin => Self::Admin,
            }
        }
    }

    #[derive(Clone, Deserialize, Serialize)]
    /// NOTE: May not be present if performed from the command line or from a
    /// legacy file.
    pub struct WhitelistInfo {
        pub username_when_whitelisted: String,
        pub whitelisted_by: Uuid,
        /// NOTE: May not be up to date, if we allow username changes.
        pub whitelisted_by_username: String,
        /// NOTE: Role of the whitelisting user at the time of the ban.
        pub whitelisted_by_role: Role,
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct WhitelistRecord {
        /// Date when the user was added to the whitelist.
        pub date: DateTime<Utc>,
        /// NOTE: Should only be None for migrations from legacy data.
        pub info: Option<WhitelistInfo>,
    }

    impl WhitelistRecord {
        pub fn whitelisted_by_role(&self) -> Role {
            self.info.as_ref().map(|info| info.whitelisted_by_role)
                // We know all legacy bans were performed by an admin, since we had no other roles
                // at the time.
                .unwrap_or(Role::Admin)
        }
    }

    #[derive(Clone, Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Whitelist(pub(super) HashMap<Uuid, WhitelistRecord>);

    impl Deref for Whitelist {
        type Target = HashMap<Uuid, WhitelistRecord>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Whitelist {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl Whitelist {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Whitelist) -> Self {
            // The whitelist start date for migrations from legacy is the current one; we
            // could record that they actually have an unknown start date, but
            // this would just complicate the format.
            let date = Utc::now();
            // We don't have any of the information we need for the whitelist for legacy
            // records.
            Whitelist(
                prev.0
                    .into_iter()
                    .map(|uid| {
                        (uid, WhitelistRecord {
                            date,
                            // We have none of the information needed for WhitelistInfo for old
                            // whitelist records.
                            info: None,
                        })
                    })
                    .collect(),
            )
        }

        /// Perform any needed validation on this whitelist that can't be done
        /// using parsing.
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
    /* impl TryFrom<Whitelist> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: Whitelist) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Whitelist::migrate(value).try_into().expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    } */
}
