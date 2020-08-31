use crate::LOG;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Write},
    sync::mpsc,
    time::Duration,
};
use tracing::{error, info, warn};
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
}

impl Tui {
    pub fn new() -> (Self, mpsc::Receiver<Message>) {
        let (msg_s, msg_r) = mpsc::channel();
        (
            Self {
                msg_s: Some(msg_s),
                background: None,
            },
            msg_r,
        )
    }

    fn inner() {}

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
                    let mut args = input.as_str().split_whitespace();

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
                                std::cmp::Ordering::Less => {
                                    error!("{} takes {} arguments", cmd_name, cmd.args)
                                },
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

                    *input = String::new();
                },
                _ => {},
            }
        }
    }

    pub fn run(&mut self) {
        enable_raw_mode().unwrap();
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture).unwrap();

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            hook(info);
        }));

        let mut msg_s = self.msg_s.take().unwrap();

        self.background = Some(std::thread::spawn(move || {
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

                    let scroll = (LOG.height(size) as i16 - size.height as i16).max(0) as u16;

                    print!("{} {} {}", LOG.height(size) as i16, size.width, size.height);

                    let logger = Paragraph::new(LOG.inner.lock().unwrap().clone())
                        .block(block)
                        .wrap(Wrap { trim: false })
                        .scroll((scroll, 0));
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
                        Self::handle_events(&mut input, &mut msg_s);
                    };
                });
            }
        }));
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        let mut stdout = io::stdout();

        disable_raw_mode().unwrap();
        execute!(stdout, LeaveAlternateScreen, DisableMouseCapture).unwrap();
    }
}
