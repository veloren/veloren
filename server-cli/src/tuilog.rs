use ratatui::{
    prelude::Line,
    style::{Color, Modifier, Style},
    text::{Span, Text},
};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tracing::warn;

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
        // TODO: this processing can probably occur in the consumer of the log lines
        // (and instead of having a TuiLog::resize the consumer can take
        // ownership of the lines and manage them itself).

        // Not super confident this is the ideal parser but it works for now and doesn't
        // depend on an old version of nom. Alternatives to consider may include
        // `vte`, `anstyle-parse`, `vt100`, or others.
        use cansi::v3::categorise_text;

        let line =
            core::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut spans = Vec::new();
        let mut lines = Vec::new();

        for out in categorise_text(line) {
            let mut style = Style::default();
            // NOTE: There are other values returned from cansi that we don't bother to use
            // for now including background color, italics, blinking, etc.
            style.fg = match out.fg {
                Some(cansi::Color::Black) => Some(Color::Black),
                Some(cansi::Color::Red) => Some(Color::Red),
                Some(cansi::Color::Green) => Some(Color::Green),
                Some(cansi::Color::Yellow) => Some(Color::Yellow),
                Some(cansi::Color::Blue) => Some(Color::Blue),
                Some(cansi::Color::Magenta) => Some(Color::Magenta),
                Some(cansi::Color::Cyan) => Some(Color::Cyan),
                Some(cansi::Color::White) => Some(Color::White),
                // "Bright" versions currently not handled
                Some(c) => {
                    warn!("Unknown color {:#?}", c);
                    style.fg
                },
                None => style.fg,
            };
            match out.intensity {
                Some(cansi::Intensity::Normal) | None => {},
                Some(cansi::Intensity::Bold) => style.add_modifier = Modifier::BOLD,
                Some(cansi::Intensity::Faint) => style.add_modifier = Modifier::DIM,
            }

            // search for newlines
            for t in out.text.split_inclusive('\n') {
                if !t.is_empty() {
                    spans.push(Span::styled(t.to_owned(), style));
                }
                if t.ends_with('\n') {
                    lines.push(Line::from(core::mem::take(&mut spans)));
                }
            }
        }
        if !spans.is_empty() {
            lines.push(Line::from(spans));
        }

        self.inner.lock().unwrap().lines.append(&mut lines);

        Ok(buf.len())
    }

    // We can potentially use this to reduce locking frequency?
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
