mod app;
mod event_handler;
mod models;
mod session;
mod terminal_handler;
mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    Terminal,
};
use std::{io, sync::atomic::{AtomicBool, Ordering}, sync::Arc};

use crate::app::App;
use crate::ui::ui;

struct TerminalRestorer;

impl Drop for TerminalRestorer {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _term_restorer = TerminalRestorer;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let mut signals = signal_hook::iterator::Signals::new(&[
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
    ])?;
    std::thread::spawn(move || {
        for _ in signals.forever() {
            r.store(false, Ordering::SeqCst);
            break;
        }
    });

    let app = App::new();
    let res = run_app(&mut terminal, app, running).await;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend + io::Write>(
    terminal: &mut Terminal<B>,
    mut app: App,
    running: Arc<AtomicBool>,
) -> Result<()> {
    while running.load(Ordering::SeqCst) {
        let terminal_area = terminal.get_frame().area();
        let current_width = terminal_area.width;
        let current_height = terminal_area.height;

        // Resize active PTY session if terminal dimensions changed
        if let Some(sel) = app.get_selected_selection() {
            if let Some(session) = app.sessions.get_mut(&sel) {
                let _ = session.resize(current_width, current_height);
            }
        }

        terminal.draw(|f| ui(f, &mut app)).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match event_handler::handle_key_event(key, &mut app, current_width, current_height).await? {
                        event_handler::AppState::Quit => return Ok(()),
                        event_handler::AppState::Continue => {}
                        event_handler::AppState::TmuxSession { path, session_name } => {
                            // Suspend workman: restore normal terminal mode
                            disable_raw_mode()?;
                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                            // Hand off to tmux (-A: attach if exists, else create)
                            let _ = std::process::Command::new("tmux")
                                .args(["new-session", "-A", "-s", &session_name, "-c"])
                                .arg(&path)
                                .status();

                            // Resume workman
                            enable_raw_mode()?;
                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                            let _ = terminal.clear();
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
