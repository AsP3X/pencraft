#!/usr/bin/env bash
# Builds Pencraft for macOS: one app for Apple Silicon and Intel Macs, in a
# disk image.
#
# Writes to app/target/installer:
#   pencraft-<version>-macos-universal.dmg   Pencraft.app and a link to Applications;
#                                            the app's update card downloads this
#
# The version comes from app/Cargo.toml. Needs a Mac with Xcode's command line
# tools (lipo, codesign, hdiutil) and rustup.
#
# Signing. With PENCRAFT_SIGN_IDENTITY set to a "Developer ID Application: ..."
# identity from a keychain, the app and the disk image are signed with it and
# the hardened runtime, and the disk image is notarised and stapled, so a
# downloaded copy starts without a Gatekeeper warning. Notarising needs one of:
#   PENCRAFT_NOTARY_PROFILE                  a profile saved with
#                                            `xcrun notarytool store-credentials`
#   PENCRAFT_NOTARY_KEY, PENCRAFT_NOTARY_KEY_ID, PENCRAFT_NOTARY_ISSUER
#                                            an App Store Connect API key: the
#                                            path of the .p8 file, its key ID
#                                            and the issuer ID
# Without PENCRAFT_SIGN_IDENTITY the app is signed "ad hoc": that is what lets
# it run on Apple Silicon at all, but Gatekeeper stops the first start of a
# downloaded copy. See docs/RELEASING.md.
set -euo pipefail
cd "$(dirname "$0")/../app"

version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -n 1)
if [ -z "$version" ]; then
  echo "app/Cargo.toml has no version." >&2
  exit 1
fi
# The oldest macOS the app is built to start on.
export MACOSX_DEPLOYMENT_TARGET=11.0

echo "Building pencraft $version (release) for Apple Silicon and Intel"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --locked --target aarch64-apple-darwin
cargo build --release --locked --target x86_64-apple-darwin

out=target/installer
work=target/macos-bundle
rm -rf "$work"
mkdir -p "$out" "$work/Pencraft.app/Contents/MacOS"
# Files of other versions would end up in a release. Start clean.
rm -f "$out"/*.dmg

lipo -create -output "$work/Pencraft.app/Contents/MacOS/pencraft" \
  target/aarch64-apple-darwin/release/pencraft \
  target/x86_64-apple-darwin/release/pencraft
sed "s/@VERSION@/$version/g" ../installer/Info.plist > "$work/Pencraft.app/Contents/Info.plist"
# The icon Info.plist names (CFBundleIconFile). design/icon/build.sh draws it.
mkdir -p "$work/Pencraft.app/Contents/Resources"
cp ../installer/Pencraft.icns "$work/Pencraft.app/Contents/Resources/Pencraft.icns"
identity="${PENCRAFT_SIGN_IDENTITY:-}"
if [ -n "$identity" ]; then
  echo "Signing with $identity"
  # Notarising refuses an app without the hardened runtime and a timestamp.
  codesign --force --options runtime --timestamp --sign "$identity" "$work/Pencraft.app"
else
  echo "PENCRAFT_SIGN_IDENTITY is not set: signing ad hoc. Gatekeeper will stop a downloaded copy." >&2
  codesign --force --sign - "$work/Pencraft.app"
fi
codesign --verify --strict "$work/Pencraft.app"

# The name must end in .dmg: app/src/update.rs looks for that.
image="$out/pencraft-$version-macos-universal.dmg"
ln -s /Applications "$work/Applications"
hdiutil create -volname "Pencraft $version" -srcfolder "$work" -ov -format UDZO "$image"

if [ -n "$identity" ]; then
  codesign --force --timestamp --sign "$identity" "$image"
  if [ -n "${PENCRAFT_NOTARY_PROFILE:-}" ]; then
    notary=(--keychain-profile "$PENCRAFT_NOTARY_PROFILE")
  elif [ -n "${PENCRAFT_NOTARY_KEY:-}" ] && [ -n "${PENCRAFT_NOTARY_KEY_ID:-}" ] && [ -n "${PENCRAFT_NOTARY_ISSUER:-}" ]; then
    notary=(--key "$PENCRAFT_NOTARY_KEY" --key-id "$PENCRAFT_NOTARY_KEY_ID" --issuer "$PENCRAFT_NOTARY_ISSUER")
  else
    # A signed image that is not notarised is stopped just like an unsigned one.
    echo "PENCRAFT_SIGN_IDENTITY is set but there is nothing to notarise with." >&2
    echo "Set PENCRAFT_NOTARY_PROFILE, or PENCRAFT_NOTARY_KEY, PENCRAFT_NOTARY_KEY_ID and PENCRAFT_NOTARY_ISSUER." >&2
    exit 1
  fi
  echo "Notarising $image (Apple usually takes a few minutes)"
  # Apple answers "Invalid" with exit code 0 on some versions: read the status.
  result=$(xcrun notarytool submit "$image" "${notary[@]}" --wait --output-format json)
  echo "$result"
  id=$(printf '%s' "$result" | sed -n 's/.*"id" *: *"\([^"]*\)".*/\1/p' | head -n 1)
  if ! printf '%s' "$result" | grep -q '"status" *: *"Accepted"'; then
    [ -n "$id" ] && xcrun notarytool log "$id" "${notary[@]}" >&2 || true
    echo "Apple did not accept the disk image." >&2
    exit 1
  fi
  xcrun stapler staple "$image"
  xcrun stapler validate "$image"
  # What Gatekeeper itself says about the app in the image.
  spctl --assess --type execute --verbose=2 "$work/Pencraft.app"
fi

lipo -archs "$work/Pencraft.app/Contents/MacOS/pencraft"
ls -lh "$image"
