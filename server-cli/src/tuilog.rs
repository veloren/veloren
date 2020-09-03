use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tracing::warn;
use tui::text::Text;

#[derive(Debug, Default, Clone)]
pub struct TuiLog<'a> {
    pub inner: Arc<Mutex<Text<'a>>>,
}

impl<'a> TuiLog<'a> {
    pub fn resize(&self, h: usize) {
        let mut inner = self.inner.lock().unwrap();

        inner.lines.truncate(h);
    }
}

impl<'a> Write for TuiLog<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        use ansi_parser::{AnsiParser, AnsiSequence, Output};
        use tui::{
            style::{Color, Modifier},
            text::{Span, Spans},
        };

        let line = String::from_utf8(buf.into())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut spans = Vec::new();
        let mut span = Span::raw("");

        for out in line.ansi_parse() {
            match out {
                Output::TextBlock(text) => {
                    span.content = format!("{}{}", span.content.to_owned(), text).into()
                },
                Output::Escape(seq) => {
                    if span.content.len() != 0 {
                        spans.push(span);

                        span = Span::raw("");
                    }

                    match seq {
                        AnsiSequence::SetGraphicsMode(values) => {
                            const COLOR_TABLE: [Color; 8] = [
                                Color::Black,
                                Color::Red,
                                Color::Green,
                                Color::Yellow,
                                Color::Blue,
                                Color::Magenta,
                                Color::Cyan,
                                Color::White,
                            ];

                            let mut iter = values.iter();

                            match iter.next().unwrap() {
                                0 => {},
                                1 => span.style.add_modifier = Modifier::BOLD,
                                2 => span.style.add_modifier = Modifier::DIM,
                                idx @ 30..=37 => {
                                    span.style.fg = Some(COLOR_TABLE[(idx - 30) as usize])
                                },
                                _ => warn!("Unknown color {:#?}", values),
                            }
                        },
                        _ => warn!("Unknown sequence {:#?}", seq),
                    }
                },
            }
        }

        if span.content.len() != 0 {
            spans.push(span);
        }

        self.inner.lock().unwrap().lines.push(Spans(spans));

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
