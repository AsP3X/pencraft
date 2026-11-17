# Releasing

A release is made by pushing a version tag. GitHub Actions builds everything
and publishes it; installed copies of Pencraft then find it on their own.

## Steps

1. Set the new version in `app/Cargo.toml` (`[workspace.package]`, `version`)
   and run `cargo build` in `app/` so `Cargo.lock` follows. This is the one
   place the version is written; the app, the installers and the workflow
   all read it from there.
2. Add a section for the version to `CHANGELOG.md`, headed
   `## [1.2.3] - YYYY-MM-DD`, and a link for it at the end of the file. The
   section becomes the release notes, and it is what people read in the app
   before they download the update.
   If nobody should stay on an older version (a fix for lost work, a file
   format older copies would damage), add a line that says only
   `<!-- required -->` to the section. See "Required updates" below.
3. Commit, and push to `master`. Wait for CI to pass.
4. Tag that commit and push the tag:

   ```
   git tag v1.2.3
   git push origin v1.2.3
   ```

The tag must be `v` plus the version in `app/Cargo.toml`. The release
workflow stops if they differ, or if the changelog has no section for the
version.

## What the workflow does

`.github/workflows/release.yml` runs on a tag that looks like `v1.2.3`:

1. **Prepare** (Linux): checks that the tag matches `app/Cargo.toml` and
   cuts the version's section out of `CHANGELOG.md`.
2. **Windows**: runs the tests, then `installer\build.ps1` builds the release
   executable, the setup wizard and the MSI.
3. **macOS**: runs the tests, then `installer/build-macos.sh` builds the app
   for Apple Silicon and Intel, joins them and packs the disk image.
4. **Publish** (Linux): writes `SHA256SUMS.txt` over all files and creates
   the GitHub release for the tag with them and the notes.

| File | For |
|---|---|
| `pencraft-<version>-setup.exe` | Windows, the setup wizard; the app installs updates with this |
| `pencraft-<version>-x64.msi` | Windows, installing for all users, deployment |
| `pencraft-<version>-x64.exe` | Windows, the plain program |
| `pencraft-<version>-macos-universal.dmg` | macOS 11 or newer, Apple Silicon and Intel; the app installs updates with this |
| `SHA256SUMS.txt` | Checking any of the files (`sha256sum -c SHA256SUMS.txt`); the app checks its download with this |

Versions follow [semantic versioning](https://semver.org/). A release must
not be marked as a pre-release or left as a draft if installed copies should
see it: the app asks GitHub for the latest full release. A tag like
`v1.2.3-rc.1` does not start the workflow.

If a build fails, nothing is published. Fix the problem on `master`, then
move the tag: `git tag -f v1.2.3`, `git push -f origin v1.2.3`.

## Required updates

Most updates are optional: the card offers them, and the user takes them,
skips that version, says "Later" (the next look asks again) or folds the
card away into a small pill.

A release whose notes have the line `<!-- required -->` is required. Every
copy older than it then asks in the middle of its window and waits for the
answer: update, or quit. The version can't be skipped, put off or folded
away. This also holds when a newer, ordinary release has come out since:
the app looks through the last 30 releases for a required one it would
otherwise skip over. The line is a comment, so GitHub doesn't show it, and
the app leaves it out of the notes it shows.

Two things keep a required update from locking anyone out of their work:
quitting from the card asks about unsaved designs like any quit, and when
the download fails the card offers "Not now", which lets the user go on
until the next look.

Use it rarely. It can't be taken back for a copy that has already seen it,
other than by publishing a newer release.

## How updating works

The look and the card are in `app/src/update.rs`, the download and the
install in `app/src/update/install.rs`.

1. **Look.** When the app starts, and then as often as Settings → General
   → Updates → "Check every" says (every 5 minutes unless changed) while it
   stays open, also when the window is left alone, a thread asks
   `https://api.github.com/repos/AsP3X/pencraft/releases/latest`. The window
   never waits for it. The request gives up after 20 seconds.
2. **Compare.** The tag is read as three numbers. If it is a higher version
   than the running one, and not the version the user chose to skip, the
   card appears.
3. **Offer.** The card names the version and shows the release notes.
   **Skip this version** remembers the version in the settings. **Later**
   hides the card until the next start. The first button is **Update and
   restart** when this copy can replace itself, and **Download** otherwise.
4. **Download.** After **Update and restart** a thread fetches
   `SHA256SUMS.txt` and the installer for the system (the file ending in
   `-setup.exe` on Windows, in `.dmg` on a Mac) into `pencraft-update` in
   the temporary folder. Both are asked for by their asset number at
   `api.github.com/repos/AsP3X/pencraft/releases/assets/<id>`. The card
   shows the progress and a **Cancel** button.
5. **Check.** The file's SHA-256, worked out by `certutil` on Windows and
   `shasum` on a Mac, must be the one `SHA256SUMS.txt` gives for the file's
   name. A file that differs is deleted and the user is told.
6. **Quit.** The app closes the usual way and asks about each unsaved
   design. Cancelling there keeps it open; the card then reads **Restart to
   update** and nothing is downloaded again.
7. **Install.** As the app exits it starts the install, which outlives it:
   - **Windows:** `pencraft-setup.exe /SILENT /SUPPRESSMSGBOXES /NORESTART
     /RELAUNCH /WAITPID=<process>`. The setup wizard first waits for that
     Pencraft to be gone (`InitializeSetup` in `installer/pencraft.iss`): a
     silent setup that finds the app running cannot close it and gives up.
     Then it shows only a progress window, installs into the folder and with
     the options of the install it finds, and because of `/RELAUNCH` starts
     Pencraft again (the `[Run]` entry with `Check: RelaunchAsked`). An install for
     everyone asks for administrator rights; Pencraft is still started as
     the user.
   - **macOS:** before quitting, the app in the disk image was copied to
     `.Pencraft-update.app` beside the running bundle (`hdiutil`, `ditto`)
     and its signature verified (`codesign --verify`). A shell script waits
     for the app to end, moves the old bundle aside, the new one into its
     place, and opens it. If the move fails the old bundle is put back.

**Download** opens the installer in the browser, or the release's page when
the release has none. A copy gets that button when it can't replace itself:

- on Windows, when there is no `unins000.exe` beside the program, which is
  how the app knows the setup wizard installed it (so not the MSI, the plain
  program or a development build);
- on a Mac, when the program is not inside an `.app` bundle or the bundle's
  folder can't be written to;
- on any system, when the release lacks the installer or `SHA256SUMS.txt`.

Things that follow from this design:

- **The file names matter.** Do not rename the setup wizard, the disk image
  or `SHA256SUMS.txt`, and keep the app in the disk image named
  `Pencraft.app`.
- **The setup wizard's `/RELAUNCH`, `/WAITPID=` and its `AppId` must stay.** Installed
  copies run whatever setup the newest release ships, with those arguments.
- **The first update is still by hand.** Versions up to 0.2.1 only know the
  browser download, and their setup wizard doesn't know `/RELAUNCH`.
- **An update the app installs itself shows no unsigned-download warning**:
  a file the app downloaded carries no mark of the web (Windows) and no
  quarantine flag (macOS). The warnings below still come with the first
  install and with the browser download.
- **The Mac install has only been tested in pieces**: the swap script and
  everything around it have tests, but `hdiutil`, `ditto` and a real swap
  were not run when this was written on Windows. Try one update on a Mac
  after the first release that has it.
- **Nothing is ever shown when the look fails.** No network, a rate limit or
  an answer that makes no sense is written to the log at `info` level and
  tried again a day later. Only **Check for updates…** in the action search
  says what went wrong, because there the user asked.
- **Some runs never look**: `--screenshot` (and with it `--export`),
  `--private`, `--background`, `--no-update-check`, `pencraft mcp`, and any
  run with Settings → General → "Look for new versions" switched off.
- **The browser is only sent to this repository**, and the download only
  asks this repository. An address in GitHub's answer that is not under
  `https://github.com/AsP3X/pencraft/` is ignored.

### While the repository is private

GitHub answers "not found" to anyone who asks about a private repository
without a sign-in. Pencraft then asks once more with a sign-in from this
computer: `GH_TOKEN`, `GITHUB_TOKEN`, or the GitHub CLI (`gh auth token`).
So today the check works for people who are signed in to GitHub with access
to this repository, and for nobody else: everyone else simply never sees a
card, and could not download the file either.

For the public to get updates, the releases must be readable without a
sign-in: make the repository public, or publish the release files to a
public repository and point `REPOSITORY` in `app/src/update.rs` at it.

## Signing

The Windows files are not signed. The Mac app is signed and notarised once
the repository has the secrets for it.

- **Windows.** The executable and both installers carry no signature.
  SmartScreen shows "Windows protected your PC" for a downloaded file until
  the user chooses **More info → Run anyway**, and the MSI and an install
  for everyone show "Unknown publisher". Signing needs a code-signing
  certificate; with one, a signing step belongs in `installer\build.ps1`
  after the build and after each installer.
- **macOS.** With the secrets below, the release signs the app and the disk
  image with a Developer ID and the hardened runtime, has Apple notarise the
  disk image and staples the ticket to it, and a downloaded copy starts
  without a warning. Without them the app is signed ad hoc, which Apple
  Silicon requires to run anything, and a downloaded copy is stopped at its
  first start ("Apple could not verify Pencraft is free of malware") until
  the user allows it under **System Settings → Privacy & Security → Open
  Anyway**. The release build says which of the two it did.

### Signing the Mac app

This needs a paid Apple Developer Program membership; the free account's
"Apple Development" certificate is not accepted by Gatekeeper. Once:

1. **The certificate.** In Xcode → Settings → Accounts → Manage Certificates,
   add a **Developer ID Application** certificate (only the account holder
   can). In Keychain Access, export it with its private key as a `.p12` file
   with a password.
2. **The notary key.** In App Store Connect → Users and Access →
   Integrations → App Store Connect API, make a team key with the
   **Developer** role and download the `.p8` file. Note the key ID and the
   issuer ID shown on that page.
3. **The secrets.** In the repository's settings under Secrets and variables
   → Actions, or with `gh secret set`:

   | Secret | What it holds |
   |---|---|
   | `MACOS_CERTIFICATE` | the `.p12` file, base64 (`base64 -i certificate.p12`) |
   | `MACOS_CERTIFICATE_PASSWORD` | the password of the `.p12` file |
   | `MACOS_NOTARY_KEY` | the text of the `.p8` file |
   | `MACOS_NOTARY_KEY_ID` | the key ID |
   | `MACOS_NOTARY_ISSUER` | the issuer ID |

The next release is then signed and notarised; a release fails rather than
publish a disk image Apple did not accept. To do the same by hand, set
`PENCRAFT_SIGN_IDENTITY` and the notary variables named at the top of
`installer/build-macos.sh`.

The first signed version asks once more for the keychain items that hold API
keys, because the app's signature changed. After that the signature stays
the same from version to version and macOS stops asking after each update.

## What a release costs

In a private repository GitHub counts Actions minutes, Windows twice and
macOS ten times. A release builds the app three times in release mode (once
on Windows, twice on the Mac), and CI builds it on both systems for every
push and pull request to `master`. In a public repository Actions are free.
