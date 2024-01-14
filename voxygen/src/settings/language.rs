use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageSettings {
    pub selected_language: String,
    #[serde(default = "default_true")]
    pub share_with_server: bool,
    pub use_english_fallback: bool,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            selected_language: i18n::REFERENCE_LANG.to_string(),
            share_with_server: true,
            use_english_fallback: true,
        }
    }
}

fn default_true() -> bool { true }
