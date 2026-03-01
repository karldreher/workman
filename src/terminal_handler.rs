use crate::app::{App, InputMode};
use crossterm::event::{self, KeyCode, KeyEvent};

pub fn handle_terminal_key_event(key: KeyEvent, app: &mut App) {
    match key {
        event::KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: event::KeyModifiers::CONTROL,
            ..
        } => {
            if let Some(sel) = app.get_selected_selection() {
                if let Some(session) = app.sessions.get_mut(&sel) {
                    let _ = session.write(&[3]); // Send ETX (Ctrl-C)
                    app.terminal_warning = Some(
                        "Ctrl-C sent. Use 'exit' or Ctrl-D to close the shell. Press Esc to detach."
                            .to_string(),
                    );
                }
            }
        }
        event::KeyEvent {
            code: KeyCode::Esc, ..
        } => {
            app.input_mode = InputMode::Normal;
            app.terminal_warning = None; // Clear warning on detach
        }
        _ => {
            if let Some(sel) = app.get_selected_selection() {
                if let Some(session) = app.sessions.get_mut(&sel) {
                    // Clear warning on any other keypress
                    if app.terminal_warning.is_some() {
                        app.terminal_warning = None;
                    }

                    // Send key to PTY
                    let data = match key.code {
                        KeyCode::Char(c) => {
                            let mut buf = [0u8; 4];
                            c.encode_utf8(&mut buf).as_bytes().to_vec()
                        }
                        KeyCode::Enter => vec![b'\r'],
                        KeyCode::Backspace => vec![8],
                        KeyCode::Tab => vec![9],
                        KeyCode::Up => vec![27, 91, 65],
                        KeyCode::Down => vec![27, 91, 66],
                        KeyCode::Right => vec![27, 91, 67],
                        KeyCode::Left => vec![27, 91, 68],
                        // Add more key codes as needed
                        _ => Vec::new(), // Don't send unknown keys
                    };
                    if !data.is_empty() {
                        let _ = session.write(&data);
                    }
                }
            }
        }
    }
}
