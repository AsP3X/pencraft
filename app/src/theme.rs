//! The editor's colours and sizes, in one place (single-source-of-truth.md: UI tokens live in the theme module).

use egui::{Color32, CornerRadius, Stroke, Visuals};

/// Height of the top bar: the title island with the space above it and a line's width below (the user's
/// design; the sidebar starts an [`ISLAND_GAP`] under it).
pub const TAB_BAR_HEIGHT: f32 = 53.0;
/// Space between the window's top edge and the title island.
pub const TITLE_ISLAND_TOP: f32 = 11.0;
/// Height of the title island.
pub const TITLE_ISLAND_HEIGHT: f32 = 41.0;
/// Room left for the macOS window buttons at the start of the title island.
pub const TRAFFIC_LIGHT_SPACE: f32 = 90.0;
/// Space between the window's left edge and the macOS close button.
pub const TRAFFIC_LIGHT_LEFT: f32 = 21.0;
/// Size of the app's name in the title island.
pub const TITLE_SIZE: f32 = 17.0;
/// Height of a tab in the title island; its ends are half circles.
pub const TAB_HEIGHT: f32 = 27.0;
/// How wide the dot is that a tab shows while its chat's assistant is at work.
pub const TAB_WORK_DOT: f32 = 7.0;
/// Side of the icon buttons in the title island.
pub const TITLE_BUTTON: f32 = 26.0;
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
/// Selection and focus colour.
pub const ACCENT: Color32 = Color32::from_rgb(0x3b, 0x82, 0xf6);
/// The colour of components: their outline, canvas label and layer name.
pub const COMPONENT: Color32 = Color32::from_rgb(0xd9, 0x46, 0xef);
/// The colour of instances and the layers inside them.
pub const INSTANCE: Color32 = Color32::from_rgb(0x8b, 0x5c, 0xf6);
/// The colour of an AI model at work: the flash on a layer it touched, and its cursor.
pub const AGENT: Color32 = Color32::from_rgb(96, 125, 255);
/// How long a layer an AI model touched flashes.
pub const AGENT_FLASH_TIME: std::time::Duration = std::time::Duration::from_millis(1200);
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
/// Width of the assistant menu.
pub const ASSISTANT_MENU_WIDTH: f32 = 240.0;
/// Size of a resize handle on screen, points.
pub const HANDLE_SIZE: f32 = 8.0;
/// How near an edge of the selection box a press must be to resize by that edge.
pub const EDGE_GRAB: f32 = 4.0;
/// How far outside a corner of the selection box a press still turns the layer.
pub const ROTATE_REACH: f32 = 22.0;
/// Radius of the rotation handle's dot.
pub const ROTATE_DOT: f32 = 5.5;
/// How far the rotation handle's dot stands off the selection box.
pub const ROTATE_DOT_GAP: f32 = 26.0;
/// How near the rotation handle's dot a press must be to grab it.
pub const ROTATE_DOT_GRAB: f32 = 12.0;
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
}

/// The light palette (`:root`).
pub const LIGHT: Palette = Palette {
    background: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    foreground: Color32::from_rgb(0x02, 0x02, 0x02),
    muted: Color32::from_rgb(0x71, 0x71, 0x7a),
    border: Color32::from_rgb(0xe4, 0xe4, 0xe7),
    canvas: Color32::from_rgb(0xf6, 0xf6, 0xf6),
    island: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    hover: Color32::from_rgb(0xf4, 0xf4, 0xf5),
    input: Color32::from_rgb(0xe4, 0xe4, 0xe7),
    primary: Color32::from_rgb(0x18, 0x18, 0x1b),
    primary_text: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    outline: Color32::from_rgb(0xe4, 0xe4, 0xe7),
};

/// The dark palette (`.dark`).
pub const DARK: Palette = Palette {
    background: Color32::from_rgb(0x18, 0x18, 0x1b),
    foreground: Color32::from_rgb(0xc8, 0xc8, 0xc8),
    muted: Color32::from_rgb(0xa1, 0xa1, 0xaa),
    border: Color32::from_rgb(0x20, 0x20, 0x20),
    canvas: Color32::from_rgb(0x18, 0x18, 0x1b),
    island: Color32::from_rgb(0x27, 0x27, 0x2a),
    hover: Color32::from_rgb(0x3f, 0x3f, 0x46),
    input: Color32::from_rgb(0x3f, 0x3f, 0x46),
    primary: Color32::from_rgb(0xfa, 0xfa, 0xfa),
    primary_text: Color32::from_rgb(0x18, 0x18, 0x1b),
    outline: Color32::from_rgb(0x3a, 0x3a, 0x3e),
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
pub const UPDATE_WIDTH: f32 = 320.0;
/// Space between the window's edge and that card.
pub const UPDATE_MARGIN: f32 = 16.0;
/// Most height the release notes take in the card before they scroll.
pub const UPDATE_NOTES_HEIGHT: f32 = 180.0;
/// Space between the card's title, its notes and its buttons.
pub const UPDATE_GAP: f32 = 10.0;

/// Space between the window's edge and the settings window (the user's settings design).
pub const SETTINGS_MARGIN: f32 = 8.0;
/// Largest size of the settings window.
pub const SETTINGS_MAX: [f32; 2] = [1600.0, 1000.0];
/// Corner radius of the settings window.
pub const SETTINGS_RADIUS: f32 = 16.0;
/// Width of the settings window's list of pages.
pub const SETTINGS_NAV_WIDTH: f32 = 208.0;
/// Height of a page's entry in that list.
pub const SETTINGS_NAV_ROW: f32 = 32.0;
/// Size of a settings page's title.
pub const SETTINGS_TITLE_SIZE: f32 = 20.0;
/// Size of a settings section's title.
pub const SETTINGS_SECTION_SIZE: f32 = 14.0;
/// Height of a settings row; one with a second line of text is taller.
pub const SETTINGS_ROW: f32 = 44.0;
/// Height of a settings row with a second line of text.
pub const SETTINGS_TALL_ROW: f32 = 56.0;
/// Height of the settings window's small controls: choices, the appearance switch.
pub const SETTINGS_CONTROL: f32 = 25.0;
/// Size of an on/off switch.
pub const SWITCH_SIZE: [f32; 2] = [25.0, 14.0];
/// Height of a provider's card in the settings window.
pub const PROVIDER_CARD: f32 = 66.0;
/// Corner radius of a provider's card.
pub const PROVIDER_RADIUS: f32 = 12.0;
/// Side of the square in front of a provider's name.
pub const PROVIDER_TILE: f32 = 32.0;
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
/// Width of the message bar while a layer is selected: the layer's chip takes the place of the long hint.
pub const MESSAGE_BAR_SELECTED_WIDTH: f32 = 408.0;
/// Height of the selected layer's chip in the message bar.
pub const SELECTION_CHIP_HEIGHT: f32 = 21.0;
/// Height of the row of ideas under the message bar.
pub const IDEA_ROW: f32 = 30.0;
/// Space between the message bar and the row of ideas under it.
pub const IDEA_ROW_GAP: f32 = 12.0;
/// Size of the zoom island at the canvas's bottom right corner.
pub const ZOOM_ISLAND: [f32; 2] = [116.0, 30.0];
