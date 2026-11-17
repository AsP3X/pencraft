# Changelog

What changed in each version of Pencraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[semantic versioning](https://semver.org/).

The section of a version is that release's notes, and it is what people read
in the app before they download the update. Write it for them.

## [Unreleased]

## [0.3.3] - 2026-11-17

Messages keep their layers, and a download bar that moves evenly.

### Changed

- **Sent messages show the layers they point at.** A layer pasted into a
  message is a chip in the sent message too, as it was while you wrote it,
  and what was selected when you sent it stands over the message.
- **The update's download bar moves evenly.** It glides to where the
  download is instead of jumping, a light runs over it while it works, and
  it says how many megabytes have arrived. The card fits a narrow window.

### Fixed

- A layer pasted at the very start or end of a message lost its chip when
  the message was sent: it showed as loose words, and the assistant was not
  told which layer was meant.

## [0.3.2] - 2026-11-17

More assistants, the web, and settings that do what they say.

### Added

- **The assistant can use the web.** Ask for real content or a real photo: it
  searches the web, reads a page, finds photos that may be reused (with who
  made them and under which licence) and puts one on a layer. Nothing needs a
  key; with a free Pexels or Pixabay key there are more photos to choose
  from. One switch in Settings → MCP turns the web off.
- **Connect a provider with its key.** The cards in Settings → Agents now
  open with everything but the key filled in: OpenAI, xAI, Google Gemini,
  OpenRouter, DeepSeek, Moonshot, Kimi For Coding, Together, Fireworks, Z.AI,
  Meta, OpenCode. Their models are loaded from the provider. Any service that
  speaks the OpenAI chat API works as a custom provider, also a model server
  on your own computer without a key.
- **Agent programs on your computer.** Besides Claude Code, the chat can talk
  to xAI's Grok, and to any other command-line agent you add with its
  program and arguments. Settings → Agents shows which are installed.
- **Three new settings pages.** Agent names: call the assistants what you
  like. Chat: the default assistant, and instructions of your own for every
  conversation. MCP: the lines to connect an outside AI client, ready to
  copy, and a switch for every tool an AI model may use.

### Changed

- **Keep writing while the assistant works.** A message sent meanwhile waits
  its turn and is sent when the assistant is done; with "Always steer
  messages" on, an API assistant reads it at its next step.
- **The agent settings work.** Chat text size, the cursor's label (Agent, the
  model's name, or none) and "Show agent input on dashboard" now do what
  they say.
- **The settings window is smaller**, the size it has in the original, and
  its forms have a way back.

### Fixed

- With a layer selected, the message bar left almost no room to type beside
  a long assistant name.
- The Anthropic card said "Connected automatically" although no key was set.

## [0.3.1] - 2026-11-16

A new home screen.

### Changed

- **The home screen starts designs faster.** Describe a screen or a page in
  the box at the top and press Return: Pencraft opens a new design and the
  assistant starts drawing it, with the chat showing.
- **Start at a screen size.** Beside the blank canvas and the starter board
  there are now a phone (390 × 844), a tablet (820 × 1180) and a desktop
  window (1440 × 900): each opens a design with one frame of that size.
- **Recent files show what is in them.** Each recent design has a small
  picture of its canvas, when it was last changed and the folder it is in.
  A file that is no longer there says so; right-click a recent file to take
  it off the list.

## [0.3.0] - 2026-11-16

Pencraft installs its own updates.

### Changed

- **Updates install themselves.** The card that offers a new version now has
  **Update and restart**: Pencraft downloads the new version, checks it,
  asks about unsaved designs, installs it and starts again. No browser and
  no setup wizard. This version still has to be installed by hand; the ones
  after it arrive this way, for a Pencraft installed with the setup wizard
  on Windows or as an app on a Mac. Other copies keep the download in the
  browser.
- **The update card reads better.** What is new is shown as paragraphs and a
  list that fit the card, instead of the changelog's short lines, with the
  bold words standing out.

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

[Unreleased]: https://github.com/AsP3X/pencraft/compare/v0.3.3...HEAD
[0.3.3]: https://github.com/AsP3X/pencraft/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/AsP3X/pencraft/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/AsP3X/pencraft/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/AsP3X/pencraft/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/AsP3X/pencraft/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AsP3X/pencraft/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AsP3X/pencraft/releases/tag/v0.1.0
