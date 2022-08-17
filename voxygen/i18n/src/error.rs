use fluent_syntax::parser::ParserError;
use std::{error::Error, fmt, ops::Range};

#[derive(Debug)]
struct Pos {
    line: usize,
    character: usize,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{};{}", self.line, self.character)
    }
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
// Ideally we wouldn't write this code, check this issue in fluent-rs.
// https://github.com/projectfluent/fluent-rs/issues/176
#[derive(Debug)]
pub enum ResourceErr {
    ParsingError {
        #[allow(dead_code)] // false-positive
        file: String,
        #[allow(dead_code)] // false-positive
        err: String,
    },
    BundleError(String),
}

impl ResourceErr {
    pub fn parsing_error(errs: Vec<ParserError>, file: String, src: &str) -> Self {
        let errs = errs
            .into_iter()
            .map(|e| {
                let Range {
                    start: from,
                    end: to,
                } = unspan(src, e.pos);
                format!("{from}..{to}, kind {:?}", e.kind)
            })
            .collect::<Vec<_>>();

        Self::ParsingError {
            file,
            err: format!("{errs:?}"),
        }
    }
}

impl fmt::Display for ResourceErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{self:#?}") }
}

impl Error for ResourceErr {}
