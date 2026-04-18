pub const MAX_SHORTCUTS: usize = 5;

const fn ascii_lower(b: u8) -> u8 {
    if b >= b'A' && b <= b'Z' { b + (b'a' - b'A') } else { b }
}

#[derive(Clone, Copy)]
pub struct Shortcut {
    pub key: char,
    pub label: &'static str,
}

impl Shortcut {
    /// Key defaults to the first character of `label`, lowercased.
    pub const fn new(label: &'static str) -> Self {
        let key = ascii_lower(label.as_bytes()[0]) as char;
        Self { key, label }
    }

    /// Explicit key override for cases where the mnemonic doesn't lead the word
    /// (e.g. `x` for "remove").
    pub const fn with_key(key: char, label: &'static str) -> Self {
        Self { key, label }
    }
}

// ── Shortcut groups ───────────────────────────────────────────────────────────
//
// Defined as const slices so the length can be checked at compile time.
// The `const _: ()` assertions below are compile errors if any group exceeds
// MAX_SHORTCUTS — adding a 6th entry will fail the build.

pub const PROJECT_SHORTCUTS: &[Shortcut] = &[
    Shortcut::new("add repo"),      // a
    Shortcut::new("terminal"),      // t
    Shortcut::new("push all"),      // p
    Shortcut::with_key('x', "remove"),
];
const _: () = assert!(
    PROJECT_SHORTCUTS.len() <= MAX_SHORTCUTS,
    "project shortcuts exceed the maximum of 5"
);

pub const WORKTREE_SHORTCUTS: &[Shortcut] = &[
    Shortcut::new("terminal"),      // t
    Shortcut::new("push"),          // p
    Shortcut::new("diff"),          // d
    Shortcut::with_key('x', "remove worktree"),
];
const _: () = assert!(
    WORKTREE_SHORTCUTS.len() <= MAX_SHORTCUTS,
    "worktree shortcuts exceed the maximum of 5"
);

pub const GLOBAL_SHORTCUTS: &[Shortcut] = &[
    Shortcut::new("new project"),   // n
    Shortcut::new("options"),       // o
    Shortcut::new("help"),          // h
    Shortcut::new("quit"),          // q
];
const _: () = assert!(
    GLOBAL_SHORTCUTS.len() <= MAX_SHORTCUTS,
    "global shortcuts exceed the maximum of 5"
);

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn assert_no_collisions(context: &str, shortcuts: &[&Shortcut]) {
        let mut seen: HashSet<char> = HashSet::new();
        for s in shortcuts {
            assert!(
                seen.insert(s.key),
                "Key collision in {context} context: '{}' is used more than once",
                s.key
            );
        }
    }

    #[test]
    fn test_no_key_collisions() {
        let project_ctx: Vec<&Shortcut> =
            PROJECT_SHORTCUTS.iter().chain(GLOBAL_SHORTCUTS.iter()).collect();
        assert_no_collisions("project", &project_ctx);

        let worktree_ctx: Vec<&Shortcut> =
            WORKTREE_SHORTCUTS.iter().chain(GLOBAL_SHORTCUTS.iter()).collect();
        assert_no_collisions("worktree", &worktree_ctx);

        assert_no_collisions("global", &GLOBAL_SHORTCUTS.iter().collect::<Vec<_>>());
    }

    #[test]
    fn test_counts_within_limit() {
        assert!(PROJECT_SHORTCUTS.len() <= MAX_SHORTCUTS);
        assert!(WORKTREE_SHORTCUTS.len() <= MAX_SHORTCUTS);
        assert!(GLOBAL_SHORTCUTS.len() <= MAX_SHORTCUTS);
    }
}
