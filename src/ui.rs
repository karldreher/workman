use crate::app::{App, FuzzyEntry, InputMode, Selection};
use crate::shortcuts::{global_shortcuts, project_shortcuts, worktree_shortcuts, Shortcut};
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
        .map(|(text, _sel, style)| ListItem::new(text.as_str()).style(*style))
        .collect();

    let tree_block = Block::default()
        .borders(Borders::ALL)
        .title(" Projects ")
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
            Constraint::Length(4), // 2 content rows + top/bottom borders
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
        InputMode::AddingRepo => " Add Repo ",
        InputMode::Options => " Options ",
        InputMode::Help => " Help ",
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

    // AddingRepo: fuzzy path picker
    if app.input_mode == InputMode::AddingRepo {
        render_add_repo(f, app, output_block, right_chunks[1]);
        return;
    }

    // Options overlay
    if app.input_mode == InputMode::Options {
        render_options(f, app, output_block, right_chunks[1]);
        return;
    }

    // Help view
    if app.input_mode == InputMode::Help {
        render_help(f, output_block, right_chunks[1]);
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
    let mut output_lines: Vec<Line> = Vec::new();

    if let Some(err) = &app.error_message {
        output_lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(Color::Yellow),
        )));
        if let Some(detail) = &app.full_error_detail {
            output_lines.push(Line::from(Span::styled(
                format!("  DETAIL: {}", detail),
                Style::default().fg(Color::DarkGray),
            )));
        }
    }

    if !app.command_output.is_empty() {
        if app.input_mode == InputMode::ViewingDiff {
            let num_display_lines = right_chunks[1].height.saturating_sub(2) as usize;
            let start = app.diff_scroll_offset;
            let end = (start + num_display_lines).min(app.command_output.len());
            for line in &app.command_output[start..end] {
                output_lines.push(Line::from(line.as_str()));
            }
        } else {
            for line in &app.command_output {
                output_lines.push(Line::from(line.as_str()));
            }
        }
    }

    match app.input_mode {
        InputMode::AddingProjectName => {
            let dim = Style::default().fg(Color::DarkGray);
            output_lines.push(Line::from(""));
            output_lines.push(Line::from(Span::styled(
                "  A project is a temporary unit of work — a named grouping of context",
                dim,
            )));
            output_lines.push(Line::from(Span::styled(
                "  across one or more repos. Give it a short name; the branch is derived",
                dim,
            )));
            output_lines.push(Line::from(Span::styled(
                "  from it automatically.",
                dim,
            )));
            output_lines.push(Line::from(""));
            output_lines.push(Line::from(vec![
                Span::styled("  Project name> ", Style::default().fg(Color::Yellow)),
                Span::raw(app.input.as_str()),
                Span::styled("_", dim),
            ]));
        }
        InputMode::EditingCommitMessage => {
            output_lines.push(Line::from(format!("  Commit msg> {}", app.input)));
        }
        _ => {}
    }

    // Contextual hints when output pane is otherwise empty
    if output_lines.is_empty() && app.input_mode == InputMode::Normal {
        let hint_style = Style::default().fg(Color::DarkGray);
        match app.get_selected_selection() {
            None if app.config.projects.is_empty() => {
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  Welcome to workman!", Style::default().fg(Color::Cyan))));
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  (n)ew project  (h)elp", hint_style)));
            }
            Some(Selection::Project(p_idx)) if app.config.projects[p_idx].worktrees.is_empty() => {
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  No repos in this project yet.", hint_style)));
                output_lines.push(Line::from(Span::styled("  (a)dd a repo — creates a worktree on the project branch", hint_style)));
            }
            _ => {}
        }
    }

    let output_paragraph = Paragraph::new(output_lines)
        .block(output_block)
        .wrap(ratatui::widgets::Wrap { trim: false });
    f.render_widget(output_paragraph, right_chunks[1]);
}

/// Renders a `Shortcut` as `(k)ey label` spans.
/// If the shortcut key matches the first letter of the label, the key is shown
/// inline: `(t)erminal`. Otherwise it appears before the label: `(x) remove`.
fn render_shortcut(s: &Shortcut) -> Vec<Span<'static>> {
    let ks = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let first = s.label.chars().next().map(|c| c.to_ascii_lowercase());
    if first == Some(s.key) {
        let rest = &s.label[s.key.len_utf8()..];
        vec![
            Span::raw("("),
            Span::styled(s.key.to_string(), ks),
            Span::raw(format!("){rest}  ")),
        ]
    } else {
        vec![
            Span::raw("("),
            Span::styled(s.key.to_string(), ks),
            Span::raw(format!(") {}  ", s.label)),
        ]
    }
}

/// Renders a named / multi-character key (Enter, Esc, Ctrl+C, ↑↓ …) + label.
fn named_key(key: &'static str, label: &'static str) -> Vec<Span<'static>> {
    let ks = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    vec![
        Span::styled(key, ks),
        Span::raw(format!(" {label}  ")),
    ]
}

fn build_help_lines(app: &App) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    match app.input_mode {
        InputMode::Normal => {
            // Line 1: context-specific shortcuts
            match app.get_selected_selection() {
                Some(Selection::Project(_)) => {
                    let mut spans: Vec<Span> = vec![Span::raw(" ")];
                    spans.extend(named_key("Enter", "expand"));
                    for s in project_shortcuts() {
                        spans.extend(render_shortcut(&s));
                    }
                    lines.push(Line::from(spans));
                }
                Some(Selection::Worktree(_, _)) => {
                    let mut spans: Vec<Span> = vec![Span::raw(" ")];
                    for s in worktree_shortcuts() {
                        spans.extend(render_shortcut(&s));
                    }
                    lines.push(Line::from(spans));
                }
                _ => {
                    // No selection — global line handles everything; push empty first line
                    lines.push(Line::from(""));
                }
            }
            // Line 2: always-visible global shortcuts
            let mut global: Vec<Span> = vec![Span::raw(" ")];
            for s in global_shortcuts() {
                global.extend(render_shortcut(&s));
            }
            lines.push(Line::from(global));
        }
        InputMode::AddingProjectName => {
            lines.push(Line::from(" Enter a name for the project"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" create   "),
                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" cancel"),
            ]));
        }
        InputMode::AddingRepo => {
            lines.push(Line::from(" Type a path, or ↑/↓ to browse  Tab: complete path"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" add repo   "),
                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" (empty) done   "),
                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" cancel"),
            ]));
        }
        InputMode::ViewingDiff => {
            lines.push(Line::from(" Viewing diff"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" scroll   "),
                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" exit"),
            ]));
        }
        InputMode::EditingCommitMessage => {
            lines.push(Line::from(" Commit message (blank = auto-message)"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("Enter", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" confirm   "),
                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" cancel"),
            ]));
        }
        InputMode::Terminal => {
            if let Some(warning) = &app.terminal_warning {
                lines.push(Line::from(Span::styled(
                    format!(" {}", warning),
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(""));
            } else {
                lines.push(Line::from(" Terminal attached"));
                lines.push(Line::from(vec![
                    Span::raw(" "),
                    Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(" detach   "),
                    Span::styled("Ctrl-B D", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(" detach tmux"),
                ]));
            }
        }
        InputMode::Options => {
            lines.push(Line::from(" Settings"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" navigate   "),
                Span::styled("Space", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" toggle   "),
                Span::styled("Esc", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" close"),
            ]));
        }
        InputMode::Help => {
            lines.push(Line::from(" Key reference"));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("Any key", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::raw(" close"),
            ]));
        }
    }
    lines
}

fn render_add_repo(
    f: &mut ratatui::Frame,
    app: &App,
    block: Block,
    area: ratatui::layout::Rect,
) {
    let mut lines: Vec<Line> = Vec::new();
    let dim = Style::default().fg(Color::DarkGray);

    // Context header
    if let Some(p_idx) = app.adding_to_project {
        if p_idx < app.config.projects.len() {
            let p = &app.config.projects[p_idx];
            lines.push(Line::from(Span::styled(
                format!(" Adding to \"{}\"  branch: {}", p.name, p.branch),
                Style::default().fg(Color::Cyan),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Type a path to a git repo, or use ↑/↓ to browse suggestions.",
        dim,
    )));
    lines.push(Line::from(Span::styled(
        "  Tab to complete a path. Enter to add. Empty Enter when done.",
        dim,
    )));
    lines.push(Line::from(""));

    // Error (if any)
    if let Some(err) = &app.error_message {
        lines.push(Line::from(Span::styled(format!("  {}", err), Style::default().fg(Color::Yellow))));
        lines.push(Line::from(""));
    }

    // Input line
    lines.push(Line::from(vec![
        Span::styled("  Path> ", Style::default().fg(Color::Yellow)),
        Span::raw(app.input.as_str()),
        Span::styled("_", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(""));

    // Suggestions
    if app.fuzzy_results.is_empty() {
        lines.push(Line::from(Span::styled("  Type a path to a git repository.", dim)));
    } else {
        // Separate known (promoted) from filesystem entries
        let known: Vec<(usize, &FuzzyEntry)> = app.fuzzy_results.iter().enumerate().filter(|(_, e)| e.known).collect();
        let new_dirs: Vec<(usize, &FuzzyEntry)> = app.fuzzy_results.iter().enumerate().filter(|(_, e)| !e.known).collect();

        if !known.is_empty() {
            lines.push(Line::from(Span::styled("  Previously used:", dim)));
            for (i, entry) in &known {
                let selected = app.fuzzy_cursor == Some(*i);
                let cursor = if selected { ">" } else { " " };
                let style = if selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green)
                };
                lines.push(Line::from(Span::styled(
                    format!("  {}  {}", cursor, entry.path.display()),
                    style,
                )));
            }
        }

        if !new_dirs.is_empty() {
            if !known.is_empty() { lines.push(Line::from("")); }
            lines.push(Line::from(Span::styled("  Filesystem:", dim)));
            for (i, entry) in &new_dirs {
                let is_git = entry.path.join(".git").exists();
                let selected = app.fuzzy_cursor == Some(*i);
                let cursor = if selected { ">" } else { " " };
                let style = if selected {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if is_git {
                    Style::default()
                } else {
                    dim
                };
                let label = if is_git {
                    format!("  {}  {}/", cursor, entry.path.display())
                } else {
                    format!("  {}  {}/  (no .git)", cursor, entry.path.display())
                };
                lines.push(Line::from(Span::styled(label, style)));
            }
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

fn render_help(
    f: &mut ratatui::Frame,
    block: Block,
    area: ratatui::layout::Rect,
) {
    let h = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let k = Style::default().fg(Color::Yellow);
    let d = Style::default();
    let dim = Style::default().fg(Color::DarkGray);

    macro_rules! row {
        ($key:expr, $desc:expr) => {
            Line::from(vec![
                Span::styled(format!("  {:20}", $key), k),
                Span::styled($desc, d),
            ])
        };
    }

    let lines: Vec<Line> = vec![
        Line::from(Span::styled(" Global", h)),
        row!("q / Ctrl+C", "Quit"),
        row!("↑ / ↓", "Navigate"),
        row!("n", "(n)ew project"),
        row!("o", "(o)ptions"),
        row!("h", "(h)elp — this screen"),
        row!("Ctrl+L", "Export log to /tmp/workman.log"),
        Line::from(""),
        Line::from(Span::styled(" Project selected", h)),
        row!("Enter", "Expand / collapse"),
        row!("a", "(a)dd repo — creates worktree on project branch"),
        row!("p", "(p)ush all worktrees"),
        row!("t", "(t)erminal at project folder"),
        row!("x", "(x) remove project and all its worktrees"),
        Line::from(""),
        Line::from(Span::styled(" Worktree selected", h)),
        row!("t", "(t)erminal in worktree"),
        row!("p", "(p)ush"),
        row!("d", "(d)iff  (↑↓ scroll, Esc exit)"),
        row!("x", "(x) remove worktree"),
        Line::from(""),
        Line::from(Span::styled(" Terminal (in-app PTY)", h)),
        row!("Esc", "Detach — session stays alive"),
        row!("Ctrl+C", "Send interrupt to shell"),
        Line::from(""),
        Line::from(Span::styled(" Tmux mode (Use Tmux = on)", h)),
        row!("Ctrl-B D", "Detach from tmux session"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close", dim)),
    ];

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
