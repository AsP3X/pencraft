# The `.quill` file: one binary file for a whole design

Date: 2026-11-21. Status: written, waiting for approval.

## Goal

A design is one file that works on any computer. Save writes it by default. It holds the document, the
pictures, shader files and imported documents the document names, and the chat about the design. It is a
binary format of our own, with no new dependency.

`.pen` is supported for opening and as an export: a `.pen` file opens, and File › Export .pen File… writes
one. It is no longer a format a document is saved as: every document that is saved lives in a `.quill` file.

The app will be renamed from Pencraft to Quill. The save format takes the new name first, in this repo, under
the app's present name. The app itself moves later, to a new repo; the old one stays until most users have
updated, and says where the app went once it is retired. That move is not part of this spec, which uses
today's names (`~/.pencraft`, `pencraft.iss`, the `pencraft` stamp key) where it points at what exists.

`.quill` is also the ending of the backup files of Quill, a handwriting notes app for Android. Such a file
does not start with our magic bytes, so it is refused with a sentence saying it is not a design.

## Behaviour

- **Endings.** `.quill` is the new format, and the only one Save writes. `.pen` is the plain JSON document: it
  opens, and is written only by an export. `.pencraft` (the zip bundle) still opens but is no longer written.
- **What a packed file is** is read from its first bytes: a file ending `.quill` or `.pencraft` is opened as
  the new format when it starts with the `.quill` magic and as an old zip bundle when it starts with `PK`. So
  an old bundle renamed to `.quill` still opens. A file ending `.pen` that starts with either is refused with
  a sentence saying to give it the ending `.quill`.
- **Save As.** It offers `.quill` only. A typed name gets `.quill` added after it (`home.v2` becomes
  `home.v2.quill`). A name ending `.pen` is refused with a sentence naming File › Export .pen File…, one ending
  `.pencraft` with a sentence that it is no longer written.
- **Menu.** File › Export .pen File… (also in the window's own menu on Windows, and "Export as .pen file…" in
  the action search) writes the plain document under a chosen name, with the files it names copied beside it,
  never over a file there. It is an export: the tab keeps its file, working folder, name and unsaved changes.
  A name ending `.quill` or `.pencraft` is refused; `.pen` is added after any other name. When the document
  holds parts a newer Pencraft added, the sentence says they are not in the `.pen` file.
- **An opened `.pen` or `.pencraft`.** Save writes the same name with the `.quill` ending beside it, the tab
  moves to that file, the pictures and files the document names come along, and a sentence says so. The file
  it was opened from is left as it is. When a file of that name is already there, Save As is asked instead.
- **`save_document` (MCP).** It writes `.quill` files only. A `.pen` path is refused with a sentence naming
  `.quill` and `export_document`, a `.pencraft` path with a sentence naming `.quill`. A document opened from a
  `.pen` or `.pencraft` file and saved without a path goes to the `.quill` file beside it, or fails when that
  is there already.
- **`export_document` (MCP).** Writes a `.pen` file as the menu's export does, through the same code.
- **Import Image copies.** The picture is copied into the design's `images/` folder and named from there, for
  every kind of document; one without a file yet uses `~/.pencraft/images` until it is saved, as generated
  pictures do. No more full paths to the user's disk.
- **Chat.** The conversation is saved in the file and comes back when the file is opened anywhere, also after
  it was renamed or moved. Details under *Chat* below.
- **Windows.** The installer ties `.quill` to the app, beside `.pen` and `.pencraft`.

Not included: fonts in the file (names only, as now), task lists, pictures pasted into the chat, a small
picture of the design stored in the file, shrinking pictures, saving only what changed, encryption, the
Finder's double click on a Mac.

## Built in four steps

Each step leaves a working app and has its own plan.

1. **The format**: the file, the document block, endings, the default, the menu, `save_document`, the
   installer. Plan: `docs/superpowers/plans/2026-11-21-quill-file.md`.
2. **Chat in the file** (*Chat* below).
3. **Import Image copies** (the bullet above).
4. **Crash recovery keeps pictures** (*Crash recovery* below).

## The file

All numbers are little-endian. The file is three parts in this order: header, index, blocks.

### Header (32 bytes)

| At | Size | What |
|---|---|---|
| 0 | 8 | Magic: `89 51 55 49 4C 4C 0D 0A` (`\x89QUILL\r\n`) |
| 8 | 4 | Layout number, `1` |
| 12 | 4 | Number of index entries |
| 16 | 8 | Length of the index in bytes |
| 24 | 4 | CRC-32 of the index bytes |
| 28 | 4 | Zero, kept for later |

The index starts at byte 32.

### Index entry

| Size | What |
|---|---|
| 1 | Kind: `1` document, `2` chat, `3` named file |
| 1 | Packing: `0` stored, `1` deflate |
| 2 | Length of the name in bytes |
| 8 | Where the block starts, from the start of the file |
| 8 | Length of the block as stored |
| 8 | Length of the block unpacked |
| 4 | CRC-32 of the block as stored |
| n | The name, UTF-8: the path the document names the file by; empty for the document and the chat |

### Blocks

- **Order.** The document, the chat when there is one, then the named files in the order `bundle::named`
  gives, then `images/credits.json`. The same document and files give the same bytes.
- **Packing.** Files ending `.png`, `.jpg`, `.jpeg`, `.webp` or `.gif` are stored. Everything else is
  deflated, and stored instead when deflate does not make it smaller.
- **One copy.** Named files with the same bytes, packed the same way, are written once: their index entries
  point at the same block. Found by length, then CRC-32, then comparing the bytes.
- **No more than opens again.** Save refuses a design whose document and files together pass 2 GB, with a
  sentence, so it never writes a file the reader would refuse.
- **Only what is named.** Files in the folder that the document does not name stay out, as now.

### Reading

- Every offset and length is checked against the file's length before anything is read or allocated.
- The unpacked lengths together may not pass 2 GB (`MOST_BYTES`, as now); an entry counts with the larger of
  its stored and unpacked length, also when it shares its block. A stored block's two lengths must be equal.
- A block starts after the index. The index holds one document at most and nothing after its last entry.
- Names go through `bundle::relative`, so none leads outside the working folder; one that does refuses the
  whole file, as now.
- A wrong CRC on the index or the document block refuses the file, with a sentence naming the part. A wrong
  CRC on a named file or the chat leaves that part out and opens the rest, with a sentence naming it.
- An entry kind this version does not know opens with the sentence about a newer version. Such entries are
  kept with the document and written back by Save, after the document and before the named files, so an older
  version does not strip what a newer one added. A higher layout number opens with a sentence saying that
  saving writes this version's layout, so parts it does not know may be left out.
- The home screen's small pictures read the header, the index and the document block from the file, and
  nothing else (`peek`, with `MOST_PEEKED` for the document's unpacked length).

## The document block: a value tree with a key table

The document is written as the same tree of values its JSON has, in binary. It lives in `pen-doc` as the
module `binary`, with no dependency beyond `serde_json`.

Writing: `Document` → `serde_json::Value` → bytes. Reading is the way back. Key order is kept
(`preserve_order`), so are unknown keys and kinds, and numbers keep their kind (whole or float), so
`.pen` → `.quill` → `.pen` gives the same bytes.

Unpacked layout:

1. One byte: the encoding's number, `1`.
2. The key table: a count, then each key as a length and its UTF-8 bytes, in the order the keys first appear.
3. One value.

Counts, lengths and whole numbers are LEB128 varints. A value is a tag byte and what follows it:

| Tag | Value | Follows |
|---|---|---|
| 0 | null | nothing |
| 1 | false | nothing |
| 2 | true | nothing |
| 3 | whole number, zero or above | varint |
| 4 | whole number below zero | varint of `-(n + 1)` |
| 5 | float | 8 bytes, IEEE 754 |
| 6 | string | length, UTF-8 bytes |
| 7 | list | count, the values |
| 8 | map | count, then pairs: the key's number in the table, the value |

The reader refuses, with a sentence and never a panic: an unknown tag or encoding number, a key number
outside the table, a count or length larger than the bytes left, a float that is not finite, text that is not
UTF-8, more than 127 lists and maps inside each other (what the `.pen` reader opens), a key named twice in
one map, keys that add up to more than 256 MB once handed to their maps, and bytes left over after the value.

The chat block uses the same encoding.

## Chat

- **In the file.** The chat block holds what `chat::history` keeps today (`said` and `total`), without the
  design's path. A pasted picture is kept by its file name only, not its full path, and is looked for in
  `~/.pencraft/chats/pictures`; where it is not there, the message shows the picture's name alone.
- **While working.** The conversation is still written to `~/.pencraft/chats` as it goes, as now, so chatting
  does not make a design unsaved. That side file also notes the CRC-32 of the chat block it belongs to.
- **Save** writes the conversation into the file and notes the new CRC in the side file.
- **Open.** When the side file's noted CRC is the file's chat block's, the side file is used: it is the same
  conversation, possibly with lines said since the last save. Otherwise the file's chat is used and the side
  file is replaced, because the file came from somewhere else.
- **Limit of this.** Lines said after the last save are on this computer only until the next save.
- `.pen` files keep their chat as now, beside the app and by path.

## Crash recovery

Pictures added to a `.quill` since it was last saved are kept: the recovery record names the document's
working folder, `bundle::sweep` leaves a folder alone while a recovery record names it, and Restore opens the
document in that folder. Discard removes it.

## How

- `pen-doc/src/binary.rs`: the value tree. `write(&Document) -> Vec<u8>`, `read(&[u8]) -> Result<Document, _>`,
  and the same pair for a plain `Value` (the chat).
- `app/src/bundle/quill.rs`: the container. Header, index, blocks, CRC-32 and deflate through the `flate2`
  already in use. It knows nothing about documents: it takes and gives entries (kind, name, bytes).
- `app/src/bundle.rs`: keeps the working folder, `named`, `carry` and the limits. `pack` writes a `.quill`;
  `open` and `peek` read a `.quill` or an old zip by the first bytes. `ENDING` becomes `quill`; `is_bundle`
  is true for `.quill` and `.pencraft`.
- `app/src/bundle/zip.rs`: the reader stays, the writer goes.
- `app/src/chat/history.rs`: the conversation to and from a chat block, and the noted CRC.
- `app/src/ops.rs` (`import_file`): copies through `imagery::store`, like a pasted picture.
- `app/src/recover.rs`, `bundle::sweep`: the working folder in the recovery record.
- `app/src/app.rs`, `menus.rs`, `palette.rs`, `agent.rs`, `app/tools.rs`, `app/pictures.rs`: endings, Save
  As, the export of a `.pen` file, the menu items, `save_document`, `export_document`.
- `installer/pencraft.iss`: the `.quill` association.
- `MECHANICS.md`, `mechanics.json`, `AI.md`, `CHANGELOG.md`: the rules and numbers above.

## Tests

- `pen-doc::binary`: every document the JSON tests use goes to bytes and back and writes the same `.pen`
  text; unknown keys and kinds survive; whole numbers and floats keep their kind; each thing the reader
  refuses is refused with a sentence.
- `bundle::quill`: entries go in and come out the same; the same input gives the same bytes; two files with
  the same bytes take one block; pictures are stored and text is deflated; a cut-off file, a wrong offset, a
  length past the end and a wrong CRC in each part give the sentence they should.
- `bundle`: a `.quill` packs, peeks and opens again; an old `.pencraft` still opens; a zip renamed to `.quill`
  opens; names that lead outside are refused; a damaged picture is left out and the rest opens.
- `chat::history`: a conversation goes into a block and back; the side file is used when its CRC matches and
  replaced when it does not; a pasted picture's full path is not in the block.
- `ops`: an imported picture from outside the folder is copied and named from `images/`.
- `recover`: a picture added to a `.quill` since the last save is there after Restore.
- App: a new document saves as `.quill`; Save on a `.pen` or `.pencraft` document writes the `.quill` beside
  it, with the pictures it names, and does not overwrite one that is there; a `.pen` path is not saved to; an
  export writes the `.pen` file and its files and leaves the tab's path, working folder and unsaved changes
  alone; `export_document` and `save_document` refuse each other's endings.
