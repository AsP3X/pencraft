# Building

The app is the Cargo workspace in `app/`. Every command on this page runs
there unless it says otherwise.

## What you need

- [rustup](https://rustup.rs/). The exact Rust version is named in
  `app/rust-toolchain.toml`; rustup installs it by itself the first time you
  run `cargo` in `app/`.
- On Windows, the MSVC build tools the Rust installer asks for. On macOS,
  Xcode's command line tools (`xcode-select --install`).
- Git. Some dependencies come straight from GitHub: the storytold forks of
  egui and winit and the craft-libs crates, each pinned to one commit in
  `app/Cargo.toml`.

One crate (`ring`, for TLS) compiles C code with the C compiler those tools
bring along. Nothing else needs installing.

## Build and run

```
cd app
cargo run
cargo run -- design.pen
cargo build --release
```

The release executable is `app/target/release/pencraft` (`pencraft.exe` on
Windows). It is a single file and can be copied anywhere.

To try the app without touching your real settings, start it with
`--private`: settings are neither read nor saved, and no tabs are restored.
`--no-update-check` keeps a run from asking GitHub for a newer version.
[app/README.md](../app/README.md) lists the other options, among them
`--screenshot`, which draws the window into a picture without taking focus.

## Checks

The quality gate is one script. CI runs the same steps on Windows and macOS,
and all of them must pass before a pull request is merged.

```
scripts/check.sh        # Windows: scripts\check.ps1
```

It runs:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

and then searches the sources to make sure `dead_code` is not silenced
anywhere. The lints are in `[workspace.lints]` of `app/Cargo.toml`: clippy's
pedantic set, with `allow` attributes and `unsafe` forbidden.

About the tests:

- They are unit tests inside the source files (`#[cfg(test)] mod tests`).
- `crates/pen-doc` checks that files round-trip: read, write, read gives the
  same bytes.
- No test opens a window or a native dialog, or talks to a server on the
  internet. The update check is tested against a server the test itself
  starts on `127.0.0.1`.

## Installers

### Windows

`installer\build.ps1` (run from the repository's root) builds the release
executable and both installers into `app\target\installer`:

| File | Made with |
|---|---|
| `pencraft-<version>-setup.exe` | [Inno Setup](https://jrsoftware.org/isinfo.php) 6.3 or newer, from `installer\pencraft.iss` |
| `pencraft-<version>-x64.msi` | [WiX Toolset](https://wixtoolset.org/) 4 or 5, from `installer\pencraft.wxs` |
| `pencraft-<version>-x64.exe` | A copy of the release executable |

```
dotnet tool install --global wix --version 5.0.2
winget install JRSoftware.InnoSetup
powershell -ExecutionPolicy Bypass -File installer\build.ps1
```

The version comes from `app\Cargo.toml`. An installer whose tool is missing
is skipped, and the script says how to get the tool.

WiX 5.0.2 is named on purpose: from version 6 on, the WiX project asks
organisations for a maintenance fee.

Two identifiers in the installer files must never change, because they are
how Windows recognises a newer version of the same program: the `UpgradeCode`
in `pencraft.wxs` and the `AppId` in `pencraft.iss`.

The setup wizard can install for one user without administrator rights. It
offers to open `.pen` files with Quill and to put the folder on `PATH`,
which is what lets an AI client run `pencraft mcp`.

### macOS

`installer/build-macos.sh` builds the app for Apple Silicon and for Intel,
joins the two into one universal `Pencraft.app`, signs it ad hoc and packs it
into `app/target/installer/pencraft-<version>-macos-universal.dmg`. It needs
a Mac; an Apple Silicon Mac builds the Intel half too.

```
./installer/build-macos.sh
```

`installer/Info.plist` is what macOS reads about the app. Its bundle
identifier, `dev.pencraft.desktop`, must never change.

The Mac app does not yet open a `.pen` file that is double-clicked in the
Finder.

### The icon

`design/icon/build.py` describes the icon. Run it after changing the drawing
(`python3 design/icon/build.py`, with Pillow, on any computer): it writes
`design/icon/quill.svg` and every icon file. `design/icon/build.sh` draws the
same files from the SVG with a Mac's own renderer. Commit what they write:

| File | Used for |
| --- | --- |
| `app/assets/icon.png` | the window on Windows and Linux (`app/src/main.rs`) |
| `app/assets/icon-macos.png` | the Dock while Quill runs outside its `.app`, as with `cargo run` |
| `installer/Pencraft.icns` | `Pencraft.app` (`installer/Info.plist`, `installer/build-macos.sh`) |
| `installer/pencraft.ico` | `pencraft.exe`, and with it the shortcuts and `.pen` files (`app/build.rs`); the setup wizard and the MSI |

## Other systems

Linux is not supported yet. The code has paths for it (fonts, settings, the
keychain through the Secret Service), and in a Linux container the workspace
compiles, clippy is clean and the tests pass. But nobody has opened the
window on Linux, so
there is no Linux download and CI does not cover it.

## Project layout

| Path | What is in it |
|---|---|
| `app/` | The Cargo workspace: the desktop app (`src/`) and the window-free document library (`crates/pen-doc`). [app/README.md](../app/README.md) describes every module. |
| `app/src/update.rs` | Looking for a newer release and the card that offers it |
| `app/scripts/` | The quality gate, as a shell and a PowerShell script |
| `installer/` | The installer definitions and their build scripts |
| `.github/workflows/` | CI and the release build |
| `docs/` | This page and [Releasing](RELEASING.md) |
| `design/` | Quill's own design, as a `.pen` file |
| `checks/` | Pictures from design reviews |
