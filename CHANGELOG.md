# Changelog

What changed in each version of Pencraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[semantic versioning](https://semver.org/).

The section of a version is that release's notes, and it is what people read
in the app before they download the update. Write it for them.

## [Unreleased]

## [0.2.1] - 2026-11-16

Pictures stay with the design they belong to.

### Fixed

- **Pictures made before the first save stay with the design.** A picture
  generated in a design that had no file yet was kept in your home folder,
  and the saved design named it by that full path, so it showed no picture
  on another computer. Saving now copies such pictures into `images/` beside
  the design. Designs saved with 0.2.0 are repaired the next time they are
  saved on the computer they were made on.
- **Save As takes the pictures along.** A design saved under a new name in
  another folder gets a copy of the pictures it names from its own folder.

## [0.2.0] - 2026-11-15

Pictures made and changed by AI, a chat for every open design, and phone
mocks that come inside the phone.

### Added

- **Pictures from a description.** File → Generate Image…, or ask the
  assistant: an image service (Google Gemini, OpenAI Images or a compatible
  one) makes the picture, which is kept beside the document in `images/`.
  Services and their keys are set up on the new Settings → Images page; keys
  are kept in the system keychain.
- **Change a picture by description.** "Make it dusk", "take the people out",
  "put this logo on that mug": in the Generate Image dialog with layers
  selected, with the "Describe a change…" field under an image fill, or
  through the assistant's new `edit_image` tool. Up to four reference
  pictures can go along. Only Google Gemini services can change pictures for
  now.
- **A chat for every open design.** Each design has its own conversation, and
  several can work at the same time. A tab shows a dot while its assistant
  works.
- **Screens inside their device.** A mock of a phone or tablet app now comes
  inside the phone, with its status bar, Dynamic Island or notch and home
  indicator. Every iPhone from the XR to the iPhone 18 Pro Max is covered,
  as are the iPhone SE, an Android phone and an iPad.
- **Standard components.** Ask for a named library (shadcn/ui, Radix and the
  like) or for plain, standard components, and the assistant follows it; a
  new `ui-components` skill teaches it how.

### Fixed

- **Copy and paste in the chat.** Text can be pasted into the message box
  from other apps, and text selected in the conversation can be copied out.
- A shader effect no longer loses its picture when several are drawn at
  once.

## [0.1.0] - 2026-11-14

The first release: a canvas design editor for `.pen` files, for Windows and
macOS.

### Added

- **The canvas.** Frames, rectangles, ellipses, polygons, text, sticky notes,
  icons and pen paths with editable points; fills, strokes, shadows, blurs
  and shader effects; snapping, nudging, and find layers.
- **Components and layout.** Components with instances, variables with light
  and dark themes, and flex layout.
- **Glass.** Frosted and liquid glass as a layer's background, glass groups,
  and corners that clip what lies under them. A `liquid-glass` skill teaches
  the assistant and other AI clients to design with it, and `check_glass`
  measures the result.
- **`.pen` files, kept faithfully.** A file read and written again is byte
  for byte the same.
- **Export** of any layer to PNG, JPEG, WebP, PDF or SVG.
- **An assistant in the app** that designs and changes the document in
  front, through Claude Code or a provider's API with your key, and a second
  model that reviews each finished design.
- **`pencraft mcp`**, so other AI clients can read and change open
  documents.
- **Installers.** A setup wizard, an MSI and the plain program for Windows,
  and a disk image for Apple Silicon and Intel Macs.
- **A card that offers newer versions.** Pencraft asks GitHub for the latest
  release when it starts and once a day, and shows what is new. A setting
  and `--no-update-check` switch it off.

[Unreleased]: https://github.com/AsP3X/pencraft/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/AsP3X/pencraft/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AsP3X/pencraft/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AsP3X/pencraft/releases/tag/v0.1.0
