use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LanguageSettings {
    pub selected_language: String,
    pub use_english_fallback: bool,
}

impl Default for LanguageSettings {
    fn default() -> Self {
        Self {
            selected_language: i18n::REFERENCE_LANG.to_string(),
            use_english_fallback: true,
        }
    }
}
