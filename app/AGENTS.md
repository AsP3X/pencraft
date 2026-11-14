# Agent rules for Pencraft

These follow the user's Rust projects (orbit, terravant, indeep: `docs/agent-rules/`) and the storytold Craft apps.

- **Quality gate**: `scripts/check.sh` must pass — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  (pedantic), tests, and the dead-code grep. Lints live in `[workspace.lints]` of `Cargo.toml`.
- **No `#[allow]`**; a narrow `#[expect(lint, reason = "…")]` on one item only. Never silence `dead_code`: use it or delete it.
- **No panics in shipped code**: no `unwrap`/`expect`/`panic!`; return `Result`. Bad files never crash the app.
- **Our files round-trip**: read → write → read gives identical bytes; unknown keys survive (`pen-doc` tests).
- **Library stays window-free**: `crates/pen-doc` has no egui; text measurement is passed in.
- **Single source of truth**: UI colours and sizes only in `src/theme.rs`; mechanics numbers in `MECHANICS.md` and `mechanics.json`.
- **Dependencies**: ask before adding; pin exactly; craft-libs crates from one commit; the egui family and winit
  patched to the storytold forks (`[patch.crates-io]`).
- **No deprecated APIs** (the egui fork deprecates `Id::new` and `Ui::id`: use `Id::unique`, `Ui::scope_id`).
- **Comments**: `// Human:` and `// Agent:` pairs on non-trivial code; `///` on every public item.
- **Native dialogs** only through craft-dialogs (never blocking rfd); quitting through craft-quit (`on_close_request`
  last in `logic`).
- Nothing is committed, pushed or published without the user asking.
