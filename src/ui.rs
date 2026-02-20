use crate::app::{App, InputMode, Selection};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

pub fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(25), // Thinner column for the tree view
                Constraint::Percentage(75), // Wider column for actions and help
            ]
            .as_ref(),
        )
        .split(f.area());

    // Tree Panel
    let items_with_data = app.get_tree_items();
    let tree_items: Vec<ListItem> = items_with_data
        .iter()
        .map(|(text, _, style)| ListItem::new(text.as_str()).style(*style))
        .collect();

    let tree_title = "Repos & Worktrees".to_string();

    let tree_block = Block::default()
        .borders(Borders::ALL)
        .title(tree_title.as_str())
        .border_style(if app.input_mode == InputMode::Normal { Style::default().fg(Color::Yellow) } else { Style::default() }); // Highlight if in normal mode

    let tree_list = List::new(tree_items)
        .block(tree_block)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol("> ");
    f.render_stateful_widget(tree_list, main_layout[0], &mut app.tree_state);

    // Right Panel: Split into Help Bar and Output/Command Pane
    let right_panel_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Thin help bar (3 lines)
            Constraint::Min(0),    // Large output/command pane
        ].as_ref())
        .split(main_layout[1]);

    // TOP-RIGHT: Help Bar
    let help_block = Block::default()
        .borders(Borders::ALL)
        .title("Help")
        .border_style(Style::default().fg(Color::LightBlue)); // Always active and visible

    let mut help_text_lines = Vec::new();
    match app.input_mode {
        InputMode::Normal => {
            match app.get_selected_selection() {
                Some(Selection::Project(_)) => {
                    help_text_lines.push(" 'a': Add Project, 'x': Del Project, 'w': Add Worktree".to_string());
                },
                Some(Selection::Worktree(_, _)) => {
                    help_text_lines.push(" 'c': Run Cmd, 'p': Push, 'r': Rm Worktree, 'd': Show Diff".to_string());
                },
                None => {
                    help_text_lines.push(" 'a': Add Project".to_string());
                }
            }
            help_text_lines.push(" Arrows: Navigate, 'q': Quit, Ctrl+L: Export log".to_string());
        },
        InputMode::AddingProjectPath => {
            help_text_lines.push(" Enter Project Path (Tab: autocomplete, Esc: cancel)".to_string());
        },
        InputMode::AddingWorktreeName => {
            help_text_lines.push(" Enter Worktree Name (Esc: cancel)".to_string());
        },
        InputMode::RunningCommand => {
            help_text_lines.push(" Enter Command (Esc: cancel)".to_string());
        },
        InputMode::ViewingDiff => {
            help_text_lines.push(" Viewing Diff (Space: scroll, Esc: exit)".to_string());
        },
    }

    let help_paragraph = Paragraph::new(help_text_lines.join("
"))
        .block(help_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help_paragraph, right_panel_chunks[0]);


    // BOTTOM-RIGHT: Output / Command Pane
    let output_pane_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.input_mode == InputMode::RunningCommand { "Command / Output" } else { "Output" })
        .border_style(if app.input_mode != InputMode::Normal { Style::default().fg(Color::Yellow) } else { Style::default() }); // Highlight if active input mode

    let mut output_content_lines = Vec::new();

    // Errors always prepend
    if let Some(err) = &app.error_message {
        output_content_lines.push(format!("ERROR: {}", err));
        if let Some(detail) = &app.full_error_detail {
            output_content_lines.push(format!("DETAIL: {}", detail));
        }
    }

    // Command output or diff
    if !app.command_output.is_empty() {
        if app.input_mode == InputMode::ViewingDiff {
            // Apply scrolling for diff output
            let num_display_lines = (right_panel_chunks[1].height as usize) - 2; // Account for borders
            let start_index = app.diff_scroll_offset;
            let end_index = (start_index + num_display_lines).min(app.command_output.len());
            output_content_lines.extend(
                app.command_output[start_index..end_index].iter().cloned()
            );
        } else {
            output_content_lines.extend(app.command_output.iter().cloned());
        }
    }

    // Input prompt and current input if active
    if app.input_mode != InputMode::Normal && app.input_mode != InputMode::ViewingDiff {
        let prompt = match app.input_mode {
            InputMode::AddingProjectPath => "Path> ".to_string(),
            InputMode::AddingWorktreeName => "Name> ".to_string(),
            InputMode::RunningCommand => "> ".to_string(),
            _ => "> ".to_string(), // Should not happen for these modes
        };
        output_content_lines.push(format!("{}{}", prompt, app.input));
    } else if app.input_mode == InputMode::Normal && app.input.len() > 0 {
         // Show pending input even in normal mode if something was typed and not submitted/cleared
         output_content_lines.push(format!("> {}", app.input));
    }
    
    let output_paragraph = Paragraph::new(output_content_lines.join("
"))
        .block(output_pane_block)
        .wrap(ratatui::widgets::Wrap { trim: false }); // Do not trim for diff/command output
    f.render_widget(output_paragraph, right_panel_chunks[1]);
}
