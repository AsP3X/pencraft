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
use crate::theme::{
    self, CHAT_ASK_WIDTH, CHAT_STATUS_GAP, CHAT_STATUS_STAY, CHIP_CORNER, CHIP_HEIGHT, CHIP_ICON, CHIP_ICON_AT, CHIP_RADIUS, COMPOSER_HEIGHT,
    ICON_SIZE, IDEA_ROW, IDEA_ROW_GAP, ISLAND_GAP, ISLAND_PADDING, ISLAND_RADIUS, MESSAGE_BAR_HEIGHT, MESSAGE_BAR_SELECTED_WIDTH,
    MESSAGE_BAR_TYPING_ROOM, MESSAGE_BAR_WIDTH, MESSAGE_TEXT_SIZE, Palette, RAIL_ACTIVE, RAIL_ARROW_STEP, RAIL_MENU_ROW, RAIL_MENU_WIDTH, RAIL_STEP,
    RAIL_WIDTH, SELECTION_CHIP_HEIGHT, SEND_BUTTON, SIDEBAR_ANIMATION, SIDEBAR_GROW_START, SIDEBAR_HEADER_PADDING, SIDEBAR_ROW, SIDEBAR_SLIDE_END,
    SIDEBAR_TEXT_SIZE, SIDEBAR_TITLE_SIZE, SIDEBAR_WIDE_WIDTH, SIDEBAR_WIDTH, ZOOM_ISLAND,
};

/// Said when a chat control is used: the chat has no assistant behind it in this version.
/// What the user is told when pasted layers are from another document.
const NOT_IN_DOCUMENT: &str = "Those layers aren't in this document, so the assistant can't be pointed at them.";
/// What the mode button says.
const NO_MODES: &str = "Modes aren't available yet.";
/// The grey text in the empty message box.
const HINT: &str = "Create, plan or research... @ to attach, / for skills";
/// The assistant named in the message box.
/// Ideas offered in a new chat; a click puts one into the message box.
const SUGGESTIONS: [&str; 6] = [
    "Plan a car-free weekend in Kyoto for two, with a rainy-day backup",
    "Design a retro-futuristic landing page for a private space travel company",
    "Design a vintage ticketing site for an independent arthouse cinema",
    "Design a playful ordering app for matcha cakes",
    "Design a soft neumorphic meditation app inspired by Nordic design",
    "Design a high-contrast realtime trading terminal",
];

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

/// The sidebar's state for one window.
pub struct Sidebar {
    /// The view in the panel.
    pub view: View,
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
        /// The files attached to it.
        files: Vec<PathBuf>,
    },
    /// Ask for files to attach to the next message.
    Attach,
    /// Stop the assistant.
    Stop,
    /// Start a new chat.
    New,
    /// The assistant menu was opened: look again for what is installed.
    Looked,
    /// Talk to the assistant with this id from now on.
    Pick(String),
    /// Open the settings where assistants are set up.
    Assistants,
    /// Take what the user did with a question of the assistant's: an answer picked, or the question closed.
    Answer(crate::conversation::Pick),
}

impl Default for Sidebar {
    fn default() -> Self {
        // Human: the sidebar starts folded away and stays so until the user opens it.
        Sidebar {
            view: View::Chat,
            collapsed: true,
            wide: false,
            draft: String::new(),
            frozen: None,
            selected: None,
            chat_asks: Vec::new(),
            attached: Vec::new(),
            chips: Vec::new(),
            posed_chip: false,
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
    /// Not in this version; says so.
    Later(&'static str),
}

/// One line of a tool menu: icon, name, shortcut key, and what it does.
type Entry = (&'static str, &'static str, &'static str, Does);

/// The pointer tools' menu: Move and Scale.
const POINTER_MENU: [Entry; 3] = [
    ("mouse-pointer-2", "Move", "V", Does::Tool(Tool::Move)),
    ("scaling", "Scale", "K", Does::Tool(Tool::Scale)),
    ("pipette", "Eyedropper", "I", Does::Tool(Tool::Eyedropper)),
];
/// The shapes' menu.
const SHAPE_MENU: [Entry; 7] = [
    ("square", "Rectangle", "R", Does::Tool(Tool::Rectangle)),
    ("circle", "Ellipse", "O", Does::Tool(Tool::Ellipse)),
    ("triangle", "Polygon", "", Does::Tool(Tool::Polygon)),
    ("sparkle", "Icon", "", Does::Tool(Tool::Icon)),
    ("code-xml", "Script", "", Does::Later("Script layers can't be made yet.")),
    ("image", "Import Image or SVG…", "", Does::Import),
    ("file-input", "Import Figma", "", Does::Later("Importing from Figma isn't available yet.")),
];

/// What the rail asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Tell the user this sentence.
    Say(&'static str),
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
    /// Not in this version; says so.
    Later(&'static str),
}

/// The tool rail's entries from top to bottom: icon, hint, what it does.
const RAIL: [(&str, &str, Act); 14] = [
    ("mouse-pointer-2", "Move (V), Scale (K), Eyedropper (I)", Act::Pointer),
    ("chevron-down", "Move tools", Act::Menu(&POINTER_MENU)),
    ("square", "Shapes", Act::Shape),
    ("chevron-down", "Shapes", Act::Menu(&SHAPE_MENU)),
    ("pen-tool", "Pen (P)", Act::Pick(Tool::Pen)),
    ("spline", "Connector (C)", Act::Pick(Tool::Connector)),
    ("type", "Text (T)", Act::Pick(Tool::Text)),
    ("frame", "Frame (F)", Act::Pick(Tool::Frame)),
    ("sticky-note", "Sticky note (N)", Act::Pick(Tool::Note)),
    ("globe", "Browser", Act::Later("Browser frames can't be placed yet.")),
    ("hand", "Hand (H)", Act::Pick(Tool::Hand)),
    ("message-circle", "Comment", Act::Later("Comments aren't available yet.")),
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
        if name == "assistants" {
            self.assistants.hold_open();
        }
        self.view = if name == "assistants" { View::Chat } else { View::named(name).unwrap_or(self.view) };
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
        // Attached files take room too: with them the bar keeps its full width.
        // Human: the narrower bar still has to leave room to type beside the chip and the assistant's name, which
        // can be long; and once something is typed, or the field is being typed in, the bar takes its full width.
        let wanted = match &chip {
            Some((_, name)) if self.attached.is_empty() => {
                let typing = !self.draft.is_empty() || ctx.memory(|memory| memory.has_focus(egui::Id::unique("message-field")));
                let measure =
                    |text: String, size: f32| ctx.fonts_mut(|fonts| fonts.layout_no_wrap(text, FontId::proportional(size), Color32::WHITE)).size().x;
                let taken =
                    bar_controls(measure(chip_text(name), SIDEBAR_TEXT_SIZE - 2.0), measure(look.assistant.to_owned(), SIDEBAR_TEXT_SIZE - 1.0));
                if typing { MESSAGE_BAR_WIDTH } else { (taken + MESSAGE_BAR_TYPING_ROOM).clamp(MESSAGE_BAR_SELECTED_WIDTH, MESSAGE_BAR_WIDTH) }
            }
            _ => MESSAGE_BAR_WIDTH,
        };
        let space = area;
        let width = wanted.min(space.width() - 2.0 * ISLAND_GAP);
        let bottom = area.bottom() - ISLAND_GAP;
        // Human: with nothing typed yet and nothing drawn yet a row of ideas stands under the bar, which makes room
        // for it, unless the settings turn the ideas off. A document with content shows none.
        let ideas = look.ideas && self.draft.is_empty() && editor.scene.is_empty();
        let bar_bottom = if ideas { bottom - IDEA_ROW - IDEA_ROW_GAP } else { bottom };
        let bar = Rect::from_min_size(pos2(space.center().x - width / 2.0, bar_bottom - MESSAGE_BAR_HEIGHT), vec2(width, MESSAGE_BAR_HEIGHT));
        let docked = Rect::from_min_size(pos2(area.left() + ISLAND_GAP, bottom - COMPOSER_HEIGHT), vec2(self.width(), COMPOSER_HEIGHT));
        let pose = Pose::at(openness, bar, docked);
        if pose.rect.width() < 4.0 * ISLAND_PADDING || pose.rect.top() < area.top() {
            return None;
        }
        // Human: in a very short window the message box keeps its place and the panel gives way.
        let full = Rect::from_min_max(pos2(docked.left(), area.top() + ISLAND_GAP), pos2(docked.right(), docked.top() - ISLAND_GAP));
        let panel = pose.above(full);
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
            let foot = pos2(bar.center().x, bar.top() - CHAT_STATUS_GAP);
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
        notice
    }

    /// The panel island: view buttons, the view's title row, a line, the view.
    fn panel(&mut self, ui: &mut Ui, panel: Rect, editor: &mut Editor, look: Look<'_>) {
        let dark = look.dark;
        let palette = theme::palette(dark);
        let buttons =
            Rect::from_min_size(panel.min + vec2(SIDEBAR_HEADER_PADDING, 13.0), vec2(panel.width() - 2.0 * SIDEBAR_HEADER_PADDING, SIDEBAR_ROW));
        ui.scope_builder(UiBuilder::new().max_rect(buttons).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, "messages-square", vec2(31.0, SIDEBAR_ROW), self.view == View::History, palette)
                .on_hover_text("Chat history")
                .clicked()
            {
                self.view = View::History;
            }
            if text_button(ui, Some("message-circle"), "Chat", vec2(66.0, SIDEBAR_ROW), self.view == View::Chat, false, palette).clicked() {
                self.view = View::Chat;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for (view, icon) in
                    [(View::Libraries, "library"), (View::Components, "component"), (View::Present, "presentation"), (View::Layers, "layers")]
                {
                    if icon_button(ui, icon, vec2(26.0, SIDEBAR_ROW), self.view == view, palette).on_hover_text(view.title()).clicked() {
                        self.view = view;
                    }
                }
            });
        });
        let title =
            Rect::from_min_max(pos2(panel.left() + 16.0, buttons.bottom() + 10.0), pos2(buttons.right(), buttons.bottom() + 10.0 + SIDEBAR_ROW));
        ui.scope_builder(UiBuilder::new().max_rect(title).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.add(egui::Label::new(RichText::new(self.view.title()).size(SIDEBAR_TITLE_SIZE)).selectable(false));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match self.view {
                View::History => {
                    if text_button(ui, Some("plus"), "New", vec2(68.0, SIDEBAR_ROW - 1.0), false, true, palette).on_hover_text("New chat").clicked() {
                        self.view = View::Chat;
                        self.draft.clear();
                        self.chat_asks.push(ChatAsk::New);
                    }
                }
                View::Chat => {
                    let (icon, hint) = if self.wide { ("minimize-2", "Narrower") } else { ("maximize-2", "Wider") };
                    let button = icon_button(ui, icon, Vec2::splat(SIDEBAR_ROW - 1.0), false, palette);
                    ui.painter().rect_stroke(button.rect, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
                    if button.on_hover_text(hint).clicked() {
                        self.wide = !self.wide;
                    }
                    if !look.chat.entries.is_empty()
                        && text_button(ui, Some("plus"), "New", vec2(68.0, SIDEBAR_ROW - 1.0), false, true, palette)
                            .on_hover_text("New chat")
                            .clicked()
                    {
                        self.chat_asks.push(ChatAsk::New);
                    }
                }
                View::Layers | View::Present | View::Components | View::Libraries => {}
            });
        });
        let line = title.bottom() + 11.0;
        ui.painter().hline(panel.x_range(), line, Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
        let body = Rect::from_min_max(pos2(panel.left() + ISLAND_PADDING, line + 1.0), pos2(panel.right() - ISLAND_PADDING, panel.bottom() - 1.0));
        // Human: a view that isn't the chat still shows a question of the assistant's, under itself.
        let body = if self.view == View::Chat {
            body
        } else {
            let (left, pick) = crate::conversation::pinned(ui, look.chat, body, look.chat_text, palette);
            self.chat_asks.extend(pick.map(ChatAsk::Answer));
            left
        };
        ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
            ui.set_clip_rect(body.intersect(ui.clip_rect()));
            match self.view {
                // Human: no chats are kept yet, so the list is empty, as in the design.
                View::History => {}
                View::Chat if look.chat.entries.is_empty() => {
                    crate::conversation::forget(ui.ctx(), look.chat_id);
                    self.start_of_chat(ui, palette);
                }
                View::Chat => {
                    let pick = crate::conversation::show(ui, look.chat, look.chat_id, look.chat_text, palette, look.animate);
                    self.chat_asks.extend(pick.map(ChatAsk::Answer));
                }
                View::Layers => {
                    ui.add_space(8.0);
                    crate::panels::layers(ui, editor, dark);
                }
                View::Components => components(ui, editor, palette),
                View::Present | View::Libraries => {
                    ui.add_space(12.0);
                    ui.label(RichText::new("Nothing here yet.").color(palette.muted));
                }
            }
        });
    }

    /// The new chat view: ideas to start from, and two short explanations.
    fn start_of_chat(&mut self, ui: &mut Ui, palette: Palette) {
        egui::ScrollArea::vertical().id_salt("new-chat").auto_shrink([false, false]).show(ui, |ui| {
            // Human: the ideas sit a little above the middle; in a short window they move up instead of scrolling.
            ui.add_space((ui.available_height() * 0.17).clamp(12.0, 93.0));
            heading(ui, "Ask me to design anything");
            ui.add_space(10.0);
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 9.0;
                ui.spacing_mut().button_padding = vec2(11.0, 6.0);
                for idea in SUGGESTIONS {
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
            ui.add_space(17.0);
            heading(ui, "Generate code");
            note(ui, "Ask the agent to build your design with React, Tailwind, Next.js, or your preferred stack.", palette);
            note(ui, "For an HTML file, select a frame and click “Export HTML” in the properties panel.", palette);
            ui.add_space(18.0);
            heading(ui, "Attach context files");
            note(ui, "Drop a PNG/JPG, DESIGN.md or any text file here or attach via +", palette);
        });
    }

    /// The message box island: the text, and under it attach, mode, assistant and send.
    fn composer(&mut self, ui: &mut Ui, composer: Rect, editor: &Editor, look: Look<'_>, notice: &mut Option<&'static str>) {
        let palette = theme::palette(look.dark);
        let controls =
            Rect::from_min_max(pos2(composer.left() + 11.0, composer.bottom() - 34.0), pos2(composer.right() - 11.0, composer.bottom() - 6.0));
        let text = Rect::from_min_max(composer.min + vec2(ISLAND_PADDING, 17.0), pos2(composer.right() - ISLAND_PADDING, controls.top() - 2.0));
        ui.scope_builder(UiBuilder::new().max_rect(text), |ui| {
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
            ui.spacing_mut().item_spacing.x = 8.0;
            if icon_button(ui, "plus", Vec2::splat(22.0), false, palette).on_hover_text("Attach files").clicked() {
                self.chat_asks.push(ChatAsk::Attach);
            }
            self.file_chips(ui, palette);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.send_controls(ui, look, notice));
        });
    }

    /// The message box as one row: attach, the text, mode, assistant and send.
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
        ui.scope_builder(UiBuilder::new().max_rect(bar.shrink2(vec2(10.0, 0.0))).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            if icon_button(ui, "plus", Vec2::splat(22.0), false, palette).on_hover_text("Attach files").clicked() {
                self.chat_asks.push(ChatAsk::Attach);
            }
            self.file_chips(ui, palette);
            if let Some((icon, name)) = chip {
                layer_chip(ui, icon, name, palette);
            }
            let hint = if chip.is_some() { "" } else { HINT };
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                self.send_controls(ui, look, notice);
                // Human: the text field's line sits a little high for its box, so the field is set 3 points lower
                // to put the typed line on the same middle line as the icons beside it.
                let field = ui.available_rect_before_wrap().translate(vec2(0.0, 3.0));
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
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.spacing_mut().button_padding = vec2(11.0, 6.0);
                // Human: an idea that doesn't fit whole is shortened with "…"; none is started in less room than
                // a few words need. Nothing reaches past the row, so the layer never covers what is beside it.
                for idea in BAR_IDEAS.into_iter().chain(SUGGESTIONS) {
                    let room = place.right() - ui.cursor().left();
                    if room < 120.0 {
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
            self.take_pasted_layers(ui, id, editor, notice);
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
            .hint_text(RichText::new(hint).font(font.clone()).color(palette.muted.gamma_multiply(0.7)))
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
            icons::paint(
                &painter,
                Rect::from_center_size(pos2(chip.left() + CHIP_ICON_AT, chip.center().y), Vec2::splat(CHIP_ICON)),
                self.chips[index].icon,
                palette.muted,
            );
            if self.chips[index].layers.len() > 1 {
                ui.interact(chip, id.with(("chip", index)), Sense::hover()).on_hover_text(self.chips[index].names());
            }
        }
        if let Some(at) = crate::chips::settle(&before, &mut self.draft, &mut self.chips) {
            set_cursor(ui.ctx(), id, at);
        }
        output.response.response
    }

    /// Turns layers pasted into the message box into a chip at the text cursor.
    fn take_pasted_layers(&mut self, ui: &Ui, id: egui::Id, editor: &Editor, notice: &mut Option<&'static str>) {
        let mut pasted = Vec::new();
        ui.input_mut(|input| {
            input.events.retain(|event| match event {
                egui::Event::Paste(text) => crate::app::parse_clipboard(text).is_none_or(|nodes| {
                    pasted.push(nodes.iter().map(|node| node.id.clone()).collect::<Vec<String>>());
                    false
                }),
                _ => true,
            });
        });
        for ids in pasted {
            let Some(chip) = crate::chips::chip(editor, &ids, &self.draft) else {
                *notice = Some(NOT_IN_DOCUMENT);
                continue;
            };
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
    }

    /// Sends what is typed: the app takes it. The sidebar stays as it is, folded away or open; folded away, the
    /// line over the message bar says what the assistant is doing.
    fn submit(&mut self) {
        // Human: the spaces typed around the message go; a layer's chip at its start or end stays whole.
        let text = crate::chips::trimmed(&self.draft).to_owned();
        // Human: files alone can be sent too; the assistant is then asked to look at them.
        if text.is_empty() && self.attached.is_empty() {
            return;
        }
        // Human: the panel shows a pasted layer as a chip with its name, and what was selected as a chip over
        // the message; the assistant reads each pasted layer's id with the name, and is told the selection.
        let (shown, pasted) = crate::chips::shown(&text, &self.chips);
        let told = crate::chips::written(&text, &self.chips, Chip::told);
        let selected = self.selected.iter().map(|(icon, name)| crate::chat::Pointed { at: None, icon, name: name.clone() });
        let pointed = selected.chain(pasted).collect();
        self.chat_asks.push(ChatAsk::Send { shown, pointed, told, files: std::mem::take(&mut self.attached) });
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
        egui::Popup::menu(&response).align(egui::RectAlign::TOP_START).gap(8.0).show(|ui| {
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

    /// Send (or stop, while the assistant works), assistant and mode, for a right-to-left row.
    fn send_controls(&mut self, ui: &mut Ui, look: Look<'_>, notice: &mut Option<&'static str>) {
        let palette = theme::palette(look.dark);
        let running = look.chat.running();
        let ready = running || !self.draft.trim().is_empty() || !self.attached.is_empty();
        let (rect, send) = ui.allocate_exact_size(Vec2::splat(SEND_BUTTON), Sense::click());
        let (fill, mark) = if ready { (palette.primary, palette.primary_text) } else { (palette.input, palette.muted) };
        ui.painter().circle_filled(rect.center(), SEND_BUTTON / 2.0, fill);
        if running {
            ui.painter().rect_filled(Rect::from_center_size(rect.center(), Vec2::splat(7.0)), 1.5, mark);
        } else {
            icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(12.0)), "arrow-up", mark);
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
        if icon_button(ui, "rocket", Vec2::splat(22.0), false, palette).on_hover_text("Mode").clicked() {
            *notice = Some(NO_MODES);
        }
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
            if response.on_hover_text(if self.collapsed { "Show the sidebar" } else { "Hide the sidebar" }).clicked() {
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
                    Act::Menu(_) | Act::Component | Act::Find | Act::Later(_) => false,
                };
                if active {
                    ui.painter().circle(slot.center(), RAIL_ACTIVE / 2.0, palette.hover, Stroke::new(1.0, palette.outline));
                }
                let (icon, size) = match act {
                    Act::Menu(_) => (icon, 10.0),
                    Act::Pointer if editor.tool == Tool::Scale => ("scaling", ICON_SIZE),
                    Act::Pointer if editor.tool == Tool::Eyedropper => ("pipette", ICON_SIZE),
                    Act::Shape => (shape_icon.unwrap_or(icon), ICON_SIZE),
                    _ => (icon, ICON_SIZE),
                };
                if matches!(act, Act::Menu(_))
                    && (response.hovered() || egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&response)))
                {
                    ui.painter().rect_filled(slot.shrink2(vec2(4.0, 1.0)), 5.0, palette.hover);
                }
                icons::paint(ui.painter(), Rect::from_center_size(slot.center(), Vec2::splat(size)), icon, tint(&response, active, palette));
                let response = response.on_hover_text(crate::keys::text(hint));
                match act {
                    Act::Pick(tool) if response.clicked() => editor.tool = tool,
                    Act::Pointer if response.clicked() && editor.tool != Tool::Scale => editor.tool = Tool::Move,
                    Act::Shape if response.clicked() && shape_icon.is_none() => editor.tool = Tool::Rectangle,
                    Act::Component if response.clicked() => editor.create_component(),
                    Act::Find if response.clicked() => ask = Some(Ask::Find),
                    Act::Later(sentence) if response.clicked() => ask = Some(Ask::Say(sentence)),
                    Act::Menu(entries) => {
                        egui::Popup::menu(&response).align(egui::RectAlign::RIGHT_START).gap(10.0).show(|ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            for (icon, label, key, does) in entries {
                                let ticked = matches!(does, Does::Tool(tool) if *tool == editor.tool);
                                if !menu_line(ui, ticked, icon, label, key, palette).clicked() {
                                    continue;
                                }
                                match does {
                                    Does::Tool(tool) => editor.tool = *tool,
                                    Does::Import => ask = Some(Ask::Import),
                                    Does::Later(sentence) => ask = Some(Ask::Say(sentence)),
                                }
                            }
                        });
                    }
                    Act::Pick(_) | Act::Pointer | Act::Shape | Act::Component | Act::Find | Act::Later(_) => {}
                }
            }
            // More: the action search.
            let more = Rect::from_min_size(pos2(rail.left(), tools.bottom() + ISLAND_GAP), Vec2::splat(RAIL_WIDTH));
            island(ui, more, round, palette);
            let response = ui.interact(more, ui.scope_id().with("more"), Sense::click());
            icons::paint(ui.painter(), Rect::from_center_size(more.center(), Vec2::splat(ICON_SIZE)), "ellipsis", tint(&response, false, palette));
            if response.on_hover_text(crate::keys::text("Search actions (⌘P)")).clicked() {
                ask = Some(Ask::Actions);
            }
        });
        ask
    }
}

/// One line of a tool menu: a tick when it is the tool in use, its icon and name, and its shortcut key.
fn menu_line(ui: &mut Ui, ticked: bool, icon: &'static str, label: &str, key: &str, palette: Palette) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(vec2(RAIL_MENU_WIDTH, RAIL_MENU_ROW), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(row, 6.0, palette.hover);
    }
    let middle = row.center().y;
    if ticked {
        icons::paint(ui.painter(), Rect::from_center_size(pos2(row.left() + 14.0, middle), Vec2::splat(11.0)), "check", palette.muted);
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(row.left() + 33.0, middle), Vec2::splat(14.0)), icon, palette.muted);
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0);
    ui.painter().text(pos2(row.left() + 49.0, middle), Align2::LEFT_CENTER, label, font.clone(), palette.foreground);
    ui.painter().text(pos2(row.right() - 12.0, middle), Align2::RIGHT_CENTER, key, font, palette.muted);
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
/// mode, the assistant's name with its arrow, and send, each with its gap.
fn bar_controls(chip: f32, assistant: f32) -> f32 {
    20.0 + (22.0 + 12.0) + (chip + 31.0 + 12.0) + (22.0 + 8.0) + (assistant + 16.0 + 8.0) + (SEND_BUTTON + 8.0)
}

/// How wide the chip of a layer called `name` is.
pub fn layer_chip_width(ui: &Ui, name: &str) -> f32 {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    ui.painter().layout_no_wrap(chip_text(name), font, Color32::WHITE).size().x + 31.0
}

/// The selected layer's chip in the message bar: its kind and name in a thin outline.
pub fn layer_chip(ui: &mut Ui, icon: &'static str, name: &str, palette: Palette) {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    let name = chip_text(name);
    let words = ui.painter().layout_no_wrap(name.clone(), font.clone(), palette.muted).size().x;
    let (chip, _) = ui.allocate_exact_size(vec2(words + 31.0, SELECTION_CHIP_HEIGHT), Sense::hover());
    ui.painter().rect_stroke(chip, 4.0, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(chip.left() + 13.0, chip.center().y), Vec2::splat(11.0)), icon, palette.muted);
    ui.painter().text(pos2(chip.left() + 23.0, chip.center().y), Align2::LEFT_CENTER, name, font, palette.muted);
}

/// The chip the attached files share: a paperclip and their number; it can be clicked.
fn file_chip(ui: &mut Ui, count: &str, palette: Palette) -> Response {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.0);
    let words = ui.painter().layout_no_wrap(count.to_owned(), font.clone(), palette.muted).size().x;
    let (chip, response) = ui.allocate_exact_size(vec2(words + 31.0, SELECTION_CHIP_HEIGHT), Sense::click());
    let ink = if response.hovered() { palette.foreground } else { palette.muted };
    ui.painter().rect_stroke(chip, 4.0, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(chip.left() + 13.0, chip.center().y), Vec2::splat(11.0)), "paperclip", ink);
    ui.painter().text(pos2(chip.left() + 23.0, chip.center().y), Align2::LEFT_CENTER, count, font, ink);
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
    /// gap above it, as tall as it has risen.
    fn above(&self, full: Rect) -> Rect {
        let bottom = self.rect.top() - ISLAND_GAP;
        Rect::from_min_max(pos2(self.rect.left(), bottom - full.height() * self.risen), pos2(self.rect.right(), bottom))
    }
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

/// The colour of a button's icon or text: bright when active or under the pointer, quieter otherwise.
fn tint(response: &Response, active: bool, palette: Palette) -> Color32 {
    if active || response.hovered() { palette.foreground } else { palette.muted }
}

/// Paints the rounded background of an island.
pub fn island(ui: &Ui, rect: Rect, radius: f32, palette: Palette) {
    ui.painter().rect(rect, radius, palette.island, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
}

/// Paints a button's background when it is active or under the pointer.
fn backdrop(ui: &Ui, response: &Response, active: bool, palette: Palette) {
    if active {
        ui.painter().rect_filled(response.rect, CHIP_RADIUS, palette.hover);
    } else if response.hovered() {
        ui.painter().rect_filled(response.rect, CHIP_RADIUS, palette.hover.gamma_multiply(0.5));
    }
}

/// A button that is only an icon, `size` large; it has a backdrop while `active` or under the pointer.
pub fn icon_button(ui: &mut Ui, icon: &'static str, size: Vec2, active: bool, palette: Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    backdrop(ui, &response, active, palette);
    icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE)), icon, tint(&response, active, palette));
    response
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
    let icon_room = if icon.is_some() { ICON_SIZE + 6.0 } else { 0.0 };
    let content = icon_room + galley.size().x;
    let left = rect.center().x - content / 2.0;
    if let Some(icon) = icon {
        icons::paint(ui.painter(), Rect::from_min_size(pos2(left, rect.center().y - ICON_SIZE / 2.0), Vec2::splat(ICON_SIZE)), icon, color);
    }
    ui.painter().text(pos2(left + icon_room, rect.center().y), Align2::LEFT_CENTER, text, FontId::proportional(SIDEBAR_TEXT_SIZE), color);
    response
}

/// The assistant's name in the message box, with a small arrow after it.
fn model_button(ui: &mut Ui, name: &str, palette: Palette) -> Response {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0);
    let width = ui.painter().layout_no_wrap(name.to_owned(), font.clone(), palette.muted).size().x;
    let (rect, response) = ui.allocate_exact_size(vec2(width + 16.0, 22.0), Sense::click());
    let color = tint(&response, false, palette);
    ui.painter().text(rect.left_center(), Align2::LEFT_CENTER, name, font, color);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(rect.right() - 5.0, rect.center().y + 1.0), Vec2::splat(10.0)), "chevron-down", color);
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
    ui.add(egui::Label::new(RichText::new(text).color(palette.muted.gamma_multiply(0.75))).wrap().selectable(false));
}

/// The components view: every reusable layer of the document; a click selects it.
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
    ui.add_space(8.0);
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
                } else if ui.selectable_label(selected, RichText::new(name).color(crate::theme::COMPONENT)).clicked() {
                    editor.selection = vec![id];
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let sent = |text: &str| vec![ChatAsk::Send { shown: text.to_owned(), pointed: Vec::new(), told: text.to_owned(), files: Vec::new() }];
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
        assert_eq!((sliding.above(full).height(), sidebar.room(SIDEBAR_GROW_START)), (0.0, 0.0));
        // Between the start of growing and the end of sliding both happen at once.
        let both = Pose::at(f32::midpoint(SIDEBAR_GROW_START, SIDEBAR_SLIDE_END), bar, docked);
        assert!(both.rect.left() > docked.left() && both.rect.height() > bar.height());
        // After the slide it grows from its bottom-left corner, and the panel island rises out of its top edge.
        let growing = Pose::at(0.8, bar, docked);
        assert_eq!((growing.rect.left(), growing.rect.bottom()), (8.0, 700.0));
        assert!(growing.rect.height() > 37.0 && growing.rect.height() < 91.0);
        let rising = growing.above(full);
        assert_eq!((rising.left(), rising.width(), rising.bottom()), (8.0, growing.rect.width(), growing.rect.top() - ISLAND_GAP));
        assert!(rising.height() > 0.0 && rising.height() < full.height());
        // The layouts change over where nothing shows: half grown.
        let change = Pose::at(SIDEBAR_GROW_START + (1.0 - SIDEBAR_GROW_START) / 2.0, bar, docked);
        assert!(change.opacity < 0.01);
        let end = Pose::at(1.0, bar, docked);
        assert_eq!((end.rect, end.radius, end.boxed, end.opacity), (docked, ISLAND_RADIUS, true, 1.0));
        assert_eq!((end.above(full), sidebar.room(1.0)), (full, SIDEBAR_WIDTH + ISLAND_GAP));
        // A bar that stands higher than the box (over the row of ideas) starts there and comes down as it grows.
        let raised = bar.translate(vec2(0.0, -42.0));
        assert_eq!(Pose::at(0.0, raised, docked).rect, raised);
        assert_eq!(Pose::at(1.0, raised, docked).rect, docked);
        // It never goes backwards on the way.
        let lefts: Vec<f32> = (0..=20u8).map(|i| Pose::at(f32::from(i) / 20.0, bar, docked).rect.left()).collect();
        assert!(lefts.windows(2).all(|pair| pair[1] <= pair[0]));
    }

    #[test]
    fn the_bar_with_a_selection_leaves_room_to_type() {
        // A short chip and a short name fit the narrow bar with room to spare; a long name asks for more.
        assert!(bar_controls(60.0, 70.0) + MESSAGE_BAR_TYPING_ROOM < MESSAGE_BAR_WIDTH);
        assert!(bar_controls(90.0, 150.0) + MESSAGE_BAR_TYPING_ROOM > MESSAGE_BAR_SELECTED_WIDTH);
        assert_eq!(bar_controls(0.0, 0.0), 179.0);
        assert_eq!((chip_text("Card"), chip_text(&"n".repeat(30)).chars().count()), ("Card".to_owned(), 22));
    }

    #[test]
    fn views_are_found_by_name() {
        assert_eq!(View::named("history"), Some(View::History));
        assert_eq!(View::named("chat"), Some(View::Chat));
        assert_eq!(View::named("hidden"), None);
    }
}
