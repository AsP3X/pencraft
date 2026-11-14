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
| `pencraft-<version>-setup.exe` | Windows, the setup wizard; the update card downloads this |
| `pencraft-<version>-x64.msi` | Windows, installing for all users, deployment |
| `pencraft-<version>-x64.exe` | Windows, the plain program |
| `pencraft-<version>-macos-universal.dmg` | macOS 11 or newer, Apple Silicon and Intel; the update card downloads this |
| `SHA256SUMS.txt` | Checking any of the files (`sha256sum -c SHA256SUMS.txt`) |

Versions follow [semantic versioning](https://semver.org/). A release must
not be marked as a pre-release or left as a draft if installed copies should
see it: the app asks GitHub for the latest full release. A tag like
`v1.2.3-rc.1` does not start the workflow.

If a build fails, nothing is published. Fix the problem on `master`, then
move the tag: `git tag -f v1.2.3`, `git push -f origin v1.2.3`.

## How the update check works

The code is in `app/src/update.rs`.

1. **Look.** When the app starts, and once every 24 hours while it stays
   open, a thread asks
   `https://api.github.com/repos/AsP3X/pencraft/releases/latest`. The window
   never waits for it. The request gives up after 20 seconds.
2. **Compare.** The tag is read as three numbers. If it is a higher version
   than the running one, and not the version the user chose to skip, the
   card appears.
3. **Offer.** The card names the version and shows the release notes.
   **Download** opens the installer for the system in the browser (the file
   ending in `-setup.exe` on Windows, in `.dmg` on a Mac), or the release's
   page when there is none. **Skip this version** remembers the version in
   the settings. **Later** hides the card until the next start.

Things that follow from this design:

- **The file names matter.** Do not rename the setup wizard or the disk
  image.
- **Pencraft does not install anything itself.** The browser downloads the
  installer and the user runs it, so the unsigned-download warnings below
  come back with every update.
- **Nothing is ever shown when the look fails.** No network, a rate limit or
  an answer that makes no sense is written to the log at `info` level and
  tried again a day later. Only **Check for updates…** in the action search
  says what went wrong, because there the user asked.
- **Some runs never look**: `--screenshot` (and with it `--export`),
  `--private`, `--background`, `--no-update-check`, `pencraft mcp`, and any
  run with Settings → General → "Look for new versions" switched off.
- **The browser is only sent to this repository.** An address in GitHub's
  answer that is not under `https://github.com/AsP3X/pencraft/` is ignored.

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

Nothing is signed with a certificate, and the Mac app is not notarised.

- **Windows.** The executable and both installers carry no signature.
  SmartScreen shows "Windows protected your PC" for a downloaded file until
  the user chooses **More info → Run anyway**, and the MSI and an install
  for everyone show "Unknown publisher". Signing needs a code-signing
  certificate; with one, a signing step belongs in `installer\build.ps1`
  after the build and after each installer.
- **macOS.** The app is signed ad hoc, which Apple Silicon requires to run
  anything, but that is no Developer ID. A downloaded copy is stopped at its
  first start ("Apple could not verify Pencraft is free of malware") until
  the user allows it under **System Settings → Privacy & Security → Open
  Anyway**. Fixing that needs an Apple Developer account: sign with the
  Developer ID certificate in `installer/build-macos.sh` instead of `-`, and
  notarise and staple the disk image.

## What a release costs

In a private repository GitHub counts Actions minutes, Windows twice and
macOS ten times. A release builds the app three times in release mode (once
on Windows, twice on the Mac), and CI builds it on both systems for every
push and pull request to `master`. In a public repository Actions are free.
