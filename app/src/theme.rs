//! The editor's colours and sizes, in one place (single-source-of-truth.md: UI tokens live in the theme module).

use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Height of the top bar: the title island with the space above it and a line's width below (the user's
/// design; the sidebar starts an [`ISLAND_GAP`] under it).
pub const TAB_BAR_HEIGHT: f32 = 48.0;
/// Room left for the macOS window buttons at the start of the title island.
pub const TRAFFIC_LIGHT_SPACE: f32 = 90.0;
/// Space between the window's left edge and the macOS close button.
pub const TRAFFIC_LIGHT_LEFT: f32 = 21.0;
/// Side of the app's icon before its name in the title bar.
pub const TITLE_MARK: f32 = 20.0;
/// Size of the app's name in the title bar, set in bold.
pub const TITLE_SIZE: f32 = 13.0;
/// Size of the other words in the title bar (a tab's name, "Share"), set in bold.
pub const TITLE_TEXT: f32 = 12.0;
/// Space at the title bar's left and right end.
pub const TITLE_PADDING: [f32; 2] = [16.0, 12.0];
/// Space between two things in the title bar.
pub const TITLE_GAP: f32 = 8.0;
/// Height of the line between the title bar's buttons and the window's own.
pub const TITLE_DIVIDER: f32 = 16.0;
/// Width of one of the window's own buttons in the title bar (minimise, maximise, close); as high as a
/// [`TITLE_BUTTON`].
pub const CAPTION_BUTTON: f32 = 40.0;
/// Side of the maximise button's square, a little smaller than the two icons beside it so it looks their size.
pub const CAPTION_SQUARE: f32 = 13.0;
/// Height of a tab in the title bar; its corners are rounded by [`CONTROL_RADIUS`].
pub const TAB_HEIGHT: f32 = 28.0;
/// Seconds a new tab takes to pour out of the "new file" button.
pub const TAB_POUR_TIME: f32 = 0.45;
/// Seconds a closed tab takes to bead up and burst.
pub const TAB_DRAIN_TIME: f32 = 0.3;
/// How large the bead is that a closed tab draws itself together into.
pub const TAB_DRAIN_BEAD: f32 = 9.0;
/// How large the two drops are that the bead bursts into, at their largest.
pub const TAB_DRAIN_SPRAY: f32 = 3.5;
/// How far apart the pouring tab and the drop at the button still melt into each other, at the start.
pub const TAB_POUR_MELT: f32 = 18.0;
/// How large the drop at the button is that a new tab pours from, at the start.
pub const TAB_POUR_DROP: f32 = 11.0;
/// How thick the thread of liquid between a pouring tab and the button is, at the start (half its height).
pub const TAB_POUR_THREAD: f32 = 4.5;
/// How far to the left of its place a pouring tab's name starts.
pub const TAB_POUR_SLIDE: f32 = 10.0;
/// How large the larger of the two beads is that the thread breaks into, at its largest.
pub const TAB_POUR_BEAD: f32 = 3.6;
/// Room kept around the pouring tab in its picture, for its swell.
pub const TAB_POUR_ROOM: f32 = 10.0;
/// Width of the lit rim of the pouring tab, and how far to each side the light on it reaches.
pub const TAB_POUR_RIM: [f32; 2] = [1.4, 46.0];
/// Side of a tab's icon, and of the icon in "Share".
pub const TAB_ICON: f32 = 14.0;
/// Space at a tab's two ends, and between its icon and its name.
pub const TAB_PADDING: [f32; 2] = [10.0, 8.0];
/// How wide the dot is that a tab shows while its chat's assistant is at work.
pub const TAB_WORK_DOT: f32 = 7.0;
/// Side of the icon buttons in the title bar.
pub const TITLE_BUTTON: f32 = 28.0;
/// How far in from the window's edge a press resizes it, where Quill draws the title bar itself (Windows).
pub const WINDOW_EDGE: f32 = 6.0;
/// How far along an edge from a corner a press resizes the window both ways.
pub const WINDOW_CORNER: f32 = 16.0;
/// Width of the sidebar island on the left (the user's sidebar design: 377 pt).
pub const SIDEBAR_WIDTH: f32 = 377.0;
/// Width of the sidebar island after its expand button was pressed.
pub const SIDEBAR_WIDE_WIDTH: f32 = 560.0;
/// Space between the window edge and an island, and between two islands.
pub const ISLAND_GAP: f32 = 8.0;
/// Corner radius of the sidebar and composer islands.
pub const ISLAND_RADIUS: f32 = 14.0;
/// Height of the message box island under the sidebar.
pub const COMPOSER_HEIGHT: f32 = 91.0;
/// Space between an island's edge and its content.
pub const ISLAND_PADDING: f32 = 14.0;
/// Space between the sidebar island's edge and its header buttons.
pub const SIDEBAR_HEADER_PADDING: f32 = 12.0;
/// Height of a row of sidebar header buttons.
pub const SIDEBAR_ROW: f32 = 28.0;
/// Corner radius of sidebar buttons and suggestion chips.
pub const CHIP_RADIUS: f32 = 8.0;
/// Height of a suggestion chip.
pub const CHIP_HEIGHT: f32 = 29.0;
/// Side of an interface icon.
pub const ICON_SIZE: f32 = 16.0;
/// Line width of an interface icon.
pub const ICON_LINE: f32 = 1.3;
/// Size of the sidebar's titles ("Chat history", "Generate code").
pub const SIDEBAR_TITLE_SIZE: f32 = 13.0;
/// Size of the text typed into the message box.
pub const MESSAGE_TEXT_SIZE: f32 = 13.5;
/// Size of the text on the sidebar's labelled buttons.
pub const SIDEBAR_TEXT_SIZE: f32 = 13.0;
/// Diameter of the round send button in the message box.
pub const SEND_BUTTON: f32 = 20.0;
/// Diameter of the send button of the chat's message box.
pub const MESSAGE_SEND: f32 = 26.0;
/// How long that button takes to turn dark once there is something to send, in seconds.
pub const MESSAGE_SEND_TIME: f32 = 0.16;
/// How long the light around it takes for one turn while the assistant works, in seconds.
pub const MESSAGE_SEND_TURN: f32 = 0.9;
/// How many lines of a message the message box grows to before it scrolls.
pub const COMPOSER_ROWS_MOST: f32 = 7.0;
/// How long the message box takes to grow by a line, in seconds.
pub const COMPOSER_GROW_TIME: f32 = 0.12;
/// How long the light takes to run along the message box's top edge while the assistant works, in seconds,
/// and the share of the edge it covers.
pub const COMPOSER_LIGHT_TURN: f32 = 1.5;
/// See [`COMPOSER_LIGHT_TURN`].
pub const COMPOSER_LIGHT_SHARE: f32 = 0.3;
/// Side of the arrow on a round send button (the message box's, and the quick field's under a picture).
pub const SEND_ARROW: f32 = 12.0;
/// Diameter of the highlight behind the active tool in the rail.
pub const RAIL_ACTIVE: f32 = 28.0;
/// Width of the tool rail beside the sidebar.
pub const RAIL_WIDTH: f32 = 36.0;
/// Height one tool takes in the rail.
pub const RAIL_STEP: f32 = 32.0;
/// Height one of the small arrows under a tool takes in the rail.
pub const RAIL_ARROW_STEP: f32 = 12.0;
/// Width of the properties panel.
pub const PROPERTIES_WIDTH: f32 = 228.0;
/// Corner radius of controls.
pub const CONTROL_RADIUS: u8 = 6;
/// Selection colour.
pub const ACCENT: Color32 = Color32::from_rgb(0x3b, 0x82, 0xf6);
/// The accent as a plate behind white words (a highlighted row, a marked chip): 5.2:1 against white, where
/// [`ACCENT`] is 3.7:1.
pub const ACCENT_PLATE: Color32 = Color32::from_rgb(0x25, 0x63, 0xeb);
/// Width of the ring around a control reached with the keyboard ([`Palette::focus`]).
pub const FOCUS_RING: f32 = 2.0;
/// Least side of the area an icon button answers clicks in (WCAG 2.5.8); its icon can be smaller.
pub const TARGET_MIN: f32 = 24.0;
/// The colour of components: their outline, canvas label and layer name.
pub const COMPONENT: Color32 = Color32::from_rgb(0xd9, 0x46, 0xef);
/// The colour of instances and the layers inside them.
pub const INSTANCE: Color32 = Color32::from_rgb(0x8b, 0x5c, 0xf6);
/// The colour of an AI model at work: the flash on a layer it touched, and its cursor.
pub const AGENT: Color32 = Color32::from_rgb(96, 125, 255);
/// How long a layer an AI model touched flashes.
pub const AGENT_FLASH_TIME: std::time::Duration = std::time::Duration::from_millis(1200);
/// How often an idle animation that runs as long as the assistant works (the working light, the stop button's
/// comet, the status pill, the waiting dots) is drawn again: about 30 frames a second, so a long run doesn't
/// keep the computer drawing at full rate. New content still shows at once.
pub const IDLE_FRAME: std::time::Duration = std::time::Duration::from_millis(33);
/// How long the message line shows a sentence.
pub const TOAST_TIME: std::time::Duration = std::time::Duration::from_secs(6);
/// How long the message line's capsule takes to open, or to take the shape of the next sentence.
pub const TOAST_MORPH: std::time::Duration = std::time::Duration::from_millis(320);
/// How long the flash on a sentence just said lasts.
pub const TOAST_FLASH: std::time::Duration = std::time::Duration::from_millis(650);
/// How long the message line takes to go at the end: it closes into a dot, then the dot fades.
pub const TOAST_LEAVE: std::time::Duration = std::time::Duration::from_millis(480);
/// The share of [`TOAST_LEAVE`] the capsule takes to close into the dot; the dot fades in the rest.
pub const TOAST_CLOSING: f32 = 0.7;
/// How far under its place the message line starts, points.
pub const TOAST_RISE: f32 = 10.0;
/// The least height of the message line's capsule (one line of text); its ends are half circles of this.
pub const TOAST_HEIGHT: f32 = 38.0;
/// The most the message line's capsule is wide, and what it keeps free at the window's sides.
pub const TOAST_WIDTH: f32 = 560.0;
/// See [`TOAST_WIDTH`].
pub const TOAST_MARGIN: f32 = 24.0;
/// Space between the capsule's edge and its text at the left and right; top and bottom share as much.
pub const TOAST_PADDING: f32 = 18.0;
/// Text size of the message line.
pub const TOAST_TEXT: f32 = 13.0;
/// The dot before a model's name on the message line, and the space after it.
pub const TOAST_DOT: f32 = 7.0;
/// See [`TOAST_DOT`].
pub const TOAST_GAP: f32 = 8.0;
/// How far over the window's bottom edge the message line sits: clear of the message bar.
pub const TOAST_LIFT: f32 = 90.0;
/// How long after one small step of a model's change the next is shown.
pub const AGENT_STEP: std::time::Duration = std::time::Duration::from_millis(80);
/// The longest the steps of one call take together; with more steps than fit they follow each other faster.
pub const AGENT_STEPS_MOST: std::time::Duration = std::time::Duration::from_millis(2500);
/// How far outside the layer the flash's outline starts, points.
pub const AGENT_FLASH_GROW: f32 = 12.0;
/// Width of the flash's outline.
pub const AGENT_FLASH_STROKE: f32 = 2.0;
/// How long the AI's cursor takes to get to where it works next.
pub const AGENT_CURSOR_TRAVEL: std::time::Duration = std::time::Duration::from_millis(450);
/// How long the AI's cursor stays after the model last did something, and how long it then takes to fade.
pub const AGENT_CURSOR_STAY: std::time::Duration = std::time::Duration::from_secs(8);
/// See [`AGENT_CURSOR_STAY`].
pub const AGENT_CURSOR_FADE: std::time::Duration = std::time::Duration::from_millis(500);
/// The AI cursor's arrow at its small size, from its tip, points.
pub const AGENT_CURSOR: [(f32, f32); 4] = [(0.0, 0.0), (0.0, 16.0), (4.5, 12.0), (11.5, 11.5)];
/// How much larger the AI's cursor is at the settings' Medium and Large sizes.
pub const AGENT_CURSOR_MEDIUM: f32 = 1.35;
/// See [`AGENT_CURSOR_MEDIUM`].
pub const AGENT_CURSOR_LARGE: f32 = 1.7;
/// Text size of the name beside the AI's cursor at its small size.
pub const AGENT_LABEL_TEXT: f32 = 11.0;
/// Corner radius of a pasted layer's chip in the message box.
pub const CHIP_CORNER: f32 = 4.0;
/// Size of the icon in a pasted layer's chip.
pub const CHIP_ICON: f32 = 12.0;
/// Size of the tiny preview in a pasted picture's chip.
pub const CHIP_THUMB: f32 = 14.0;
/// How far from the chip's left edge the middle of its icon is.
pub const CHIP_ICON_AT: f32 = 10.0;
/// The colour of a problem the chat reports.
pub const PROBLEM: Color32 = Color32::from_rgb(0xdc, 0x26, 0x26);
/// How much of the chat's width a message of the user's takes at most.
pub const CHAT_BUBBLE_SHARE: f32 = 0.86;
/// The room between two lines of the conversation.
pub const CHAT_GAP: f32 = 10.0;
/// The room between two notes on what the assistant did, which read as one list.
pub const CHAT_NOTE_GAP: f32 = 6.0;
/// The colour of what the assistant is doing in the conversation: the liquid's, as in the task list.
pub const CHAT_ACCENT: Color32 = LIQUID;
/// How high the line of one note is, at the small text size.
pub const CHAT_NOTE_ROW: f32 = 22.0;
/// The least time a tool call has to take for its note to say how long it took, in seconds.
pub const CHAT_NOTE_TIMED: f32 = 0.1;
/// From how many notes on a stretch of work is folded into one line once the assistant goes on.
pub const CHAT_STEPS_FOLD: usize = 2;
/// How long folding such a stretch, or opening it again, takes, in seconds.
pub const CHAT_STEPS_TIME: f32 = 0.32;
/// How high the one line of a folded stretch is, at the small text size.
pub const CHAT_STEPS_HEAD: f32 = 22.0;
/// The size of each tool's round icon on that line, and how far each lies over the one before it.
pub const CHAT_STEPS_ICON: f32 = 18.0;
/// See [`CHAT_STEPS_ICON`].
pub const CHAT_STEPS_LAP: f32 = 6.0;
/// How long the light takes to run once through the words of a tool at work, in seconds.
pub const CHAT_SHIMMER_TURN: f32 = 1.5;
/// How many letters to each side of its middle that light reaches.
pub const CHAT_SHIMMER_BAND: f32 = 5.0;
/// How far under its place a message of the user's starts when it is sent, and how long it takes to rise.
pub const CHAT_SEND_RISE: f32 = 44.0;
/// See [`CHAT_SEND_RISE`].
pub const CHAT_SEND_TIME: f32 = 0.4;
/// The corners of a message of the user's, and the one corner that points at the message box.
pub const CHAT_BUBBLE_RADIUS: u8 = 14;
/// See [`CHAT_BUBBLE_RADIUS`].
pub const CHAT_BUBBLE_TAIL: u8 = 4;
/// How many of the letters the assistant wrote last are still fading in.
pub const CHAT_WRITE_FADE: usize = 14;
/// The radius of the drop that stands after the assistant's words while it writes.
pub const CHAT_CARET: f32 = 3.5;
/// The room between an answer and the line under it (what it took, and the button that copies it).
pub const CHAT_FOOT_GAP: f32 = 4.0;
/// How far under the panel's header the conversation fades out as it scrolls up.
pub const CHAT_FADE: f32 = 22.0;
/// How wide the conversation's scroll bar is, and how wide under the pointer.
pub const CHAT_SCROLL_BAR: f32 = 3.0;
/// See [`CHAT_SCROLL_BAR`].
pub const CHAT_SCROLL_HELD: f32 = 6.0;
/// The size of the round button the assistant's next steps are collected behind, beside the message box.
pub const OFFER_BUTTON: f32 = 40.0;
/// How high the pill of one step is, the room between two pills, and the room beside a pill's words.
pub const OFFER_PILL: f32 = 32.0;
/// See [`OFFER_PILL`].
pub const OFFER_GAP: f32 = 6.0;
/// See [`OFFER_PILL`].
pub const OFFER_PAD: f32 = 12.0;
/// The widest a pill gets, and on how many lines a step's words may stand in it: a step of the most letters
/// one may have still shows whole.
pub const OFFER_MOST_WIDTH: f32 = 300.0;
/// See [`OFFER_MOST_WIDTH`].
pub const OFFER_ROWS: usize = 3;
/// The narrowest a pill is made where there is little room beside the button.
pub const OFFER_NARROWEST: f32 = 150.0;
/// The room the cross that removes a step takes at the right end of its pill: all of it can be clicked.
pub const OFFER_CROSS: f32 = 30.0;
/// The size of the count on the folded button.
pub const OFFER_BADGE: f32 = 18.0;
/// How long the steps take to open or fold, in seconds, and how much later each pill sets off than the one
/// before it, as a share of that.
pub const OFFER_OPEN_TIME: f32 = 0.34;
/// See [`OFFER_OPEN_TIME`].
pub const OFFER_OPEN_APART: f32 = 0.18;
/// How fast a pill glides to its place when the others make room or close a gap.
pub const OFFER_GLIDE: f32 = 16.0;
/// How many seconds lie between two steps that come together, and how long a new pill glows.
pub const OFFER_BORN_APART: f32 = 0.08;
/// See [`OFFER_BORN_APART`].
pub const OFFER_GLOW_TIME: f32 = 0.9;
/// How long a step that was sent flies into the conversation, in seconds, and how many seconds lie between
/// two pills when all are sent.
pub const OFFER_FLY_TIME: f32 = 0.62;
/// See [`OFFER_FLY_TIME`].
pub const OFFER_FLY_APART: f32 = 0.09;
/// How long a dropped pill falls away, in seconds.
pub const OFFER_DROP_TIME: f32 = 0.22;
/// How long the button shows its tick when the last step was taken, the ring around it widens when steps
/// come, and the count of new steps rises over it, each in seconds.
pub const OFFER_DONE_TIME: f32 = 0.9;
/// See [`OFFER_DONE_TIME`].
pub const OFFER_RING_TIME: f32 = 0.8;
/// See [`OFFER_DONE_TIME`].
pub const OFFER_PLUS_TIME: f32 = 1.0;
/// How much taller the gap between the conversation and the message box is while the counter of what an
/// answer takes stands in it: the counter's pill and the gap over it.
pub const CHAT_SPENT_STRIP: f32 = CHAT_SPENT_PILL + ISLAND_GAP;
/// How tall the counter's pill over the message bar is while the sidebar is folded away.
pub const CHAT_SPENT_PILL: f32 = 22.0;
/// The room beside the words in that pill.
pub const CHAT_SPENT_PAD: f32 = 10.0;
/// The size of a note's icon, with the circle that goes around it while its tool works.
pub const CHAT_NOTE_ICON: f32 = 16.0;
/// The share of that size the icon itself has while its tool works, inside the circle.
pub const CHAT_ICON_AT_WORK: f32 = 0.6;
/// The width of the circle's line.
pub const CHAT_ICON_RING: f32 = 1.5;
/// Seconds the comet takes once around the icon.
pub const CHAT_ICON_TURN: f32 = 1.1;
/// How much of the circle the comet's tail covers, in radians.
pub const CHAT_ICON_TAIL: f32 = 4.4;
/// How fast the icon breathes while its tool works, in radians a second.
pub const CHAT_ICON_BREATH: f32 = 3.2;
/// The share of its size an icon with a moving picture of its own settles from when its tool is done.
pub const CHAT_ICON_FROM_OWN: f32 = 0.8;
/// Seconds a page of the reading book takes to turn.
pub const TOOL_PAGE_TIME: f32 = 1.1;
/// Seconds between two blinks of the looking eye.
pub const TOOL_BLINK_EVERY: f32 = 2.6;
/// Seconds a blink takes.
pub const TOOL_BLINK_TIME: f32 = 0.18;
/// Seconds the pencil takes to write its line and go back.
pub const TOOL_WRITE_TIME: f32 = 1.4;
/// Seconds a note shows its tool at work at the least, however fast the tool was.
pub const CHAT_ICON_LEAST: f32 = 1.2;
/// Seconds an icon takes to settle when its tool is done or has failed.
pub const CHAT_ICON_SETTLE: f32 = 0.5;
/// How far the circle widens as it fades when the tool is done or has failed.
pub const CHAT_ICON_BURST: f32 = 3.5;
/// How far to each side the icon of a failed call shakes at most.
pub const CHAT_ICON_SHAKE: f32 = 2.5;
/// The height of the one-line status over the message bar.
pub const CHAT_STATUS_HEIGHT: f32 = 28.0;
/// The room between that line and the bar.
pub const CHAT_STATUS_GAP: f32 = 8.0;
/// The room between the line's island and what stands in it, left and right.
pub const CHAT_STATUS_PAD: f32 = 12.0;
/// Seconds the line takes to turn from one thing into the next: the island finds its new width while the
/// old words fade and the new ones come.
pub const CHAT_STATUS_MORPH: f32 = 0.36;
/// Seconds the line keeps showing a tool's note at the least, however fast the tool was.
pub const CHAT_STATUS_HOLD: f32 = 2.4;
/// Seconds the line still says that the answer is there after a run.
pub const CHAT_STATUS_STAY: f64 = 12.0;
/// The widest the assistant's question over the message bar gets.
pub const CHAT_ASK_WIDTH: f32 = 440.0;
/// The room between the edge of a question's island or card and what stands in it.
pub const CHAT_ASK_PAD: f32 = 14.0;
/// The least height of an answer to pick.
pub const CHAT_ASK_OPTION: f32 = 32.0;
/// The room between two answers.
pub const CHAT_ASK_OPTION_GAP: f32 = 6.0;
/// The room between an answer's edge and its words, left and right.
pub const CHAT_ASK_OPTION_PAD: f32 = 11.0;
/// The corner radius of an answer.
pub const CHAT_ASK_OPTION_RADIUS: f32 = 8.0;
/// The room between a question, its answers and the line under them.
pub const CHAT_ASK_GAP: f32 = 10.0;
/// The side of the button that closes a question.
pub const CHAT_ASK_CLOSE: f32 = 22.0;
/// The size of the box before an answer that shows whether it is ticked.
pub const CHAT_ASK_TICK: f32 = 15.0;
/// The height of the button that sends what is chosen in a question.
pub const CHAT_ASK_SEND: f32 = 28.0;
/// Seconds a new line of the conversation takes to fade in.
pub const CHAT_APPEAR: f32 = 0.24;
/// How far below its place a new line starts.
pub const CHAT_RISE: f32 = 6.0;
/// Letters a second at which the assistant's words are let out.
pub const CHAT_WRITE_PACE: f32 = 70.0;
/// The share of the letters still waiting that is let out each second on top of that.
pub const CHAT_WRITE_CATCH_UP: f32 = 5.0;
/// The size of one of the three dots that wait for the model.
pub const CHAT_DOT: f32 = 4.0;
/// The room between two of them.
pub const CHAT_DOT_GAP: f32 = 3.0;
/// How far a dot rises.
pub const CHAT_DOT_LIFT: f32 = 3.0;
/// How fast the dots rise and fall, in radians a second.
pub const CHAT_DOT_PACE: f32 = 6.0;
/// From how many seconds on a wait for the model says how long it has lasted.
pub const CHAT_WAIT_COUNTED: u64 = 3;
/// How many lines high the place under a wait is that shows what the model thinks: always this many.
pub const CHAT_THOUGHT_ROWS: usize = 2;
/// How fast the thought's lines glide up when a new one begins: the share of the way left that is covered
/// each second, as the rate of an exponential approach.
pub const CHAT_THOUGHT_GLIDE: f32 = 14.0;
/// How long the button that copies a problem's details says that it did, in seconds.
pub const CHAT_COPIED_STAYS: f64 = 1.6;
/// How much larger the chat's text is at the settings' Medium and Large sizes.
pub const CHAT_TEXT_MEDIUM: f32 = 1.15;
/// See [`CHAT_TEXT_MEDIUM`].
pub const CHAT_TEXT_LARGE: f32 = 1.3;
/// The liquid a task list pours in with ([`crate::liquid`]): its body.
pub const LIQUID: Color32 = Color32::from_rgb(0xff, 0x7a, 0x14);
/// The liquid's deeper colour, away from its edge.
pub const LIQUID_CORE: Color32 = Color32::from_rgb(0xee, 0x5c, 0x00);
/// The liquid's rim where the travelling light is not.
pub const LIQUID_RIM: Color32 = Color32::from_rgb(0xff, 0xa6, 0x4d);
/// The soft glow around the liquid.
pub const LIQUID_GLOW: Color32 = Color32::from_rgb(0xff, 0x6a, 0x00);
/// How close two shapes of liquid come before they grow together, points.
pub const LIQUID_MELT: f32 = 10.0;
/// How wide the liquid's lit rim is, points.
pub const LIQUID_RIM_WIDTH: f32 = 1.8;
/// How far inside its edge the liquid has its deeper colour, points.
pub const LIQUID_CORE_DEPTH: f32 = 9.0;
/// How fast the white bloom beside the lit rim fades with distance, points.
pub const LIQUID_BLOOM: f32 = 3.2;
/// How fast the soft glow around the liquid fades with distance, points.
pub const LIQUID_HALO: f32 = 7.0;
/// How far from the liquid anything of it is drawn, points.
pub const LIQUID_REACH: f32 = 22.0;
/// How long the light takes to travel along the liquid's rim once, seconds.
pub const LIQUID_LIGHT_PACE: f32 = 1.0;
/// How far to each side the travelling light reaches, as a share of the liquid's width.
pub const LIQUID_LIGHT_SHARE: f32 = 0.22;
/// The task list at the top of the chat panel ([`crate::tasklist`]): room above its header and under its rows.
pub const TASKS_PAD: f32 = 10.0;
/// How high the task list's header line is.
pub const TASKS_HEAD: f32 = 18.0;
/// The room between the header and the bar that shows how far the list is.
pub const TASKS_BAR_GAP: f32 = 9.0;
/// How thick that bar is.
pub const TASKS_BAR: f32 = 3.0;
/// The room between the bar and the first task.
pub const TASKS_ROWS_GAP: f32 = 8.0;
/// How high one task's row is.
pub const TASKS_ROW: f32 = 28.0;
/// How far a task's frame reaches past its row to each side.
pub const TASKS_ROW_REACH: f32 = 7.0;
/// The mark before a task: a ring, a tick, or the blob of the task at work.
pub const TASKS_MARK: f32 = 16.0;
/// The room between a task's mark and its words.
pub const TASKS_MARK_GAP: f32 = 9.0;
/// How fast the list's height follows what it shows (per second; larger is quicker).
pub const TASKS_GLIDE: f32 = 14.0;
/// How long the bar takes to reach a new length, seconds.
pub const TASKS_BAR_TIME: f32 = 0.45;
/// How long the pool of liquid gathers before the first drop falls, seconds.
pub const TASKS_POOL: f32 = 0.76;
/// How far the pool hangs under the list's top edge, points.
pub const TASKS_POUR_BAND: f32 = 18.0;
/// The shortest wait between two drops, and how much longer it may be, seconds.
pub const TASKS_POUR_GAP: f32 = 0.11;
/// See [`TASKS_POUR_GAP`].
pub const TASKS_POUR_GAP_MORE: f32 = 0.13;
/// The share of a drop's fall after which it has landed; the rest it lies flat.
pub const TASKS_POUR_LAND: f32 = 0.86;
/// How long a row's liquid takes to pour out and soak away, seconds.
pub const TASKS_POUR_SOAK: f32 = 0.58;
/// How long the small drop from a finished task to the next one runs, seconds.
pub const TASKS_STEP_TIME: f32 = 0.45;
/// How long a finished list's line takes to melt into one drop when the list moves into the conversation, seconds.
pub const TASKS_HAND_MELT: f32 = 0.25;
/// How long that drop falls to where the list is filed.
pub const TASKS_HAND_FALL: f32 = 0.3;
/// How long it takes to pour out there and soak away.
pub const TASKS_HAND_SOAK: f32 = 0.5;
/// How long after a task is finished the next one is shown at work, seconds.
pub const TASKS_STEP_NEXT: f32 = 0.3;
/// How long a finished task's mark bursts, seconds.
pub const TASKS_BURST: f32 = 0.7;
/// How long the frame of a task flares when the task comes to be at work, seconds.
pub const TASKS_FLARE: f32 = 0.8;
/// How long the light takes around the frame of the task at work, and around its blob, seconds.
pub const TASKS_FRAME_TURN: f32 = 2.8;
/// See [`TASKS_FRAME_TURN`].
pub const TASKS_BLOB_TURN: f32 = 1.5;
/// How long the blob of the task at work takes to swell and shrink once, and to go through its shapes, seconds.
pub const TASKS_BLOB_PULSE: f32 = 1.6;
/// See [`TASKS_BLOB_PULSE`].
pub const TASKS_BLOB_WARP: f32 = 3.6;
/// The most texels to a point the liquid is painted with, however fine the screen is.
pub const LIQUID_FINE: f32 = 1.5;
/// Size of the window a provider's models are chosen in, where the app's window has that much room.
pub const MODEL_PICKER_SIZE: [f32; 2] = [520.0, 580.0];
/// Height of one model's row in that window.
pub const MODEL_PICKER_ROW: f32 = 34.0;
/// How dark the backdrop behind that window is, out of 255; it lies over the settings' own backdrop.
pub const MODEL_PICKER_BACKDROP: u8 = 96;
/// Room kept free at the right of a settings row for its control, where the row's second line ends.
pub const SETTINGS_ROW_CONTROL_ROOM: f32 = 64.0;
/// Width of a name's text field at the right of a settings row.
pub const SETTINGS_NAME_FIELD: f32 = 200.0;
/// Width of the assistant menu.
pub const ASSISTANT_MENU_WIDTH: f32 = 300.0;
/// Height of one line of the assistant menu.
pub const ASSISTANT_MENU_ROW: f32 = 28.0;
/// The most lines the assistant menu shows before its list scrolls.
pub const ASSISTANT_MENU_ROWS: usize = 10;
/// Size of a resize handle on screen, points.
pub const HANDLE_SIZE: f32 = 8.0;
/// How near an edge of the selection box a press must be to resize by that edge.
pub const EDGE_GRAB: f32 = 4.0;
/// How near, in screen points, a dragged layer's edge or middle must be to another's to land on it.
pub const SNAP_REACH: f32 = 5.0;
/// The colour of the line through what a dragged layer lined up with.
pub const SNAP_GUIDE: Color32 = Color32::from_rgb(0xf4, 0x3f, 0x5e);
/// How far such a line runs on past what it joins, at each end.
pub const SNAP_GUIDE_OVER: f32 = 6.0;
/// How far the bar at each end of an equal gap's line reaches to either side of it.
pub const SNAP_GAP_BAR: f32 = 4.0;
/// The radius of the eyedropper's swatch beside the pointer, and how far its middle is from the pointer each way.
pub const EYEDROPPER_SWATCH: f32 = 11.0;
/// See [`EYEDROPPER_SWATCH`].
pub const EYEDROPPER_OFFSET: f32 = 20.0;
/// How far outside a corner of the selection box a press still turns the layer.
pub const ROTATE_REACH: f32 = 22.0;
/// Radius of the rotation handle's dot.
pub const ROTATE_DOT: f32 = 5.5;
/// How far the rotation handle's dot stands off the selection box.
pub const ROTATE_DOT_GAP: f32 = 26.0;
/// How near the rotation handle's dot a press must be to grab it.
pub const ROTATE_DOT_GRAB: f32 = 12.0;
/// Width of the round handle at each end of a selected connector.
pub const WIRE_END: f32 = 10.0;
/// Width of the square handle of a connector's waypoint.
pub const WIRE_POINT: f32 = 8.0;
/// Width of the dot in the middle of a connector's stretch, which adds a waypoint when pulled.
pub const WIRE_MID: f32 = 6.0;
/// How near a connector's handle or line a press must be to take it.
pub const WIRE_REACH: f32 = 8.0;
/// Width of a path point's dot while drawing with the pen.
pub const PEN_ANCHOR: f32 = 8.0;
/// Width of a path handle's square while drawing with the pen.
pub const PEN_HANDLE: f32 = 9.0;
/// How near the first point a press must be to close the path.
pub const PEN_REACH: f32 = 8.0;
/// How near a line of a path the pointer must be to take hold of it.
pub const PEN_STRETCH_REACH: f32 = 10.0;
/// Width of the find-layers island.
pub const FIND_WIDTH: f32 = 300.0;
/// Height of the find-layers field's row.
pub const FIND_BAR: f32 = 44.0;
/// Height of one match's line in find layers.
pub const FIND_ROW: f32 = 32.0;
/// How many matches show before the list scrolls.
pub const FIND_ROWS: u8 = 10;
/// Space above and below the matches.
pub const FIND_PADDING: f32 = 6.0;
/// Text size in find layers.
pub const FIND_TEXT: f32 = 12.0;
/// Corner radius of the find-layers island.
pub const FIND_RADIUS: f32 = 12.0;
/// Radius of a corner-radius handle's circle.
pub const RADIUS_HANDLE: f32 = 4.5;
/// How far inside its corner a corner-radius handle sits while the radius is smaller than that.
pub const RADIUS_HANDLE_INSET: f32 = 12.0;
/// The shortest side, on screen, a box must have for its corner-radius handles to show.
pub const RADIUS_HANDLE_ROOM: f32 = 56.0;
/// Text size of the size label under the selection.
pub const SIZE_LABEL_TEXT: f32 = 12.5;
/// Text size of a board's name above it on the canvas.
pub const FRAME_NAME_TEXT: f32 = 11.0;
/// A board narrower than this on screen shows no name, unless it is selected or under the pointer.
pub const FRAME_NAME_LEAST: f32 = 40.0;
/// Space between a board's top edge and its name.
pub const FRAME_NAME_GAP: f32 = 4.0;
/// How far the plate behind a whole name reaches past the text, sideways and up and down.
pub const FRAME_NAME_PLATE: egui::Vec2 = egui::vec2(3.0, 1.0);
/// Corner radius of the plate behind a whole name.
pub const FRAME_NAME_PLATE_RADIUS: f32 = 3.0;
/// How far the pill of a selected board's name reaches past the text; the size label's padding.
pub const FRAME_NAME_PILL: egui::Vec2 = egui::vec2(6.0, 2.5);
/// Corner radius of the pill of a selected board's name; the size label's.
pub const FRAME_NAME_PILL_RADIUS: f32 = 5.0;
/// Seconds a board's name takes to move to a new place and to turn into the pill or back.
pub const FRAME_NAME_MORPH: f32 = 0.22;

/// The colours of one appearance.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Panels and window (`--background`).
    pub background: Color32,
    /// Main text (`--foreground`).
    pub foreground: Color32,
    /// Secondary text (`--muted-foreground`).
    pub muted: Color32,
    /// Lines between areas (`--border`).
    pub border: Color32,
    /// Behind the design (`--canvas-bg`).
    pub canvas: Color32,
    /// Floating toolbar and popovers (`--island-bg`).
    pub island: Color32,
    /// Hovered and selected rows (`--accent`).
    pub hover: Color32,
    /// Input fields (`--input`).
    pub input: Color32,
    /// Primary button (`--primary`).
    pub primary: Color32,
    /// Text on the primary button (`--primary-foreground`).
    pub primary_text: Color32,
    /// Edges of islands and of the outlined buttons on them (the user's sidebar design).
    pub outline: Color32,
    /// The cards the settings window groups its rows in, a step lighter than the island they lie on.
    pub card: Color32,
    /// Text of the redesigned parts, the title bar first (`--rd-text` in `design/pencraft.pen`).
    pub strong: Color32,
    /// Their quiet text and icons (`--rd-muted`).
    pub soft: Color32,
    /// Their filled controls: the tab in front, a button under the pointer (`--rd-field`).
    pub field: Color32,
    /// Their thin lines (`--rd-line`).
    pub line: Color32,
    /// Text a step quieter than [`Palette::muted`] (a caption under a caption, a hint): still at least 4.5:1
    /// against the island, card and window background, unlike `muted` faded by opacity.
    pub quiet: Color32,
    /// The ring around a control reached with the keyboard: at least 3:1 against every surface of the
    /// appearance (WCAG 1.4.11).
    pub focus: Color32,
}

/// The light palette (`:root`).
pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    foreground: Color32::from_rgb(0x02, 0x02, 0x02),
    // Human: 4.98:1 on `input` and 5.22:1 on `field`; the original's #71717a was 3.8:1 and 4.0:1 there.
    muted: Color32::from_rgb(0x5f, 0x5f, 0x68),
    border: Color32::from_rgb(0xe4, 0xe4, 0xe7),
    canvas: Color32::from_rgb(0xf6, 0xf6, 0xf6),
    island: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    hover: Color32::from_rgb(0xf4, 0xf4, 0xf5),
    input: Color32::from_rgb(0xe4, 0xe4, 0xe7),
    primary: Color32::from_rgb(0x18, 0x18, 0x1b),
    primary_text: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    outline: Color32::from_rgb(0xe4, 0xe4, 0xe7),
    card: Color32::from_rgb(0xff, 0xff, 0xff),
    strong: Color32::from_rgb(0x18, 0x18, 0x1b),
    soft: Color32::from_rgb(0x55, 0x55, 0x5d),
    field: Color32::from_rgb(0xe9, 0xe9, 0xec),
    line: Color32::from_rgb(0xe4, 0xe4, 0xe8),
    quiet: Color32::from_rgb(0x6e, 0x6e, 0x77),
    focus: Color32::from_rgb(0x25, 0x63, 0xeb),
};

/// The dark palette (`.dark`).
pub const DARK: Palette = Palette {
    background: Color32::from_rgb(0x18, 0x18, 0x1b),
    foreground: Color32::from_rgb(0xc8, 0xc8, 0xc8),
    // Human: 4.69:1 on `input` and `hover`; the original's #a1a1aa was 4.07:1 there.
    muted: Color32::from_rgb(0xad, 0xad, 0xb5),
    border: Color32::from_rgb(0x20, 0x20, 0x20),
    canvas: Color32::from_rgb(0x18, 0x18, 0x1b),
    island: Color32::from_rgb(0x27, 0x27, 0x2a),
    hover: Color32::from_rgb(0x3f, 0x3f, 0x46),
    input: Color32::from_rgb(0x3f, 0x3f, 0x46),
    primary: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    primary_text: Color32::from_rgb(0x18, 0x18, 0x1b),
    outline: Color32::from_rgb(0x3a, 0x3a, 0x3e),
    card: Color32::from_rgb(0x2e, 0x2e, 0x32),
    strong: Color32::from_rgb(0xec, 0xec, 0xee),
    soft: Color32::from_rgb(0xb0, 0xb0, 0xb8),
    field: Color32::from_rgb(0x2c, 0x2c, 0x30),
    line: Color32::from_rgb(0x31, 0x31, 0x35),
    quiet: Color32::from_rgb(0x96, 0x96, 0x9e),
    focus: Color32::from_rgb(0x60, 0xa5, 0xfa),
};

/// The palette for an appearance.
#[must_use]
pub fn palette(dark: bool) -> Palette {
    if dark { DARK } else { LIGHT }
}

/// egui's visuals for an appearance, built from the palette.
#[must_use]
pub fn visuals(dark: bool) -> Visuals {
    let p = palette(dark);
    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = p.background;
    v.window_fill = p.island;
    v.extreme_bg_color = p.input.gamma_multiply(if dark { 0.5 } else { 0.35 });
    v.faint_bg_color = p.hover;
    v.override_text_color = Some(p.foreground);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;
    let radius = CornerRadius::same(CONTROL_RADIUS);
    for widget in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open, &mut v.widgets.noninteractive] {
        widget.corner_radius = radius;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.weak_bg_fill = p.input.gamma_multiply(if dark { 0.45 } else { 0.4 });
    v.widgets.inactive.bg_fill = v.widgets.inactive.weak_bg_fill;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = p.hover;
    v.widgets.hovered.bg_fill = p.hover;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, p.border);
    v
}

/// Width of the card that offers a newer version.
pub const UPDATE_WIDTH: f32 = 360.0;
/// Space between the window's edge and that card.
pub const UPDATE_MARGIN: f32 = 16.0;
/// Most height the release notes take in the card before they scroll.
pub const UPDATE_NOTES_HEIGHT: f32 = 220.0;
/// Space between the card's head, its notes and its buttons.
pub const UPDATE_GAP: f32 = 12.0;
/// Side of the square behind the card's icon.
pub const UPDATE_BADGE: f32 = 36.0;
/// Size of the card's title.
pub const UPDATE_TITLE_SIZE: f32 = 14.5;
/// Height of the card's buttons.
pub const UPDATE_BUTTON_HEIGHT: f32 = 30.0;
/// Space inside the card's buttons, left and right of their text.
pub const UPDATE_BUTTON_PADDING: f32 = 12.0;
/// Room a list item's dot takes before its text in the release notes.
pub const UPDATE_BULLET: f32 = 14.0;
/// Space between two paragraphs or list items of the release notes.
pub const UPDATE_NOTE_GAP: f32 = 6.0;
/// Thickness of the bar that shows how far a download is.
pub const UPDATE_BAR: f32 = 6.0;
/// Height of the pill the update card folds away into; its ends are half circles.
pub const UPDATE_PILL: f32 = 34.0;
/// Space between the pill's ends and what is in it.
pub const UPDATE_PILL_PADDING: f32 = 13.0;
/// How long the download bar takes to glide to where the download is, seconds: the download arrives in
/// pieces, the bar moves evenly.
pub const UPDATE_BAR_GLIDE: f32 = 0.35;
/// How long the light takes to run once over the bar's filled part, and the piece of a bar without a known
/// length once across, seconds.
pub const UPDATE_BAR_SWEEP: f32 = 1.5;
/// How much of the bar the travelling piece takes while the length isn't known, or the file is being checked.
pub const UPDATE_BAR_PIECE: f32 = 0.3;

/// Least space between the window's edges and the settings window, left and right, and top and bottom (the
/// original's settings dialog: `w-[calc(100%-2rem)]`, `h-[min(695px,calc(100vh-120px))]`).
pub const SETTINGS_MARGIN: [f32; 2] = [16.0, 60.0];
/// Largest size of the settings window (the original's: `max-w-[1100px]`, `h-[min(695px,…)]`).
pub const SETTINGS_MAX: [f32; 2] = [1100.0, 695.0];
/// Corner radius of the settings window (the original's settings dialog: `rounded-3xl`, 24 px).
pub const SETTINGS_RADIUS: f32 = 24.0;
/// Width of the list of pages at the settings window's left (the original's: `sm:w-[232px]`).
pub const SETTINGS_NAV_WIDTH: f32 = 232.0;
/// Height of a page's entry in that list.
pub const SETTINGS_NAV_ROW: f32 = 32.0;
/// Size of a settings page's title.
pub const SETTINGS_TITLE_SIZE: f32 = 20.0;
/// Height of a settings row with one line of text; one with more text is as tall as that asks for.
pub const SETTINGS_ROW: f32 = 46.0;
/// Height of a settings row that leads somewhere: a provider, a service.
pub const SETTINGS_LINK_ROW: f32 = 54.0;
/// Space between a settings card's left and right edges and what is in its rows.
pub const SETTINGS_ROW_PAD: f32 = 16.0;
/// Corner radius of a settings card.
pub const SETTINGS_CARD_RADIUS: f32 = 12.0;
/// Space above a card's heading.
pub const SETTINGS_GROUP_GAP: f32 = 26.0;
/// Most width of a settings page: a row's name and its control stay within one glance.
pub const SETTINGS_PAGE_WIDTH: f32 = 620.0;
/// Space between the list of pages and the page.
pub const SETTINGS_PAGE_INSET: f32 = 40.0;
/// Height of the bar under a settings form that holds its buttons.
pub const SETTINGS_FOOTER: f32 = 60.0;
/// Height of one choice of a segmented control.
pub const SETTINGS_SEGMENT: f32 = 26.0;
/// Size of an on/off switch.
pub const SWITCH_SIZE: [f32; 2] = [32.0, 18.0];
/// Side of the square in front of a row's name in the settings window.
pub const SETTINGS_TILE: f32 = 30.0;
/// Height of a tile to click in the settings' grid of providers.
pub const SETTINGS_TILE_BUTTON: f32 = 52.0;
/// How many providers the grid shows before "Show more".
pub const SETTINGS_PROVIDERS_SHOWN: usize = 8;
/// How many tools the Permissions page shows before "Show all".
pub const SETTINGS_TOOLS_SHOWN: usize = 6;
/// Height of a text field in the settings window.
pub const FIELD_HEIGHT: f32 = 30.0;
/// Colour of a provider that is connected (the design's green state pill).
pub const CONNECTED: Color32 = Color32::from_rgb(0x22, 0xc5, 0x5e);
/// Colour of a provider whose state is being checked (the design's amber state pill).
pub const CHECKING: Color32 = Color32::from_rgb(0xf5, 0x9e, 0x0b);
/// Width of the message box while it is a bar under the canvas (the sidebar folded away); less in a narrow window.
pub const MESSAGE_BAR_WIDTH: f32 = 620.0;
/// Height of that bar; its ends are half circles.
pub const MESSAGE_BAR_HEIGHT: f32 = 41.0;
/// Seconds the sidebar takes to fold away or come back, with the message box moving and changing shape.
pub const SIDEBAR_ANIMATION: f32 = 0.34;
/// The part of that time by which the message box has finished sliding to the left edge.
pub const SIDEBAR_SLIDE_END: f32 = 0.6;
/// The part of that time at which the islands start to grow. It is before [`SIDEBAR_SLIDE_END`], so the box is
/// already growing as it arrives and the motion never stands still.
pub const SIDEBAR_GROW_START: f32 = 0.3;
/// Space between the properties island's edge and its content (the user's properties design).
pub const PROPS_PADDING: f32 = 16.0;
/// Height of a field, button or title row in the properties island.
pub const PROPS_ROW: f32 = 24.0;
/// Corner radius of the properties island's fields and buttons.
pub const PROPS_RADIUS: f32 = 5.0;
/// Space between two fields side by side, and between two rows of fields.
pub const PROPS_GAP: f32 = 7.0;
/// Size of the properties island's text.
pub const PROPS_TEXT: f32 = 11.5;
/// Size of the properties island's small captions ("Dimensions", "Fill Width").
pub const PROPS_SMALL_TEXT: f32 = 10.5;
/// Side of an icon in the properties island.
pub const PROPS_ICON: f32 = 13.0;
/// Width of a tool menu opened from the rail (icon, name and shortcut, with room for the tick).
pub const RAIL_MENU_WIDTH: f32 = 200.0;
/// Height of one line of a tool menu.
pub const RAIL_MENU_ROW: f32 = 28.0;
/// Width of the action search.
pub const ACTIONS_WIDTH: f32 = 742.0;
/// Most height of the action search's list.
pub const ACTIONS_HEIGHT: f32 = 400.0;
/// Height of one action's line.
pub const ACTIONS_ROW: f32 = 36.0;
/// Distance of the action search from the top of the window.
pub const ACTIONS_TOP: f32 = 96.0;
/// Corner radius of the action search.
pub const ACTIONS_RADIUS: f32 = 14.0;
/// Width of a menu opened from the properties island, such as the kinds of effect.
pub const PROPS_MENU_WIDTH: f32 = 160.0;
/// The least room for typing in the message bar while a layer is selected and nothing is typed yet.
pub const MESSAGE_BAR_TYPING_ROOM: f32 = 150.0;
/// The least width of the message bar while a layer is selected: the layer's chip takes the place of the long hint.
pub const MESSAGE_BAR_SELECTED_WIDTH: f32 = 408.0;
/// Height of the selected layer's chip in the message bar.
pub const SELECTION_CHIP_HEIGHT: f32 = 21.0;
/// Height of the row of ideas under the message bar.
pub const IDEA_ROW: f32 = 30.0;
/// Space between the message bar and the row of ideas under it.
pub const IDEA_ROW_GAP: f32 = 12.0;
/// Size of the zoom island at the canvas's bottom right corner.
pub const ZOOM_ISLAND: [f32; 2] = [116.0, 30.0];
/// Size of the title of a small window over the app, such as "Generate image" (as "Settings" is drawn).
pub const DIALOG_TITLE_SIZE: f32 = 15.0;
/// How dark the backdrop behind a window over the app is, out of 255: the settings, "Generate image".
pub const DIALOG_BACKDROP: u8 = 140;
/// Space between the edge of a small window over the app and its content.
pub const DIALOG_PADDING: i8 = 20;
/// Space between the parts of a small window over the app: its title, its fields, its buttons.
pub const DIALOG_GAP: f32 = 12.0;
/// Least width of a button in a small window over the app (as the settings' forms' main button).
pub const DIALOG_BUTTON: f32 = 92.0;
/// Space between a text field's outline and its text, across and down (as in the settings' fields).
pub const FIELD_MARGIN: [i8; 2] = [11, 7];
/// Width of the "Generate image" window's content.
pub const GENERATE_WIDTH: f32 = 440.0;
/// Lines of text the description field of the "Generate image" window shows.
pub const GENERATE_LINES: usize = 4;
/// Side of a picture in the "Generate image" window's row of pictures (as the app's other small pictures).
pub const PIECE_SIDE: f32 = 64.0;
/// Side, in pixels, a small picture in that row is made at: twice its side, so it stays sharp on a fine screen.
pub const PIECE_PIXELS: u32 = 128;
/// Space between two pictures of that row.
pub const PIECE_GAP: f32 = 8.0;
/// Size of a picture's name and role under it in that row.
pub const PIECE_TEXT: f32 = 11.5;
/// Side of the cross that takes a picture out of that row.
pub const PIECE_CROSS: f32 = 18.0;
/// Height of the "Change this" / "Reference" toggle under a picture of that row.
pub const PIECE_TOGGLE: f32 = 20.0;
/// Space across between the edge of that toggle's chip and its words.
pub const PIECE_CHIP_PADDING: f32 = 6.0;
/// How far the cross that takes a picture out of that row reaches past the picture's top right corner.
pub const PIECE_CROSS_OUT: f32 = 6.0;
/// Words on the accent colour, such as a picture's "Change this" while it is marked.
pub const ON_ACCENT: Color32 = Color32::WHITE;
/// Width of the Cancel button that takes the place of the quick field's button while its picture is made.
pub const PROPS_CANCEL: f32 = 56.0;

/// Most width of the home screen's column; in a narrower window it takes what there is.
pub const HOME_WIDTH: f32 = 920.0;
/// Space between the window's edge and the home screen's column, left and right.
pub const HOME_MARGIN: f32 = 28.0;
/// Space above the home screen's headline, and under its last line.
pub const HOME_TOP: f32 = 44.0;
/// Size of the home screen's headline.
pub const HOME_TITLE_SIZE: f32 = 27.0;
/// Space between the headline and the box a design is described in.
pub const HOME_TITLE_GAP: f32 = 18.0;
/// Size of the text typed into that box.
pub const HOME_PROMPT_TEXT: f32 = 14.5;
/// Diameter of that box's round send button.
pub const HOME_SEND: f32 = 26.0;
/// Size of the home screen's small text: a card's second line, the assistant's name, the shortcuts.
pub const HOME_SMALL_TEXT: f32 = 11.5;
/// Size of a card's name on the home screen.
pub const HOME_CARD_TEXT: f32 = 13.0;
/// Space above a section's title on the home screen ("Start from", "Recent").
pub const HOME_SECTION_GAP: f32 = 38.0;
/// Space between a section's title and its cards.
pub const HOME_SECTION_TITLE_GAP: f32 = 12.0;
/// Space between two cards of the home screen, across and down.
pub const HOME_CARD_GAP: f32 = 12.0;
/// Corner radius of a card on the home screen.
pub const HOME_CARD_RADIUS: f32 = 12.0;
/// Space between a card's edge and its picture and text.
pub const HOME_CARD_PADDING: f32 = 8.0;
/// Least width of a card to start a design from; a row holds as many as fit.
pub const HOME_START_WIDTH: f32 = 150.0;
/// Height of the picture on such a card.
pub const HOME_START_PICTURE: f32 = 92.0;
/// Least width of a recent file's card.
pub const HOME_RECENT_WIDTH: f32 = 200.0;
/// Height of a recent file's picture, as a share of its width.
pub const HOME_RECENT_SHAPE: f32 = 0.6;
/// Height of the two lines of text under a card's picture, with the space around them.
pub const HOME_CARD_LINES: f32 = 50.0;
/// Space between a card's picture and what is drawn in it.
pub const HOME_PICTURE_INSET: f32 = 10.0;
/// The most room, in points across and down, a recent file's small picture is made for: the largest a card's
/// picture gets, in a window one card wide.
pub const HOME_THUMB_SIZE: [f32; 2] = [400.0, 240.0];
/// The most points a unit of the design takes in that picture, so a small design isn't blown up.
pub const HOME_THUMB_SCALE: f32 = 2.0;
/// Height of the note the home screen shows while there are no recent files.
pub const HOME_EMPTY_HEIGHT: f32 = 112.0;
/// Height of a shortcut's key on the home screen's last line.
pub const HOME_KEY_HEIGHT: f32 = 20.0;

/// The space the welcome film is drawn in; it is fitted into the window whole.
pub const SPLASH_STAGE: egui::Vec2 = egui::vec2(1280.0, 800.0);
/// Seconds the welcome film runs.
pub const SPLASH_LENGTH: f32 = 9.6;
/// Seconds into the welcome film at which its two buttons are there; skipping goes here.
pub const SPLASH_READY: f32 = 8.95;
/// The film's dark ground, and its text on the light one.
pub const SPLASH_INK: Color32 = Color32::from_rgb(0x0b, 0x0b, 0x0c);
/// The film's light ground, and its text on the dark one.
pub const SPLASH_PAPER: Color32 = Color32::from_rgb(0xf4, 0xf2, 0xec);
/// Islands on the film's dark ground (the message box, the toolbar).
pub const SPLASH_INK_ISLAND: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1f);
/// Edges and unlit marks on the film's dark ground.
pub const SPLASH_INK_EDGE: Color32 = Color32::from_rgb(0x34, 0x34, 0x3a);
/// Quiet text on the film's dark ground.
pub const SPLASH_INK_MUTED: Color32 = Color32::from_rgb(0x8a, 0x8a, 0x92);
/// Edges on the film's light ground.
pub const SPLASH_PAPER_EDGE: Color32 = Color32::from_rgb(0xde, 0xdb, 0xd2);
/// Quiet text on the film's light ground.
pub const SPLASH_PAPER_MUTED: Color32 = Color32::from_rgb(0x8b, 0x87, 0x7c);
/// Marks not reached yet on the film's light ground.
pub const SPLASH_PAPER_FAINT: Color32 = Color32::from_rgb(0xcb, 0xc7, 0xbb);
/// The page drawn in the film, in its dark theme.
pub const SPLASH_PAGE_DARK: Color32 = Color32::from_rgb(0x13, 0x13, 0x16);
/// Quiet text of that page: on its light theme, and on its dark one.
pub const SPLASH_PAGE_MUTED: [Color32; 2] = [Color32::from_rgb(0x6b, 0x6b, 0x6b), Color32::from_rgb(0x9a, 0x9a, 0xa2)];
/// The fills the page's picture is given in turn while it is restyled; the first one stays.
pub const SPLASH_FILLS: [Color32; 2] = [Color32::from_rgb(0xf9, 0x73, 0x16), Color32::from_rgb(0x16, 0xa3, 0x4a)];
/// What lies under each of the page's four covers until its picture is there.
pub const SPLASH_COVERS: [Color32; 4] = [
    Color32::from_rgb(0xc8, 0x55, 0x3d),
    Color32::from_rgb(0x2f, 0x5d, 0x50),
    Color32::from_rgb(0xe3, 0xb2, 0x3c),
    Color32::from_rgb(0x28, 0x33, 0x4a),
];
/// Size of the film's first line ("Describe it."), in the stage's points.
pub const SPLASH_TITLE: f32 = 184.0;
/// Size of its two other lines.
pub const SPLASH_HEADING: f32 = 124.0;
/// Size of the name at its end.
pub const SPLASH_NAME: f32 = 140.0;
