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

        let len = inner.lines.len().saturating_sub(h);
        inner.lines.drain(..len);
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
        let mut lines = Vec::new();

        for out in line.ansi_parse() {
            match out {
                Output::TextBlock(text) => {
                    // search for newlines
                    for t in text.split_inclusive('\n') {
                        span.content.to_mut().push_str(t);
                        if t.ends_with('\n') && span.content.len() != 0 {
                            spans.push(std::mem::replace(&mut span, Span::raw("")));
                            lines.push(std::mem::take(&mut spans));
                        }
                    }
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
        if !spans.is_empty() {
            lines.push(spans);
        }

        let mut lines = lines.into_iter().map(Spans).collect::<Vec<_>>();
        self.inner.lock().unwrap().lines.append(&mut lines);

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
