//! Versioned banlist settings files.

// NOTE: Needed to allow the second-to-last migration to call try_into().

use super::{BANLIST_FILENAME as FILENAME, MIGRATION_UPGRADE_GUARANTEE};
use crate::settings::editable::{EditableSetting, Version};
use authc::Uuid;
use core::convert::{TryFrom, TryInto};
use serde::{Deserialize, Serialize};

/// NOTE: Always replace this with the latest banlist version. Then update the
/// BanlistRaw, the TryFrom<BanlistRaw> for Banlist, the previously most recent
/// module, and add a new module for the latest version!  Please respect the
/// migration upgrade guarantee found in the parent module with any upgrade.
pub use self::v1::*;

/// Versioned settings files, one per version (v0 is only here as an example; we
/// do not expect to see any actual v0 settings files).
#[derive(Deserialize, Serialize)]
pub enum BanlistRaw {
    V0(v0::Banlist),
    V1(Banlist),
}

impl From<Banlist> for BanlistRaw {
    fn from(value: Banlist) -> Self {
        // Replace variant with that of current latest version.
        Self::V1(value)
    }
}

impl TryFrom<BanlistRaw> for (Version, Banlist) {
    type Error = <Banlist as EditableSetting>::Error;

    fn try_from(value: BanlistRaw) -> Result<Self, <Banlist as EditableSetting>::Error> {
        use BanlistRaw::*;
        Ok(match value {
            // Old versions
            V0(value) => (Version::Old, value.try_into()?),
            // Latest version (move to old section using the pattern of other old version when it
            // is no longer latest).
            V1(mut value) => (value.validate()?, value),
        })
    }
}

type Final = Banlist;

impl EditableSetting for Banlist {
    type Error = BanError;
    type Legacy = legacy::Banlist;
    type Setting = BanlistRaw;

    const FILENAME: &'static str = FILENAME;
}

#[derive(Clone, Copy, Debug)]
pub enum BanKind {
    Ban,
    Unban,
}

#[derive(Clone, Copy, Debug)]
pub enum BanErrorKind {
    /// An end date went past a start date.
    InvalidDateRange {
        start_date: chrono::DateTime<chrono::Utc>,
        end_date: chrono::DateTime<chrono::Utc>,
    },
    /// Cannot unban an already-unbanned user.
    AlreadyUnbanned,
    /// Permission denied to perform requested action.
    PermissionDenied(BanKind),
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BanError {
    kind: BanErrorKind,
    /// Uuid of affected user
    uuid: Uuid,
    /// Username of affected user (as of ban/unban time).
    username: String,
}

mod legacy {
    use super::{v0 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use authc::Uuid;
    use core::convert::TryInto;
    use hashbrown::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    pub struct BanRecord {
        pub username_when_banned: String,
        pub reason: String,
    }

    #[derive(Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Banlist(pub(super) HashMap<Uuid, BanRecord>);

    impl From<Banlist> for Final {
        /// Legacy migrations can be migrated to the latest version through the
        /// process of "chaining" migrations, starting from
        /// `next::Banlist`.
        ///
        /// Note that legacy files are always valid, which is why we implement
        /// From rather than TryFrom.
        fn from(value: Banlist) -> Self {
            next::Banlist::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE)
        }
    }
}

/// This module represents a banlist version that isn't actually used.  It is
/// here and part of the migration process to provide an example for how to
/// perform a migration for an old version; please use this as a reference when
/// constructing new migrations.
mod v0 {
    use super::{legacy as prev, v1 as next, Final, MIGRATION_UPGRADE_GUARANTEE};
    use crate::settings::editable::{EditableSetting, Version};
    use authc::Uuid;
    use core::convert::{TryFrom, TryInto};
    use hashbrown::HashMap;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize)]
    pub struct BanRecord {
        pub username_when_banned: String,
        pub reason: String,
    }

    #[derive(Clone, Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Banlist(pub(super) HashMap<Uuid, BanRecord>);

    impl Banlist {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Banlist) -> Self {
            Banlist(
                prev.0
                    .into_iter()
                    .map(
                        |(
                            uid,
                            prev::BanRecord {
                                username_when_banned,
                                reason,
                            },
                        )| {
                            (uid, BanRecord {
                                username_when_banned,
                                reason,
                            })
                        },
                    )
                    .collect(),
            )
        }

        /// Perform any needed validation on this banlist that can't be done
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
    impl TryFrom<Banlist> for Final {
        type Error = <Final as EditableSetting>::Error;

        #[allow(clippy::useless_conversion)]
        fn try_from(mut value: Banlist) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Banlist::migrate(value)
                .try_into()
                .expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    }
}

mod v1 {
    use super::{v0 as prev, BanError, BanErrorKind, BanKind, Final};
    use crate::settings::editable::{EditableSetting, Error, Version};
    use authc::Uuid;
    use chrono::{prelude::*, Utc};
    use common::comp::AdminRole;
    use core::{mem, ops::Deref};
    use hashbrown::{hash_map, HashMap};
    use serde::{Deserialize, Serialize};
    use tracing::warn;
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
    pub struct BanInfo {
        pub performed_by: Uuid,
        /// NOTE: May not be up to date, if we allow username changes.
        pub performed_by_username: String,
        /// NOTE: Role of the banning user at the time of the ban.
        pub performed_by_role: Role,
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct Ban {
        pub reason: String,
        /// NOTE: Should only be None for migrations from legacy data.
        pub info: Option<BanInfo>,
        /// NOTE: Should always be higher than start_date, if both are
        /// present!
        pub end_date: Option<DateTime<Utc>>,
    }

    impl Ban {
        /// Returns true if the ban is expired, false otherwise.
        pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
            self.end_date.map_or(false, |end_date| end_date <= now)
        }

        pub fn performed_by_role(&self) -> Role {
            self.info.as_ref().map(|info| info.performed_by_role)
                // We know all legacy bans were performed by an admin, since we had no other roles
                // at the time.
                .unwrap_or(Role::Admin)
        }
    }

    type Unban = BanInfo;

    #[derive(Clone, Deserialize, Serialize)]
    pub enum BanAction {
        Unban(Unban),
        Ban(Ban),
    }

    impl BanAction {
        pub fn ban(&self) -> Option<&Ban> {
            match self {
                BanAction::Unban(_) => None,
                BanAction::Ban(ban) => Some(ban),
            }
        }
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct BanRecord {
        /// Username of the user upon whom the action was performed, when it was
        /// performed.
        pub username_when_performed: String,
        pub action: BanAction,
        /// NOTE: When migrating from legacy versions, this will just be the
        /// time of the first migration (only applies to BanRecord).
        pub date: DateTime<Utc>,
    }

    impl BanRecord {
        /// Returns true if this record represents an expired ban, false
        /// otherwise.
        fn is_expired(&self, now: DateTime<Utc>) -> bool {
            match &self.action {
                BanAction::Ban(ban) => ban.is_expired(now),
                BanAction::Unban(_) => true,
            }
        }

        /// The history vector in a BanEntry is stored forwards (from oldest
        /// entry to newest), so `prev_record` is the previous entry in
        /// this vector when iterating forwards (by array index).
        ///
        /// Errors are:
        ///
        /// AlreadyUnbanned if an unban comes after anything but a ban.
        ///
        /// Permission(Unban) if an unban attempt is by a user with a lower role
        /// level than the original banning party.
        ///
        /// PermissionDenied(Ban) if a ban length is made shorter by a user with
        /// a role level than the original banning party.
        ///
        /// InvalidDateRange if the end date of the ban exceeds the start date.
        fn validate(&self, prev_record: Option<&BanRecord>) -> Result<(), BanErrorKind> {
            // Check to make sure the actions temporally line up--if they don't, we will
            // prevent warn an administrator (since this may indicate a system
            // clock issue and could require manual editing to resolve).
            // However, we will not actually invalidate the ban list for this, in case
            // this would otherwise prevent people from adding a new ban.
            //
            // We also deliberately leave the bad order intact, in case this reflects
            // history more accurately than the system clock does.
            if let Some(prev_record) = prev_record {
                if prev_record.date > self.date {
                    warn!(
                        "Ban list history is inconsistent, or a just-added ban was behind a \
                         historical entry in the ban
                          record; please investigate the contents of the file (might indicate a \
                         system clock change?)."
                    );
                }
            }
            let ban = match (&self.action, prev_record.map(|record| &record.action)) {
                // A ban is always valid if it follows an unban.
                (BanAction::Ban(ban), None) | (BanAction::Ban(ban), Some(BanAction::Unban(_))) => {
                    ban
                },
                // A ban record following a ban is valid if either the role of the person doing the
                // banning is at least the privilege level of the person who did the ban, or the
                // ban's new end time is at least the previous end time.
                (BanAction::Ban(new_ban), Some(BanAction::Ban(old_ban))) => {
                    match (new_ban.end_date, old_ban.end_date) {
                        // New role ≥ old role
                        _ if new_ban.performed_by_role() >= old_ban.performed_by_role() => new_ban,
                        // Permanent ban retracted to temp ban.
                        (Some(_), None) => {
                            return Err(BanErrorKind::PermissionDenied(BanKind::Ban));
                        },
                        // Temp ban retracted to shorter temp ban.
                        (Some(new_date), Some(old_date)) if new_date < old_date => {
                            return Err(BanErrorKind::PermissionDenied(BanKind::Ban));
                        },
                        // Anything else (extension to permanent ban, or temp ban extension to
                        // longer temp ban).
                        _ => new_ban,
                    }
                },
                // An unban record is invalid if it does not follow a ban.
                (BanAction::Unban(_), None) | (BanAction::Unban(_), Some(BanAction::Unban(_))) => {
                    return Err(BanErrorKind::AlreadyUnbanned);
                },
                // An unban record following a ban is valid if the role of the person doing the
                // unbanning is at least the privilege level of the person who did the ban.
                (BanAction::Unban(unban), Some(BanAction::Ban(ban))) => {
                    return if unban.performed_by_role >= ban.performed_by_role() {
                        Ok(())
                    } else {
                        Err(BanErrorKind::PermissionDenied(BanKind::Unban))
                    };
                },
            };

            // End date of a ban must be at least as big as the start date.
            if let Some(end_date) = ban.end_date {
                if self.date > end_date {
                    return Err(BanErrorKind::InvalidDateRange {
                        start_date: self.date,
                        end_date,
                    });
                }
            }
            Ok(())
        }
    }

    #[derive(Clone, Deserialize, Serialize)]
    pub struct BanEntry {
        /// The latest ban record for this user.
        pub current: BanRecord,
        /// Historical ban records for this user, stored in order from oldest to
        /// newest.
        pub history: Vec<BanRecord>,
        /// A *hint* about whether the system thinks this entry is expired,
        /// mostly to make it easier for someone manually going through
        /// a file to see whether an entry is currently in effect or
        /// not.  This is based off the contents of `current`.
        pub expired: bool,
    }

    impl Deref for BanEntry {
        type Target = BanRecord;

        fn deref(&self) -> &Self::Target { &self.current }
    }

    impl BanEntry {
        /// Both validates, and updates the hint bit if it's inconsistent with
        /// reality.
        ///
        /// If we were invalid, returns an error.  Otherwise, returns Ok(v),
        /// where v is Latest if the hint bit was modified, Old
        /// otherwise.
        fn validate(
            &mut self,
            now: DateTime<Utc>,
            uuid: Uuid,
        ) -> Result<Version, <Final as EditableSetting>::Error> {
            let make_error = |current_entry: &BanRecord| {
                let username = current_entry.username_when_performed.clone();
                move |kind| BanError {
                    kind,
                    uuid,
                    username,
                }
            };
            // First, go forwards through history (also forwards in terms of the iterator
            // direction), validating each entry in turn.
            let mut prev_entry = None;
            for current_entry in &self.history {
                current_entry
                    .validate(prev_entry)
                    .map_err(make_error(current_entry))?;
                prev_entry = Some(current_entry);
            }

            // History has now been validated, so validate the current entry.
            self.current
                .validate(prev_entry)
                .map_err(make_error(&self.current))?;

            // Make sure the expired hint is correct, and if not indicate that we should
            // resave the file.
            let is_expired = self.current.is_expired(now);
            if self.expired != is_expired {
                self.expired = is_expired;
                Ok(Version::Old)
            } else {
                Ok(Version::Latest)
            }
        }
    }

    #[derive(Clone, Deserialize, Serialize, Default)]
    #[serde(transparent)]
    pub struct Banlist(pub(super) HashMap<Uuid, BanEntry>);

    impl Deref for Banlist {
        type Target = HashMap<Uuid, BanEntry>;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl Banlist {
        /// Attempt to perform the ban action `action` for the user with UUID
        /// `uuid` and username `username`, starting from time `now`
        /// (the information about the banning party will
        /// be in the `action` record), with a settings file maintained at path
        /// root `data_dir`.
        ///
        /// If trying to unban an already unbanned player, or trying to ban but
        /// the ban status would not immediately change, the "overwrite"
        /// boolean should also be set to true.
        ///
        /// We try to detect duplicates (bans that would have no effect) and
        /// return None if such effects are encountered.  Otherwise, we
        /// return Some(result), which works as follows.
        ///
        /// If the ban was invalid for any reason, then neither the in-memory
        /// banlist nor the on-disk banlist are modified.  If the ban
        /// entry is valid but the file encounters an error that
        /// prevents it from being atomically written to disk, we return an
        /// error but retain the change in memory.  Otherwise, we
        /// complete successfully and atomically write the banlist to
        /// disk.
        ///
        /// Note that the IO operation is only *guaranteed* atomic in the weak
        /// sense that either the whole page is written or it isn't; we
        /// cannot guarantee that the data we read in order to modify
        /// the file was definitely up to date, so we could be missing
        /// information if the file was manually edited or a function
        /// edits it without going through the usual specs resources.
        /// So, please be careful with ad hoc modifications to the file while
        /// the server is running.
        ///
        /// Panics if provided a ban action with info set to None, as info: None
        /// should only be used for legacy records.
        ///
        /// TODO: Consider creating a new type specifically for the entry to
        /// avoid needing the precondition on info.
        #[must_use]
        pub fn ban_action(
            &mut self,
            data_dir: &std::path::Path,
            now: DateTime<Utc>,
            uuid: Uuid,
            username_when_performed: String,
            action: BanAction,
            overwrite: bool,
        ) -> Option<Result<(), Error<Final>>> {
            assert!(
                matches!(
                    action,
                    BanAction::Unban(_) | BanAction::Ban(Ban { info: Some(_), .. })
                ),
                "The info field is only None for legacy reasons--any new bans should have it set!",
            );

            let ban_record = BanRecord {
                username_when_performed,
                action,
                date: now,
            };

            // Perform an atomic edit.
            Some(
                self.edit(data_dir.as_ref(), |banlist| {
                    match banlist.0.entry(uuid) {
                        hash_map::Entry::Vacant(v) => {
                            // If this is an unban, it will have no effect, so return early.
                            if matches!(ban_record.action, BanAction::Unban(_)) {
                                return None;
                            }
                            // Otherwise, this will at least potentially have an effect (assuming it
                            // succeeds).
                            v.insert(BanEntry {
                                current: ban_record,
                                history: Vec::new(),
                                // This is a hint anyway, but expired will also be set to true
                                // before saving by the call `edit`
                                // makes to `validate` (through `try_into`), which will set it to
                                // true in the event that the ban
                                // time was so short that it expired during the interval
                                // between creating the action and saving it.
                                //
                                // TODO: Decide if we even care enough about this case to worry
                                // about the gap. Probably not, even
                                // though it does involve time!
                                expired: false,
                            });
                            Some(())
                        },
                        hash_map::Entry::Occupied(mut o) => {
                            let entry = o.get_mut();
                            // If overwrite is off, check that this entry (if successful) would
                            // actually change the ban status.
                            if !overwrite
                                && entry.current.is_expired(now) == ban_record.is_expired(now)
                            {
                                return None;
                            }
                            // Push the current (most recent) entry to the back of the history list.
                            entry
                                .history
                                .push(mem::replace(&mut entry.current, ban_record));
                            Some(())
                        },
                    }
                })?
                .1,
            )
        }
    }

    impl Banlist {
        /// One-off migration from the previous version.  This must be
        /// guaranteed to produce a valid settings file as long as it is
        /// called with a valid settings file from the previous version.
        pub(super) fn migrate(prev: prev::Banlist) -> Self {
            // The ban start date for migrations from legacy is the current one; we could
            // record that they actually have an unknown start date, but this
            // would just complicate the format.
            let date = Utc::now();
            Banlist(
                prev.0
                    .into_iter()
                    .map(
                        |(
                            uid,
                            prev::BanRecord {
                                username_when_banned,
                                reason,
                            },
                        )| {
                            (uid, BanEntry {
                                current: BanRecord {
                                    username_when_performed: username_when_banned,
                                    // We only recorded unbans pre-migration.
                                    action: BanAction::Ban(Ban {
                                        reason,
                                        // We don't know who banned this user pre-migration.
                                        info: None,
                                        // All bans pre-migration are of unlimited duration.
                                        end_date: None,
                                    }),
                                    date,
                                },
                                // Old bans never expire, so set the expiration hint to false.
                                expired: false,
                                // There is no known ban history yet.
                                history: Vec::new(),
                            })
                        },
                    )
                    .collect(),
            )
        }

        /// Perform any needed validation on this banlist that can't be done
        /// using parsing.
        ///
        /// The returned version being "Old" indicates the loaded setting has
        /// been modified during validation (this is why validate takes
        /// `&mut self`).
        pub(super) fn validate(&mut self) -> Result<Version, <Final as EditableSetting>::Error> {
            let mut version = Version::Latest;
            let now = Utc::now();
            for (&uuid, value) in self.0.iter_mut() {
                if matches!(value.validate(now, uuid)?, Version::Old) {
                    // Update detected.
                    version = Version::Old;
                }
            }
            Ok(version)
        }
    }

    // NOTE: Whenever there is a version upgrade, copy this note as well as the
    // commented-out code below to the next version, then uncomment the code
    // for this version.
    /* impl TryFrom<Banlist> for Final {
        type Error = <Final as EditableSetting>::Error;

        fn try_from(mut value: Banlist) -> Result<Final, Self::Error> {
            value.validate()?;
            Ok(next::Banlist::migrate(value).try_into().expect(MIGRATION_UPGRADE_GUARANTEE))
        }
    } */
}
