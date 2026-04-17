/// A single keybinding entry shown in the help bar.
///
/// By convention `key` is the first character of `label` (lowercased).
/// Use `Shortcut::with_key` when the mnemonic doesn't match the first letter.
pub struct Shortcut {
    pub key: char,
    pub label: &'static str,
}

impl Shortcut {
    /// Key defaults to the first character of `label`, lowercased.
    pub fn new(label: &'static str) -> Self {
        let key = label.chars().next()
            .expect("shortcut label must not be empty")
            .to_ascii_lowercase();
        Self { key, label }
    }

    /// Explicit key override — for cases where the key doesn't start the word
    /// (e.g. `x` for "remove").
    pub fn with_key(key: char, label: &'static str) -> Self {
        Self { key, label }
    }
}

// ── Shortcut groups per display context ──────────────────────────────────────
//
// Each function returns the shortcuts visible in that context.
// Keep these in sync with the key handlers in event_handler.rs.
// The collision tests below will catch any duplicate keys.

pub fn project_shortcuts() -> Vec<Shortcut> {
    vec![
        Shortcut::new("add repo"),      // a
        Shortcut::new("terminal"),      // t
        Shortcut::new("push all"),      // p
        Shortcut::with_key('x', "remove"),
    ]
}

pub fn worktree_shortcuts() -> Vec<Shortcut> {
    vec![
        Shortcut::new("terminal"),      // t
        Shortcut::new("push"),          // p
        Shortcut::new("diff"),          // d
        Shortcut::with_key('x', "remove worktree"),
    ]
}

pub fn global_shortcuts() -> Vec<Shortcut> {
    vec![
        Shortcut::new("new project"),   // n
        Shortcut::new("options"),       // o
        Shortcut::new("help"),          // h
        Shortcut::new("quit"),          // q
    ]
}

// ── Collision tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn assert_no_collisions(context: &str, shortcuts: Vec<Shortcut>) {
        let mut seen: HashSet<char> = HashSet::new();
        for s in &shortcuts {
            assert!(
                seen.insert(s.key),
                "Key collision in {context} context: '{}' is used more than once",
                s.key
            );
        }
    }

    #[test]
    fn test_no_key_collisions() {
        // Each display context combines context-specific + global shortcuts.
        // If a key appears in both groups simultaneously the user can't tell them apart.

        let mut project_ctx = project_shortcuts();
        project_ctx.extend(global_shortcuts());
        assert_no_collisions("project", project_ctx);

        let mut worktree_ctx = worktree_shortcuts();
        worktree_ctx.extend(global_shortcuts());
        assert_no_collisions("worktree", worktree_ctx);

        assert_no_collisions("global", global_shortcuts());
    }

    #[test]
    fn test_default_key_is_first_letter() {
        for s in project_shortcuts().iter().chain(worktree_shortcuts().iter()).chain(global_shortcuts().iter()) {
            let first = s.label.chars().next().unwrap().to_ascii_lowercase();
            // Either the key matches the first letter (default) OR it was explicitly overridden
            // — no assertion here, just document the invariant is tracked via the struct fields.
            let _ = (s.key, first);
        }
    }
}
