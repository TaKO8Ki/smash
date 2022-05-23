use crossterm::event::{
    read, DisableMouseCapture, EnableMouseCapture, Event as TermEvent, KeyCode, KeyEvent,
    KeyModifiers,
};
use crossterm::style::{Attribute, Color, Print, SetAttribute, SetForegroundColor};
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use signal_hook::{self, iterator::Signals};
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;
use tracing_subscriber;

struct Shell {}

enum Event {
    Input(TermEvent),
    ScreenResized,
    NoCompletion,
}

impl Shell {
    fn new() -> Self {
        Self {}
    }
}

struct SmashState {
    columns: usize,
    shell: Shell,
}

impl SmashState {
    fn render_prompt(&mut self) {
        let screen_size = terminal::size().unwrap();
        self.columns = screen_size.0 as usize;

        tracing::debug!(?self.columns);

        let mut stdout = std::io::stdout();
        queue!(
            stdout,
            SetAttribute(Attribute::Bold),
            SetAttribute(Attribute::Reverse),
            Print("$"),
            SetAttribute(Attribute::Reset),
            Print(&format!(
                "{space:>width$}\r",
                space = " ",
                width = self.columns - 1
            ))
        )
        .ok();

        let (mut prompt_str, mut prompt_len) = (String::new(), 0);
        if let Ok(current_dir) = std::env::current_dir() {
            let mut path = current_dir.to_str().unwrap().to_string();

            // "/Users/chandler/games/doom" -> "~/venus/games/doom"
            if let Some(home_dir) = dirs::home_dir() {
                let home_dir = home_dir.to_str().unwrap();
                if path.starts_with(&home_dir) {
                    path = path.replace(home_dir, "~");
                }
            }

            prompt_len += path.len();
            prompt_str.push_str(&path);
        }
        prompt_str.push_str(" $ ");
        queue!(stdout, Print(prompt_str.replace("\n", "\r\n"))).ok();
        stdout.flush().unwrap();
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let mut shell = Shell::new();
    let mut state = SmashState { columns: 0, shell };

    for (key, value) in std::env::vars() {}

    enable_raw_mode().ok();
    state.render_prompt();

    let (tx, rx) = mpsc::channel();
    let tx2 = tx.clone();
    std::thread::spawn(move || {
        let signals = Signals::new(&[signal_hook::SIGWINCH]).unwrap();
        for signal in signals {
            match signal {
                signal_hook::SIGWINCH => {
                    tx2.send(Event::ScreenResized).ok();
                }
                _ => {
                    tracing::warn!("unhandled signal: {}", signal);
                }
            }
        }

        unreachable!();
    });

    'main: loop {
        let mut started_at = None;

        match crossterm::event::poll(Duration::from_millis(100)) {
            Ok(true) => loop {
                tracing::debug!("eventtttttttttttttttttt");
                if let Ok(TermEvent::Key(ev)) = crossterm::event::read() {
                    match (ev.code, ev.modifiers) {
                        (KeyCode::Char('q'), KeyModifiers::NONE) => break 'main,
                        (KeyCode::Enter, KeyModifiers::NONE) => {
                            print!("\r\n");
                            disable_raw_mode().ok();
                            enable_raw_mode().ok();
                            state.render_prompt();
                        }
                        _ => (),
                    }
                }

                match crossterm::event::poll(Duration::from_millis(0)) {
                    Ok(true) => (),
                    _ => break,
                }
            },
            _ => {
                if let Ok(_) = rx.try_recv() {
                    started_at = Some(std::time::SystemTime::now());
                    // self.handle_event(ev);
                }
            }
        }
    }

    // execute!(stdout, terminal::LeaveAlternateScreen).unwrap();

    terminal::disable_raw_mode().unwrap();
}
