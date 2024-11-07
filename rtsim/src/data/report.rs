use common::{
    resources::TimeOfDay,
    rtsim::{Actor, SiteId},
    terrain::SpriteKind,
};
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::ops::Deref;
use vek::*;

pub use common::rtsim::ReportId;

/// Represents a single piece of information known by an rtsim entity.
///
/// Reports are the medium through which rtsim represents information sharing
/// between NPCs, factions, and sites. They can represent deaths, attacks,
/// changes in diplomacy, or any other piece of information representing a
/// singular event that might be communicated.
///
/// Note that they should not be used to communicate sentiments like 'this actor
/// is friendly': the [`crate::data::Sentiment`] system should be used for that.
/// Some events might generate both a report and a change in sentiment. For
/// example, the murder of an NPC might generate both a murder report and highly
/// negative sentiments.
#[derive(Clone, Serialize, Deserialize)]
pub struct Report {
    pub kind: ReportKind,
    pub at_tod: TimeOfDay,
}

impl Report {
    /// The time, in in-game seconds, for which the report will be remembered
    fn remember_for(&self) -> f64 {
        const DAYS: f64 = 60.0 * 60.0 * 24.0;
        match &self.kind {
            ReportKind::Death { killer, .. } => {
                if killer.is_some() {
                    // Murder is less easy to forget
                    DAYS * 15.0
                } else {
                    DAYS * 5.0
                }
            },
            // TODO: Could consider what was stolen here
            ReportKind::Theft { .. } => DAYS * 1.5,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum ReportKind {
    Death {
        actor: Actor,
        killer: Option<Actor>,
    },
    Theft {
        thief: Actor,
        /// Where the theft happened.
        site: Option<SiteId>,
        /// What was stolen.
        sprite: SpriteKind,
    },
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Reports {
    pub reports: HopSlotMap<ReportId, Report>,
}

impl Reports {
    pub fn create(&mut self, report: Report) -> ReportId { self.reports.insert(report) }

    pub fn cleanup(&mut self, current_time: TimeOfDay) {
        // Forget reports that are too old
        self.reports.retain(|_, report| {
            (current_time.0 - report.at_tod.0).max(0.0) < report.remember_for()
        });
        // TODO: Limit global number of reports
    }
}

impl Deref for Reports {
    type Target = HopSlotMap<ReportId, Report>;

    fn deref(&self) -> &Self::Target { &self.reports }
}
