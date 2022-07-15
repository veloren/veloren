//! Versioned admins settings files.

// NOTE: Needed to allow the second-to-last migration to call try_into().

use super::{ADMINS_FILENAME as FILENAME, MIGRATION_UPGRADE_GUARANTEE};
use crate::settings::editable::{EditableSetting, Version};
use core::convert::{Infallible, TryFrom, TryInto};
use serde::{Deserialize, Serialize};

/// NOTE: Always replace this with the latest admins version. Then update the
/// AdminsRaw, the TryFrom<AdminsRaw> for Admins, the previously most recent
/// module, and add a new module for the latest version!  Please respect the
/// migration upgrade guarantee found in the parent module with any upgrade.
pub use self::v1::*;

/// Versioned settings files, one per version (v0 is only here as an example; we
/// do not expect to see any actual v0 settings files).
#[derive(Deserialize, Serialize)]
pub enum AdminsRaw {
    V0(v0::Admins),
    V1(Admins),
}

impl From<Admins> for AdminsRaw {
    fn from(value: Admins) -> Self {
        // Replace variant with that of current latest version.
        Self::V1(value)
    }
}

impl TryFrom<AdminsRaw> for (Version, Admins) {
    type Error = <Admins as EditableSetting>::Error;

    fn try_from(value: AdminsRaw) -> Result<Self, <Admins as EditableSetting>::Error> {
        use AdminsRaw::*;
        Ok(match value {
            // Old versions
            V0(value) => (Version::Old, value.try_into()?),
            // Latest version (move to old section using the pattern of other old version when it
            // is no longer latest).
            V1(mut value) => (value.validate()?, value),
        })
    }
}

type Final = Admins;

impl EditableSetting for Admins {
    type Error = Infallible;
    type Legacy = legacy::Admins;
    type Setting = AdminsRaw;

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
    pub struct Admins(pub(super) HashSet<Uuid>);

    impl From<Admins> for Final {
        /// Legacy migrations can be migrated to the latest version through the
        /// process of "chaining" migrations, starting from
        /// `next::Admins`.
        ///
        /// Note that legacy files are always valid, which is why we implement
        /// From rather than TryFrom.
        fn from(value: Admins) -> Self {
            next::Admins::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE)
        }
    }
}

/// This module represents a admins version that isn't actually used.  It is
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
    pub struct Admins(pub(super) HashSet<Uuid>);

    impl Admins {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Admins) -> Self { Admins(prev.0) }

        /// Perform any needed validation on this admins that can't be done
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
    impl TryFrom<Admins> for Final {
        type Error = <Final as EditableSetting>::Error;

        #[allow(clippy::useless_conversion)]
        fn try_from(mut value: Admins) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Admins::migrate(value)
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
    /// to one used in another versioned store (like banlist::Role), we
    /// *must* have our own versioned copy! This ensures that if there's an
    /// update to the role somewhere else, the conversion function between
    /// them will break, letting people make an intelligent decision.
    ///
    /// In particular, *never remove variants from this enum* (or any other enum
    /// in a versioned settings file) without bumping the version and
    /// writing a migration that understands how to properly deal with
    /// existing instances of the old variant (you can delete From instances
    /// for the old variants at this point).  Otherwise, we will lose
    /// compatibility with old settings files, since we won't be able to
    /// deserialize them!
    #[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialOrd, PartialEq, Serialize)]
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
    /// NOTE: This does not include info structs like other settings, because we
    /// (deliberately) provide no interface for creating new mods or admins
    /// except through the command line, ensuring that the host of the
    /// server has total control over these things and avoiding the creation
    /// of code paths to alter the admin list that are accessible during normal
    /// gameplay.
    pub struct AdminRecord {
        /// NOTE: Should only be None for migrations from legacy data.
        pub username_when_admined: Option<String>,
        /// Date that the user was given this role.
        pub date: DateTime<Utc>,
        pub role: Role,
    }

    #[derive(Clone, Deserialize, Serialize, Default)]
    #[serde(transparent)]
    /// NOTE: Records should only be unavailable for cases where we are
    /// migration from a legacy version.
    pub struct Admins(pub(super) HashMap<Uuid, AdminRecord>);

    impl Deref for Admins {
        type Target = HashMap<Uuid, AdminRecord>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for Admins {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl Admins {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Admins) -> Self {
            // The role assignment date for migrations from legacy is the current one; we
            // could record that they actually have an unknown start date, but
            // this would just complicate the format.
            let date = Utc::now();
            Admins(
                prev.0
                    .into_iter()
                    .map(|uid| {
                        (uid, AdminRecord {
                            date,
                            // We don't have username information for old admin records.
                            username_when_admined: None,
                            // All legacy roles are Admin, because we didn't have any other roles at
                            // the time.
                            role: Role::Admin,
                        })
                    })
                    .collect(),
            )
        }

        /// Perform any needed validation on this admins that can't be done
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
    /* impl TryFrom<Admins> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: Admins) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Admins::migrate(value).try_into().expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    } */
}
