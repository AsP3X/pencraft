#!/usr/bin/env bash
# Signs the checksums of a draft release with the offline key and makes the
# release public. docs/RELEASING.md, "Signing the checksums".
#
#   installer/sign-release.sh 1.2.3 [path/to/minisign.key]
#
# The release workflow leaves a draft with the files and SHA256SUMS.txt. This
# downloads them all, checks every file against SHA256SUMS.txt, signs
# SHA256SUMS.txt with minisign (which asks for the key's password), uploads
# SHA256SUMS.txt.minisig and publishes the release. Installed copies of Quill
# only take an update whose checksums carry this signature, made for exactly
# this version.
#
# Without a path, minisign's own key is used (~/.minisign/minisign.key).
# Needs minisign and the GitHub CLI, signed in with write access. Run it in
# the repository, so gh knows which one is meant.
set -euo pipefail

version=${1:?"Say which version: installer/sign-release.sh 1.2.3 [path/to/minisign.key]"}
key=${2:-}
tag="v$version"

for tool in gh minisign; do
  if ! command -v "$tool" > /dev/null; then
    echo "$tool is not installed." >&2
    exit 1
  fi
done
if [ -n "$key" ] && [ ! -f "$key" ]; then
  echo "There is no key at $key." >&2
  exit 1
fi

if [ "$(gh release view "$tag" --json isDraft --jq .isDraft)" != "true" ]; then
  echo "$tag is not a draft: it is already public, so there is nothing to sign here." >&2
  exit 1
fi

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
gh release download "$tag" --dir "$work"
cd "$work"

if [ ! -f SHA256SUMS.txt ]; then
  echo "The draft has no SHA256SUMS.txt." >&2
  exit 1
fi
# The two files the app installs must be among the checksums, under this version's names.
for ending in "-$version-setup.exe" "-$version-macos-universal.dmg"; do
  if ! grep -q -- "$ending\$" SHA256SUMS.txt; then
    echo "SHA256SUMS.txt lists no file ending in $ending." >&2
    exit 1
  fi
done
if command -v sha256sum > /dev/null; then
  sha256sum --check --strict SHA256SUMS.txt
else
  shasum -a 256 --check --strict SHA256SUMS.txt
fi

# The trusted comment is signed too. The app compares it with the release's
# version, so these checksums can't be handed out again under a later one.
if [ -n "$key" ]; then
  minisign -S -s "$key" -m SHA256SUMS.txt -t "Quill $version"
else
  minisign -S -m SHA256SUMS.txt -t "Quill $version"
fi

gh release upload "$tag" SHA256SUMS.txt.minisig --clobber
gh release edit "$tag" --draft=false
echo "$tag is signed and public."
