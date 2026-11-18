# Changelog

What changed in each version of Pencraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[semantic versioning](https://semver.org/).

The section of a version is that release's notes, and it is what people read
in the app before they download the update. Write it for them.

## [Unreleased]

## [0.3.14] - 2026-11-18

The assistant can ask you questions, and a design can travel as one file.

### Added

- **The assistant can ask you.** When a request could go two ways, the
  assistant asks instead of guessing: a question with a few answers to
  click. With the sidebar folded away, the status line over the message
  bar grows into the question; in the chat it is a card. Click an answer,
  or type your own. After finishing something the assistant can also
  offer next steps to start with one click.
- **Bundles.** File › Save As Bundle… puts a design and everything it
  shows into one `.pencraft` file: its pictures, also those that lay
  somewhere else on your disk, its shader files and the pictures' credits.
  Send that one file and the design arrives whole. Pencraft opens bundles
  like `.pen` files, and File › Save As .pen File… turns one back into a
  `.pen` file with its pictures beside it. The assistant saves a bundle
  when it is given a path ending in `.pencraft`.
- **Pencraft says what only it draws.** A design saved as a `.pen` file
  under a new name tells you when it uses something other programs that
  open `.pen` files leave out: connectors, masks, materials, pattern
  fills, component states, text on a circle or along a line.
- **Files from a newer Pencraft.** A file a newer version wrote opens with
  a note that this version may not draw all of it. What it doesn't know is
  kept when you save.

### Changed

- **A mark of Pencraft's own in `.pen` files.** Every save writes a key
  `pencraft` with the number of Pencraft's format and the names of what
  the design uses that only Pencraft draws. Other programs ignore it.

## [0.3.13] - 2026-11-18

Pencraft has an icon of its own.

<!-- required -->

### Added

- **An icon.** Pencraft has an icon of its own: a pen nib setting a point
  of a path. It is on the app in the Dock and the Finder on a Mac, and on
  the program, its shortcuts, the taskbar, `.pen` files and the installers
  on Windows.

## [0.3.12] - 2026-11-18

Fonts are right again while Pencraft works in the background.

### Fixed

- **Fonts while Pencraft is in the background.** When an AI client opened
  a document or wrote text while Pencraft's window was hidden or behind
  other windows, a font that hadn't been used since the app started was
  never loaded: after a wait of three seconds the text was measured and
  drawn in Arial instead, in screenshots, exports and the boxes the model
  was told. The font is now loaded at once.

## [0.3.11] - 2026-11-18

Logos in every style, shapes that cut what is inside them, and text round circles and along lines.

### Added

- **Text round a circle.** A text layer can follow the circle that fits
  its box, at the top or at the bottom, for the rim of a seal, a badge or
  a roundel: ask the assistant for "the name round the rim". It is drawn
  on the canvas and in PNG, PDF and SVG exports. In the properties panel
  a text has Straight, Circle Top and Circle Bottom, and the angle it is
  turned by.
- **Text along any line.** A text can also follow a line of any shape,
  for a name on a waving scroll or over an arch: ask the assistant for
  "the name along the ribbon", or choose "Line" for a text in the
  properties panel and bend the line by its points (Enter, or "Edit
  Line").
- **A shape as a mask.** A shape layer can cut the layers after it in its
  group to its outline: water and a swan inside a shield, shading inside a
  letter, hatching at any angle. Ask the assistant to "put it inside the
  shield", or tick "Mask Layers Above" on a shape under Appearance. Drawn
  on the canvas and in every export, and marked "mask" in the layers list.
- **An outline round text.** A text layer's stroke now draws an outline
  round its letters, outside them, for a team's name or a sticker; a
  second stroke draws a keyline round the first.

### Changed

- **Logos that are logos, in every style.** Asked for a logo, a model
  used to draw the subject's most obvious picture out of rectangles, put
  it in a shield and hang a ribbon under it. It now works the way an
  identity designer does: it chooses a style from what you ask for (a
  flat mark, a vintage badge, a crest or full coat of arms, a seal, an
  engraving, a mascot, a luxury monogram, a dimensional app icon and
  more), looks for one idea, draws three rough marks in black, keeps the
  best and builds it exactly. A detailed logo is built in passes, from the
  silhouette to the fine lines, and comes with a small version for favicon
  sizes. Pencraft has a `logos` skill for this, with the rules of each
  style, the numbers and worked logos.
- **A logo is checked as a logo.** Every picture a model takes of a logo
  now comes with the same logo drawn 24 pixels small, and with what the app
  counted in it: lights that disagree, too many line weights or colours, a
  detailed logo without its small version, a name set in a typewriter
  face. The second model that reviews finished designs now reviews logos
  too, however few layers they have, by the rules of their style.

### Fixed

- **A font weight a family doesn't have is said.** Text that asked for a
  weight its family lacks was drawn in the nearest one without a word; a
  model is now told which weight was drawn.
- **A model is told where letters are.** A text's box is taller than its
  letters, so names sat too high or too low beside a mark. Pencraft now
  gives a model the box round the letters as drawn.
- **A change that changes nothing fails.** An update a model sent with
  its properties in the wrong place was answered as done; it now says
  where they belong.

## [0.3.10] - 2026-11-18

Watch a model build its design step by step, and a message line that reads well.

### Changed

- **You see a model build its design.** What an AI model makes or changes
  in one go used to stand on the canvas all at once. Now it is shown a
  step at a time: each new layer appears, each change is made and each
  deleted layer goes at its turn, with a flash, and the model's cursor
  moves along.
- **A message line that reads well.** The short messages over the bottom
  of the window (what a model just changed, what a button couldn't do)
  were cramped into a narrow box. They now sit on a capsule as wide as
  their text, with the model's name set apart, and it opens out of a dot,
  flashes, changes shape when the next message comes, and closes into the
  dot again before it goes.
- **A model keeps every part of a design inside its frame.** A background
  or a glow larger than the frame used to pass as long as the frame cut it
  off, and its box then lay over the designs beside it. Pencraft now tells
  the model about any layer that reaches over the edge of its frame, and
  the guide says how to build it inside.

## [0.3.9] - 2026-11-17

Draw and edit connectors on the canvas, and zoom without stalls.

### Added

- **A connector tool.** Press C or pick the tool on the rail, press on a
  layer and pull to another: a connector joins them and follows them from
  then on. A new connector looks like the ones the design already has.
- **Handles on a connector.** A selected connector shows a handle at each
  end and at each waypoint. Drag an end to another layer or to one of its
  sides, drag a waypoint to move it, pull the dot in the middle of a
  stretch to add one, and double-click a waypoint to take it out.
- **Connector settings.** The properties island has a Connector section:
  the route, what sits at each end, marker size, curvature or corner
  radius, dashes, and a button that removes the waypoints.

### Changed

- **Zooming no longer stalls on designs with glass, blurs and shadows.**
  Each time the zoom passed a doubling, Pencraft re-made every effect in
  the design, which froze the window for up to seconds. It now keeps
  drawing while you zoom, re-makes only what is on screen once the zoom
  rests, and fills in the rest while you look.
- **A connector is picked on its line**, not on the box around it, so the
  layers it runs between stay easy to click.

## [0.3.8] - 2026-11-17

Wires that follow their nodes, components with states, and pattern fills.

### Added

- **Connectors.** A line between two layers that follows them when they
  move: a wire in a node editor, an arrow in a flow chart. It runs curved,
  at right angles or straight, and passes through waypoints you give it, so
  it can be led around a node. Its look is yours to set: colour and
  thickness, how far a curve swings, rounded turns, dashes, round or square
  ends, and an arrow, bar, triangle, dot or diamond at either end. For now
  an assistant sets connectors up; the canvas has no tool for them yet.
- **Component states.** A component names its states once (selected,
  disabled, on), and an instance picks one. A selected row no longer has
  to be overridden by hand on every screen.
- **Pattern fills.** A fill can repeat dots, a grid or lines across a
  layer, cut to its shape: the dot grid of a canvas or a ruled sheet,
  without a layer for every dot.
- **A colour variable at less than full strength.** A fill or stroke can
  use a variable with an opacity, so a tinted badge follows the variable
  and the theme instead of a colour written out by hand.

### Changed

- **Glass on a plain screen.** When there is nothing behind a glass layer,
  the assistant is now told to use a plain fill there, and no longer adds a
  backdrop you didn't ask for.

### Fixed

- **SVG export of paths.** A path layer was written with two `transform`
  attributes, which some programs refuse to open.
- **A list of choices is as wide as its button.** The list was two
  outlines narrower than the button it opens from.

## [0.3.7] - 2026-11-17

The lists you choose from look like the rest of Pencraft.

### Changed

- **The lists of choices match the app's other menus.** A list such as
  "Kind" in an image service's form, and the ones in the Generate Image
  dialog, now shows a tick before the choice in use and a soft fill under
  the pointer, and is as wide as the button it opens from.

## [0.3.6] - 2026-11-17

A new settings window, and updates that arrive within minutes.

### Added

- **Search the settings.** A field over the list of pages finds a setting,
  a tool or a provider by its name and takes you to its page.
- **Follow the computer's look.** Theme now offers System beside Light and
  Dark (Settings → General).
- **Check now.** The Updates card says when Pencraft last looked for a new
  version and looks again at a click.
- **Test connection** in a provider's form asks the provider for its models
  and says whether it answered.
- **Turn all on** brings back every tool that was switched off
  (Settings → Permissions).

### Changed

- **The settings window is rebuilt.** Seven pages in three groups: General
  and Canvas; Assistants, Chat and Images; Permissions and MCP. Every
  setting is in one place only, in a card with the ones it belongs to, and
  its switch sits close to its name.
- **Assistants** lists the ones you have set up first, with their state,
  then the providers you can add, in a grid you can search. An assistant's
  names are now in its own form.
- **Permissions** holds everything AI may do: work on your documents, use
  the web, make pictures, and each tool. **MCP** keeps the server and how
  to connect an app, one app at a time.
- The keys for Pexels and Pixabay moved to Settings → Images.
- A form's buttons stay in a bar under it, which says what is still missing.
- **Pencraft looks for a new version every 5 minutes** instead of every
  6 hours. You can choose 15 minutes, an hour or a day instead; a saved
  "Every 6 hours" becomes 5 minutes.
- Switches are larger.

### Removed

- The Language and "Default SVG generation provider" choices, which had
  only one option each, and the two sign-in buttons that did nothing yet.

### Fixed

- **Screens no longer come in a box with their name on it twice.** An
  assistant used to put each screen into a grey container with a caption
  that repeated the name the canvas already shows above it. Assistants are
  now told that a screen is a frame of its own, and Pencraft points the
  container out when one is made anyway, with or without a caption, and
  again whenever the assistant looks at its work.

## [0.3.5] - 2026-11-17

The Mac app is signed and checked by Apple.

### Fixed

- **No more "Apple could not verify Pencraft" on the Mac.** A downloaded
  copy used to be stopped at its first start and had to be allowed in
  System Settings. Pencraft is now signed with a Developer ID and notarised
  by Apple, and opens like any other app.

### Changed

- The first start of this version may ask once more for the keychain items
  that hold your API keys, because the app's signature changed. After this
  it stays the same from version to version.

## [0.3.4] - 2026-11-17

An update card that gets out of the way, and updates that can be required.

### Added

- **Fold the update card away.** The arrow in its corner shrinks the card to
  a small pill that still says there is an update, and how far its download
  is. A click opens it again.
- **Choose how often Pencraft looks for a new version**: every hour, every
  6 hours or every day (Settings → General → Updates). It now also looks
  while the window is left alone, and a version answered with "Later" is
  offered again at the next look.
- **Required updates.** A release can be marked as required. It then asks
  in the middle of the window and can't be skipped or put off: update, or
  quit. If its download fails you can go on for now.

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

[Unreleased]: https://github.com/AsP3X/pencraft/compare/v0.3.14...HEAD
[0.3.14]: https://github.com/AsP3X/pencraft/compare/v0.3.13...v0.3.14
[0.3.13]: https://github.com/AsP3X/pencraft/compare/v0.3.12...v0.3.13
[0.3.12]: https://github.com/AsP3X/pencraft/compare/v0.3.11...v0.3.12
[0.3.11]: https://github.com/AsP3X/pencraft/compare/v0.3.10...v0.3.11
[0.3.10]: https://github.com/AsP3X/pencraft/compare/v0.3.9...v0.3.10
[0.3.9]: https://github.com/AsP3X/pencraft/compare/v0.3.8...v0.3.9
[0.3.8]: https://github.com/AsP3X/pencraft/compare/v0.3.7...v0.3.8
[0.3.7]: https://github.com/AsP3X/pencraft/compare/v0.3.6...v0.3.7
[0.3.6]: https://github.com/AsP3X/pencraft/compare/v0.3.5...v0.3.6
[0.3.5]: https://github.com/AsP3X/pencraft/compare/v0.3.4...v0.3.5
[0.3.4]: https://github.com/AsP3X/pencraft/compare/v0.3.3...v0.3.4
[0.3.3]: https://github.com/AsP3X/pencraft/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/AsP3X/pencraft/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/AsP3X/pencraft/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/AsP3X/pencraft/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/AsP3X/pencraft/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AsP3X/pencraft/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/AsP3X/pencraft/releases/tag/v0.1.0
