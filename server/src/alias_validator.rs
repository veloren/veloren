use std::fmt::{self, Display};

#[derive(Debug, Default)]
pub struct AliasValidator {
    banned_substrings: Vec<String>,
}

impl AliasValidator {
    pub fn new(banned_substrings: Vec<String>) -> Self {
        let banned_substrings = banned_substrings
            .iter()
            .map(|string| string.to_lowercase())
            .collect();

        AliasValidator { banned_substrings }
    }

    pub fn validate(&self, alias: &str) -> Result<(), ValidatorError> {
        let lowercase_alias = alias.to_lowercase();

        for banned_word in self.banned_substrings.iter() {
            if lowercase_alias.contains(banned_word) {
                return Err(ValidatorError::Forbidden(
                    alias.to_owned(),
                    banned_word.to_owned(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum ValidatorError {
    Forbidden(String, String),
}

impl Display for ValidatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forbidden(name, _) => write!(
                formatter,
                "Character name \"{}\" contains a banned word",
                name
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_matches() {
        let banned_substrings = vec!["bad".to_owned(), "worse".to_owned()];
        let validator = AliasValidator::new(banned_substrings);

        let bad_alias = "Badplayery Mc WorsePlayeryFace";
        let result = validator.validate(bad_alias);

        assert_eq!(
            result,
            Err(ValidatorError::Forbidden(
                bad_alias.to_owned(),
                "bad".to_owned()
            ))
        );
    }

    #[test]
    fn single_lowercase_match() {
        let banned_substrings = vec!["blue".to_owned()];
        let validator = AliasValidator::new(banned_substrings);

        let bad_alias = "blueName";
        let result = validator.validate(bad_alias);

        assert_eq!(
            result,
            Err(ValidatorError::Forbidden(
                bad_alias.to_owned(),
                "blue".to_owned()
            ))
        );
    }

    #[test]
    fn single_case_insensitive_match() {
        let banned_substrings = vec!["GrEEn".to_owned()];
        let validator = AliasValidator::new(banned_substrings);

        let bad_alias = "gReenName";
        let result = validator.validate(bad_alias);

        assert_eq!(
            result,
            Err(ValidatorError::Forbidden(
                bad_alias.to_owned(),
                "green".to_owned()
            ))
        );
    }

    #[test]
    fn mp_matches() {
        let banned_substrings = vec!["orange".to_owned()];
        let validator = AliasValidator::new(banned_substrings);

        let good_alias = "ReasonableName";
        let result = validator.validate(good_alias);

        assert_eq!(result, Ok(()));
    }
}
