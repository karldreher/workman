use crate::app::{App, InputMode, Selection};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    text::{Line, Span},
};
use vt100::Color as Vt100Color;

pub fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(67),
        ].as_ref())
        .split(f.area());

    // ── Left Panel: Project & Repo Tree ──────────────────────────────────
    let items_with_data = app.get_tree_items();
    let tree_items: Vec<ListItem> = items_with_data
        .iter()
        .map(|(text, sel, style)| {
            let s = if *sel == Selection::Separator {
                style.add_modifier(Modifier::DIM)
            } else {
                *style
            };
            ListItem::new(text.as_str()).style(s)
        })
        .collect();

    let tree_block = Block::default()
        .borders(Borders::ALL)
        .title(" Projects & Repos ")
        .border_style(if app.input_mode == InputMode::Normal {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let tree_list = List::new(tree_items)
        .block(tree_block)
        .highlight_style(
            Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(tree_list, main_layout[0], &mut app.tree_state);

    // ── Right Panel: Help bar + Output/Terminal ──────────────────────────
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
        ].as_ref())
        .split(main_layout[1]);

    // Help bar
    let help_block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .border_style(Style::default().fg(Color::LightBlue));

    let help_lines = build_help_lines(app);
    let help_paragraph = Paragraph::new(help_lines)
        .block(help_block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help_paragraph, right_chunks[0]);

    // Output / Terminal pane
    let pane_title = match app.input_mode {
        InputMode::Terminal => " Terminal (Attached) ",
        InputMode::SelectingRepos => " Select Repos ",
        InputMode::Options => " Options ",
        _ => " Output ",
    };
    let output_block = Block::default()
        .borders(Borders::ALL)
        .title(pane_title)
        .border_style(if app.input_mode != InputMode::Normal {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    // SelectingRepos: show checklist in right panel
    if app.input_mode == InputMode::SelectingRepos {
        render_repo_selector(f, app, output_block, right_chunks[1]);
        return;
    }

    // Options overlay
    if app.input_mode == InputMode::Options {
        render_options(f, app, output_block, right_chunks[1]);
        return;
    }

    // Terminal session rendering
    let selected = app.tree_state.selected().and_then(|i| items_with_data.get(i).map(|item| item.1));
    if let Some(sel) = selected {
        if let Some(session) = app.sessions.get(&sel) {
            let parser = session.parser.lock().unwrap();
            let screen = parser.screen();
            let (rows, cols) = screen.size();

            let mut lines = Vec::new();
            for row_idx in 0..rows {
                let mut spans = Vec::new();
                for col_idx in 0..cols {
                    if let Some(cell) = screen.cell(row_idx, col_idx) {
                        let mut style = Style::default();
                        style = style.fg(map_vt100_color(cell.fgcolor()));
                        style = style.bg(map_vt100_color(cell.bgcolor()));
                        if cell.bold() { style = style.add_modifier(Modifier::BOLD); }
                        if cell.italic() { style = style.add_modifier(Modifier::ITALIC); }
                        if cell.underline() { style = style.add_modifier(Modifier::UNDERLINED); }
                        spans.push(Span::styled(cell.contents(), style));
                    } else {
                        spans.push(Span::raw(" "));
                    }
                }
                lines.push(Line::from(spans));
            }

            let terminal_paragraph = Paragraph::new(lines).block(output_block);
            f.render_widget(terminal_paragraph, right_chunks[1]);

            let (cursor_row, cursor_col) = screen.cursor_position();
            let cursor_x = right_chunks[1].x + 1 + cursor_col;
            let cursor_y = right_chunks[1].y + 1 + cursor_row;
            f.set_cursor_position((cursor_x, cursor_y));
            return;
        }
    }

    // Standard output / input prompt rendering
    let mut output_lines: Vec<String> = Vec::new();

    if let Some(err) = &app.error_message {
        output_lines.push(format!("ERROR: {}", err));
        if let Some(detail) = &app.full_error_detail {
            output_lines.push(format!("DETAIL: {}", detail));
        }
    }

    if !app.command_output.is_empty() {
        if app.input_mode == InputMode::ViewingDiff {
            let num_display_lines = right_chunks[1].height.saturating_sub(2) as usize;
            let start = app.diff_scroll_offset;
            let end = (start + num_display_lines).min(app.command_output.len());
            output_lines.extend(app.command_output[start..end].iter().cloned());
        } else {
            output_lines.extend(app.command_output.iter().cloned());
        }
    }

    match app.input_mode {
        InputMode::AddingRepoPath => {
            output_lines.push(format!("Repo path> {}", app.input));
        }
        InputMode::AddingProjectName => {
            output_lines.push(format!("Project name> {}", app.input));
        }
        InputMode::AddingProjectBranch => {
            output_lines.push(format!("Branch name> {}", app.input));
        }
        InputMode::EditingCommitMessage => {
            output_lines.push(format!("Commit msg> {}", app.input));
        }
        _ => {}
    }

    let output_paragraph = Paragraph::new(output_lines.join("\n"))
        .block(output_block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(output_paragraph, right_chunks[1]);
}

fn build_help_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    match app.input_mode {
        InputMode::Normal => {
            match app.get_selected_selection() {
                Some(Selection::Project(_)) => {
                    lines.push(Line::from(" [Enter] expand/collapse  [w] add worktrees  [r] delete project  [p] push all  [c] terminal"));
                }
                Some(Selection::Worktree(_, _)) => {
                    lines.push(Line::from(" [c] terminal  [p] push  [d] diff  [r] remove worktree"));
                }
                Some(Selection::Repo(_)) => {
                    lines.push(Line::from(" [x] remove repo"));
                }
                _ => {
                    lines.push(Line::from(" No selection"));
                }
            }
            lines.push(Line::from(" [n] new project  [a] add repo  [o] options  [q] quit  [Ctrl+L] export log"));
        }
        InputMode::AddingRepoPath => {
            lines.push(Line::from(" Enter repo path (Tab: autocomplete, Enter: confirm, Esc: cancel)"));
        }
        InputMode::AddingProjectName => {
            lines.push(Line::from(" Enter project name (Enter: next, Esc: cancel)"));
        }
        InputMode::AddingProjectBranch => {
            lines.push(Line::from(" Enter branch name for all repos in this project (Enter: next, Esc: cancel)"));
        }
        InputMode::SelectingRepos => {
            lines.push(Line::from(" ↑/↓ navigate  Space: toggle  Enter: confirm  Esc: cancel"));
        }
        InputMode::ViewingDiff => {
            lines.push(Line::from(" Viewing diff (Space: scroll, Esc: exit)"));
        }
        InputMode::EditingCommitMessage => {
            lines.push(Line::from(" Enter commit message (blank = auto, Enter: confirm, Esc: cancel)"));
        }
        InputMode::Terminal => {
            if let Some(warning) = &app.terminal_warning {
                lines.push(Line::from(vec![
                    Span::styled(format!(" {}", warning), Style::default().fg(Color::Yellow)),
                ]));
            } else {
                lines.push(Line::from(" Terminal mode (Esc: detach, Ctrl-B D: detach tmux)"));
            }
        }
        InputMode::Options => {
            lines.push(Line::from(" ↑/↓ navigate  Space/Enter: toggle  Esc: close"));
        }
    }
    lines
}

fn render_repo_selector(
    f: &mut ratatui::Frame,
    app: &App,
    block: Block,
    area: ratatui::layout::Rect,
) {
    let available = app.available_repos();
    let mut lines: Vec<Line> = Vec::new();

    let context = if app.adding_to_project.is_some() {
        format!("Add repos to project '{}' (branch: {})", app.pending_project_name, app.pending_project_branch)
    } else {
        format!("Select repos for project '{}' (branch: {})", app.pending_project_name, app.pending_project_branch)
    };
    lines.push(Line::from(Span::styled(context, Style::default().fg(Color::Cyan))));
    lines.push(Line::from(""));

    if available.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No repos available. Add repos with 'a' first.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (display_idx, (repo_idx, repo)) in available.iter().enumerate() {
            let is_selected = app.repo_selection.get(*repo_idx).copied().unwrap_or(false);
            let is_cursor = display_idx == app.repo_cursor;

            let checkbox = if is_selected { "[x]" } else { "[ ]" };
            let cursor_sym = if is_cursor { "> " } else { "  " };
            let style = if is_cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}  {}  ({})", cursor_sym, checkbox, repo.name, repo.path.display()),
                style,
            )));
        }
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn render_options(
    f: &mut ratatui::Frame,
    app: &App,
    block: Block,
    area: ratatui::layout::Rect,
) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(" Settings", Style::default().add_modifier(Modifier::BOLD))));
    lines.push(Line::from(""));

    let tmux_checked = if app.config.settings.use_tmux { "[x]" } else { "[ ]" };
    let tmux_cursor = if app.options_cursor == 0 { "> " } else { "  " };
    let tmux_style = if app.options_cursor == 0 {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(Span::styled(
        format!("{}{}  Use Tmux  (replaces built-in terminal with tmux sessions)", tmux_cursor, tmux_checked),
        tmux_style,
    )));

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn map_vt100_color(color: Vt100Color) -> Color {
    match color {
        Vt100Color::Default => Color::Reset,
        Vt100Color::Idx(i) => Color::Indexed(i),
        Vt100Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
