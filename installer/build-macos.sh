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
# The app is signed "ad hoc": that is what lets it run on Apple Silicon at
# all, but it is not a Developer ID signature and the app is not notarised,
# so Gatekeeper stops the first start. See docs/RELEASING.md.
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
codesign --force --sign - "$work/Pencraft.app"
codesign --verify --strict "$work/Pencraft.app"

# The name must end in .dmg: app/src/update.rs looks for that.
image="$out/pencraft-$version-macos-universal.dmg"
ln -s /Applications "$work/Applications"
hdiutil create -volname "Pencraft $version" -srcfolder "$work" -ov -format UDZO "$image"

lipo -archs "$work/Pencraft.app/Contents/MacOS/pencraft"
ls -lh "$image"
