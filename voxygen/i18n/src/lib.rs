mod raw;

use fluent_bundle::{bundle::FluentBundle, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use unic_langid::LanguageIdentifier;

use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, io};

use assets::{source::DirEntry, AssetExt, AssetGuard, AssetHandle, ReloadWatcher};
use tracing::warn;
// Re-export because I don't like prefix
use common_assets as assets;

// Re-export for argument creation
pub use fluent::fluent_args;
pub use fluent_bundle::FluentArgs;

/// The reference language, aka the more up-to-date localization data.
/// Also the default language at first startup.
pub const REFERENCE_LANG: &str = "en";

/// How a language can be described
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageMetadata {
    /// A human friendly language name (e.g. "English (US)")
    pub language_name: String,

    /// A short text identifier for this language (e.g. "en_US")
    ///
    /// On the opposite of `language_name` that can change freely,
    /// `language_identifier` value shall be stable in time as it
    /// is used by setting components to store the language
    /// selected by the user.
    pub language_identifier: String,
}

/// Store font metadata
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Font {
    /// Key to retrieve the font in the asset system
    pub asset_key: String,

    /// Scale ratio to resize the UI text dynamically
    scale_ratio: f32,
}

impl Font {
    /// Scale input size to final UI size
    #[must_use]
    pub fn scale(&self, value: u32) -> u32 { (value as f32 * self.scale_ratio).round() as u32 }
}

/// Store font metadata
pub type Fonts = HashMap<String, Font>;

/// Store internationalization data
struct Language {
    /// The bundle storing all localized texts
    pub(crate) bundle: FluentBundle<FluentResource, IntlLangMemoizer>,
    /// Whether to convert the input text encoded in UTF-8
    /// into a ASCII version by using the `deunicode` crate.
    // FIXME (i18n convert_utf8_to_ascii):
    #[allow(dead_code)]
    convert_utf8_to_ascii: bool,

    /// Font configuration is stored here
    pub(crate) fonts: Fonts,

    pub(crate) metadata: LanguageMetadata,
}

impl Language {
    fn try_msg<'a>(&'a self, key: &str, args: Option<&'a FluentArgs>) -> Option<Cow<str>> {
        let bundle = &self.bundle;
        let msg = bundle.get_message(key)?;
        let mut errs = Vec::new();
        let msg = bundle.format_pattern(msg.value()?, args, &mut errs);
        for err in errs {
            tracing::error!("err: {err} for {key}");
        }

        Some(msg)
    }

    fn try_variation<'a>(
        &'a self,
        key: &str,
        seed: u16,
        args: Option<&'a FluentArgs>,
    ) -> Option<Cow<'a, str>> {
        let bundle = &self.bundle;
        let msg = bundle.get_message(key)?;
        let mut attrs = msg.attributes();

        if attrs.len() != 0 {
            let idx = usize::from(seed) % attrs.len();
            // unwrap is ok here, because idx is bound to attrs.len()
            // by using modulo operator.
            //
            // For example:
            // (I)
            // * attributes = [.x = 5, .y = 7, z. = 4]
            // * len = 3
            // * seed can be 12, 50, 1
            // 12 % 3 = 0, attrs.skip(0) => first element
            // 50 % 3 = 2, attrs.skip(2) => third element
            // 1 % 3 = 1, attrs.skip(1) => second element
            // (II)
            // * attributes = []
            // * len = 0
            // * no matter what seed is, we return None in code above
            let variation = attrs.nth(idx).unwrap();
            let mut errs = Vec::new();
            let msg = bundle.format_pattern(variation.value(), args, &mut errs);
            for err in errs {
                tracing::error!("err: {err} for {key}");
            }

            Some(msg)
        } else {
            None
        }
    }
}

impl assets::Compound for Language {
    fn load(cache: assets::AnyCache, path: &str) -> Result<Self, assets::BoxedError> {
        let manifest = cache
            .load::<raw::Manifest>(&[path, ".", "_manifest"].concat())?
            .cloned();
        let raw::Manifest {
            convert_utf8_to_ascii,
            fonts,
            metadata,
        } = manifest;

        let lang_id: LanguageIdentifier = metadata.language_identifier.parse()?;
        let mut bundle = FluentBundle::new_concurrent(vec![lang_id]);

        // Here go dragons
        for id in cache.load_dir::<raw::Resource>(path, true)?.ids() {
            if id.ends_with("_manifest") {
                continue;
            }

            match cache.load(id) {
                Ok(handle) => {
                    use std::{error::Error, fmt, ops::Range};

                    #[derive(Debug)]
                    struct Pos {
                        #[allow(dead_code)] // false-positive
                        line: usize,
                        #[allow(dead_code)] // false-positive
                        character: usize,
                    }

                    fn unspan(src: &str, span: Range<usize>) -> Range<Pos> {
                        let count = |idx| {
                            let mut line = 1;
                            let mut character = 1;
                            for ch in src.bytes().take(idx) {
                                // Count characters
                                character += 1;

                                // Count newlines
                                if ch == b'\n' {
                                    line += 1;
                                    // If found new line, reset character count
                                    character = 1;
                                }
                            }
                            Pos { line, character }
                        };
                        let Range { start, end } = span;
                        count(start)..count(end)
                    }

                    // TODO:
                    // better error handling?
                    #[derive(Debug)]
                    enum ResourceErr {
                        ParsingError {
                            #[allow(dead_code)] // false-positive
                            file: String,
                            #[allow(dead_code)] // false-positive
                            err: String,
                        },
                        BundleError(String),
                    }

                    impl fmt::Display for ResourceErr {
                        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                            write!(f, "{self:?}")
                        }
                    }

                    impl Error for ResourceErr {}

                    let source: &raw::Resource = &*handle.read();
                    let resource =
                        FluentResource::try_new(source.src.clone()).map_err(|(_ast, errs)| {
                            let file = id.to_owned();
                            let errs = errs
                                .into_iter()
                                .map(|e| {
                                    let pos = unspan(&source.src, e.pos);
                                    format!("{pos:?}, kind {:?}", e.kind)
                                })
                                .collect::<Vec<_>>();

                            ResourceErr::ParsingError {
                                file,
                                err: format!("{errs:?}"),
                            }
                        })?;

                    bundle
                        .add_resource(resource)
                        .map_err(|e| ResourceErr::BundleError(format!("{e:?}")))?;
                },
                Err(err) => {
                    // TODO: shouldn't we just panic here?
                    warn!("Unable to load asset {id}, error={err:?}");
                },
            }
        }

        Ok(Self {
            bundle,
            convert_utf8_to_ascii,
            fonts,
            metadata,
        })
    }
}

/// The central data structure to handle localization in Veloren
// inherit Copy + Clone from AssetHandle (what?)
#[derive(Copy, Clone)]
pub struct LocalizationHandle {
    active: AssetHandle<Language>,
    watcher: ReloadWatcher,
    fallback: Option<AssetHandle<Language>>,
    pub use_english_fallback: bool,
}

/// Read `LocalizationGuard`
// arbitrary choice to minimize changing all of veloren
pub type Localization = LocalizationGuard;

/// RAII guard returned from `Localization::read`(), resembles `AssetGuard`
pub struct LocalizationGuard {
    active: AssetGuard<Language>,
    fallback: Option<AssetGuard<Language>>,
}

impl LocalizationGuard {
    /// DEPRECATED
    ///
    /// Get a localized text from the given key
    ///
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
    /// If the key is not present in the localization object
    /// then the key itself is returned.
    ///
    /// NOTE: this function shouldn't be used in new code.
    /// It is kept for compatibility with old code that uses
    /// old style dot-separated keys and this function internally
    /// replaces them with dashes.
    // FIXME (i18n old style keys):
    // this is deprecated, fix all usages of this asap
    pub fn get(&self, key: &str) -> Cow<str> {
        // Fluent uses `-` as informal separator, while in the past with our
        // RON based system we used `.` for that purpose.
        self.get_msg(&key.replace('.', "-"))
    }

    /// Get a localized text from the given key
    ///
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
    pub fn try_msg(&self, key: &str) -> Option<Cow<str>> {
        self.active
            .try_msg(key, None)
            .or_else(|| self.fallback.as_ref().and_then(|fb| fb.try_msg(key, None)))
    }

    /// Get a localized text from the given key
    ///
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
    /// If the key is not present in the localization object
    /// then the key itself is returned.
    pub fn get_msg(&self, key: &str) -> Cow<str> {
        // NOTE: we clone the key if translation was missing
        // We could use borrowed version, but it would mean that
        // `key`, `self`, and result should have the same lifetime.
        // Which would make it impossible to use with runtime generated keys.
        self.try_msg(key)
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    /// Get a localized text from the given key using given arguments
    ///
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
    pub fn try_msg_ctx<'a>(&'a self, key: &str, args: &'a FluentArgs) -> Option<Cow<'static, str>> {
        // NOTE: as after using args we get our result owned (because you need
        // to clone pattern during forming value from args), this conversion
        // to Cow;:Owned is no-op.
        // We could use String here, but using Cow everywhere in i18n API is
        // prefered for consistency.
        self.active
            .try_msg(key, Some(args))
            .or_else(|| {
                self.fallback
                    .as_ref()
                    .and_then(|fb| fb.try_msg(key, Some(args)))
            })
            .map(|x| {
                // NOTE:
                // Hack. Remove Unicode Directionality Marks, conrod doesn't support them.
                let res = x.replace('\u{2068}', "").replace('\u{2069}', "");
                Cow::Owned(res)
            })
    }

    /// Get a localized text from the given key using given arguments
    ///
    /// First lookup is done in the active language, second in
    /// the fallback (if present).
    /// If the key is not present in the localization object
    /// then the key itself is returned.
    pub fn get_msg_ctx<'a>(&'a self, key: &str, args: &'a FluentArgs) -> Cow<'static, str> {
        self.try_msg_ctx(key, args)
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    pub fn try_variation(&self, key: &str, seed: u16) -> Option<Cow<str>> {
        self.active.try_variation(key, seed, None).or_else(|| {
            self.fallback
                .as_ref()
                .and_then(|fb| fb.try_variation(key, seed, None))
        })
    }

    pub fn get_variation(&self, key: &str, seed: u16) -> Cow<str> {
        self.try_variation(key, seed)
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    pub fn try_variation_ctx<'a>(
        &'a self,
        key: &str,
        seed: u16,
        args: &'a FluentArgs,
    ) -> Option<Cow<str>> {
        self.active
            .try_variation(key, seed, Some(args))
            .or_else(|| {
                self.fallback
                    .as_ref()
                    .and_then(|fb| fb.try_variation(key, seed, Some(args)))
            })
            .map(|x| {
                // NOTE:
                // Hack. Remove Unicode Directionality Marks, conrod doesn't support them.
                let res = x.replace('\u{2068}', "").replace('\u{2069}', "");
                Cow::Owned(res)
            })
    }

    pub fn get_variation_ctx<'a>(&'a self, key: &str, seed: u16, args: &'a FluentArgs) -> Cow<str> {
        self.try_variation_ctx(key, seed, args)
            .unwrap_or_else(|| Cow::Owned(key.to_owned()))
    }

    #[must_use]
    pub fn fonts(&self) -> &Fonts { &self.active.fonts }

    #[must_use]
    pub fn metadata(&self) -> &LanguageMetadata { &self.active.metadata }
}

impl LocalizationHandle {
    pub fn set_english_fallback(&mut self, use_english_fallback: bool) {
        self.use_english_fallback = use_english_fallback;
    }

    #[must_use]
    pub fn read(&self) -> LocalizationGuard {
        LocalizationGuard {
            active: self.active.read(),
            fallback: if self.use_english_fallback {
                self.fallback.map(|f| f.read())
            } else {
                None
            },
        }
    }

    /// # Errors
    /// Returns error if active language can't be loaded
    pub fn load(specifier: &str) -> Result<Self, assets::Error> {
        let default_key = ["voxygen.i18n.", REFERENCE_LANG].concat();
        let language_key = ["voxygen.i18n.", specifier].concat();
        let is_default = language_key == default_key;
        let active = Language::load(&language_key)?;
        Ok(Self {
            active,
            watcher: active.reload_watcher(),
            fallback: if is_default {
                None
            } else {
                // TODO: shouldn't this panic?
                Language::load(&default_key).ok()
            },
            use_english_fallback: false,
        })
    }

    #[must_use]
    pub fn load_expect(specifier: &str) -> Self {
        Self::load(specifier).expect("Can't load language files")
    }

    pub fn reloaded(&mut self) -> bool { self.watcher.reloaded() }
}

struct FindManifests;

impl assets::DirLoadable for FindManifests {
    fn select_ids<S: assets::Source + ?Sized>(
        source: &S,
        specifier: &str,
    ) -> io::Result<Vec<assets::SharedString>> {
        let mut specifiers = Vec::new();

        source.read_dir(specifier, &mut |entry| {
            if let DirEntry::Directory(spec) = entry {
                let manifest_spec = [spec, ".", "_manifest"].concat();

                if source.exists(DirEntry::File(&manifest_spec, "ron")) {
                    specifiers.push(manifest_spec.into());
                }
            }
        })?;

        Ok(specifiers)
    }
}

#[derive(Clone, Debug)]
struct LocalizationList(Vec<LanguageMetadata>);

impl assets::Compound for LocalizationList {
    fn load(cache: assets::AnyCache, specifier: &str) -> Result<Self, assets::BoxedError> {
        // List language directories
        let languages = assets::load_dir::<FindManifests>(specifier, false)
            .unwrap_or_else(|e| panic!("Failed to get manifests from {}: {:?}", specifier, e))
            .ids()
            .filter_map(|spec| cache.load::<raw::Manifest>(spec).ok())
            .map(|localization| localization.read().metadata.clone())
            .collect();

        Ok(LocalizationList(languages))
    }
}

/// Load all the available languages located in the voxygen asset directory
#[must_use]
pub fn list_localizations() -> Vec<LanguageMetadata> {
    let LocalizationList(list) = LocalizationList::load_expect_cloned("voxygen.i18n");
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_localization_list() {
        let list = list_localizations();
        assert!(!list.is_empty());
    }

    #[test]
    fn validate_reference_language() { let _ = LocalizationHandle::load_expect(REFERENCE_LANG); }

    #[test]
    fn validate_all_localizations() {
        let list = list_localizations();
        for meta in list {
            let _ = LocalizationHandle::load_expect(&meta.language_identifier);
        }
    }

    #[test]
    #[ignore]
    // Generate translation stats
    fn test_all_localizations() {
        // FIXME (i18n translation stats):
        use std::{fs, io::Write};

        let output = assets::find_root()
            .unwrap()
            .join("translation_analysis.csv");
        let mut f = fs::File::create(output).expect("couldn't write csv file");

        writeln!(
            f,
            "country_code,file_name,translation_key,status,git_commit"
        )
        .unwrap();
    }
}
