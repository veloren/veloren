use crate::LOG;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};
use tracing::{debug, error, info, warn};
use tui::{
    backend::CrosstermBackend,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Terminal,
};

#[derive(Debug, Clone)]
pub enum Message {
    Quit,
}

pub struct Command<'a> {
    pub name: &'a str,
    pub description: &'a str,
    // Whether or not the command splits the arguments on whitespace
    pub split_spaces: bool,
    pub args: usize,
    pub cmd: fn(Vec<String>, &mut mpsc::Sender<Message>),
}

pub const COMMANDS: [Command; 2] = [
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

pub struct Tui {
    msg_s: Option<mpsc::Sender<Message>>,
    background: Option<std::thread::JoinHandle<()>>,
    running: Arc<AtomicBool>,
}

impl Tui {
    pub fn new() -> (Self, mpsc::Receiver<Message>) {
        let (msg_s, msg_r) = mpsc::channel();
        (
            Self {
                msg_s: Some(msg_s),
                background: None,
                running: Arc::new(AtomicBool::new(true)),
            },
            msg_r,
        )
    }

    fn handle_events(input: &mut String, msg_s: &mut mpsc::Sender<Message>) {
        use crossterm::event::*;
        if let Event::Key(event) = read().unwrap() {
            match event.code {
                KeyCode::Char('c') => {
                    if event.modifiers.contains(KeyModifiers::CONTROL) {
                        msg_s.send(Message::Quit).unwrap()
                    } else {
                        input.push('c');
                    }
                },
                KeyCode::Char(c) => input.push(c),
                KeyCode::Backspace => {
                    input.pop();
                },
                KeyCode::Enter => {
                    debug!(?input, "tui mode: command entered");
                    parse_command(input, msg_s);

                    *input = String::new();
                },
                _ => {},
            }
        }
    }

    pub fn run(&mut self, basic: bool) {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Self::shutdown();
            hook(info);
        }));

        let mut msg_s = self.msg_s.take().unwrap();
        let running = Arc::clone(&self.running);

        if basic {
            std::thread::spawn(move || {
                while running.load(Ordering::Relaxed) {
                    let mut line = String::new();

                    match io::stdin().read_line(&mut line) {
                        Err(e) => {
                            error!(
                                ?e,
                                "couldn't read from stdin, cli commands are disabled now!"
                            );
                            break;
                        },
                        Ok(0) => {
                            //Docker seem to send EOF all the time
                            warn!("EOF received, cli commands are disabled now!");
                            break;
                        },
                        Ok(_) => {
                            debug!(?line, "basic mode: command entered");
                            parse_command(&line, &mut msg_s);
                        },
                    }
                }
            });
        } else {
            self.background = Some(std::thread::spawn(move || {
                // Start the tui
                let mut stdout = io::stdout();
                execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();

                enable_raw_mode().unwrap();

                let backend = CrosstermBackend::new(stdout);
                let mut terminal = Terminal::new(backend).unwrap();

                let mut input = String::new();

                if let Err(e) = terminal.clear() {
                    error!(?e, "clouldn't clean terminal");
                };

                while running.load(Ordering::Relaxed) {
                    if let Err(e) = terminal.draw(|f| {
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

                        let mut wrap = Wrap::default();
                        wrap.scroll_callback = Some(Box::new(|text_area, lines| {
                            LOG.resize(text_area.height as usize);
                            let len = lines.len() as u16;
                            (len.saturating_sub(text_area.height), 0)
                        }));

                        let logger = Paragraph::new(LOG.inner.lock().unwrap().clone())
                            .block(block)
                            .wrap(wrap);
                        f.render_widget(logger, log_rect);

                        let text: Text = input.as_str().into();

                        let block = Block::default().borders(Borders::ALL);
                        let size = block.inner(input_rect);

                        let x = (size.x + text.width() as u16).min(size.width);

                        let input_field = Paragraph::new(text).block(block);
                        f.render_widget(input_field, input_rect);

                        f.set_cursor(x, size.y);
                    }) {
                        warn!(?e, "couldn't draw frame");
                    };
                    if crossterm::event::poll(Duration::from_millis(100)).unwrap() {
                        Self::handle_events(&mut input, &mut msg_s);
                    };
                }
            }));
        }
    }

    fn shutdown() {
        let mut stdout = io::stdout();

        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
        disable_raw_mode().unwrap();
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.background.take().map(|m| m.join());
        Self::shutdown();
    }
}

fn parse_command(input: &str, msg_s: &mut mpsc::Sender<Message>) {
    let mut args = input.split_whitespace();

    if let Some(cmd_name) = args.next() {
        if let Some(cmd) = COMMANDS.iter().find(|cmd| cmd.name == cmd_name) {
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
                std::cmp::Ordering::Less => error!("{} takes {} arguments", cmd_name, cmd.args),
                std::cmp::Ordering::Greater => {
                    warn!("{} only takes {} arguments", cmd_name, cmd.args);
                    let cmd = cmd.cmd;

                    cmd(args, msg_s)
                },
                std::cmp::Ordering::Equal => {
                    let cmd = cmd.cmd;

                    cmd(args, msg_s)
                },
            }
        } else {
            error!("{} not found", cmd_name);
        }
    }
}
