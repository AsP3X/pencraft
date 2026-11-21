# Changelog

What changed in each version of Pencraft. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[semantic versioning](https://semver.org/).

The section of a version is that release's notes, and it is what people read
in the app before they download the update. Write it for them.

## [Unreleased]

### Added

- **The assistant's suggestions wait beside the message box.** The next
  steps it offers no longer sit in the conversation. They are collected
  behind a round button beside the message box: open it and they fan out as
  small pills. Click one to send it, "Do all" to send them together, or the
  cross at a pill's end to remove it. New suggestions join the ones you
  haven't used, and the button shows how many are waiting. The assistant
  never starts on a suggestion by itself: once it has offered next steps, its
  answer changes nothing more, and a step is only done when you pick it.
- An empty chat offers six ideas picked at random from more than a hundred,
  and a button beside them deals six others.
- A message that waits for the assistant has a "Remove" button under it, for
  the one sent too soon or no longer wanted.
- A button under each answer copies it.

### Changed

- **The chat moves more and reads more easily.** Your message rises out of
  the message box; each thing the assistant does slides in as its own line,
  with a light running through it while it works and the time it took when
  it is done; and the assistant's words fade in behind a small orange drop.
- Once the assistant goes on, the steps it took fold into one line such as
  "5 steps · 8 s". Click it to see them again.
- What an answer takes counts up in a small pill over the message box while
  the assistant works, and then stands under the answer. With the sidebar
  folded away the pill over the message bar keeps the last sum. Hold the
  pointer over it for the parts and the conversation's sum.
- The message box grows with what you write, up to seven lines. The send
  button turns dark when there is something to send and becomes a stop
  button while the assistant works.
- The conversation has a thin scroll bar and fades out under the task list
  and the header; the lines that boxed the task list in are gone.

### Fixed

- An assistant that ended its answer while its task list was unfinished
  left the job half done. It is now sent back to the list, so a long job is
  worked through to its last task. If it stops twice more without finishing
  a task, the list is left and the chat says so.
- A step that failed only said that it failed. It now says why: the reason
  stands in its line, and the pointer over it shows all of it.
- When Claude Code's provider didn't accept its key, the chat waited for
  minutes without a word. It now says so at once.
- Text in the message box could run into the buttons under it.

## [0.3.27] - 2026-11-20

See what each answer took in tokens and money, follow a long job task by
task, paste pictures into your messages, and ask for charts, slides, landing
pages, Android screens and designs at the size their destination needs.

### Added

- **A long job shows its tasks.** Before the assistant starts on something
  big (a whole screen, several screens, a redesign) it splits the work into
  tasks. The list stands at the top of the chat while the conversation
  scrolls, with a tick on what is done and a bar for how far it is. When the
  list is worked through it is filed into the conversation.
- **The assistant knows more kinds of design.** Charts that are true to
  their numbers, slides, landing pages, Android screens in Material 3, and
  designs at the exact size of where they go (social posts, store listings,
  print, email), with more interface components to draw from.
- Paste a picture into a chat message. A screenshot or a copied picture
  becomes a small chip with a tiny preview, right where you pasted it, and the
  assistant gets the picture at that place in your sentence. Hold the pointer
  over the chip to see it larger. The picture is still there when you open the
  design another day.
- The chat says under each answer how many tokens it took and what it cost.
  The price comes from the provider's own bill, from prices you typed in for
  a model, or from the public list at models.dev, which is fetched once a day;
  a switch in the settings turns the look-up off.

## [0.3.26] - 2026-11-20

Search for models and assistants instead of scrolling for them, let the
provider fill in a model's numbers, and answer the assistant's questions
with several answers and your own.

### Changed

- **The assistant menu in the message box has a search and groups.** The
  menu lists the models under their provider instead of repeating the
  provider on every line, marks the one in use with a tick, and has a
  search field that finds an assistant by a few of its letters. The arrow
  keys move and Return chooses; "Manage…" opens the assistants in the
  settings.
- **Questions take several answers and one of your own.** When the
  assistant asks something, you tick as many of its answers as apply, add
  your own in the field under them if you like, and send them together.
  The next steps it offers work the same way, so several can be started at
  once. Before, the first click on an answer was the answer.
- **Choosing a provider's models has a window of its own.** "Choose
  models…" in a provider's form opens the provider's list over the
  settings, with a tick before each model. A search field finds a model by
  a few of its letters, typed in order but not necessarily side by side
  ("ds4p" finds deepseek-v4-pro), and shows which letters it matched. The
  arrow keys move through the list and Return ticks a model; "Select all"
  and "Clear" work on what is shown. Before, the list was a short strip
  under the form with no search.
- **A model's context window and max tokens come from the provider.**
  Where a provider says how much a model reads and writes, the form fills
  those numbers in for the models you choose, instead of the same
  placeholder for all. Ollama and Ollama Cloud are asked about each chosen
  model; they say how much it reads, and the max tokens then come down to
  that. Numbers you typed yourself stay as they are.

## [0.3.25] - 2026-11-20

An answer that a provider breaks off is asked for again by itself, and the
chat holds still while the model thinks.

### Fixed

- **An answer that breaks off is asked for again.** When a provider
  reports an error in the middle of an answer ("Internal Server Error"),
  or the connection drops there, Pencraft now sends the request again by
  itself, twice at most, and says so in the chat. Before, the run simply
  ended and you had to ask again. Nothing is done twice: the tools of an
  answer only run once the answer is whole.
- **The chat holds still while the model thinks.** The glimpse of the
  model's thinking grew, shrank and swapped its sentence as the words came
  in, and pushed the conversation around with it. It now has a place that
  is always two lines high: the thought flows through it, and when a new
  line begins the lines glide up by one.

## [0.3.24] - 2026-11-20

A problem in the chat now says enough to act on it or report it, and its
details can be copied.

### Added

- **A problem in the chat says more, and can be copied.** Under a problem
  the chat now shows which assistant was answering and through which
  provider, what it was doing, how far into the answer it was and when it
  happened. "Copy details" puts all of that, with your Pencraft version,
  on the clipboard to paste into a report; nothing of your conversation
  and no key is in it. An error a provider sends in the middle of an
  answer no longer stands alone as "Internal Server Error": it says that
  it came from the provider, with the provider's error code and, where
  the provider gives one, the ID of the request.

## [0.3.23] - 2026-11-20

The chat says what the assistant is doing while you wait and shows a glimpse
of what it thinks, and your own messages read from a straight left edge.

### Added

- **The chat says what the assistant is waiting on.** Where it only said
  "Thinking…", the line now says what the wait is for: "Waiting for
  Opus 5.5…" until the model answers at all, "Thinking…" while it thinks,
  and after a tool "Looking at the picture…", "Reading the review…",
  "Finding another way…" when the tool failed, or "Planning the next
  step…". From three seconds on it counts how long the wait has lasted.
- **A glimpse of what the model thinks.** Where the model's thinking can be
  read (Anthropic's API, Claude Code, Ollama, and other providers that
  send it), the sentence it is thinking now stands under that line in
  quiet letters. Thinking that took three seconds or more leaves a note in
  the conversation, such as "Thought for 8 s".

### Fixed

- **Your messages in the chat read from the left.** The lines of a message
  in its bubble were each pushed to the bubble's right edge, so a longer
  message started ragged. They now start at the bubble's left edge and
  wrap like any text; the bubble still stands at the right of the panel.

## [0.3.22] - 2026-11-20

Chat with models that run on your own computer through Ollama, or with
Ollama's hosted models.

### Added

- **Ollama in the chat.** Ollama has its own card in Settings → Agents: its
  address on this computer is filled in and it needs no key. With Ollama
  running, load its models, pick the ones you want, and they are in the
  assistant menu. If Ollama isn't running or has no models yet, the form
  says so and what to do. Pencraft talks to Ollama through Ollama's own
  API and asks it to read the whole conversation; through the
  OpenAI-compatible address Ollama read only the first part of it, and
  the model answered without knowing its job.
- **Ollama Cloud.** A second card connects Ollama's hosted models with an
  API key from ollama.com, for when the models shouldn't run on your own
  computer. Both can be set up side by side.

## [0.3.21] - 2026-11-19

Paste pictures and SVG, lock layers, pick colours, and move a selected group
as one.

### Added

- **Paste pictures and SVG.** A picture on the clipboard, such as a
  screenshot, is pasted with ⌘V as a layer filled with it; the picture is
  kept with the design's other pictures. SVG copied as text, as icon sites
  and other design tools offer it, is pasted as the shape it describes
  instead of as a page of code.
- **Lock layers.** A locked layer stays where it is: clicks on the canvas
  go through it to what is behind, and it can't be moved, resized or
  turned. Lock or unlock the selection with ⇧⌘L or from the action search,
  or click the lock beside a layer in the layers list, which is also where
  a locked layer is selected. Locking a frame locks everything in it.
- **Eyedropper.** Press I and click anything on the canvas to give its
  colour to the selected layers, as their fill, or as their stroke with ⌥
  held. With nothing selected the colour is copied as a hex value. It
  takes solid fills; over a gradient or a picture it takes the fill
  underneath.
- **Shadow spread.** A shadow can reach further out than its shape before
  it fades, or be drawn in with a negative value: the Spread field is under
  Blur in a shadow's settings. Files that already had a `spread` now show
  it.

### Changed

- **A selected group moves as one, and only a double click goes inside.**
  With a group, a frame or an instance selected, a press on anything in it
  takes the whole of it, so a drag moves all of it; before, the click went
  to the layer under the pointer. Double-click a layer to step inside and
  select it; from then on a click picks the other layers in there. The same
  holds for a selected board: drag any of its layers to move the board.

## [0.3.20] - 2026-11-19

Layers snap to each other, unsaved work survives a crash, chats are
remembered, and PDFs are vectors.

### Added

- **Layers line up with their neighbours as you move them.** "Snap to
  objects" in Settings → Canvas now does what it says. A layer you move or
  resize lands on the edges and the middle of the layers beside it, and of
  the frame it is in, when it comes within a few points of them, and a line
  shows what it lined up with. A layer moved between two others, or to the
  end of a row, also lands where the gaps are equal, and each equal gap is
  marked.
- **Distribute.** "Distribute horizontally" and "Distribute vertically"
  spread three or more selected layers so the gaps between them are equal.
  They are in the action search (⌘P) and, with three or more layers
  selected, under the alignment buttons.
- **Unsaved work comes back after a crash.** While you work, Pencraft keeps
  every design that has unsaved changes on the side, every few seconds. If
  it didn't close properly (a crash, a power cut, a forced quit), the next
  start names those designs and offers to restore them. A restored design
  opens with your changes in it, still unsaved; one step back with Undo is
  the file as it was last saved. Undo steps from before the crash and an
  answer the assistant was in the middle of are not kept.
- **A design's chat is remembered.** What you and the assistant said about
  a design is there again when you open the design the next time, and the
  assistant is told the last of it. It is kept on this computer, not in the
  design's file. Notes on the tools the assistant used are not kept, and a
  design you move or rename starts a new chat.

### Changed

- **A PDF export is vectors, with real text.** Shapes, gradients, pictures,
  icons and connectors are written as what they are, and text as text in
  its own font, which goes into the file: the page stays sharp at any size
  and its words can be selected and searched. A layer with a shadow, a
  blur, a pattern or an angular gradient, and text set on a circle or along
  a line, is put on the page as a picture of itself. A design that uses a
  material, a background blur or a shader is still one picture on its page,
  as before.
- **A board's name takes its board.** Click a board's name to select the
  board, and drag the name to move it, which helps on a board so full that
  every click inside lands on a layer.
- **Opening a file while Pencraft is running opens it there.** A design
  opened from the Windows Explorer or with `pencraft FILE` now opens as a
  tab in the Pencraft that is already running, which comes to the front,
  instead of starting a second one. Double-clicking a design in the Finder
  on a Mac doesn't open it yet.
- **Large designs draw a little faster when zoomed far out.** What would be
  smaller than a pixel on screen is no longer drawn.

## [0.3.19] - 2026-11-19

Board names stay readable when you zoom out.

### Changed

- **Board names no longer crowd each other when you zoom out.** A board's
  name used to be drawn in full at every zoom, so on a canvas of many
  boards the names ran into one another. A name now gets only the width of
  its board and ends in "…" where it doesn't fit. Very small boards show
  no name, and neither does a board whose name would lie on the board
  above it. Point at a board to read its whole name.
- **A selected board shows its name as a label above it.** The name sits
  in the selection colour above the turning handle, the counterpart of the
  size under the board, and no longer has the handle running through it.
  It moves there and back as you select and let go; with "Animations &
  effects" switched off it changes at once.

## [0.3.18] - 2026-11-19

Large designs are quick to work on.

### Changed

- **Editing a large design is quick.** Moving a layer or changing a
  property used to work the whole design out again, which on a design of
  11,000 layers took an eighth of a second for every step of a drag.
  Pencraft now works out only the screen you are changing: that drag runs
  at 67 pictures a second where it ran at 8. Changing a component, a
  variable or the theme still works everything out, since it can show
  anywhere.
- **Zooming and scrolling a large design no longer stutters.** After a
  zoom came to rest, or a scroll reached a new part of the canvas, the
  window stopped for a moment several times while the design was drawn
  anew. That is now done in the background, and the window keeps moving.

## [0.3.17] - 2026-11-19

Large designs keep moving after a zoom.

### Fixed

- **A large design no longer crawls after zooming.** In 0.3.16, a design
  with hundreds of glass or frosted layers made the window stutter for up
  to a minute each time a zoom came to rest, while their pictures were
  made in the background. That now takes one to three seconds on a design
  of 11,000 layers, and the window stays usable meanwhile.

## [0.3.16] - 2026-11-19

Zooming stays smooth, also on designs with a lot of text or glass.

### Changed

- **Zooming no longer stutters on text.** While you zoomed, Pencraft drew
  every letter anew at each size the zoom passed through, which made the
  window lag and now and then freeze for a moment. Text now keeps up with
  the zoom and is sharp again the moment the zoom rests. On a Mac at 120
  pictures a second, a zoom in and out that came 20 to 29 pictures late
  now comes at most one late.
- **Glass, blurs and shadows no longer hold the window up.** After a zoom
  or a long scroll their pictures are made in the background. Until they
  are ready you see the ones you had, and the window keeps moving.
- **Less memory for text.** The picture Pencraft keeps its letters in is
  at most a quarter of the size it could grow to before.
- **Very large letters are a little softer.** Letters that stand more than
  384 pixels high on screen are drawn enlarged from that size.

## [0.3.15] - 2026-11-19

The canvas reaches the top of the window.

### Changed

- **No bar behind the title.** With a design open, the title and the
  buttons at the top right float over the canvas like the other islands,
  and the canvas runs up to the window's top edge behind them. The empty
  space between them still moves the window.

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

[Unreleased]: https://github.com/AsP3X/pencraft/compare/v0.3.27...HEAD
[0.3.27]: https://github.com/AsP3X/pencraft/compare/v0.3.26...v0.3.27
[0.3.26]: https://github.com/AsP3X/pencraft/compare/v0.3.25...v0.3.26
[0.3.25]: https://github.com/AsP3X/pencraft/compare/v0.3.24...v0.3.25
[0.3.24]: https://github.com/AsP3X/pencraft/compare/v0.3.23...v0.3.24
[0.3.23]: https://github.com/AsP3X/pencraft/compare/v0.3.22...v0.3.23
[0.3.22]: https://github.com/AsP3X/pencraft/compare/v0.3.21...v0.3.22
[0.3.21]: https://github.com/AsP3X/pencraft/compare/v0.3.20...v0.3.21
[0.3.20]: https://github.com/AsP3X/pencraft/compare/v0.3.19...v0.3.20
[0.3.19]: https://github.com/AsP3X/pencraft/compare/v0.3.18...v0.3.19
[0.3.18]: https://github.com/AsP3X/pencraft/compare/v0.3.17...v0.3.18
[0.3.17]: https://github.com/AsP3X/pencraft/compare/v0.3.16...v0.3.17
[0.3.16]: https://github.com/AsP3X/pencraft/compare/v0.3.15...v0.3.16
[0.3.15]: https://github.com/AsP3X/pencraft/compare/v0.3.14...v0.3.15
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
