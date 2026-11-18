#!/usr/bin/env bash
# Draws every icon file of Pencraft from design/icon/pencraft.svg. Run it after changing the SVG and commit
# what it writes:
#
#   app/assets/icon.png          the window's icon on Windows and Linux (256 px)
#   app/assets/icon-macos.png    the Dock icon of a Pencraft that runs outside its .app (512 px, Apple's grid)
#   installer/Pencraft.icns      the icon of Pencraft.app
#   installer/pencraft.ico       the icon of pencraft.exe, its shortcuts, .pen files and the installers
#
# Needs a Mac (swift and iconutil, both part of Xcode's command line tools) and python3 with Pillow.
set -euo pipefail
cd "$(dirname "$0")"
root=../..
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# One compiled renderer instead of starting the Swift interpreter for every size.
swiftc -O -o "$work/render" render.swift
mkdir -p "$root/app/assets"
"$work/render" pencraft.svg "$root/app/assets/icon.png" 256
"$work/render" pencraft.svg "$root/app/assets/icon-macos.png" 512 mac

# macOS: the plate on Apple's grid with its shadow, at every size the Finder and the Dock ask for.
set_dir="$work/Pencraft.iconset"
mkdir "$set_dir"
for size in 16 32 128 256 512; do
  "$work/render" pencraft.svg "$set_dir/icon_${size}x${size}.png" "$size" mac
  "$work/render" pencraft.svg "$set_dir/icon_${size}x${size}@2x.png" "$((size * 2))" mac
done
iconutil --convert icns --output "$root/installer/Pencraft.icns" "$set_dir"

# Windows: the plate fills the square. Each size is drawn from the SVG, not scaled down from a big one.
sizes="16 20 24 32 40 48 64 96 128 256"
for size in $sizes; do
  "$work/render" pencraft.svg "$work/win-$size.png" "$size"
done
python3 - "$work" "$root/installer/pencraft.ico" $sizes <<'PY'
import sys
from PIL import Image

work, out, sizes = sys.argv[1], sys.argv[2], [int(s) for s in sys.argv[3:]]
images = [Image.open(f"{work}/win-{s}.png").convert("RGBA") for s in sizes]
# Plain bitmaps, not PNG: the resource compiler of older Windows SDKs refuses PNG entries.
images[-1].save(out, format="ICO", bitmap_format="bmp", sizes=[(s, s) for s in sizes], append_images=images[:-1])
PY
ls -l "$root/app/assets" "$root/installer/Pencraft.icns" "$root/installer/pencraft.ico"
