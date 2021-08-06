use crate::{
    gitfragments::{LocalizationEntryState, LocalizationState, ALL_LOCALIZATION_STATES},
    raw::RawLanguage,
};
use hashbrown::HashMap;
use std::path::PathBuf;

#[derive(Default, Debug, PartialEq)]
pub(crate) struct LocalizationStats {
    pub(crate) uptodate_entries: usize,
    pub(crate) notfound_entries: usize,
    pub(crate) unused_entries: usize,
    pub(crate) outdated_entries: usize,
    pub(crate) errors: usize,
}

#[allow(clippy::type_complexity)]
pub(crate) struct LocalizationAnalysis {
    language_identifier: String,
    pub(crate) data: HashMap<Option<LocalizationState>, Vec<(PathBuf, String, Option<git2::Oid>)>>,
}

impl LocalizationStats {
    /// Calculate key count that actually matter for the status of the
    /// translation Unused entries don't break the game
    pub(crate) fn get_real_entry_count(&self) -> usize {
        self.outdated_entries + self.notfound_entries + self.errors + self.uptodate_entries
    }
}

impl LocalizationAnalysis {
    pub(crate) fn new(language_identifier: &str) -> Self {
        let mut data = HashMap::new();
        for key in ALL_LOCALIZATION_STATES.iter() {
            data.insert(*key, vec![]);
        }
        Self {
            language_identifier: language_identifier.to_owned(),
            data,
        }
    }

    fn show<W: std::io::Write>(
        &self,
        state: Option<LocalizationState>,
        ref_language: &RawLanguage<LocalizationEntryState>,
        be_verbose: bool,
        output: &mut W,
    ) {
        let entries = self.data.get(&state).unwrap_or_else(|| {
            panic!(
                "called on invalid state: {}",
                LocalizationState::print(&state)
            )
        });
        if entries.is_empty() {
            return;
        }
        writeln!(output, "\n\t[{}]", LocalizationState::print(&state)).unwrap();
        for (path, key, commit_id) in entries {
            if be_verbose {
                let our_commit = LocalizationAnalysis::print_commit(commit_id);
                let ref_commit = ref_language
                    .fragments
                    .get(path)
                    .and_then(|entry| entry.string_map.get(key))
                    .and_then(|s| s.commit_id)
                    .map(|s| format!("{}", s))
                    .unwrap_or_else(|| "None".to_owned());
                writeln!(output, "{:60}| {:40} | {:40}", key, our_commit, ref_commit).unwrap();
            } else {
                writeln!(output, "{}", key).unwrap();
            }
        }
    }

    fn csv<W: std::io::Write>(&self, state: Option<LocalizationState>, output: &mut W) {
        let entries = self
            .data
            .get(&state)
            .unwrap_or_else(|| panic!("called on invalid state: {:?}", state));
        for (path, key, commit_id) in entries {
            let our_commit = LocalizationAnalysis::print_commit(commit_id);
            writeln!(
                output,
                "{},{:?},{},{},{}",
                self.language_identifier,
                path,
                key,
                LocalizationState::print(&state),
                our_commit
            )
            .unwrap();
        }
    }

    fn print_commit(commit_id: &Option<git2::Oid>) -> String {
        commit_id
            .map(|s| format!("{}", s))
            .unwrap_or_else(|| "None".to_owned())
    }
}

pub(crate) fn print_translation_stats(
    language_identifier: &str,
    ref_language: &RawLanguage<LocalizationEntryState>,
    stats: &LocalizationStats,
    state_map: &LocalizationAnalysis,
    be_verbose: bool,
) {
    let real_entry_count = stats.get_real_entry_count() as f32;
    let uptodate_percent = (stats.uptodate_entries as f32 / real_entry_count) * 100_f32;
    let outdated_percent = (stats.outdated_entries as f32 / real_entry_count) * 100_f32;
    let untranslated_percent = ((stats.errors + stats.errors) as f32 / real_entry_count) * 100_f32;

    // Display
    if be_verbose {
        println!(
            "\n{:60}| {:40} | {:40}",
            "Key name", language_identifier, ref_language.manifest.metadata.language_identifier,
        );
    } else {
        println!("\nKey name");
    }

    for state in &ALL_LOCALIZATION_STATES {
        if state == &Some(LocalizationState::UpToDate) {
            continue;
        }
        state_map.show(*state, ref_language, be_verbose, &mut std::io::stdout());
    }

    println!(
        "\n{} up-to-date, {} outdated, {} unused, {} not found, {} unknown entries",
        stats.uptodate_entries,
        stats.outdated_entries,
        stats.unused_entries,
        stats.notfound_entries,
        stats.errors,
    );

    println!(
        "{:.2}% up-to-date, {:.2}% outdated, {:.2}% untranslated\n",
        uptodate_percent, outdated_percent, untranslated_percent,
    );
}

pub(crate) fn print_csv_stats<W: std::io::Write>(state_map: &LocalizationAnalysis, output: &mut W) {
    for state in &ALL_LOCALIZATION_STATES {
        state_map.csv(*state, output);
    }
}

pub(crate) fn print_overall_stats(
    analysis: HashMap<String, (LocalizationAnalysis, LocalizationStats)>,
) {
    let mut overall_uptodate_entry_count = 0;
    let mut overall_outdated_entry_count = 0;
    let mut overall_untranslated_entry_count = 0;
    let mut overall_real_entry_count = 0;

    println!("-----------------------------------------------------------------------------");
    println!("Overall Translation Status");
    println!("-----------------------------------------------------------------------------");
    println!(
        "{:12}| {:8} | {:8} | {:8} | {:8} | {:8}",
        "", "up-to-date", "outdated", "untranslated", "unused", "errors",
    );

    let mut i18n_stats: Vec<(&String, &(_, LocalizationStats))> = analysis.iter().collect();
    i18n_stats.sort_by_key(|(_, (_, v))| v.notfound_entries);

    for (path, (_, test_result)) in i18n_stats {
        let LocalizationStats {
            uptodate_entries: uptodate,
            outdated_entries: outdated,
            unused_entries: unused,
            notfound_entries: untranslated,
            errors,
        } = test_result;
        overall_uptodate_entry_count += uptodate;
        overall_outdated_entry_count += outdated;
        overall_untranslated_entry_count += untranslated;
        overall_real_entry_count += test_result.get_real_entry_count();

        println!(
            "{:12}|{:8}    |{:6}    |{:8}      |{:6}    |{:8}",
            path, uptodate, outdated, untranslated, unused, errors,
        );
    }

    println!(
        "\n{:.2}% up-to-date, {:.2}% outdated, {:.2}% untranslated",
        (overall_uptodate_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
        (overall_outdated_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
        (overall_untranslated_entry_count as f32 / overall_real_entry_count as f32) * 100_f32,
    );
    println!("-----------------------------------------------------------------------------\n");
}
