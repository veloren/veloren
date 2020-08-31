use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tui::{layout::Rect, text::Text};

#[derive(Debug, Default, Clone)]
pub struct TuiLog<'a> {
    pub inner: Arc<Mutex<Text<'a>>>,
}

impl<'a> TuiLog<'a> {
    pub fn resize(&self, h: usize) {
        let mut inner = self.inner.lock().unwrap();

        if inner.height() > h {
            let length = inner.height() - h;
            inner.lines.drain(0..length);
        }
    }

    pub fn height(&self, rect: Rect) -> u16 {
        // TODO: There's probably a better solution
        let inner = self.inner.lock().unwrap();
        let mut h = 0;

        for line in inner.lines.iter() {
            let mut w = 0;

            for word in line.0.iter() {
                if word.width() + w > rect.width as usize {
                    h += (word.width() / rect.width as usize).min(1);
                    w = word.width() % rect.width as usize;
                } else {
                    w += word.width();
                }
            }

            h += 1;
        }

        h as u16
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
                                2 => span.style.add_modifier = Modifier::DIM,
                                idx @ 30..=37 => {
                                    span.style.fg = Some(COLOR_TABLE[(idx - 30) as usize])
                                },
                                _ => println!("{:#?}", values),
                            }
                        },
                        _ => println!("{:#?}", seq),
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
