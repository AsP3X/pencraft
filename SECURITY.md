# Security

## Supported versions

Security fixes go into the latest release. Pencraft tells you when a newer
version is out, so the fix for an older version is to update.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Report it privately through GitHub: on the repository's **Security** tab,
choose **Report a vulnerability**
([direct link](https://github.com/AsP3X/pencraft/security/advisories/new)).
Describe what you found, how to reproduce it, and what an attacker could do
with it.

You can expect a first answer within a week. Once a fix is released, the
report is published as a security advisory, with credit to you unless you
prefer otherwise.

## What Pencraft does with your data

Knowing this helps to judge a report.

- **It reads and writes the files you open.** `.pen` files, the pictures
  their fills name, shader files, and what you export. Its settings are a
  `config.toml` in the Craft suite's settings folder.
- **It keeps API keys in the system's keychain.** Windows Credential
  Manager or the macOS Keychain, never a file. A key is sent only to the
  provider it belongs to.
- **It talks to the AI provider you chose.** The chat sends your messages,
  the layers they are about and pictures of the design to Claude Code on
  your computer or to the provider's API. Nothing is sent until you write to
  the assistant.
- **It runs a local door for AI clients.** While "Let AI models work on
  documents" is on, Pencraft listens on `127.0.0.1` on a random port, and
  `pencraft mcp` connects clients to it with a token that is made anew at
  every start. It is not reachable from other computers.
- **It runs programs you installed.** The `claude` program for the chat, and
  your browser when you choose to download an update there.
- **It asks GitHub for the latest release.** At start and once a day, from
  `api.github.com`, sending only the app's name and version. If GitHub says
  the repository cannot be seen without a sign-in, it asks again with the
  one from `GH_TOKEN`, `GITHUB_TOKEN` or the GitHub CLI; that token goes to
  `api.github.com` only and is never saved or logged. Settings → General →
  "Look for new versions" switches the check off.
- **It installs an update when you ask it to.** Nothing is downloaded before
  you choose **Update and restart**. Then the installer of that release is
  fetched from this repository's releases by its file number (no address in
  GitHub's answer is followed), saved in the temporary folder, and compared
  with the SHA-256 checksum in the release's `SHA256SUMS.txt`; a file that
  differs is deleted and not run. On Windows the setup wizard is then run;
  on a Mac the app is copied out of the disk image and put in the old one's
  place. The checksum comes from the same release as the file, so it guards
  against a damaged or swapped download, not against a release that was
  itself tampered with. A copy that can't replace itself opens a page of
  this repository's releases in your browser instead, and no other address.
- **The releases are not code-signed**, and the Mac app is not notarised.
  `SHA256SUMS.txt` in each release lets you check a download.

Problems in any of these areas are in scope. So is anything that makes
Pencraft run a command, write a file or send data that the user did not ask
for, for example through a crafted `.pen` file, shader, picture, release
answer or MCP message.
