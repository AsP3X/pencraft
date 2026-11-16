# Updates that install themselves

Date: 2026-11-16. Status: approved, being built.

## Goal

A newer version is installed from inside Pencraft: one click, no browser, no setup wizard, and the app comes
back up as the new version. Windows and macOS.

## Behaviour

- The card's first button reads **Update and restart** when this copy can replace itself, and **Download**
  (today's browser download) when it can't.
- After the click the card shows the download's progress and a **Cancel** button.
- The downloaded file is checked against the release's `SHA256SUMS.txt`. A file that doesn't match is deleted
  and the user is told; nothing is installed.
- Then the app quits the usual way, asking about each unsaved design. Cancelling there keeps the app open; the
  card then offers **Restart to update** without downloading again.
- Windows: the setup wizard runs with `/SILENT` (a small progress window), into the folder and with the options
  of the install it finds, and starts Pencraft again. An install for everyone asks for administrator rights once.
- macOS: the new `Pencraft.app` is copied out of the disk image beside the running one; once the app has quit,
  a small script puts it in the old one's place and opens it.
- A copy that can't replace itself: not installed by the setup wizard on Windows (the MSI, the plain program,
  a development build), or on a Mac not inside `Pencraft.app` or in a folder that can't be written to.
- While the repository is private, the download uses the same GitHub sign-in the look does.

Not included: downloading before the user asks, updating the MSI install, Linux.

## How

- `update::Release` also names the installer file and the checksum file by their GitHub asset id. The download
  address is built from the id (`/repos/AsP3X/pencraft/releases/assets/<id>`), so nothing in GitHub's answer
  can send the download elsewhere.
- `update::install` (new, `src/update/install.rs`): where this copy lives and whether it can be replaced; the
  download with progress and cancel; the checksum, computed by the system's own tool (`certutil`, `shasum`), so
  no new dependency; getting the Mac bundle ready; and starting the install as the app quits.
- `update::Updates` gains the card's stages: offered, downloading, ready.
- `App` keeps the prepared update, asks the window to close, and starts the install in `on_exit`. Cancelling the
  unsaved question forgets it.
- `installer/pencraft.iss`: a `[Run]` entry that starts Pencraft after a silent install asked for with
  `/RELAUNCH`, as the user who started the setup.

## Tests

- The release's files are found by name and id; the download address is always this repository's.
- A checksum is found in `SHA256SUMS.txt` and in what `certutil` and `shasum` print.
- A download from a local server arrives whole, reports progress, stops when cancelled, and is refused when its
  checksum differs.
- The Mac bundle is found from the executable's path; the swap script is right.
- The card's stages: a click starts the download, a failure goes back to the offer, cancelling the quit leaves
  "Restart to update".
- By hand on Windows: a real update from an installed older build. The Mac path is not run from Windows.
