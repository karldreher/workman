# Contributing to workman

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Node.js](https://nodejs.org/) 22+
- [Tauri CLI](https://tauri.app/start/prerequisites/) — installed automatically via `npm run tauri`
- macOS/Linux: Xcode Command Line Tools or standard build tools
- Linux: webkit2gtk and related system packages

```bash
# Linux system dependencies
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf
```

## Run in development mode

```bash
git clone https://github.com/karldreher/workman
cd workman
npm install
npm run tauri dev
```

Hot-reload is active: frontend changes reflect immediately; Rust changes trigger a backend rebuild.

## Run tests

```bash
# Rust unit tests
cargo test --manifest-path src-tauri/Cargo.toml

# TypeScript typecheck
npx tsc --noEmit

# Frontend build
npm run build
```

## Build a release bundle

```bash
npm run tauri build
# Output: src-tauri/target/release/bundle/
```

## Replace placeholder icons

The repo ships with minimal placeholder icons. To use your own:

```bash
# Generate all required sizes from a 512×512+ RGBA PNG
npm run tauri icon path/to/your-icon.png
```

## Code style

### Rust

- All public items — functions, structs, fields, enums, modules — must have a `///` doc comment.
- Inline comments (`//`) only when the *why* is non-obvious: a hidden constraint, a subtle invariant, a platform workaround. Don't narrate what the code does.
- Reference: [rustdoc guide](https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html)

### TypeScript / React

- All exported interfaces, types, functions, hooks, and React components must have a TSDoc `/** */` comment.
- Props interfaces: document any field whose purpose isn't immediately obvious from its name and type.
- Inline comments follow the same rule as Rust — only when the *why* is non-obvious.
- Reference: [TSDoc](https://tsdoc.org/)

### General

- Conventional commit messages: lowercase, imperative, prefixed (`feat:`, `fix:`, `refactor:`, etc.), no trailing period, under 72 characters.
- No over-engineering. Don't add abstractions, error handling, or feature flags for scenarios that can't happen or aren't planned.
