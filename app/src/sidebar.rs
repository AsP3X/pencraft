//! The left sidebar and the tool rail beside it, after the user's sidebar design.
//!
//! The sidebar is two rounded islands: a panel with a row of view buttons (chat history, chat, layers and
//! more) over the chosen view, and the message box under it. The rail is a column of round islands with the
//! canvas tools; it stays when the sidebar is folded away.

use std::path::PathBuf;

use egui::{Align, Align2, Color32, FontId, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};
use pen_doc::model::Node;

use crate::chat::{Chat, Choice};
use crate::chips::Chip;
use crate::editor::{Editor, Tool};
use crate::icons;
use crate::tasklist::{tint as blend, turned};
use crate::theme::{
    self, BAR_IDEA_GAP, BAR_IDEA_LEAST, BUTTON_ICON_GAP, CHAT_ACCENT, CHAT_ASK_WIDTH, CHAT_SPENT_PAD, CHAT_SPENT_PILL, CHAT_SPENT_STRIP,
    CHAT_START_DROP, CHAT_START_DROP_RANGE, CHAT_START_HEAD_GAP, CHAT_START_IDEA_GAP, CHAT_START_SECTION_GAP, CHAT_STATUS_GAP, CHAT_STATUS_STAY,
    CHIP_CORNER, CHIP_HEIGHT, CHIP_ICON, CHIP_ICON_AT, CHIP_PADDING, CHIP_RADIUS, CHIP_THUMB, COMPONENT, COMPOSER_CONTROLS, COMPOSER_GAP,
    COMPOSER_GROW_TIME, COMPOSER_HEIGHT, COMPOSER_LIGHT_SHARE, COMPOSER_LIGHT_TURN, COMPOSER_LIGHT_WHITE, COMPOSER_LIGHT_WIDTH, COMPOSER_ROWS_MOST,
    COMPOSER_TEXT_GAP, COMPOSER_TEXT_TOP, FOCUS_RING, HOVER_FAINT, ICON_SIZE, IDEA_ROW, IDEA_ROW_GAP, IDLE_FRAME, ISLAND_GAP, ISLAND_PADDING,
    ISLAND_RADIUS, MENU_GAP, MESSAGE_BAR_ENDS, MESSAGE_BAR_GAP, MESSAGE_BAR_HEIGHT, MESSAGE_BAR_INSET, MESSAGE_BAR_SELECTED_WIDTH,
    MESSAGE_BAR_TYPING_ROOM, MESSAGE_BAR_WIDTH, MESSAGE_FIELD_DROP, MESSAGE_SEND, MESSAGE_SEND_TIME, MESSAGE_SEND_TURN, MESSAGE_TEXT_SIZE,
    MODEL_BUTTON_ARROW, MODEL_BUTTON_HEIGHT, MODEL_BUTTON_ROOM, OUTLINE_SOFT, Palette, RAIL_ACTIVE, RAIL_ARROW_STEP, RAIL_FOCUS, RAIL_MENU_ARROW,
    RAIL_MENU_AT, RAIL_MENU_ICON, RAIL_MENU_PLATE, RAIL_MENU_RADIUS, RAIL_MENU_ROW, RAIL_MENU_TICK, RAIL_MENU_WIDTH, RAIL_STEP, RAIL_WIDTH,
    RULE_FADE, SELECTION_CHIP, SELECTION_CHIP_HEIGHT, SEND_ARROW, SEND_REST, SIDEBAR_ANIMATION, SIDEBAR_BUTTONS, SIDEBAR_GROW_START,
    SIDEBAR_HEADER_PADDING, SIDEBAR_HEADER_TOP, SIDEBAR_LIST_TOP, SIDEBAR_NOTE_TOP, SIDEBAR_ROW, SIDEBAR_RULE_GAP, SIDEBAR_SLIDE_END,
    SIDEBAR_TEXT_SIZE, SIDEBAR_TITLE_GAP, SIDEBAR_TITLE_LEFT, SIDEBAR_TITLE_SIZE, SIDEBAR_VIEW_GAP, SIDEBAR_WIDE_WIDTH, SIDEBAR_WIDTH, STOP_RING,
    STOP_SQUARE, TARGET_MIN, ZOOM_ISLAND,
};

/// Said when a chat control is used: the chat has no assistant behind it in this version.
/// What the user is told when a pasted picture can't be kept for the message.
const PICTURE_NOT_TAKEN: &str = "Couldn't paste the picture into the message.";
/// What the user is told when a picture is pasted into a message that carries all it can.
const TOO_MANY_FILES: &str = "A message can carry 10 files.";
/// What the user is told when pasted layers are from another document.
const NOT_IN_DOCUMENT: &str = "Those layers aren't in this document, so the assistant can't be pointed at them.";
/// The grey text in the empty message box.
const HINT: &str = "Create, plan or research... @ to attach, / for skills";
/// The assistant named in the message box.
/// The icon of the button that deals another handful of ideas.
const SHUFFLE_ICON: &str = "shuffle";

/// Ideas offered under the message bar while the sidebar is folded away; the new chat's ideas follow them.
const BAR_IDEAS: [&str; 3] = [
    "Design a control panel for a humanoid robotics factory floor",
    "Compare how Linear and Jira triage a bug, with the workflows side by side",
    "Design a warm, minimal booking site for a Lisbon coworking space",
];

/// What the sidebar's panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    /// Earlier chats.
    History,
    /// A new chat.
    Chat,
    /// The document's layers.
    Layers,
    /// Presentations.
    Present,
    /// The document's components.
    Components,
    /// Shared libraries.
    Libraries,
}

impl View {
    /// The view called `name` on the command line (`--sidebar`); `hidden` is not a view and returns `None`.
    #[must_use]
    pub fn named(name: &str) -> Option<View> {
        match name {
            "history" => Some(View::History),
            "chat" => Some(View::Chat),
            "layers" => Some(View::Layers),
            "present" => Some(View::Present),
            "components" => Some(View::Components),
            "libraries" => Some(View::Libraries),
            _ => None,
        }
    }

    fn title(self) -> &'static str {
        match self {
            View::History => "Chat history",
            View::Chat => "New chat",
            View::Layers => "Layers",
            View::Present => "Presentation",
            View::Components => "Components",
            View::Libraries => "Libraries",
        }
    }
}

/// What the settings say about how the sidebar looks.
#[derive(Clone, Copy)]
pub struct Look<'a> {
    /// Dark instead of light.
    pub dark: bool,
    /// Show the row of ideas under the message bar while nothing is typed.
    pub ideas: bool,
    /// How much larger than its small size the conversation's text is.
    pub chat_text: f32,
    /// The chat the panel shows: the one of the document in front.
    pub chat: &'a Chat,
    /// That chat's id, by which what moves in its conversation is remembered.
    pub chat_id: u64,
    /// The assistants the user can pick from.
    pub assistants: &'a [Choice],
    /// The name of the assistant the chat talks to.
    pub assistant: &'a str,
    /// Things move ("Animations & effects" in the settings).
    pub animate: bool,
}

/// A kept chat as the History view lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PastRow {
    /// Its file, by which the app opens or deletes it.
    pub file: PathBuf,
    /// The first thing the user said in it.
    pub title: String,
    /// Its document's name and when it was last added to ("shop.quill · 2 days ago").
    pub note: String,
    /// It is the chat the panel shows.
    pub current: bool,
}

/// The sidebar's state for one window.
pub struct Sidebar {
    /// The view in the panel.
    pub view: View,
    /// The kept chats the History view lists, newest first. `None` when they have to be read (again): the
    /// app fills them in while the History view shows.
    pub past: Option<Vec<PastRow>>,
    /// Folded away: only the tool rail shows.
    pub collapsed: bool,
    /// Widened with the new chat's expand button.
    wide: bool,
    /// The text in the message box.
    draft: String,
    /// How far open the sidebar is held, for a picture of the animation's middle; `None` lets it run.
    frozen: Option<f32>,
    /// What is selected in the document in front, as the message bar's chip shows it; a sent message carries it.
    selected: Option<(&'static str, String)>,
    /// What the user asked of the chat this frame, for the app to take.
    pub chat_asks: Vec<ChatAsk>,
    /// The files attached to the message being written.
    pub attached: Vec<PathBuf>,
    /// The layers pasted into the message box, each standing in the text as its token.
    chips: Vec<Chip>,
    /// A picture asked for the selection as a pasted chip in a sentence (`--sidebar chip`); made once the
    /// document is laid out.
    posed_chip: bool,
    /// The task list's motion is held at this many seconds into it, for a picture of it (`--sidebar pouring`).
    posed_tasks: Option<f32>,
    /// The ideas an empty chat offers right now ([`crate::ideas`]), and the chat they were picked for.
    ideas: Vec<&'static str>,
    ideas_for: Option<u64>,
    /// What the ideas are picked by when the sidebar is set up for a picture, so they are the same every
    /// time; `None` picks afresh.
    ideas_seed: Option<u64>,
    /// The assistant menu of the message box, while it is open.
    assistants: crate::assistants::Menu,
    /// Whether the assistant was at work the last time the sidebar was drawn, and when it last finished (in
    /// egui's time), for the status line over the message bar.
    chat_ended: (bool, Option<f64>),
}

/// What is being written in one chat, and what its status line remembers: the part of the sidebar that belongs
/// to a chat. The app keeps it for a chat while another one is in front.
#[derive(Debug, Default, PartialEq)]
pub struct Writing {
    draft: String,
    attached: Vec<PathBuf>,
    chips: Vec<Chip>,
    chat_ended: (bool, Option<f64>),
}

impl Writing {
    /// Whether nothing is written or attached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.draft.is_empty() && self.attached.is_empty()
    }
}

/// Something the user asked of the chat.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatAsk {
    /// Send this message: as the panel shows it, and as the assistant is told it.
    Send {
        /// With each pasted layer as its name.
        shown: String,
        /// The layers it points at: what was selected, and where each pasted layer's name stands in `shown`.
        pointed: Vec<crate::chat::Pointed>,
        /// With each pasted layer as its id and name.
        told: String,
        /// The pictures pasted into it, in the order they stand in it: each one's name and its file.
        pasted: Vec<(String, PathBuf)>,
        /// The files attached to it.
        files: Vec<PathBuf>,
    },
    /// Ask for files to attach to the next message.
    Attach,
    /// Stop the assistant.
    Stop,
    /// Start a new chat.
    New,
    /// Show the kept chat in this file in the chat panel, to go on with it.
    OpenPast(PathBuf),
    /// Delete the kept chat in this file (the app asks first).
    DeletePast(PathBuf),
    /// The assistant menu was opened: look again for what is installed.
    Looked,
    /// Talk to the assistant with this id from now on.
    Pick(String),
    /// Open the settings where assistants are set up.
    Assistants,
    /// Take what the user did with a question of the assistant's: an answer picked, or the question closed.
    Answer(crate::conversation::Pick),
    /// Take what the user did with the next steps the assistant offers: one sent, all sent, or one dropped.
    Offer(crate::offers::Act),
}

impl ChatAsk {
    /// Sending the plain words `text` with the files `files`: a message that points at nothing.
    #[must_use]
    pub fn said(text: String, files: Vec<PathBuf>) -> Self {
        ChatAsk::Send { shown: text.clone(), pointed: Vec::new(), told: text, pasted: Vec::new(), files }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        // Human: the sidebar starts folded away and stays so until the user opens it.
        Sidebar {
            view: View::Chat,
            past: None,
            collapsed: true,
            wide: false,
            draft: String::new(),
            frozen: None,
            selected: None,
            chat_asks: Vec::new(),
            attached: Vec::new(),
            chips: Vec::new(),
            posed_chip: false,
            posed_tasks: None,
            ideas: Vec::new(),
            ideas_for: None,
            ideas_seed: None,
            assistants: crate::assistants::Menu::default(),
            chat_ended: (false, None),
        }
    }
}

/// What a line of a tool menu does.
#[derive(Clone, Copy)]
enum Does {
    /// Chooses a tool.
    Tool(Tool),
    /// Asks for a picture or SVG file to add.
    Import,
}

/// One line of a tool menu: icon, name, shortcut key, and what it does.
type Entry = (&'static str, &'static str, &'static str, Does);

/// The pointer tools' menu: Move and Scale.
const POINTER_MENU: [Entry; 3] = [
    ("mouse-pointer-2", "Move", "V", Does::Tool(Tool::Move)),
    ("scaling", "Scale", "K", Does::Tool(Tool::Scale)),
    ("pipette", "Eyedropper", "I", Does::Tool(Tool::Eyedropper)),
];
/// The shapes' menu. (The design's "Import Figma" and "Script" stay out until they can be done.)
const SHAPE_MENU: [Entry; 5] = [
    ("square", "Rectangle", "R", Does::Tool(Tool::Rectangle)),
    ("circle", "Ellipse", "O", Does::Tool(Tool::Ellipse)),
    ("triangle", "Polygon", "", Does::Tool(Tool::Polygon)),
    ("sparkle", "Icon", "", Does::Tool(Tool::Icon)),
    ("image", "Import Image or SVG…", "", Does::Import),
];

/// What the rail asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Ask for a picture or SVG file and add it to the document.
    Import,
    /// Open the action search.
    Actions,
    /// Open find layers.
    Find,
}

/// What one entry of the tool rail does.
#[derive(Clone, Copy)]
enum Act {
    /// Chooses a tool.
    Pick(Tool),
    /// Chooses the pointer tool used last, Move or Scale.
    Pointer,
    /// Chooses the shape tool used last.
    Shape,
    /// Opens a menu of tools.
    Menu(&'static [Entry]),
    /// Makes the selection a component.
    Component,
    /// Opens find layers.
    Find,
}

/// The tool rail's entries from top to bottom: icon, hint, what it does. (The design's Browser and Comment
/// stay out until they can be done.)
const RAIL: [(&str, &str, Act); 12] = [
    ("mouse-pointer-2", "Move (V), Scale (K), Eyedropper (I)", Act::Pointer),
    ("chevron-down", "Move tools", Act::Menu(&POINTER_MENU)),
    ("square", "Shapes", Act::Shape),
    ("chevron-down", "Shapes", Act::Menu(&SHAPE_MENU)),
    ("pen-tool", "Pen (P)", Act::Pick(Tool::Pen)),
    ("spline", "Connector (C)", Act::Pick(Tool::Connector)),
    ("type", "Text (T)", Act::Pick(Tool::Text)),
    ("frame", "Frame (F)", Act::Pick(Tool::Frame)),
    ("sticky-note", "Sticky note (N)", Act::Pick(Tool::Note)),
    ("hand", "Hand (H)", Act::Pick(Tool::Hand)),
    ("component", "Create component (⌥⌘K)", Act::Component),
    ("search", "Find layers (⌘F)", Act::Find),
];

impl Sidebar {
    /// Width of the sidebar island.
    #[must_use]
    pub fn width(&self) -> f32 {
        if self.wide { SIDEBAR_WIDE_WIDTH } else { SIDEBAR_WIDTH }
    }

    /// Sets the sidebar up for a picture (`--sidebar`): a view's name, `hidden` for the folded sidebar, or
    /// `moving` / `morphing` for the two halves of the animation between them.
    pub fn pose(&mut self, name: &str) {
        self.collapsed = name == "hidden";
        self.frozen = match name {
            "moving" => Some(0.15),
            "morphing" => Some(0.8),
            _ => None,
        };
        self.posed_chip = name == "chip";
        self.ideas_seed = Some(0);
        // Human: `writing` shows the message box grown for a message of several lines.
        if name == "writing" {
            "Now create the view for a single picture. It needs a design that fits the album and the library, with the \
             same glass bars, a close button at the top left and the date under the picture, and it must not look generic."
                .clone_into(&mut self.draft);
        }
        // Human: the task list in the middle of what it does: pooling, dropping its rows, stepping on.
        self.posed_tasks = match name {
            "pooling" => Some(0.62),
            "pouring" | "adding" => Some(1.12),
            "handing" => Some(0.42),
            "landing" => Some(0.78),
            "stepping" => Some(0.2),
            _ => None,
        };
        if name == "assistants" {
            self.assistants.hold_open();
        }
        let chat =
            matches!(name, "assistants" | "tasks" | "pooling" | "pouring" | "stepping" | "finished" | "adding" | "handing" | "landing" | "writing");
        self.view = if chat { View::Chat } else { View::named(name).unwrap_or(self.view) };
    }

    /// How far open the sidebar is right now: 0 folded away, 1 open. It glides between the two when `animate`.
    pub fn openness(&self, ctx: &egui::Context, animate: bool) -> f32 {
        let time = if animate { SIDEBAR_ANIMATION } else { 0.0 };
        self.frozen.unwrap_or_else(|| ctx.animate_bool_with_time(egui::Id::unique("sidebar-open"), !self.collapsed, time))
    }

    /// The width the sidebar takes from the canvas at `openness`: nothing while the bar slides, then growing
    /// with the islands.
    #[must_use]
    pub fn room(&self, openness: f32) -> f32 {
        (self.width() + ISLAND_GAP) * grown(openness)
    }

    /// Draws the sidebar's two islands over `area` (the window under the top bar, from its left edge to the
    /// canvas's right edge) at `openness`. Returns a sentence to tell the user, if a control asks for one.
    // Human: folded away, the sidebar is only the message box, a bar under the middle of the canvas. Opening,
    // the bar first slides to the left edge; then it grows into the box under the sidebar while the panel
    // island rises out of its top edge. Folding runs the same backwards.
    // Agent: MAIN-THREAD; both islands are floating layers over the canvas; WRITES editor.selection and
    // editor.doc through the layers view.
    pub fn show(&mut self, ctx: &egui::Context, area: Rect, openness: f32, editor: &mut Editor, look: Look<'_>) -> Option<&'static str> {
        let dark = look.dark;
        let palette = theme::palette(dark);
        let mut notice = None;
        self.deal_ideas(look.chat_id, look.chat.entries.is_empty());
        if self.posed_chip
            && let Some(chip) = crate::chips::chip(editor, &editor.selection, "")
        {
            self.posed_chip = false;
            self.draft = format!("Make {} a little wider and give it a soft shadow", chip.token);
            self.chips = vec![chip];
        }
        // Human: folded away, the bar stands in the middle of `area`, with or without a selection. While a
        // layer is selected its chip takes the place of the long hint, so the bar is narrower.
        let chip = selection_chip(editor);
        self.selected.clone_from(&chip);
        let wanted = self.bar_width(ctx, chip.as_ref(), look.assistant);
        let space = area;
        let width = wanted.min(space.width() - 2.0 * ISLAND_GAP);
        let bottom = area.bottom() - ISLAND_GAP;
        // Human: with nothing typed yet and nothing drawn yet a row of ideas stands under the bar, which makes room
        // for it, unless the settings turn the ideas off. A document with content shows none.
        let ideas = look.ideas && self.draft.is_empty() && editor.scene.is_empty();
        let bar_bottom = if ideas { bottom - IDEA_ROW - IDEA_ROW_GAP } else { bottom };
        let bar = Rect::from_min_size(pos2(space.center().x - width / 2.0, bar_bottom - MESSAGE_BAR_HEIGHT), vec2(width, MESSAGE_BAR_HEIGHT));
        // Human: the message box grows with what is written in it, and the panel over it gives way.
        let tall = self.composer_height(ctx, look.animate);
        let docked = Rect::from_min_size(pos2(area.left() + ISLAND_GAP, bottom - tall), vec2(self.width(), tall));
        let pose = Pose::at(openness, bar, docked);
        // Human: what an answer takes counts up in a pill between the conversation and the message box while
        // the assistant works; the panel makes room for it, a little at a time. When the answer is there, its
        // sum stands under it in the conversation. Folded away, the pill over the bar keeps the last sum.
        let counted = crate::conversation::counter_size(ctx, look.chat, false);
        let counting = crate::conversation::counter_size(ctx, look.chat, true);
        let strip = ctx.animate_bool(egui::Id::unique("chat-counter"), counting.is_some()) * CHAT_SPENT_STRIP;
        if pose.rect.width() < 4.0 * ISLAND_PADDING || pose.rect.top() < area.top() {
            return None;
        }
        // Human: in a very short window the message box keeps its place and the panel gives way.
        let full = Rect::from_min_max(pos2(docked.left(), area.top() + ISLAND_GAP), pos2(docked.right(), docked.top() - ISLAND_GAP - strip));
        let panel = pose.above(full, strip);
        if full.height() > 2.0 * SIDEBAR_ROW + 4.0 * ISLAND_PADDING && panel.height() > 2.0 {
            egui::Area::new(egui::Id::unique("sidebar-panel")).fade_in(false).fixed_pos(panel.min).order(egui::Order::Foreground).show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(panel.size(), Sense::hover());
                island(ui, rect, pose.radius.min(rect.height() / 2.0), palette);
                // Human: the content is laid out for the finished island and shows through the growing one, so
                // nothing reflows on the way; it fades in over the second half of the growing.
                if pose.boxed {
                    ui.scope_builder(UiBuilder::new().max_rect(full), |ui| {
                        ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                        ui.multiply_opacity(pose.opacity);
                        self.panel(ui, full, editor, look);
                    });
                }
            });
        }
        egui::Area::new(egui::Id::unique("message-box")).fade_in(false).fixed_pos(pose.rect.min).order(egui::Order::Foreground).show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(pose.rect.size(), Sense::hover());
            island(ui, rect, pose.radius, palette);
            // Human: the box being typed in has a stronger edge.
            if pose.boxed && ctx.memory(|memory| memory.has_focus(egui::Id::unique("message-field"))) {
                ui.painter().rect_stroke(rect, pose.radius, (1.0, palette.muted.gamma_multiply(OUTLINE_SOFT * pose.opacity)), StrokeKind::Inside);
            }
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                ui.set_clip_rect(rect.intersect(ui.clip_rect()));
                ui.multiply_opacity(pose.opacity);
                if pose.boxed {
                    self.composer(ui, rect, editor, look, &mut notice);
                } else {
                    self.message_bar(ui, rect, chip.as_ref(), editor, look, &mut notice);
                }
            });
        });
        let slot = Rect::from_min_max(pos2(docked.left(), full.bottom()), pos2(docked.right(), docked.top()));
        if let Some(words) = counting.filter(|_| pose.boxed) {
            counter_strip(ctx, slot, words.x, look.chat, palette, pose.opacity * strip / CHAT_SPENT_STRIP);
        }
        // Human: the ideas are there only with the sidebar folded; they fade as it starts to open.
        let fade = 1.0 - (openness * 5.0).clamp(0.0, 1.0);
        // Human: folded away, the conversation can't be seen. One line over the bar then says what the assistant
        // is doing, and for a while after that its answer is there; a click on it opens the chat.
        let now = ctx.input(|input| input.time);
        let running = look.chat.running();
        self.chat_ended = match (self.chat_ended, running) {
            (_, true) => (true, None),
            ((true, _), false) => (false, Some(now)),
            (kept, false) => kept,
        };
        let since = self.chat_ended.1.map_or(f64::INFINITY, |ended| now - ended);
        if since < CHAT_STATUS_STAY {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(CHAT_STATUS_STAY - since));
        }
        if fade > 0.0 {
            // Human: folded away, the counter is a small pill right over the bar, where it stays in sight; the
            // line saying what the assistant is doing stands over the pill.
            let over_bar = counted.map_or(bar.top(), |words| counter_pill(ctx, bar, words.x, look.chat, palette, fade)) - CHAT_STATUS_GAP;
            let foot = pos2(bar.center().x, over_bar);
            // Human: a question of the assistant's is a dialog in the same place, as wide as the window allows.
            let ask_width = (space.width() - 2.0 * ISLAND_GAP).min(CHAT_ASK_WIDTH);
            let line = crate::conversation::Line { foot, most_width: bar.width(), ask_width, fresh: since < CHAT_STATUS_STAY, opacity: fade };
            match crate::conversation::status_line(ctx, &line, look.chat, palette, look.animate) {
                Some(crate::conversation::Tap::Open) => {
                    self.collapsed = false;
                    self.view = View::Chat;
                }
                Some(crate::conversation::Tap::Pick(pick)) => self.chat_asks.push(ChatAsk::Answer(pick)),
                None => {}
            }
        } else {
            // The open chat shows the answer itself.
            self.chat_ended.1 = None;
        }
        if ideas && fade > 0.0 {
            let row =
                Rect::from_min_max(pos2(area.left() + ISLAND_GAP, bottom - IDEA_ROW), pos2(area.right() - 2.0 * ISLAND_GAP - ZOOM_ISLAND[0], bottom));
            self.idea_row(ctx, row, fade, palette);
        }
        self.offers(ctx, area.right(), (pose.rect, pose.boxed), look);
        notice
    }

    /// The next steps the assistant offers, collected beside the message box `spot` (its rectangle, and whether
    /// it is the box under the open sidebar), wherever it stands; `edge` is where the room for them ends.
    // Human: the pills keep clear of what stands to the right of them (the properties, the window's edge).
    fn offers(&mut self, ctx: &egui::Context, edge: f32, spot: (Rect, bool), look: Look<'_>) {
        let (anchor, beside) = spot;
        let room = edge - anchor.right() - 2.0 * ISLAND_GAP;
        let offers = crate::offers::Look { anchor, palette: theme::palette(look.dark), animate: look.animate, beside, room };
        self.chat_asks.extend(crate::offers::show(ctx, look.chat.offers(), look.chat_id, offers).map(ChatAsk::Offer));
    }

    /// How wide the folded message bar wants to be, with the selection's `chip` in it and `assistant` named.
    // Human: attached files take room too: with them the bar keeps its full width. The narrower bar still has to
    // leave room to type beside the chip and the assistant's name, which can be long; and once something is
    // typed, or the field is being typed in, the bar takes its full width.
    fn bar_width(&self, ctx: &egui::Context, chip: Option<&(&'static str, String)>, assistant: &str) -> f32 {
        let Some((_, name)) = chip.filter(|_| self.attached.is_empty()) else { return MESSAGE_BAR_WIDTH };
        let typing = !self.draft.is_empty() || ctx.memory(|memory| memory.has_focus(egui::Id::unique("message-field")));
        let measure =
            |text: String, size: f32| ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, FontId::proportional(size), Color32::WHITE)).size().x;
        let taken = bar_controls(measure(chip_text(name), SIDEBAR_TEXT_SIZE - 2.0), measure(assistant.to_owned(), SIDEBAR_TEXT_SIZE - 1.0));
        if typing { MESSAGE_BAR_WIDTH } else { (taken + MESSAGE_BAR_TYPING_ROOM).clamp(MESSAGE_BAR_SELECTED_WIDTH, MESSAGE_BAR_WIDTH) }
    }

    /// The panel island: view buttons, the view's title row, a line, the view.
    fn panel(&mut self, ui: &mut Ui, panel: Rect, editor: &mut Editor, look: Look<'_>) {
        let dark = look.dark;
        let palette = theme::palette(dark);
        let [history, chat, other, new] = SIDEBAR_BUTTONS;
        let buttons = Rect::from_min_size(
            panel.min + vec2(SIDEBAR_HEADER_PADDING, SIDEBAR_HEADER_TOP),
            vec2(panel.width() - 2.0 * SIDEBAR_HEADER_PADDING, SIDEBAR_ROW),
        );
        ui.scope_builder(UiBuilder::new().max_rect(buttons).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "messages-square", vec2(history, SIDEBAR_ROW), self.view == View::History, "Chat history", palette).clicked() {
                self.view = View::History;
                // The list is read again each time the view is opened.
                self.past = None;
            }
            if text_button(ui, Some("message-circle"), "Chat", vec2(chat, SIDEBAR_ROW), self.view == View::Chat, false, palette).clicked() {
                self.view = View::Chat;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = SIDEBAR_VIEW_GAP;
                for (view, icon) in
                    [(View::Libraries, "library"), (View::Components, "component"), (View::Present, "presentation"), (View::Layers, "layers")]
                {
                    if icon_button(ui, icon, vec2(other, SIDEBAR_ROW), self.view == view, view.title(), palette).clicked() {
                        self.view = view;
                    }
                }
            });
        });
        let title = Rect::from_min_max(
            pos2(panel.left() + SIDEBAR_TITLE_LEFT, buttons.bottom() + SIDEBAR_TITLE_GAP),
            pos2(buttons.right(), buttons.bottom() + SIDEBAR_TITLE_GAP + SIDEBAR_ROW),
        );
        ui.scope_builder(UiBuilder::new().max_rect(title).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.add(egui::Label::new(RichText::new(self.view.title()).size(SIDEBAR_TITLE_SIZE)).selectable(false));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match self.view {
                View::History => {
                    if text_button(ui, Some("plus"), "New", vec2(new, SIDEBAR_ROW - 1.0), false, true, palette).on_hover_text("New chat").clicked() {
                        self.view = View::Chat;
                        self.chat_asks.push(ChatAsk::New);
                    }
                }
                View::Chat => {
                    let (icon, hint) = if self.wide { ("minimize-2", "Narrower") } else { ("maximize-2", "Wider") };
                    let button = icon_button(ui, icon, Vec2::splat(SIDEBAR_ROW - 1.0), false, hint, palette);
                    ui.painter().rect_stroke(button.rect, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
                    if button.clicked() {
                        self.wide = !self.wide;
                    }
                    if !look.chat.entries.is_empty()
                        && text_button(ui, Some("plus"), "New", vec2(new, SIDEBAR_ROW - 1.0), false, true, palette)
                            .on_hover_text("New chat")
                            .clicked()
                    {
                        self.chat_asks.push(ChatAsk::New);
                    }
                }
                View::Layers | View::Present | View::Components | View::Libraries => {}
            });
        });
        let line = title.bottom() + SIDEBAR_RULE_GAP;
        // Human: the chat has no line under the header: its task list belongs to the header, and the
        // conversation fades out under both as it scrolls up.
        if self.view != View::Chat {
            ui.painter().hline(panel.x_range(), line, Stroke::new(1.0, palette.outline.gamma_multiply(RULE_FADE)));
        }
        let body = Rect::from_min_max(pos2(panel.left() + ISLAND_PADDING, line + 1.0), pos2(panel.right() - ISLAND_PADDING, panel.bottom() - 1.0));
        // Human: a view that isn't the chat still shows a question of the assistant's, under itself. The chat
        // has the task list of a long job above its conversation.
        let list = crate::tasklist::Look { area: body, reach: ISLAND_PADDING, palette, animate: look.animate, held: self.posed_tasks };
        let body = if self.view == View::Chat {
            crate::tasklist::show(ui, look.chat, look.chat_id, list)
        } else {
            let (left, pick) = crate::conversation::pinned(ui, look.chat, body, look.chat_text, palette);
            self.chat_asks.extend(pick.map(ChatAsk::Answer));
            left
        };
        ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(body.intersect(ui.clip_rect()));
            match self.view {
                View::History => self.chat_asks.extend(past_chats(ui, self.past.as_deref().unwrap_or_default(), palette)),
                View::Chat if look.chat.entries.is_empty() => {
                    crate::conversation::forget(ui.ctx(), look.chat_id);
                    self.start_of_chat(ui, palette);
                }
                View::Chat => {
                    let pick = crate::conversation::show(ui, look.chat, look.chat_id, look.chat_text, palette, look.animate);
                    self.chat_asks.extend(pick.map(ChatAsk::Answer));
                }
                View::Layers => {
                    ui.add_space(SIDEBAR_LIST_TOP);
                    crate::panels::layers(ui, editor, dark);
                }
                View::Components => components(ui, editor, palette),
                View::Present | View::Libraries => {
                    ui.add_space(SIDEBAR_NOTE_TOP);
                    ui.label(RichText::new("Nothing here yet.").color(palette.muted));
                }
            }
        });
        // Human: a task list that was worked through moves from the top into the conversation, over both.
        if self.view == View::Chat {
            crate::tasklist::hand_over(ui, look.chat_id, list);
        }
    }

    /// Picks the ideas the chat `chat` offers while it is `empty`, unless they are picked already.
    // Human: every empty chat gets a handful of ideas of its own, picked when it is first shown empty; a chat
    // that has been talked in and is emptied again gets new ones.
    fn deal_ideas(&mut self, chat: u64, empty: bool) {
        if !empty {
            self.ideas_for = None;
        } else if self.ideas_for != Some(chat) {
            let seed = self.ideas_seed.unwrap_or_else(|| crate::ideas::fresh_seed(chat));
            self.ideas = crate::ideas::pick(seed, crate::ideas::SHOWN);
            self.ideas_for = Some(chat);
        }
    }

    /// The new chat view: ideas to start from, and two short explanations.
    fn start_of_chat(&mut self, ui: &mut Ui, palette: Palette) {
        egui::ScrollArea::vertical().id_salt("new-chat").auto_shrink([false, false]).show(ui, |ui| {
            // Human: the ideas sit a little above the middle; in a short window they move up instead of scrolling.
            ui.add_space((ui.available_height() * CHAT_START_DROP).clamp(CHAT_START_DROP_RANGE[0], CHAT_START_DROP_RANGE[1]));
            // Human: there are far more ideas than the six that show; the button deals another six.
            ui.horizontal(|ui| {
                heading(ui, "Ask me to design anything");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if icon_button(ui, SHUFFLE_ICON, Vec2::splat(TARGET_MIN), false, "Other ideas", palette).clicked() {
                        self.ideas = crate::ideas::pick(crate::ideas::fresh_seed(0), crate::ideas::SHOWN);
                    }
                });
            });
            ui.add_space(CHAT_START_HEAD_GAP);
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = CHAT_START_IDEA_GAP;
                ui.spacing_mut().button_padding = CHIP_PADDING;
                for idea in self.ideas.clone() {
                    let chip = egui::Button::new(idea)
                        .truncate()
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::new(1.0, palette.outline))
                        .corner_radius(CHIP_RADIUS)
                        .min_size(vec2(0.0, CHIP_HEIGHT));
                    if ui.add(chip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        idea.clone_into(&mut self.draft);
                    }
                }
            });
            ui.add_space(CHAT_START_SECTION_GAP[0]);
            heading(ui, "Generate code");
            note(ui, "Ask the agent to build your design with React, Tailwind, Next.js, or your preferred stack.", palette);
            note(ui, "For an HTML file, select a frame and click “Export HTML” in the properties panel.", palette);
            ui.add_space(CHAT_START_SECTION_GAP[1]);
            heading(ui, "Attach context files");
            note(ui, "Drop a PNG/JPG, DESIGN.md or any text file here or attach via +", palette);
        });
    }

    /// The message box island: the text, and under it attach, mode, assistant and send.
    fn composer(&mut self, ui: &mut Ui, composer: Rect, editor: &Editor, look: Look<'_>, notice: &mut Option<&'static str>) {
        let palette = theme::palette(look.dark);
        let controls = Rect::from_min_max(
            pos2(composer.left() + COMPOSER_CONTROLS[0], composer.bottom() - COMPOSER_CONTROLS[1]),
            pos2(composer.right() - COMPOSER_CONTROLS[0], composer.bottom() - COMPOSER_CONTROLS[2]),
        );
        // Human: the text has the upper floor to itself and never runs into the controls under it: the box
        // grows with it, and past its tallest the text scrolls.
        let text = Rect::from_min_max(
            composer.min + vec2(ISLAND_PADDING, COMPOSER_TEXT_TOP),
            pos2(composer.right() - ISLAND_PADDING, controls.top() - COMPOSER_TEXT_GAP),
        );
        ui.scope_builder(UiBuilder::new().max_rect(text), |ui| {
            ui.set_clip_rect(text.intersect(ui.clip_rect()));
            egui::ScrollArea::vertical().id_salt("message").max_height(text.height()).show(ui, |ui| {
                // Human: Return sends, alone or with ⌘ (Ctrl on Windows); ⇧Return starts a new line. While the
                // assistant works, what is sent waits its turn or goes to the assistant at once, as the settings say.
                let edit = self.message_field(ui, true, HINT, f32::INFINITY, editor, look, notice);
                if edit.has_focus() && send_pressed(ui) {
                    self.submit();
                }
            });
        });
        ui.scope_builder(UiBuilder::new().max_rect(controls).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = COMPOSER_GAP;
            if icon_button(ui, "plus", Vec2::splat(TARGET_MIN), false, "Attach files", palette).clicked() {
                self.chat_asks.push(ChatAsk::Attach);
            }
            self.file_chips(ui, palette);
            // Human: what is selected shows as a chip here too, where there is room beside the assistant's name.
            if let Some((icon, name)) = self.selected.clone() {
                let assistant = ui.painter().layout_no_wrap(look.assistant.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0), Color32::WHITE);
                let others = (MESSAGE_SEND + COMPOSER_GAP) + (assistant.size().x + MODEL_BUTTON_ROOM + COMPOSER_GAP);
                if ui.available_width() - others >= layer_chip_width(ui, &name) {
                    layer_chip(ui, icon, &name, palette);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.send_controls(ui, look));
        });
        if look.animate && look.chat.running() {
            working_light(ui, composer);
        }
    }

    /// How tall the message box is for what is written in it: two lines at least, a line more for each line
    /// written up to [`COMPOSER_ROWS_MOST`], gliding to its new height when `animate`.
    fn composer_height(&self, ctx: &egui::Context, animate: bool) -> f32 {
        let font = FontId::proportional(MESSAGE_TEXT_SIZE);
        let wrap = self.width() - 2.0 * ISLAND_PADDING;
        let (row, written) =
            ctx.fonts_mut(|fonts| (fonts.row_height(&font), fonts.layout(self.draft.clone(), font.clone(), Color32::WHITE, wrap).size().y));
        let rows = (written / row).ceil().clamp(2.0, COMPOSER_ROWS_MOST);
        let tall = COMPOSER_HEIGHT + (rows - 2.0) * row;
        if animate { ctx.animate_value_with_time(egui::Id::unique("composer-height"), tall, COMPOSER_GROW_TIME) } else { tall }
    }

    /// The message box as one row: attach, the text, assistant and send.
    fn message_bar(
        &mut self,
        ui: &mut Ui,
        bar: Rect,
        chip: Option<&(&'static str, String)>,
        editor: &Editor,
        look: Look<'_>,
        notice: &mut Option<&'static str>,
    ) {
        let palette = theme::palette(look.dark);
        ui.scope_builder(UiBuilder::new().max_rect(bar.shrink2(vec2(MESSAGE_BAR_INSET, 0.0))).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = MESSAGE_BAR_GAP;
            if icon_button(ui, "plus", Vec2::splat(TARGET_MIN), false, "Attach files", palette).clicked() {
                self.chat_asks.push(ChatAsk::Attach);
            }
            self.file_chips(ui, palette);
            if let Some((icon, name)) = chip {
                layer_chip(ui, icon, name, palette);
            }
            let hint = if chip.is_some() { "" } else { HINT };
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = COMPOSER_GAP;
                self.send_controls(ui, look);
                // Human: the text field's line sits a little high for its box, so the field is set 3 points lower
                // to put the typed line on the same middle line as the icons beside it.
                let field = ui.available_rect_before_wrap().translate(vec2(0.0, MESSAGE_FIELD_DROP));
                ui.scope_builder(UiBuilder::new().max_rect(field).layout(Layout::left_to_right(Align::Center)), |ui| {
                    let edit = self.message_field(ui, false, hint, field.width(), editor, look, notice);
                    // Human: the one-line field doesn't give up the keyboard for Return with ⌘ or Ctrl held, so the
                    // keys are looked at here, not whether the field was left.
                    if (edit.has_focus() || edit.lost_focus()) && send_pressed(ui) {
                        self.submit();
                    }
                });
            });
        });
    }

    /// The row of ideas along the bottom edge; a click puts one into the message box. What doesn't fit is cut
    /// off at the row's end.
    fn idea_row(&mut self, ctx: &egui::Context, row: Rect, opacity: f32, palette: Palette) {
        if row.width() < 4.0 * ISLAND_PADDING {
            return;
        }
        egui::Area::new(egui::Id::unique("idea-row")).fade_in(false).fixed_pos(row.min).order(egui::Order::Foreground).show(ctx, |ui| {
            let (place, _) = ui.allocate_exact_size(row.size(), Sense::hover());
            ui.scope_builder(UiBuilder::new().max_rect(place).layout(Layout::left_to_right(Align::Center)), |ui| {
                ui.set_clip_rect(place.intersect(ui.clip_rect()));
                ui.multiply_opacity(opacity);
                ui.spacing_mut().item_spacing.x = BAR_IDEA_GAP;
                ui.spacing_mut().button_padding = CHIP_PADDING;
                // Human: an idea that doesn't fit whole is shortened with "…"; none is started in less room than
                // a few words need. Nothing reaches past the row, so the layer never covers what is beside it.
                for idea in BAR_IDEAS.into_iter().chain(self.ideas.clone()) {
                    let room = place.right() - ui.cursor().left();
                    if room < BAR_IDEA_LEAST {
                        break;
                    }
                    let chip = egui::Button::new(idea)
                        .truncate()
                        .fill(palette.island)
                        .stroke(Stroke::new(1.0, palette.outline))
                        .corner_radius(CHIP_RADIUS)
                        .min_size(vec2(0.0, IDEA_ROW - 1.0));
                    let clicked = ui.scope(|ui| {
                        ui.set_max_width(room);
                        ui.add(chip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
                    });
                    if clicked.inner {
                        idea.clone_into(&mut self.draft);
                    }
                }
            });
        });
    }

    /// The text field of the message box, with pasted layers drawn as chips.
    // Human: layers copied on the canvas and pasted here become a chip in the text: its
    // kind's icon and its name on a tinted ground, or "3 layers" for several, with the names under the pointer.
    // Agent: MAIN-THREAD; takes paste events that hold layers before the text field sees them.
    #[expect(clippy::too_many_arguments, reason = "one text field drawn in two shapes, with what both need")]
    fn message_field(
        &mut self,
        ui: &mut Ui,
        lines: bool,
        hint: &str,
        width: f32,
        editor: &Editor,
        look: Look<'_>,
        notice: &mut Option<&'static str>,
    ) -> Response {
        let palette = theme::palette(look.dark);
        let id = egui::Id::unique("message-field");
        if ui.memory(|memory| memory.has_focus(id)) {
            self.take_pasted(ui, id, editor, notice);
        }
        let font = FontId::proportional(MESSAGE_TEXT_SIZE);
        let before = self.draft.clone();
        let chips = self.chips.clone();
        let (plain, tinted) = (
            egui::TextFormat { font_id: font.clone(), color: palette.foreground, ..Default::default() },
            egui::TextFormat { font_id: font.clone(), color: palette.muted, background: palette.hover, ..Default::default() },
        );
        let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
            let text = text.as_str();
            let mut job = egui::text::LayoutJob::default();
            let mut from = 0;
            for (place, _) in crate::chips::places(text, &chips) {
                job.append(&text[from..place.start], 0.0, plain.clone());
                job.append(&text[place.clone()], 0.0, tinted.clone());
                from = place.end;
            }
            job.append(&text[from..], 0.0, plain.clone());
            job.wrap.max_width = if lines { wrap } else { f32::INFINITY };
            ui.fonts_mut(|fonts| fonts.layout_job(job))
        };
        let field = if lines { egui::TextEdit::multiline(&mut self.draft).desired_rows(2) } else { egui::TextEdit::singleline(&mut self.draft) };
        let output = field
            .id(id)
            .frame(egui::Frame::new())
            .background_color(Color32::TRANSPARENT)
            .hint_text(RichText::new(hint).font(font.clone()).color(palette.quiet))
            .font(font)
            .return_key(egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Enter))
            .desired_width(width)
            .layouter(&mut layouter)
            .show(ui);
        // The chips' edges and icons, over the tinted ground the text was laid out with.
        let painter = ui.painter().with_clip_rect(output.text_clip_rect);
        let offset = output.galley_pos.to_vec2();
        for (place, index) in crate::chips::places(&self.draft, &self.chips) {
            let chars = |bytes: usize| egui::text::CCursor::new(self.draft[..bytes].chars().count());
            let (start, end) = (output.galley.pos_from_cursor(chars(place.start)), output.galley.pos_from_cursor(chars(place.end)));
            // A chip too long for its line is left as tinted text.
            if (start.top() - end.top()).abs() > 1.0 {
                continue;
            }
            let chip = Rect::from_min_max(start.left_top(), end.left_bottom()).translate(offset);
            painter.rect_stroke(chip, CHIP_CORNER, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
            let icon_at = pos2(chip.left() + CHIP_ICON_AT, chip.center().y);
            // Human: a pasted picture shows itself, tiny, where a layer shows its kind's icon; the pointer
            // over the chip shows it larger.
            if let Some(file) = &self.chips[index].picture {
                if !crate::chat::pasted::paint_thumb(ui, &painter, Rect::from_center_size(icon_at, Vec2::splat(CHIP_THUMB)), file) {
                    icons::paint(&painter, Rect::from_center_size(icon_at, Vec2::splat(CHIP_ICON)), self.chips[index].icon, palette.muted);
                }
                ui.interact(chip, id.with(("chip", index)), Sense::hover()).on_hover_ui(|ui| crate::chat::pasted::show_preview(ui, file));
                continue;
            }
            icons::paint(&painter, Rect::from_center_size(icon_at, Vec2::splat(CHIP_ICON)), self.chips[index].icon, palette.muted);
            if self.chips[index].layers.len() > 1 {
                ui.interact(chip, id.with(("chip", index)), Sense::hover()).on_hover_text(self.chips[index].names());
            }
        }
        if let Some(at) = crate::chips::settle(&before, &mut self.draft, &mut self.chips) {
            set_cursor(ui.ctx(), id, at);
        }
        // A picture whose chip was taken out of the text is not kept.
        for gone in chips.iter().filter(|chip| !self.chips.contains(chip)).filter_map(|chip| chip.picture.as_deref()) {
            crate::chat::pasted::forget(gone);
        }
        output.response.response
    }

    /// Turns layers and pictures pasted into the message box into chips at the text cursor.
    // Human: a screenshot or a copied picture pasted while writing a message belongs to the message, at
    // the place it was pasted: "make the header look like this one", and the picture stands right there.
    // Agent: WRITES a pasted picture to its file ([`crate::chat::pasted`]) before its chip is made.
    fn take_pasted(&mut self, ui: &Ui, id: egui::Id, editor: &Editor, notice: &mut Option<&'static str>) {
        let (mut layers, mut pictures) = (Vec::new(), Vec::new());
        ui.input_mut(|input| {
            input.events.retain(|event| match event {
                egui::Event::Paste(text) => crate::app::parse_clipboard(text).is_none_or(|nodes| {
                    layers.push(nodes.iter().map(|node| node.id.clone()).collect::<Vec<String>>());
                    false
                }),
                egui::Event::PasteImage(picture) => {
                    pictures.push(std::sync::Arc::clone(picture));
                    false
                }
                _ => true,
            });
        });
        for ids in layers {
            match crate::chips::chip(editor, &ids, &self.draft) {
                Some(chip) => self.place_chip(ui, id, chip),
                None => *notice = Some(NOT_IN_DOCUMENT),
            }
        }
        for picture in pictures {
            let carried = self.attached.len() + self.chips.iter().filter(|chip| chip.picture.is_some()).count();
            if carried >= crate::chat::files::MOST_FILES {
                *notice = Some(TOO_MANY_FILES);
                break;
            }
            let chip = crate::chat::pasted::save(&picture).and_then(|file| {
                crate::chips::picture_chip(&file, &self.draft).ok_or_else(|| {
                    crate::chat::pasted::forget(&file);
                    "the message holds too many pictures".to_owned()
                })
            });
            match chip {
                Ok(chip) => self.place_chip(ui, id, chip),
                Err(problem) => {
                    log::warn!("a pasted picture couldn't be taken: {problem}");
                    *notice = Some(PICTURE_NOT_TAKEN);
                }
            }
        }
    }

    /// Puts `chip` into the message at the text cursor.
    fn place_chip(&mut self, ui: &Ui, id: egui::Id, chip: Chip) {
        let count = self.draft.chars().count();
        let cursor = egui::TextEdit::load_state(ui.ctx(), id)
            .and_then(|state| state.cursor.char_range())
            .map_or(count, |range| range.primary.index.0.min(count));
        let at = self.draft.char_indices().nth(cursor).map_or(self.draft.len(), |(byte, _)| byte);
        // Human: a space follows the chip, so the user types on at once.
        self.draft.insert_str(at, &format!("{} ", chip.token));
        set_cursor(ui.ctx(), id, cursor + chip.token.chars().count() + 1);
        self.chips.push(chip);
    }

    /// Sends what is typed: the app takes it. The sidebar stays as it is, folded away or open; folded away, the
    /// line over the message bar says what the assistant is doing.
    fn submit(&mut self) {
        // Human: the spaces typed around the message go; a layer's chip at its start or end stays whole.
        // The mark that says where a pasted picture belongs is the app's own; one that got into the text
        // some other way (pasted from elsewhere) is not taken for one.
        self.draft.retain(|letter| letter != crate::chat::files::MARK);
        let text = crate::chips::trimmed(&self.draft).to_owned();
        // Human: files alone can be sent too; the assistant is then asked to look at them.
        if text.is_empty() && self.attached.is_empty() {
            return;
        }
        // Human: the panel shows a pasted layer as a chip with its name, and what was selected as a chip over
        // the message; the assistant reads each pasted layer's id with the name, and is told the selection.
        let (shown, pasted) = crate::chips::shown(&text, &self.chips);
        let told = crate::chips::written(&text, &self.chips, Chip::told);
        let selected = self.selected.iter().map(|(icon, name)| crate::chat::Pointed { at: None, icon, name: name.clone(), picture: None });
        let pointed = selected.chain(pasted).collect();
        let pasted = crate::chips::pictures(&text, &self.chips);
        self.chat_asks.push(ChatAsk::Send { shown, pointed, told, pasted, files: std::mem::take(&mut self.attached) });
        self.chips.clear();
        self.draft.clear();
        self.view = View::Chat;
    }

    /// Trades what is being written in the sidebar for `other`: the chat in front has changed.
    // Human: every open document has a chat of its own. A message half written in one is not sent to another
    // by switching tabs; it waits where it was written.
    pub fn trade_writing(&mut self, other: &mut Writing) {
        std::mem::swap(&mut self.draft, &mut other.draft);
        std::mem::swap(&mut self.attached, &mut other.attached);
        std::mem::swap(&mut self.chips, &mut other.chips);
        std::mem::swap(&mut self.chat_ended, &mut other.chat_ended);
    }

    /// Adds files to the message being written; one that is there already, or beyond the most a message
    /// carries, is left out.
    pub fn attach(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            if self.attached.len() < crate::chat::files::MOST_FILES && !self.attached.contains(&path) {
                self.attached.push(path);
            }
        }
    }

    /// The attached files after the + button: one chip with a paperclip and how many there are. A click opens
    /// their names; a click on a name takes that file off again.
    // Human: the row has little room beside the assistant's name, so the files share one chip.
    fn file_chips(&mut self, ui: &mut Ui, palette: Palette) {
        if self.attached.is_empty() {
            return;
        }
        let response = file_chip(ui, &self.attached.len().to_string(), palette).on_hover_text("Attached files");
        let mut removed = None;
        egui::Popup::menu(&response).align(egui::RectAlign::TOP_START).gap(MENU_GAP[0]).show(|ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            for (index, path) in self.attached.iter().enumerate() {
                let name = crate::chat::files::name(path);
                let icon = if crate::chat::files::picture_kind(path).is_some() { "image" } else { "file-text" };
                if menu_line(ui, false, icon, &name, "×", palette).on_hover_text("Remove").clicked() {
                    removed = Some(index);
                }
            }
        });
        if let Some(index) = removed {
            self.attached.remove(index);
        }
    }

    /// Send (or stop, while the assistant works) and the assistant, for a right-to-left row.
    fn send_controls(&mut self, ui: &mut Ui, look: Look<'_>) {
        let palette = theme::palette(look.dark);
        let running = look.chat.running();
        let ready = !running && (!self.draft.trim().is_empty() || !self.attached.is_empty());
        let (rect, send) = ui.allocate_exact_size(Vec2::splat(MESSAGE_SEND), Sense::click());
        // Human: the button is grey while there is nothing to send and turns dark, growing a little, once
        // there is. While the assistant works it is the stop button: a square in a ring a light runs around.
        let id = egui::Id::unique("message-send-ready");
        let dark = match (look.animate, ready) {
            (true, _) => ui.ctx().animate_bool_with_time(id, ready, MESSAGE_SEND_TIME),
            (false, true) => 1.0,
            (false, false) => 0.0,
        };
        let (centre, radius) = (rect.center(), MESSAGE_SEND / 2.0);
        if running {
            ui.painter().circle(centre, radius - STOP_RING[0], palette.island, Stroke::new(STOP_RING[1], palette.outline));
            if look.animate {
                let head = turned(ui.input(|input| input.time), MESSAGE_SEND_TURN) * std::f32::consts::TAU;
                crate::conversation::comet(ui.painter(), centre, radius - STOP_RING[0], head, CHAT_ACCENT);
                ui.ctx().request_repaint_after(IDLE_FRAME);
            }
            ui.painter().rect_filled(Rect::from_center_size(centre, Vec2::splat(STOP_SQUARE[0])), STOP_SQUARE[1], palette.foreground);
        } else {
            ui.painter().circle_filled(centre, radius * (SEND_REST + (1.0 - SEND_REST) * dark), blend(palette.input, palette.primary, dark));
            let mark = blend(palette.muted, palette.primary_text, dark);
            icons::paint(ui.painter(), Rect::from_center_size(centre, Vec2::splat(SEND_ARROW + 1.0)), "arrow-up", mark);
        }
        if send.on_hover_text(if running { "Stop" } else { "Send" }).clicked() {
            if running {
                self.chat_asks.push(ChatAsk::Stop);
            } else {
                self.submit();
            }
        }
        let model = model_button(ui, look.assistant, palette).on_hover_text("Assistant");
        if model.clicked() {
            self.chat_asks.push(ChatAsk::Looked);
            self.assistants.opened(look.assistants, look.assistant);
        }
        // Human: the menu lists what can answer on this computer: Claude Code when it is installed, and every
        // provider that has a key. It has a search field, so a click inside it doesn't close it; choosing does.
        let mut menu = egui::Popup::menu(&model).close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside);
        if self.assistants.held_open() {
            menu = menu.open(true);
        }
        menu.show(|ui| match self.assistants.show(ui, look.assistants, look.assistant, palette) {
            Some(crate::assistants::Act::Pick(id)) => {
                self.chat_asks.push(ChatAsk::Pick(id));
                ui.close();
            }
            Some(crate::assistants::Act::Manage) => {
                self.chat_asks.push(ChatAsk::Assistants);
                ui.close();
            }
            None => {}
        });
    }

    /// Draws the tool rail with its top-left corner at `at`. Returns what it asks the app to do, if anything.
    // Human: the rail's two menus show a tick before the tool in use, each tool's icon and
    // name, and its shortcut key at the right.
    // Agent: MAIN-THREAD; WRITES editor.tool, and editor.doc when the component entry is used.
    pub fn rail(&mut self, ctx: &egui::Context, at: egui::Pos2, editor: &mut Editor, dark: bool) -> Option<Ask> {
        let palette = theme::palette(dark);
        let mut ask = None;
        egui::Area::new(egui::Id::unique("tool-rail")).fixed_pos(at).order(egui::Order::Foreground).show(ctx, |ui| {
            let steps: f32 = RAIL.iter().map(|(icon, ..)| step(icon)).sum();
            let tools_height = steps + 2.0;
            let (rail, _) =
                ui.allocate_exact_size(vec2(RAIL_WIDTH, RAIL_WIDTH + ISLAND_GAP + tools_height + ISLAND_GAP + RAIL_WIDTH), Sense::hover());
            let round = RAIL_WIDTH / 2.0;
            // The fold button.
            let fold = Rect::from_min_size(rail.min, Vec2::splat(RAIL_WIDTH));
            island(ui, fold, round, palette);
            let response = ui.interact(fold, ui.scope_id().with("fold"), Sense::click());
            icons::paint(ui.painter(), Rect::from_center_size(fold.center(), Vec2::splat(ICON_SIZE)), "panel-left", tint(&response, false, palette));
            focus_ring(ui, &response, fold, round, palette);
            let says = if self.collapsed { "Show the sidebar" } else { "Hide the sidebar" };
            name_button(&response, says);
            if response.on_hover_text(says).clicked() {
                self.collapsed = !self.collapsed;
            }
            // The tools.
            let tools = Rect::from_min_size(pos2(rail.left(), fold.bottom() + ISLAND_GAP), vec2(RAIL_WIDTH, tools_height));
            island(ui, tools, round, palette);
            let mut top = tools.top() + 1.0;
            let shape_icon = SHAPE_MENU.iter().find(|(.., does)| matches!(does, Does::Tool(tool) if *tool == editor.tool)).map(|(icon, ..)| *icon);
            for (index, (icon, hint, act)) in RAIL.into_iter().enumerate() {
                let slot = Rect::from_min_size(pos2(tools.left(), top), vec2(RAIL_WIDTH, step(icon)));
                top = slot.bottom();
                let response = ui.interact(slot, ui.scope_id().with(("tool", index)), Sense::click());
                let active = match act {
                    Act::Pick(tool) => editor.tool == tool,
                    Act::Pointer => matches!(editor.tool, Tool::Move | Tool::Scale | Tool::Eyedropper),
                    Act::Shape => shape_icon.is_some(),
                    Act::Menu(_) | Act::Component | Act::Find => false,
                };
                if active {
                    ui.painter().circle(slot.center(), RAIL_ACTIVE / 2.0, palette.hover, Stroke::new(1.0, palette.outline));
                }
                let (icon, size) = match act {
                    Act::Menu(_) => (icon, RAIL_MENU_ARROW),
                    Act::Pointer if editor.tool == Tool::Scale => ("scaling", ICON_SIZE),
                    Act::Pointer if editor.tool == Tool::Eyedropper => ("pipette", ICON_SIZE),
                    Act::Shape => (shape_icon.unwrap_or(icon), ICON_SIZE),
                    _ => (icon, ICON_SIZE),
                };
                if matches!(act, Act::Menu(_)) && (lit(&response) || egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response))) {
                    ui.painter().rect_filled(slot.shrink2(vec2(RAIL_MENU_PLATE[0], RAIL_MENU_PLATE[1])), RAIL_MENU_PLATE[2], palette.hover);
                }
                icons::paint(ui.painter(), Rect::from_center_size(slot.center(), Vec2::splat(size)), icon, tint(&response, active, palette));
                focus_ring(ui, &response, slot.shrink2(vec2(RAIL_FOCUS[0], 0.0)), RAIL_FOCUS[1], palette);
                let says = crate::keys::text(hint);
                name_button(&response, &says);
                let response = response.on_hover_text(says);
                match act {
                    Act::Pick(tool) if response.clicked() => editor.tool = tool,
                    Act::Pointer if response.clicked() && editor.tool != Tool::Scale => editor.tool = Tool::Move,
                    Act::Shape if response.clicked() && shape_icon.is_none() => editor.tool = Tool::Rectangle,
                    Act::Component if response.clicked() => editor.create_component(),
                    Act::Find if response.clicked() => ask = Some(Ask::Find),
                    Act::Menu(entries) => {
                        egui::Popup::menu(&response).align(egui::RectAlign::RIGHT_START).gap(MENU_GAP[1]).show(|ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            for (icon, label, key, does) in entries {
                                let ticked = matches!(does, Does::Tool(tool) if *tool == editor.tool);
                                if !menu_line(ui, ticked, icon, label, key, palette).clicked() {
                                    continue;
                                }
                                match does {
                                    Does::Tool(tool) => editor.tool = *tool,
                                    Does::Import => ask = Some(Ask::Import),
                                }
                            }
                        });
                    }
                    Act::Pick(_) | Act::Pointer | Act::Shape | Act::Component | Act::Find => {}
                }
            }
            // More: the action search.
            let more = Rect::from_min_size(pos2(rail.left(), tools.bottom() + ISLAND_GAP), Vec2::splat(RAIL_WIDTH));
            island(ui, more, round, palette);
            let response = ui.interact(more, ui.scope_id().with("more"), Sense::click());
            icons::paint(ui.painter(), Rect::from_center_size(more.center(), Vec2::splat(ICON_SIZE)), "ellipsis", tint(&response, false, palette));
            focus_ring(ui, &response, more, round, palette);
            let says = crate::keys::text("Search actions (⌘P)");
            name_button(&response, &says);
            if response.on_hover_text(says).clicked() {
                ask = Some(Ask::Actions);
            }
        });
        ask
    }
}

/// One line of a tool menu: a tick when it is the tool in use, its icon and name, and its shortcut key.
fn menu_line(ui: &mut Ui, ticked: bool, icon: &'static str, label: &str, key: &str, palette: Palette) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(vec2(RAIL_MENU_WIDTH, RAIL_MENU_ROW), Sense::click());
    if lit(&response) {
        ui.painter().rect_filled(row, RAIL_MENU_RADIUS, palette.hover);
    }
    focus_ring(ui, &response, row, RAIL_MENU_RADIUS, palette);
    name_button(&response, label);
    let middle = row.center().y;
    if ticked {
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(row.left() + RAIL_MENU_AT[0], middle), Vec2::splat(RAIL_MENU_TICK)),
            "check",
            palette.muted,
        );
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(row.left() + RAIL_MENU_AT[1], middle), Vec2::splat(RAIL_MENU_ICON)), icon, palette.muted);
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0);
    ui.painter().text(pos2(row.left() + RAIL_MENU_AT[2], middle), Align2::LEFT_CENTER, label, font.clone(), palette.foreground);
    ui.painter().text(pos2(row.right() - RAIL_MENU_AT[3], middle), Align2::RIGHT_CENTER, key, font, palette.muted);
    response
}

/// What the message bar shows for the selection: an icon and a name, or `None` with nothing selected.
fn selection_chip(editor: &Editor) -> Option<(&'static str, String)> {
    match editor.selection.as_slice() {
        [] => None,
        [id] => pen_doc::edit::find(&editor.doc, id).map(|node| (crate::panels::kind_icon(&node.kind), node.display_name())),
        many => Some(("layers", format!("{} layers", many.len()))),
    }
}

/// A layer's name as its chip in the message bar shows it: cut short when it is long.
fn chip_text(name: &str) -> String {
    if name.chars().count() > 22 { name.chars().take(21).chain(['…']).collect() } else { name.to_owned() }
}

/// The width everything in the message bar but the text field takes, with a selection's chip whose text is
/// `chip` wide and an assistant's name that is `assistant` wide: the bar's ends, the attach button, the chip,
/// the assistant's name with its arrow, and send, each with its gap.
fn bar_controls(chip: f32, assistant: f32) -> f32 {
    MESSAGE_BAR_ENDS
        + (TARGET_MIN + MESSAGE_BAR_GAP)
        + (chip + SELECTION_CHIP[0] + MESSAGE_BAR_GAP)
        + (assistant + MODEL_BUTTON_ROOM + COMPOSER_GAP)
        + (MESSAGE_SEND + COMPOSER_GAP)
}

/// The light that runs along the top edge of the message box `composer` while the assistant works.
// Agent: MAIN-THREAD; asks for the next frame at the idle rate (`IDLE_FRAME`).
fn working_light(ui: &Ui, composer: Rect) {
    const PIECES: u8 = 24;
    let (from, to) = (composer.left() + ISLAND_RADIUS, composer.right() - ISLAND_RADIUS);
    let reach = (to - from) * COMPOSER_LIGHT_SHARE;
    let start = from - reach + turned(ui.input(|input| input.time), COMPOSER_LIGHT_TURN) * (to - from + reach);
    let painter = ui.painter().with_clip_rect(Rect::from_x_y_ranges(from..=to, composer.y_range()).intersect(ui.clip_rect()));
    let at = |piece: u8| pos2(start + reach * f32::from(piece) / f32::from(PIECES), composer.top() + 1.0);
    for piece in 0..PIECES {
        // Brightest in its middle, and gone at both ends.
        let strength = 1.0 - (2.0 * (f32::from(piece) + 0.5) / f32::from(PIECES) - 1.0).abs();
        let color = blend(CHAT_ACCENT, Color32::WHITE, COMPOSER_LIGHT_WHITE * strength.powi(3)).gamma_multiply(strength);
        painter.line_segment([at(piece), at(piece + 1)], Stroke::new(COMPOSER_LIGHT_WIDTH, color));
    }
    ui.ctx().request_repaint_after(IDLE_FRAME);
}

/// How wide the chip of a layer called `name` is.
pub fn layer_chip_width(ui: &Ui, name: &str) -> f32 {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    ui.painter().layout_no_wrap(chip_text(name), font, Color32::WHITE).size().x + SELECTION_CHIP[0]
}

/// The selected layer's chip in the message bar: its kind and name in a thin outline.
pub fn layer_chip(ui: &mut Ui, icon: &'static str, name: &str, palette: Palette) {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    let name = chip_text(name);
    let words = ui.painter().layout_no_wrap(name.clone(), font.clone(), palette.muted).size().x;
    let (chip, _) = ui.allocate_exact_size(vec2(words + SELECTION_CHIP[0], SELECTION_CHIP_HEIGHT), Sense::hover());
    ui.painter().rect_stroke(chip, SELECTION_CHIP[4], Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(chip.left() + SELECTION_CHIP[1], chip.center().y), Vec2::splat(SELECTION_CHIP[3])),
        icon,
        palette.muted,
    );
    ui.painter().text(pos2(chip.left() + SELECTION_CHIP[2], chip.center().y), Align2::LEFT_CENTER, name, font, palette.muted);
}

/// The chip the attached files share: a paperclip and their number; it can be clicked.
fn file_chip(ui: &mut Ui, count: &str, palette: Palette) -> Response {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    let words = ui.painter().layout_no_wrap(count.to_owned(), font.clone(), palette.muted).size().x;
    let (chip, response) = ui.allocate_exact_size(vec2(words + SELECTION_CHIP[0], SELECTION_CHIP_HEIGHT), Sense::click());
    let ink = if response.hovered() { palette.foreground } else { palette.muted };
    ui.painter().rect_stroke(chip, SELECTION_CHIP[4], Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(chip.left() + SELECTION_CHIP[1], chip.center().y), Vec2::splat(SELECTION_CHIP[3])),
        "paperclip",
        ink,
    );
    ui.painter().text(pos2(chip.left() + SELECTION_CHIP[2], chip.center().y), Align2::LEFT_CENTER, count, font, ink);
    response
}

/// The message box at one moment of the sidebar's animation.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pose {
    /// Where it is.
    rect: Rect,
    /// Its corner radius.
    radius: f32,
    /// Drawn as the box under the sidebar (two rows) instead of as the bar (one row).
    boxed: bool,
    /// How visible its content is; it dips to nothing where the two layouts change over.
    opacity: f32,
    /// How much of the panel island has risen out of its top edge: 0 none, 1 all.
    risen: f32,
}

impl Pose {
    /// The pose at `openness` between the folded-away `bar` (0) and the `docked` box (1).
    // Human: the bar slides left to where the box will be, quick at first and settling as it arrives; before it
    // has arrived it starts to change size and corners, its bottom edge staying put. The two motions overlap,
    // so there is no pause between them.
    fn at(openness: f32, bar: Rect, docked: Rect) -> Pose {
        let slide = slid(openness);
        let grow = grown(openness);
        let size = bar.size() + (docked.size() - bar.size()) * grow;
        let left = bar.left() + (docked.left() - bar.left()) * slide;
        // Human: the bar may stand higher than the box's bottom edge (over the row of ideas); it comes down
        // as it grows.
        let bottom = bar.bottom() + (docked.bottom() - bar.bottom()) * grow;
        let rect = Rect::from_min_size(pos2(left, bottom - size.y), size);
        let radius = bar.height() / 2.0 + (ISLAND_RADIUS - bar.height() / 2.0) * grow;
        Pose { rect, radius, boxed: grow >= 0.5, opacity: (2.0 * grow - 1.0).abs(), risen: grow }
    }

    /// Where the panel island is at this moment, on its way to `full`: as wide as the message box, standing a
    /// gap above it (and `strip` more, where the counter stands), as tall as it has risen.
    fn above(&self, full: Rect, strip: f32) -> Rect {
        let bottom = self.rect.top() - ISLAND_GAP - strip;
        Rect::from_min_max(pos2(self.rect.left(), bottom - full.height() * self.risen), pos2(self.rect.right(), bottom))
    }
}

/// The counter of what the answer under way has taken so far, as a pill in the gap `slot` between the
/// conversation and the message box; `words` is how wide its words are. It comes up into place as it shows.
// Agent: MAIN-THREAD; a floating layer of its own, since the islands around it are.
fn counter_strip(ctx: &egui::Context, slot: Rect, words: f32, chat: &crate::chat::Chat, palette: Palette, opacity: f32) {
    // Human: nothing is there while the sidebar is folded.
    if opacity <= 0.0 {
        return;
    }
    let size = vec2((words + 2.0 * CHAT_SPENT_PAD).min(slot.width()), CHAT_SPENT_PILL);
    let pill = Rect::from_center_size(slot.center() + vec2(0.0, (1.0 - opacity) * ISLAND_GAP), size);
    egui::Area::new(egui::Id::unique("chat-counter-strip")).fade_in(false).fixed_pos(pill.min).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.multiply_opacity(opacity);
        island(ui, pill, CHAT_SPENT_PILL / 2.0, palette);
        crate::conversation::counter(ui, pill, chat, palette, 1.0);
    });
}

/// The counter as a small pill right over the message bar `bar`, for the sidebar folded away; `words` is how
/// wide its words are. Returns where the pill's top is, so what stands over the bar can stand over the pill.
// Human: over the canvas the words need a ground of their own, so they get an island like the bar's.
// Agent: MAIN-THREAD; a floating layer over the canvas.
fn counter_pill(ctx: &egui::Context, bar: Rect, words: f32, chat: &crate::chat::Chat, palette: Palette, opacity: f32) -> f32 {
    let size = vec2((words + 2.0 * CHAT_SPENT_PAD).min(bar.width()), CHAT_SPENT_PILL);
    let pill = Rect::from_center_size(pos2(bar.center().x, bar.top() - CHAT_STATUS_GAP - CHAT_SPENT_PILL / 2.0), size);
    egui::Area::new(egui::Id::unique("chat-counter-pill")).fade_in(false).fixed_pos(pill.min).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.multiply_opacity(opacity);
        island(ui, pill, CHAT_SPENT_PILL / 2.0, palette);
        crate::conversation::counter(ui, pill, chat, palette, 1.0);
    });
    pill.top()
}

/// How far the message box has slid to the left edge at `openness`: fast at first, slowing as it arrives.
fn slid(openness: f32) -> f32 {
    let left = 1.0 - (openness / SIDEBAR_SLIDE_END).clamp(0.0, 1.0);
    1.0 - left * left * left
}

/// How far the islands have grown at `openness`: nothing at first (the bar only slides), then 0 to 1.
fn grown(openness: f32) -> f32 {
    smooth((openness - SIDEBAR_GROW_START) / (1.0 - SIDEBAR_GROW_START))
}

/// Eases a step from 0 to 1 in and out (smoothstep).
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Height one rail entry takes: the small arrows are shorter than the tools.
fn step(icon: &str) -> f32 {
    if icon == "chevron-down" { RAIL_ARROW_STEP } else { RAIL_STEP }
}

/// The colour of a button's icon or text: bright when active, under the pointer or reached with the keyboard,
/// quieter otherwise.
fn tint(response: &Response, active: bool, palette: Palette) -> Color32 {
    if active || lit(response) { palette.foreground } else { palette.muted }
}

/// Whether a control is under the pointer or reached with the keyboard: either one lights it up.
#[must_use]
pub fn lit(response: &Response) -> bool {
    response.hovered() || response.has_focus()
}

/// Tells a screen reader that `response` is a button called `label` (its words, or its tooltip when it is an
/// icon).
// Human: a button painted by hand is only a clickable rectangle to the accessibility tree until it is named.
// Agent: WRITES the AccessKit node for this frame; call once per button per frame.
pub fn name_button(response: &Response, label: &str) {
    let enabled = response.enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
}

/// Draws the keyboard focus ring around `rect`, `radius` round, while `response` has the focus.
pub fn focus_ring(ui: &Ui, response: &Response, rect: Rect, radius: f32, palette: Palette) {
    if response.has_focus() {
        ui.painter().rect_stroke(rect, radius, Stroke::new(FOCUS_RING, palette.focus), StrokeKind::Inside);
    }
}

/// Paints the rounded background of an island.
pub fn island(ui: &Ui, rect: Rect, radius: f32, palette: Palette) {
    ui.painter().rect(rect, radius, palette.island, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
}

/// Paints a button's background when it is active, under the pointer or reached with the keyboard, and the
/// focus ring in the last case.
fn backdrop(ui: &Ui, response: &Response, active: bool, palette: Palette) {
    if active {
        ui.painter().rect_filled(response.rect, CHIP_RADIUS, palette.hover);
    } else if lit(response) {
        ui.painter().rect_filled(response.rect, CHIP_RADIUS, palette.hover.gamma_multiply(HOVER_FAINT));
    }
    focus_ring(ui, response, response.rect, CHIP_RADIUS, palette);
}

/// A button that is only an icon, `size` large; it has a backdrop while `active`, under the pointer or reached
/// with the keyboard. `hint` is its tooltip and what a screen reader calls it.
pub fn icon_button(ui: &mut Ui, icon: &'static str, size: Vec2, active: bool, hint: &str, palette: Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    backdrop(ui, &response, active, palette);
    icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE)), icon, tint(&response, active, palette));
    name_button(&response, hint);
    response.on_hover_text(hint)
}

/// A button with text, an icon before it if given, and an outline if `outlined`; the content is centred.
pub fn text_button(ui: &mut Ui, icon: Option<&'static str>, text: &str, size: Vec2, active: bool, outlined: bool, palette: Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    backdrop(ui, &response, active, palette);
    if outlined {
        ui.painter().rect_stroke(rect, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    }
    let color = tint(&response, active || outlined, palette);
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    let icon_room = if icon.is_some() { ICON_SIZE + BUTTON_ICON_GAP } else { 0.0 };
    let content = icon_room + galley.size().x;
    let left = rect.center().x - content / 2.0;
    if let Some(icon) = icon {
        icons::paint(ui.painter(), Rect::from_min_size(pos2(left, rect.center().y - ICON_SIZE / 2.0), Vec2::splat(ICON_SIZE)), icon, color);
    }
    ui.painter().text(pos2(left + icon_room, rect.center().y), Align2::LEFT_CENTER, text, FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    name_button(&response, text);
    response
}

/// The assistant's name in the message box, with a small arrow after it.
fn model_button(ui: &mut Ui, name: &str, palette: Palette) -> Response {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0);
    let width = ui.painter().layout_no_wrap(name.to_owned(), font.clone(), palette.muted).size().x;
    let (rect, response) = ui.allocate_exact_size(vec2(width + MODEL_BUTTON_ROOM, MODEL_BUTTON_HEIGHT), Sense::click());
    let color = tint(&response, false, palette);
    ui.painter().text(rect.left_center(), Align2::LEFT_CENTER, name, font, color);
    icons::paint(
        ui.painter(),
        Rect::from_center_size(
            pos2(rect.right() - MODEL_BUTTON_ARROW[0], rect.center().y + MODEL_BUTTON_ARROW[1]),
            Vec2::splat(MODEL_BUTTON_ARROW[2]),
        ),
        "chevron-down",
        color,
    );
    response
}

/// Puts the text cursor of the text field `id` at character `at`.
fn set_cursor(ctx: &egui::Context, id: egui::Id, at: usize) {
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
    egui::TextEdit::store_state(ctx, id, state);
}

/// Whether the keys pressed this frame send the message: Return, alone or with ⌘ or Ctrl. With ⇧ it is a new
/// line, and with Alt it is left to the system.
// Human: people send with Ctrl+Return on Windows and ⌘Return on a Mac as often as with Return alone.
fn send_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.key_pressed(egui::Key::Enter) && !input.modifiers.shift && !input.modifiers.alt)
}

/// A title inside a view.
fn heading(ui: &mut Ui, text: &str) {
    ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TITLE_SIZE)).selectable(false));
}

/// A paragraph of quieter text under a title.
fn note(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add(egui::Label::new(RichText::new(text).color(palette.quiet)).wrap().selectable(false));
}

/// The components view: every reusable layer of the document; a click selects it.
/// The History view: the kept chats, newest first. Gives what the user asked of one: open it, or delete it.
// Human: a line is a button: a click, or Return or the space bar with the keyboard on it, opens the chat in
// the chat panel. The bin at its end deletes it (the app asks first). Tab goes from a line to its bin and
// on to the next line; a screen reader hears a line as its title, its document and its date.
// Agent: MAIN-THREAD; draws only, the app reads and changes the files.
fn past_chats(ui: &mut Ui, rows: &[PastRow], palette: Palette) -> Option<ChatAsk> {
    if rows.is_empty() {
        ui.add_space(SIDEBAR_NOTE_TOP);
        ui.label(RichText::new("No chats yet. What you and the assistant say is kept here.").color(palette.muted));
        return None;
    }
    let [height, gap] = theme::HISTORY_ROW;
    let [inset, title_top, note_top] = theme::HISTORY_ROW_INSET;
    let mut asked = None;
    ui.add_space(SIDEBAR_LIST_TOP);
    egui::ScrollArea::vertical().id_salt("past-chats").auto_shrink([false, false]).show(ui, |ui| {
        ui.spacing_mut().item_spacing.y = gap;
        for row in rows {
            let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
            backdrop(ui, &response, row.current, palette);
            name_button(&response, &format!("{}, {}", row.title, row.note));
            let bin = Rect::from_center_size(pos2(rect.right() - inset - SIDEBAR_ROW / 2.0, rect.center().y), Vec2::splat(SIDEBAR_ROW));
            let width = bin.left() - inset - (rect.left() + inset);
            let lines = [
                (&row.title, SIDEBAR_TEXT_SIZE, tint(&response, row.current, palette), title_top),
                (&row.note, theme::HISTORY_NOTE_TEXT, palette.muted, note_top),
            ];
            for (text, size, color, down) in lines {
                let mut job = egui::text::LayoutJob::simple_singleline(text.clone(), FontId::proportional(size), color);
                job.wrap = egui::text::TextWrapping::truncate_at_width(width);
                let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
                ui.painter().galley(pos2(rect.left() + inset, rect.top() + down), galley, color);
            }
            // The bin lies over the line's end and takes the clicks there.
            let deleted =
                ui.scope_builder(UiBuilder::new().max_rect(bin), |ui| icon_button(ui, "trash", bin.size(), false, "Delete this chat", palette)).inner;
            if deleted.clicked() {
                asked = Some(ChatAsk::DeletePast(row.file.clone()));
            } else if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                asked = Some(ChatAsk::OpenPast(row.file.clone()));
            }
        }
    });
    asked
}

fn components(ui: &mut Ui, editor: &mut Editor, palette: Palette) {
    fn collect(nodes: &[Node], out: &mut Vec<(String, String)>) {
        for node in nodes {
            if node.reusable == Some(true) {
                out.push((node.id.clone(), node.display_name()));
            }
            collect(node.kids(), out);
        }
    }
    let mut found = Vec::new();
    collect(&editor.doc.children, &mut found);
    ui.add_space(SIDEBAR_LIST_TOP);
    if found.is_empty() {
        ui.label(RichText::new("No components in this document.").color(palette.muted));
        return;
    }
    egui::ScrollArea::vertical().id_salt("components").auto_shrink([false, false]).show(ui, |ui| {
        for (id, name) in found {
            let selected = editor.selection.contains(&id);
            // Human: a click selects the component; the plus puts a new instance of it in the middle of the view.
            ui.horizontal(|ui| {
                if ui.small_button("+").on_hover_text("Insert an instance").clicked() {
                    editor.insert_instance(&id);
                } else if ui.selectable_label(selected, RichText::new(name).color(COMPONENT)).clicked() {
                    editor.selection = vec![id];
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Frames of the History view with `rows` in a panel 300 wide: one without events, one for each of
    /// `steps` with its events, and two more. Gives what was asked in any of the frames.
    fn history_frames(rows: &[PastRow], steps: &[Vec<egui::Event>]) -> Vec<ChatAsk> {
        let ctx = egui::Context::default();
        let palette = theme::palette(false);
        let mut asked = Vec::new();
        for frame in 0..steps.len() + 3 {
            let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(300.0, 400.0));
            let events = frame.checked_sub(1).and_then(|step| steps.get(step)).cloned().unwrap_or_default();
            let input = egui::RawInput { screen_rect: Some(screen), focused: true, events, ..Default::default() };
            let mut drawn = ctx.run_ui(input, |ui| {
                ui.scope_builder(UiBuilder::new().max_rect(screen), |ui| asked.extend(past_chats(ui, rows, palette)));
            });
            drawn.textures_delta.clear();
        }
        asked
    }

    fn click(at: egui::Pos2) -> Vec<egui::Event> {
        let button =
            |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        vec![egui::Event::PointerMoved(at), button(true), button(false)]
    }

    #[test]
    fn a_past_chat_is_opened_by_its_line_and_deleted_by_its_bin_with_the_pointer_or_the_keyboard() {
        let row = |name: &str| PastRow {
            file: PathBuf::from(format!("/chats/{name}.json")),
            title: name.to_owned(),
            note: "shop.quill · Yesterday".to_owned(),
            current: false,
        };
        let rows = [row("first"), row("second")];
        let opens = |asked: &[ChatAsk], name: &str| matches!(asked, [ChatAsk::OpenPast(file)] if file.ends_with(name));
        let deletes = |asked: &[ChatAsk], name: &str| matches!(asked, [ChatAsk::DeletePast(file)] if file.ends_with(name));
        let [height, gap] = theme::HISTORY_ROW;
        let middle = |line: f32| SIDEBAR_LIST_TOP + line * (height + gap) + height / 2.0;
        // A click on a line opens its chat; a click on the bin at its end asks to delete it.
        let asked = history_frames(&rows, &[click(pos2(60.0, middle(1.0)))]);
        assert!(opens(&asked, "second.json"), "{asked:?}");
        let bin = 300.0 - theme::HISTORY_ROW_INSET[0] - SIDEBAR_ROW / 2.0;
        let asked = history_frames(&rows, &[click(pos2(bin - 4.0, middle(0.0)))]);
        assert!(deletes(&asked, "first.json"), "{asked:?}");
        // With the keyboard: Tab reaches the first line and Return opens it; another Tab reaches its bin,
        // the next one the second line.
        let key = |key| vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }];
        let tab = || key(egui::Key::Tab);
        let asked = history_frames(&rows, &[tab(), key(egui::Key::Enter)]);
        assert!(opens(&asked, "first.json"), "{asked:?}");
        let asked = history_frames(&rows, &[tab(), tab(), key(egui::Key::Space)]);
        assert!(deletes(&asked, "first.json"), "{asked:?}");
        let asked = history_frames(&rows, &[tab(), tab(), tab(), key(egui::Key::Enter)]);
        assert!(opens(&asked, "second.json"), "{asked:?}");
        // Without chats the view says so and asks for nothing.
        assert_eq!(history_frames(&[], &[click(pos2(60.0, 30.0))]), Vec::new());
    }

    #[test]
    fn what_is_being_written_stays_with_its_chat() {
        let mut sidebar = Sidebar { draft: "Make it blue".to_owned(), ..Sidebar::default() };
        sidebar.attach([PathBuf::from("/a.png")]);
        // Another chat comes to the front: the message box is empty for it, and the first chat keeps its own.
        let mut first = Writing::default();
        assert!(first.is_empty());
        sidebar.trade_writing(&mut first);
        assert!(sidebar.draft.is_empty() && sidebar.attached.is_empty() && !first.is_empty());
        sidebar.draft = "A footer".to_owned();
        // Back to the first: its message is there again, and the second chat's waits in turn.
        let mut second = Writing::default();
        sidebar.trade_writing(&mut second);
        sidebar.trade_writing(&mut first);
        assert_eq!((sidebar.draft.as_str(), sidebar.attached.len(), second.draft.as_str()), ("Make it blue", 1, "A footer"));
        assert!(first.is_empty());
    }

    /// Draws the sidebar for one frame with `events`; `open` or folded away.
    fn frame(sidebar: &mut Sidebar, ctx: &egui::Context, editor: &mut Editor, open: bool, events: Vec<egui::Event>) {
        let area = Rect::from_min_size(pos2(0.0, 40.0), vec2(1352.0, 838.0));
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1352.0, 878.0))),
            events,
            focused: true,
            ..Default::default()
        };
        let chat = Chat::default();
        let mut drawn = ctx.run_ui(input, |ui| {
            let look =
                Look { dark: false, ideas: false, chat_text: 1.0, chat: &chat, chat_id: 1, assistants: &[], assistant: "Test", animate: false };
            sidebar.show(ui.ctx(), area, if open { 1.0 } else { 0.0 }, editor, look);
        });
        // The font picture this frame made is for a screen; there is none here.
        drawn.textures_delta.clear();
    }

    /// What typing "hi" into the message box and then pressing Return with `modifiers` does: the messages
    /// sent, what is left in the box, and whether the sidebar is folded away afterwards.
    fn typed_then_return(open: bool, modifiers: egui::Modifiers) -> (Vec<ChatAsk>, String, bool) {
        let ctx = egui::Context::default();
        let mut editor = Editor::new(pen_doc::Document::default(), None, 1);
        let mut sidebar = Sidebar { collapsed: !open, ..Sidebar::default() };
        frame(&mut sidebar, &ctx, &mut editor, open, Vec::new());
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::unique("message-field")));
        frame(&mut sidebar, &ctx, &mut editor, open, vec![egui::Event::Text("hi".to_owned())]);
        let press = egui::Event::Key { key: egui::Key::Enter, physical_key: Some(egui::Key::Enter), pressed: true, repeat: false, modifiers };
        // The keyboard says which modifier keys are held before it says which key was pressed.
        frame(&mut sidebar, &ctx, &mut editor, open, vec![egui::Event::ModifiersChanged(modifiers), press]);
        (std::mem::take(&mut sidebar.chat_asks), sidebar.draft.clone(), sidebar.collapsed)
    }

    #[test]
    fn return_sends_with_or_without_ctrl_and_leaves_the_sidebar_as_it_is() {
        let sent = |text: &str| vec![ChatAsk::said(text.to_owned(), Vec::new())];
        let ctrl = egui::Modifiers::CTRL | egui::Modifiers::COMMAND;
        // From the bar of the folded sidebar: sent, and the sidebar stays folded away.
        assert_eq!(typed_then_return(false, egui::Modifiers::NONE), (sent("hi"), String::new(), true));
        assert_eq!(typed_then_return(false, ctrl), (sent("hi"), String::new(), true));
        // From the box of the open sidebar: sent, and the sidebar stays open.
        assert_eq!(typed_then_return(true, egui::Modifiers::NONE), (sent("hi"), String::new(), false));
        assert_eq!(typed_then_return(true, ctrl), (sent("hi"), String::new(), false));
        // With Shift it is a new line in the box, and nothing is sent.
        assert_eq!(
            typed_then_return(true, egui::Modifiers::SHIFT),
            (
                Vec::new(),
                "hi
"
                .to_owned(),
                false
            )
        );
    }

    #[test]
    fn every_rail_icon_is_bundled() {
        for (icon, ..) in RAIL {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
        assert!(icons::outline("lucide", SHUFFLE_ICON).is_some());
    }

    #[test]
    fn an_empty_chat_keeps_its_ideas_and_another_chat_gets_its_own() {
        let mut sidebar = Sidebar::default();
        sidebar.deal_ideas(1, true);
        let first = sidebar.ideas.clone();
        assert_eq!(first.len(), crate::ideas::SHOWN);
        // Drawn again, the same chat shows the same ideas; they don't change under the pointer.
        sidebar.deal_ideas(1, true);
        assert_eq!(sidebar.ideas, first);
        // Once the chat has been talked in, its ideas are picked anew the next time it is empty.
        sidebar.deal_ideas(1, false);
        assert_eq!(sidebar.ideas_for, None);
        // Set up for a picture, what is picked is the same every time.
        let mut posed = Sidebar::default();
        posed.pose("chat");
        posed.deal_ideas(1, true);
        let mut again = Sidebar::default();
        again.pose("chat");
        again.deal_ideas(2, true);
        assert_eq!(posed.ideas, again.ideas);
    }

    #[test]
    fn the_message_box_slides_then_grows_and_the_two_overlap() {
        let bar = Rect::from_min_size(pos2(400.0, 663.0), vec2(560.0, 37.0));
        let docked = Rect::from_min_size(pos2(8.0, 609.0), vec2(377.0, 91.0));
        let full = Rect::from_min_max(pos2(8.0, 61.0), pos2(385.0, 601.0));
        let sidebar = Sidebar::default();
        let start = Pose::at(0.0, bar, docked);
        assert_eq!((start.rect, start.boxed, start.opacity), (bar, false, 1.0));
        // Until the growing starts it only moves: the bar's size, on its way left, nothing above it.
        let sliding = Pose::at(SIDEBAR_GROW_START, bar, docked);
        assert_eq!((sliding.rect.size(), sliding.rect.bottom()), (bar.size(), bar.bottom()));
        assert!(sliding.rect.left() < bar.left() && sliding.rect.left() > docked.left());
        assert_eq!((sliding.above(full, 0.0).height(), sidebar.room(SIDEBAR_GROW_START)), (0.0, 0.0));
        // Between the start of growing and the end of sliding both happen at once.
        let both = Pose::at(f32::midpoint(SIDEBAR_GROW_START, SIDEBAR_SLIDE_END), bar, docked);
        assert!(both.rect.left() > docked.left() && both.rect.height() > bar.height());
        // After the slide it grows from its bottom-left corner, and the panel island rises out of its top edge.
        let growing = Pose::at(0.8, bar, docked);
        assert_eq!((growing.rect.left(), growing.rect.bottom()), (8.0, 700.0));
        assert!(growing.rect.height() > 37.0 && growing.rect.height() < 91.0);
        let rising = growing.above(full, 0.0);
        assert_eq!((rising.left(), rising.width(), rising.bottom()), (8.0, growing.rect.width(), growing.rect.top() - ISLAND_GAP));
        assert!(rising.height() > 0.0 && rising.height() < full.height());
        // The layouts change over where nothing shows: half grown.
        let change = Pose::at(SIDEBAR_GROW_START + (1.0 - SIDEBAR_GROW_START) / 2.0, bar, docked);
        assert!(change.opacity < 0.01);
        let end = Pose::at(1.0, bar, docked);
        assert_eq!((end.rect, end.radius, end.boxed, end.opacity), (docked, ISLAND_RADIUS, true, 1.0));
        assert_eq!((end.above(full, 0.0), sidebar.room(1.0)), (full, SIDEBAR_WIDTH + ISLAND_GAP));
        // A bar that stands higher than the box (over the row of ideas) starts there and comes down as it grows.
        let raised = bar.translate(vec2(0.0, -42.0));
        assert_eq!(Pose::at(0.0, raised, docked).rect, raised);
        assert_eq!(Pose::at(1.0, raised, docked).rect, docked);
        // It never goes backwards on the way.
        let lefts: Vec<f32> = (0..=20u8).map(|i| Pose::at(f32::from(i) / 20.0, bar, docked).rect.left()).collect();
        assert!(lefts.windows(2).all(|pair| pair[1] <= pair[0]));
    }

    #[test]
    fn the_message_box_grows_with_what_is_written_and_stops_growing() {
        let mut sidebar = Sidebar::default();
        let ctx = egui::Context::default();
        let mut heights = Vec::new();
        let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| {
            for lines in [0, 1, 4, 40] {
                sidebar.draft = vec!["A line of the message"; lines].join("\n");
                heights.push(sidebar.composer_height(ui.ctx(), false));
            }
        });
        drawn.textures_delta.clear();
        // Nothing and one line take the box as it always was; four lines a taller one; forty no more than the most.
        assert!((heights[0] - COMPOSER_HEIGHT).abs() < f32::EPSILON && (heights[1] - COMPOSER_HEIGHT).abs() < f32::EPSILON);
        assert!(heights[2] > COMPOSER_HEIGHT && heights[3] > heights[2]);
        assert!(heights[3] < COMPOSER_HEIGHT + COMPOSER_ROWS_MOST * 24.0);
    }

    #[test]
    fn the_bar_with_a_selection_leaves_room_to_type() {
        // A short chip and a short name fit the narrow bar with room to spare; a long name asks for more.
        assert!(bar_controls(60.0, 70.0) + MESSAGE_BAR_TYPING_ROOM < MESSAGE_BAR_WIDTH);
        assert!(bar_controls(90.0, 150.0) + MESSAGE_BAR_TYPING_ROOM > MESSAGE_BAR_SELECTED_WIDTH);
        // Attach is TARGET_MIN (24) wide.
        assert_eq!(bar_controls(0.0, 0.0), 157.0);
        assert_eq!((chip_text("Card"), chip_text(&"n".repeat(30)).chars().count()), ("Card".to_owned(), 22));
    }

    #[test]
    fn views_are_found_by_name() {
        assert_eq!(View::named("history"), Some(View::History));
        assert_eq!(View::named("chat"), Some(View::Chat));
        assert_eq!(View::named("hidden"), None);
    }
}
