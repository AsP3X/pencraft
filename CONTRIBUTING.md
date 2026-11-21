# Contributing to Quill

Thank you for wanting to help. This page covers reporting problems, proposing
changes, and what a pull request needs.

By taking part you agree to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Reporting a bug

Open a [bug report](https://github.com/AsP3X/pencraft/issues/new/choose).
It helps a lot if you include:

- what you did, what you expected, and what happened instead
- the Quill version (Settings → General → Updates names it) and your system
- the `.pen` file, or a small one that shows the problem, if you can share it
- a picture of the window; `pencraft your.pen --screenshot shot.png` takes one

Do not report security problems in a public issue. See [SECURITY.md](SECURITY.md).

## Suggesting a change

Open a [feature request](https://github.com/AsP3X/pencraft/issues/new/choose)
and describe the problem you want solved before the solution. For anything
larger than a small fix, please open an issue before writing code, so nobody
builds something that does not fit.

## Working on the code

The app is in `app/`. [docs/BUILDING.md](docs/BUILDING.md) has the full
details. In short:

```
git clone https://github.com/AsP3X/pencraft
cd pencraft/app
cargo run -- --private
```

`--private` keeps your experiments away from your real settings: nothing is
read from them and nothing is saved.

Before you open a pull request, the quality gate must pass. CI runs the same
steps on Windows and macOS:

```
cd app
scripts/check.sh        # Windows: scripts\check.ps1
```

That is `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
-- -D warnings`, `cargo test --workspace`, and a search that makes sure
`dead_code` is never silenced.

### The rules of this code

[app/AGENTS.md](app/AGENTS.md) is the full list. The ones that reject most
first attempts:

- **No `#[allow]`.** A narrow `#[expect(lint, reason = "…")]` on one item
  only, and never for `dead_code`: use the code or delete it.
- **No panics in shipped code.** No `unwrap`, `expect` or `panic!` outside
  tests; return a `Result`. A bad file never crashes the app.
- **Files round-trip.** Read, write, read gives identical bytes, and keys
  Quill does not know survive.
- **Colours and sizes of the interface live in `src/theme.rs`**, nowhere
  else. Numbers of the mechanics live in `MECHANICS.md` and `mechanics.json`.
- **Dependencies are asked for first** and pinned exactly.
- **Comments come in pairs** on non-trivial code: `// Human:` says why in
  plain words, `// Agent:` states what the code blocks on, spawns or sends.
  Every public item has a `///` line.
- **Native dialogs only through craft-dialogs**, quitting through craft-quit.

### What a good change looks like

- **One thing per pull request.** A fix and an unrelated clean-up are two
  pull requests.
- **Tests come with it**, next to the code, named as a sentence that says
  what must hold: `a_newer_version_is_offered_unless_it_was_skipped`.
- **Tests stay local.** No test may call a server on the internet, open a
  browser or a native dialog, or write outside the system's temporary folder.
- **Anything on screen comes with a picture.** `--screenshot` draws the
  window without taking focus; say in the pull request which picture shows
  the change.
- **Text is plain.** Messages say what happened and what to do, in ordinary
  words.

### Commits

Write the subject line as a short sentence that says what the change does,
for example `Offer a newer version when GitHub has one`. Explain the reason
in the body when it is not obvious.

### Changelog

If users will notice the change, add a line under `## [Unreleased]` in
[CHANGELOG.md](CHANGELOG.md).

## Pull requests

1. Fork the repository and branch from `master`.
2. Make the change, with tests.
3. Run the quality gate.
4. Open the pull request and fill in the template.

A maintainer reviews it. Small fixes are usually quick; larger changes can take
a few rounds.

## License of contributions

Quill is licensed under the Apache License 2.0 with the Commons Clause
([LICENSE](LICENSE)). Unless you
say otherwise, a contribution you submit is licensed under those same terms,
without any additional conditions, and you confirm that you have the right to
submit it.
