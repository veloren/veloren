use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::{Arc, Mutex},
};

/// Maximum number of log lines retained in the circular buffer.
pub const MAX_LOG_LINES: usize = 2000;

/// Severity level detected from a tracing log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Unknown,
}

impl LogLevel {
    /// Parse the level tag that `tracing_subscriber` emits, e.g. ` INFO `,
    /// ` WARN `, etc.
    fn from_line(line: &str) -> Self {
        // tracing-subscriber fmt output looks like:
        //   2024-01-01T00:00:00.000Z  INFO nova_forge_server: message
        // We just scan for the all-caps level keyword.
        if line.contains(" ERROR ") || line.contains(" ERROR\t") {
            LogLevel::Error
        } else if line.contains(" WARN ") || line.contains(" WARN\t") {
            LogLevel::Warn
        } else if line.contains(" INFO ") || line.contains(" INFO\t") {
            LogLevel::Info
        } else if line.contains(" DEBUG ") || line.contains(" DEBUG\t") {
            LogLevel::Debug
        } else if line.contains(" TRACE ") || line.contains(" TRACE\t") {
            LogLevel::Trace
        } else {
            LogLevel::Unknown
        }
    }
}

/// A single captured log entry.
#[derive(Clone, Debug)]
pub struct LogLine {
    pub level: LogLevel,
    pub text: String,
}

/// Shared, thread-safe log buffer that the GUI reads from.
pub type SharedLog = Arc<Mutex<VecDeque<LogLine>>>;

/// Create a fresh `SharedLog`.
pub fn new_shared_log() -> SharedLog {
    Arc::new(Mutex::new(VecDeque::with_capacity(MAX_LOG_LINES)))
}

/// A `Write` impl that captures tracing output and appends plain-text lines
/// to a `SharedLog`, stripping ANSI escape codes in the process.
#[derive(Clone)]
pub struct GuiLog {
    pub shared: SharedLog,
    /// Accumulates partial lines (bytes not yet ending with '\n').
    pending: Arc<Mutex<Vec<u8>>>,
}

impl GuiLog {
    pub fn new(shared: SharedLog) -> Self {
        Self {
            shared,
            pending: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn flush_pending(&self) {
        let mut pending = self.pending.lock().unwrap();
        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).into_owned();
            pending.clear();
            self.push_line(text);
        }
    }

    fn push_line(&self, raw: String) {
        let text = strip_ansi(&raw);
        let text = text.trim_end_matches('\n').to_owned();
        if text.is_empty() {
            return;
        }
        let level = LogLevel::from_line(&text);
        let entry = LogLine { level, text };
        let mut buf = self.shared.lock().unwrap();
        if buf.len() >= MAX_LOG_LINES {
            buf.pop_front();
        }
        buf.push_back(entry);
    }
}

impl Write for GuiLog {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut pending = self.pending.lock().unwrap();
        pending.extend_from_slice(buf);

        // Flush complete lines from pending.
        while let Some(pos) = pending.iter().position(|&b| b == b'\n') {
            let line_bytes = pending.drain(..=pos).collect::<Vec<_>>();
            drop(pending); // release lock before pushing
            let text = String::from_utf8_lossy(&line_bytes).into_owned();
            self.push_line(text);
            pending = self.pending.lock().unwrap();
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_pending();
        Ok(())
    }
}

/// Strip ANSI / VT100 escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC — consume until end of escape sequence.
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // consume parameter / intermediate bytes then final byte
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // other ESC sequences: skip next char
        } else {
            out.push(c);
        }
    }
    out
}
