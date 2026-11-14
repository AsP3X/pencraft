# One chat per open design

Date: 2026-11-14. Status: approved, being built.

## Goal

With two designs open, each has its own chat. Switching tabs switches the chat. A chat that is working in one
design does not block sending in another, and it keeps working on its own design while another is in front.

## Behaviour

- Each open design (tab) has its own conversation, run, half-written message, attached files and pasted layers.
  The home screen (no design open) has one of its own.
- Chats run at the same time. The send/stop button and the status line show the chat of the design in front.
- A chat's tool calls work on the chat's own design when they name no `document`, whichever design is in front.
  Clients outside Pencraft (Cursor, Codex, ...) work on the design in front, as before.
- A chat that is not in front does not change which tab is in front (`select_layers`, `open_document`).
- A chat follows the design it opens (`open_document`), when that design has no conversation of its own yet. So
  a chat started on the home screen moves into the design it starts.
- The reviewer is shown what was changed in the chat's own design.
- A tab whose chat is working shows a small dot. Closing a tab stops its chat.

Not included: keeping chats after a design is closed or the app quits.

## How

- `Editor::id`: a number that stays with an open design however its tab moves.
- `app::talk::Talk`: one conversation: its id, the design it belongs to, the `Chat`, and what was being written
  in it while another chat is in front. `App::talks` holds them; the one for the design in front is made when
  first needed.
- `link::Call::chat`: the id of the Pencraft chat a call comes from, `None` for an outside client. The API engine
  sets it directly; Claude Code is given the address `/mcp?chat=<id>` and the local server reads it from there.
- `App::document` resolves a call without `document` to its chat's design; a chat whose design was closed gets
  an error instead of another design.
- `App::designed` is kept by design id instead of tab position (which went wrong when a tab closed or moved).
- The conversation's motion and scroll position in egui's memory are kept per chat.

## Tests

- `link`: the chat id is read from the address.
- `chat::code`: the address Claude Code is given names the chat.
- `app::talk`: which design a call works on (own design in the background, closed design, outside client).
- `sidebar`: what is being written is handed over and taken back whole.
