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
    let term_restorer = TerminalRestorer;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    // Handle SIGTERM
    let mut signals = signal_hook::iterator::Signals::new(&[signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])?;
    std::thread::spawn(move || {
        for _ in signals.forever() {
            r.store(false, Ordering::SeqCst);
            break;
        }
    });

    let app = App::new();
    let res = run_app(&mut terminal, app, running).await;

    drop(term_restorer);

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend + io::Write>(terminal: &mut Terminal<B>, mut app: App, running: Arc<AtomicBool>) -> Result<()> {
    while running.load(Ordering::SeqCst) {
        let terminal_area = terminal.get_frame().area();
        let current_width = terminal_area.width;
        let current_height = terminal_area.height;

        // Resize active session if terminal size changed
        if let Some(sel) = app.get_selected_selection() {
            if let Some(session) = app.sessions.get_mut(&sel) {
                // Check if current terminal dimensions are different from session
                // This is a placeholder, a better way would be to query actual PTY size or store it
                let _ = session.resize(current_width, current_height);
            }
        }


        terminal.draw(|f| ui(f, &mut app)).map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press { // Only process key press events
                    match event_handler::handle_key_event(key, &mut app, current_width, current_height).await? {
                        event_handler::AppState::Quit => return Ok(()),
                        event_handler::AppState::Continue => {},
                    }
                }
            }
        }
    }
    Ok(())
}
