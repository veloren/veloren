#![deny(unsafe_code)]

#[macro_use] extern crate lazy_static;

use common::clock::Clock;
use server::{Event, Input, Server, ServerSettings};
use tracing::{error, info, warn, Level};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

use clap::{App, Arg};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Write},
    sync::{mpsc, Arc, Mutex},
    time::Duration,
};
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

const TPS: u64 = 30;
const RUST_LOG_ENV: &str = "RUST_LOG";

#[derive(Debug, Clone)]
enum Message {
    Quit,
}

const COMMANDS: [Command; 2] = [
    Command {
        name: "quit",
        description: "Closes the server",
        split_spaces: true,
        args: 0,
        cmd: |_, sender| sender.send(Message::Quit).unwrap(),
    },
    Command {
        name: "help",
        description: "List all command available",
        split_spaces: true,
        args: 0,
        cmd: |_, _| {
            info!("===== Help =====");
            for command in COMMANDS.iter() {
                info!("{} - {}", command.name, command.description)
            }
            info!("================");
        },
    },
];

struct Command<'a> {
    pub name: &'a str,
    pub description: &'a str,
    // Whether or not the command splits the arguments on whitespace
    pub split_spaces: bool,
    pub args: usize,
    pub cmd: fn(Vec<String>, &mut mpsc::Sender<Message>),
}

lazy_static! {
    static ref LOG: TuiLog<'static> = TuiLog::default();
}

#[derive(Debug, Default, Clone)]
struct TuiLog<'a> {
    inner: Arc<Mutex<Text<'a>>>,
}

impl<'a> TuiLog<'a> {
    fn resize(&self, h: usize) {
        let mut inner = self.inner.lock().unwrap();

        if inner.height() > h {
            let length = inner.height() - h;
            inner.lines.drain(0..length);
        }
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

fn main() -> io::Result<()> {
    let matches = App::new("Veloren server cli")
        .version(
            format!(
                "{}-{}",
                env!("CARGO_PKG_VERSION"),
                common::util::GIT_HASH.to_string()
            )
            .as_str(),
        )
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("The veloren server cli provides an easy to use interface to start a veloren server")
        .arg(
            Arg::with_name("basic")
                .short("b")
                .long("basic")
                .help("Disables the tui")
                .takes_value(false),
        )
        .get_matches();

    let basic = matches.is_present("basic");

    // Init logging
    let filter = match std::env::var_os(RUST_LOG_ENV).map(|s| s.into_string()) {
        Some(Ok(env)) => {
            let mut filter = EnvFilter::new("veloren_world::sim=info")
                .add_directive("veloren_world::civ=info".parse().unwrap())
                .add_directive(LevelFilter::INFO.into());
            for s in env.split(',').into_iter() {
                match s.parse() {
                    Ok(d) => filter = filter.add_directive(d),
                    Err(err) => println!("WARN ignoring log directive: `{}`: {}", s, err),
                };
            }
            filter
        },
        _ => EnvFilter::from_env(RUST_LOG_ENV)
            .add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive(LevelFilter::INFO.into()),
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter(filter);

    if basic {
        subscriber.init();
    } else {
        subscriber.with_writer(|| LOG.clone()).init();
    }

    let (sender, receiver) = mpsc::channel();

    if !basic {
        start_tui(sender);
    }

    info!("Starting server...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();
    let server_port = &settings.gameserver_address.port();
    let metrics_port = &settings.metrics_address.port();
    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    info!("Server is ready to accept connections.");
    info!(?metrics_port, "starting metrics at port");
    info!(?server_port, "starting server at port");

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity: _ } => info!("Client connected!"),
                Event::ClientDisconnected { entity: _ } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        match receiver.try_recv() {
            Ok(msg) => match msg {
                Message::Quit => {
                    info!("Closing the server");
                    break;
                },
            },
            Err(e) => match e {
                mpsc::TryRecvError::Empty => {},
                mpsc::TryRecvError::Disconnected => panic!(),
            },
        };

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }

    if !basic {
        stop_tui();
    }

    Ok(())
}

fn start_tui(mut sender: mpsc::Sender<Message>) {
    enable_raw_mode().unwrap();
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();

    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        stop_tui();
        hook(info);
    }));

    std::thread::spawn(move || {
        // Start the tui
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();

        let mut input = String::new();

        let _ = terminal.clear();

        loop {
            let _ = terminal.draw(|f| {
                let (log_rect, input_rect) = if f.size().height > 6 {
                    let mut log_rect = f.size();
                    log_rect.height -= 3;

                    let mut input_rect = f.size();
                    input_rect.y = input_rect.height - 3;
                    input_rect.height = 3;

                    (log_rect, input_rect)
                } else {
                    (f.size(), Rect::default())
                };

                let block = Block::default().borders(Borders::ALL);
                let size = block.inner(log_rect);

                LOG.resize(size.height as usize);

                let logger = Paragraph::new(LOG.inner.lock().unwrap().clone())
                    .block(block)
                    .wrap(Wrap { trim: false });
                f.render_widget(logger, log_rect);

                let text: Text = input.as_str().into();

                let block = Block::default().borders(Borders::ALL);
                let size = block.inner(input_rect);

                let x = (size.x + text.width() as u16).min(size.width);

                let input_field = Paragraph::new(text).block(block);
                f.render_widget(input_field, input_rect);

                f.set_cursor(x, size.y);

                use crossterm::event::*;

                if poll(Duration::from_millis(10)).unwrap() {
                    if let Event::Key(event) = read().unwrap() {
                        match event.code {
                            KeyCode::Char('c') => {
                                if event.modifiers.contains(KeyModifiers::CONTROL) {
                                    sender.send(Message::Quit).unwrap()
                                } else {
                                    input.push('c');
                                }
                            },
                            KeyCode::Char(c) => input.push(c),
                            KeyCode::Backspace => {
                                input.pop();
                            },
                            KeyCode::Enter => {
                                let mut args = input.as_str().split_whitespace();

                                if let Some(cmd_name) = args.next() {
                                    if let Some(cmd) =
                                        COMMANDS.iter().find(|cmd| cmd.name == cmd_name)
                                    {
                                        let args = args.collect::<Vec<_>>();

                                        let (arg_len, args) = if cmd.split_spaces {
                                            (
                                                args.len(),
                                                args.into_iter()
                                                    .map(|s| s.to_string())
                                                    .collect::<Vec<String>>(),
                                            )
                                        } else {
                                            (1, vec![args.into_iter().collect::<String>()])
                                        };

                                        match arg_len.cmp(&cmd.args) {
                                            std::cmp::Ordering::Less => {
                                                error!("{} takes {} arguments", cmd_name, cmd.args)
                                            },
                                            std::cmp::Ordering::Greater => {
                                                warn!(
                                                    "{} only takes {} arguments",
                                                    cmd_name, cmd.args
                                                );
                                                let cmd = cmd.cmd;

                                                cmd(args, &mut sender)
                                            },
                                            std::cmp::Ordering::Equal => {
                                                let cmd = cmd.cmd;

                                                cmd(args, &mut sender)
                                            },
                                        }
                                    } else {
                                        error!("{} not found", cmd_name);
                                    }
                                }

                                input = String::new();
                            },
                            _ => {},
                        }
                    }
                }
            });
        }
    });
}

fn stop_tui() {
    let mut stdout = io::stdout();

    disable_raw_mode().unwrap();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
}
