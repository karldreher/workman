use crate::app::{App, FuzzyEntry, InputMode, Selection};
use crate::shortcuts::{GLOBAL_SHORTCUTS, PROJECT_SHORTCUTS, WORKTREE_SHORTCUTS, Shortcut};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    text::{Line, Span},
};
use vt100::Color as Vt100Color;

pub fn ui(f: &mut ratatui::Frame, app: &mut App) {
    // ── Root layout: full-width help bar at top, content area below ──────
    let root_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // 6 inner rows + top/bottom borders
            Constraint::Min(0),
        ])
        .split(f.area());

    // ── Help bar (full width) — 60/40: description | shortcuts ──────────
    let help_block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .border_style(Style::default().fg(Color::LightBlue));

    let help_inner = help_block.inner(root_layout[0]);
    f.render_widget(help_block, root_layout[0]);

    let help_split = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(help_inner);

    f.render_widget(
        Paragraph::new(context_description(app)).wrap(Wrap { trim: true }),
        help_split[0],
    );
    f.render_widget(
        Paragraph::new(context_shortcut_lines(app)),
        help_split[1],
    );

    // ── Content area: 60% tree | 40% output ─────────────────────────────
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(root_layout[1]);

    let output_area = main_layout[1];

    // ── Left Panel: Project tree ─────────────────────────────────────────
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
        .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
        .highlight_symbol("> ");
    f.render_stateful_widget(tree_list, main_layout[0], &mut app.tree_state);

    // ── Right Panel: Output / Terminal ───────────────────────────────────
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

    if app.input_mode == InputMode::AddingRepo {
        render_add_repo(f, app, output_block, output_area);
        return;
    }
    if app.input_mode == InputMode::Options {
        render_options(f, app, output_block, output_area);
        return;
    }
    if app.input_mode == InputMode::Help {
        render_help(f, output_block, output_area);
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
            f.render_widget(terminal_paragraph, output_area);

            let (cursor_row, cursor_col) = screen.cursor_position();
            f.set_cursor_position((output_area.x + 1 + cursor_col, output_area.y + 1 + cursor_row));
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
            let num_lines = output_area.height.saturating_sub(2) as usize;
            let start = app.diff_scroll_offset;
            let end = (start + num_lines).min(app.command_output.len());
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
            output_lines.push(Line::from(""));
            output_lines.push(Line::from(vec![
                Span::styled("  Project name> ", Style::default().fg(Color::Yellow)),
                Span::raw(app.input.as_str()),
                Span::styled("_", Style::default().fg(Color::DarkGray)),
            ]));
        }
        InputMode::EditingCommitMessage => {
            output_lines.push(Line::from(format!("  Commit msg> {}", app.input)));
        }
        _ => {}
    }

    // Contextual hints when output pane is empty
    if output_lines.is_empty() && app.input_mode == InputMode::Normal {
        let hint = Style::default().fg(Color::DarkGray);
        match app.get_selected_selection() {
            None if app.config.projects.is_empty() => {
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  Welcome to workman!", Style::default().fg(Color::Cyan))));
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  Press (n) to create your first project.", hint)));
            }
            Some(Selection::Project(p_idx)) if app.config.projects[p_idx].worktrees.is_empty() => {
                output_lines.push(Line::from(""));
                output_lines.push(Line::from(Span::styled("  Press (a) to add a repo to this project.", hint)));
            }
            _ => {}
        }
    }

    let output_paragraph = Paragraph::new(output_lines)
        .block(output_block)
        .wrap(Wrap { trim: false });
    f.render_widget(output_paragraph, output_area);
}

/// Renders a `Shortcut` as `(k)ey label` spans (no trailing padding — one per line).
fn render_shortcut(s: &Shortcut) -> Vec<Span<'static>> {
    let ks = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let first = s.label.chars().next().map(|c| c.to_ascii_lowercase());
    if first == Some(s.key) {
        let rest = &s.label[s.key.len_utf8()..];
        vec![
            Span::raw("("),
            Span::styled(s.key.to_string(), ks),
            Span::raw(format!("){rest}")),
        ]
    } else {
        vec![
            Span::raw("("),
            Span::styled(s.key.to_string(), ks),
            Span::raw(format!(") {}", s.label)),
        ]
    }
}

/// One line in the shortcuts column: bold key + dim label.
fn named_key_line(key: &'static str, label: &'static str) -> Line<'static> {
    let ks = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    Line::from(vec![
        Span::styled(key, ks),
        Span::raw(format!("  {label}")),
    ])
}

/// Left column of the help bar: a few sentences describing the current context.
fn context_description(app: &App) -> Vec<Line<'static>> {
    let dim = Style::default().fg(Color::DarkGray);

    // Terminal warning takes priority with a different colour
    if app.input_mode == InputMode::Terminal {
        if let Some(w) = &app.terminal_warning {
            return vec![Line::from(Span::styled(w.clone(), Style::default().fg(Color::Yellow)))];
        }
    }

    let text: String = match app.input_mode {
        InputMode::Normal => match app.get_selected_selection() {
            Some(Selection::Project(p_idx)) => {
                let p = &app.config.projects[p_idx];
                if p.worktrees.is_empty() {
                    format!(
                        "Project \"{}\" has no repos yet. \
                         Press (a) to add a repo — a worktree will be created on branch {}.",
                        p.name, p.branch
                    )
                } else {
                    format!(
                        "Project \"{}\" · branch {}. \
                         Add repos to grow this project, open a terminal at the project root, \
                         or push all worktrees at once.",
                        p.name, p.branch
                    )
                }
            }
            Some(Selection::Worktree(p_idx, w_idx)) => {
                let p = &app.config.projects[p_idx];
                let wt = &p.worktrees[w_idx];
                format!(
                    "Worktree {} in project \"{}\". \
                     Open a terminal to work here, push your changes, or inspect the diff.",
                    wt.repo_name, p.name
                )
            }
            None if app.config.projects.is_empty() => {
                "No projects yet. A project groups worktrees across repos on the same branch \
                 — a temporary unit of work. Press (n) to create one."
                    .to_string()
            }
            _ => "Navigate with ↑↓. Select a project or worktree to see available actions."
                .to_string(),
        },
        InputMode::AddingProjectName => {
            "New project. A project groups worktrees from different repos, all on the same branch \
             — a temporary unit of work. Give it a short name; the branch is derived automatically."
                .to_string()
        }
        InputMode::AddingRepo => {
            if let Some(p_idx) = app.adding_to_project {
                if p_idx < app.config.projects.len() {
                    let p = &app.config.projects[p_idx];
                    return vec![Line::from(Span::styled(
                        format!(
                            "Adding repos to \"{}\" (branch: {}). \
                             Each repo you add creates a worktree on that branch. \
                             Type a path or pick from suggestions. \
                             Press Enter on an empty line when done.",
                            p.name, p.branch
                        ),
                        dim,
                    ))];
                }
            }
            "Adding a repo to the project. Type a path to a git repo.".to_string()
        }
        InputMode::EditingCommitMessage => {
            "Enter a commit message for the push. Leave blank for a default git message. \
             Staged and unstaged changes will be committed and pushed."
                .to_string()
        }
        InputMode::ViewingDiff => {
            "Viewing uncommitted changes in this worktree. Scroll with ↑↓. Press Esc to return."
                .to_string()
        }
        InputMode::Terminal => {
            "Terminal session active. Press Esc to detach — the session stays alive \
             and can be re-attached."
                .to_string()
        }
        InputMode::Options => "Settings. Changes are saved immediately.".to_string(),
        InputMode::Help => "Keybinding reference. Press any key to close.".to_string(),
        InputMode::ConfirmDelete => {
            let warn = Style::default().fg(Color::Yellow);
            let text = match app.pending_delete {
                Some(Selection::Project(p_idx)) => format!(
                    "Remove project \"{}\" and all its worktrees? This cannot be undone.",
                    app.config.projects[p_idx].name
                ),
                Some(Selection::Worktree(p_idx, w_idx)) => format!(
                    "Remove worktree \"{}\" from project \"{}\"? This cannot be undone.",
                    app.config.projects[p_idx].worktrees[w_idx].repo_name,
                    app.config.projects[p_idx].name
                ),
                None => "Confirm deletion?".to_string(),
            };
            return vec![Line::from(Span::styled(text, warn))];
        }
    };

    vec![Line::from(Span::styled(text, dim))]
}

/// Right column of the help bar: one line per shortcut/key for the current context.
fn context_shortcut_lines(app: &App) -> Vec<Line<'static>> {
    match app.input_mode {
        InputMode::Normal => match app.get_selected_selection() {
            Some(Selection::Project(_)) => {
                let mut lines = vec![named_key_line("Enter", "expand")];
                lines.extend(PROJECT_SHORTCUTS.iter().map(|s| Line::from(render_shortcut(s))));
                lines
            }
            Some(Selection::Worktree(_, _)) => {
                WORKTREE_SHORTCUTS.iter().map(|s| Line::from(render_shortcut(s))).collect()
            }
            _ => GLOBAL_SHORTCUTS.iter().map(|s| Line::from(render_shortcut(s))).collect(),
        },
        InputMode::AddingProjectName => vec![
            named_key_line("Enter", "create"),
            named_key_line("Esc", "cancel"),
        ],
        InputMode::AddingRepo => vec![
            named_key_line("Enter", "add repo"),
            named_key_line("Enter", "(empty) done"),
            named_key_line("Tab", "complete path"),
            named_key_line("↑↓", "browse"),
            named_key_line("Esc", "cancel"),
        ],
        InputMode::EditingCommitMessage => vec![
            named_key_line("Enter", "confirm"),
            named_key_line("Esc", "cancel"),
        ],
        InputMode::ViewingDiff => vec![
            named_key_line("↑↓", "scroll"),
            named_key_line("Esc", "exit"),
        ],
        InputMode::Terminal => vec![
            named_key_line("Esc", "detach"),
            named_key_line("Ctrl-B D", "tmux detach"),
        ],
        InputMode::Options => vec![
            named_key_line("↑↓", "navigate"),
            named_key_line("Space", "toggle"),
            named_key_line("Esc", "close"),
        ],
        InputMode::Help => vec![named_key_line("any key", "close")],
        InputMode::ConfirmDelete => vec![
            named_key_line("y / Enter", "confirm delete"),
            named_key_line("n / Esc", "cancel"),
        ],
    }
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
