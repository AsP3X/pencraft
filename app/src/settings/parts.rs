//! The pieces the settings pages are built from: headings, cards of rows, and the controls at a row's right.

use egui::{
    Align, Align2, Color32, FontFamily, FontId, Layout, Margin, Pos2, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2,
    vec2,
};

use crate::icons;
use crate::render::BOLD_FAMILY;
use crate::theme::{
    CHECKING, CHIP_RADIUS, CONNECTED, FIELD_HEIGHT, Palette, SETTINGS_CARD_RADIUS, SETTINGS_GROUP_GAP, SETTINGS_LINK_ROW, SETTINGS_ROW,
    SETTINGS_ROW_CONTROL_ROOM, SETTINGS_ROW_PAD, SETTINGS_SEGMENT, SETTINGS_TILE, SETTINGS_TILE_BUTTON, SETTINGS_TITLE_SIZE, SIDEBAR_TEXT_SIZE,
    SWITCH_SIZE,
};

/// The bold face at `size`, for titles and headings.
pub(super) fn strong(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name(BOLD_FAMILY.into()))
}

/// A page's title.
pub(super) fn title(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add(egui::Label::new(RichText::new(text).font(strong(SETTINGS_TITLE_SIZE)).color(palette.foreground)).selectable(false));
}

/// A paragraph under a page's title saying what the page is for.
pub(super) fn about(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add_space(8.0);
    ui.scope(|ui| {
        ui.set_max_width(ui.available_width() - 60.0);
        ui.add(egui::Label::new(RichText::new(text).size(SIDEBAR_TEXT_SIZE).color(palette.muted)).wrap());
    });
}

/// The name over a card.
pub(super) fn heading(ui: &mut Ui, text: &str, palette: Palette) {
    heading_with(ui, text, palette, |_| {});
}

/// The name over a card, with `right` laid out at its right end, from the right.
pub(super) fn heading_with(ui: &mut Ui, text: &str, palette: Palette, right: impl FnOnce(&mut Ui)) {
    ui.add_space(SETTINGS_GROUP_GAP);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    ui.painter().text(pos2(rect.left() + 4.0, rect.center().y), Align2::LEFT_CENTER, text, strong(12.0), palette.muted);
    let inner = rect.shrink2(vec2(4.0, 0.0));
    right(&mut ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)).id_salt(("heading", text))));
    ui.add_space(8.0);
}

/// A quiet line or two between a heading and its card.
pub(super) fn hint(ui: &mut Ui, text: &str, palette: Palette) {
    egui::Frame::new().inner_margin(Margin { left: 4, right: 4, top: 0, bottom: 0 }).show(ui, |ui| {
        ui.add(egui::Label::new(RichText::new(text).size(12.0).color(palette.muted)).wrap());
    });
    ui.add_space(8.0);
}

/// A warning under a field.
pub(super) fn warning(ui: &mut Ui, text: &str) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(CHECKING)).wrap());
}

/// A quiet line under a field.
pub(super) fn note(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(palette.muted)).wrap());
}

/// What stands before a row's name.
#[derive(Clone, Copy)]
pub(super) enum Lead<'a> {
    /// Nothing: the name starts at the card's edge.
    Nothing,
    /// A square with the first letter of this name.
    // Human: brand marks aren't bundled, so the square shows the provider's first letter.
    Letter(&'a str),
    /// A square with an icon.
    Icon(&'static str),
    /// A square in the primary colour with an icon, before a name in bold: the one switch a page is about.
    Strong(&'static str),
}

impl Lead<'_> {
    /// How much room the square takes before the name.
    fn room(self) -> f32 {
        match self {
            Lead::Nothing => 0.0,
            Lead::Letter(_) | Lead::Icon(_) | Lead::Strong(_) => SETTINGS_TILE + 12.0,
        }
    }

    /// Paints the square in `rect`.
    fn paint(self, ui: &Ui, rect: Rect, palette: Palette) {
        match self {
            Lead::Nothing => {}
            Lead::Letter(name) => {
                ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover);
                let initial: String = name.chars().take(1).flat_map(char::to_uppercase).collect();
                ui.painter().text(rect.center(), Align2::CENTER_CENTER, initial, strong(13.0), palette.foreground);
            }
            Lead::Icon(icon) => {
                ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover);
                icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(15.0)), icon, palette.muted);
            }
            Lead::Strong(icon) => {
                ui.painter().rect_filled(rect, CHIP_RADIUS, palette.primary);
                icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(16.0)), icon, palette.primary_text);
            }
        }
    }
}

/// A card: settings that belong together, one row each, with a line between two rows.
pub(super) fn card<R>(ui: &mut Ui, palette: Palette, add: impl FnOnce(&mut Rows<'_>) -> R) -> R {
    let frame = egui::Frame::new().fill(palette.card).stroke(Stroke::new(1.0, palette.outline)).corner_radius(SETTINGS_CARD_RADIUS);
    frame
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            add(&mut Rows { ui, palette, any: false, mono: false, pad: SETTINGS_ROW_PAD })
        })
        .inner
}

/// The rows of a card, while it is being filled.
pub(super) struct Rows<'u> {
    /// Where the rows are laid out.
    pub ui: &'u mut Ui,
    palette: Palette,
    /// A row is there already, so the next one starts with a line.
    any: bool,
    /// The rows' names are set in the monospace face (names of tools).
    pub mono: bool,
    /// Space between the rows' left and right ends and what is in them.
    pad: f32,
}

impl Rows<'_> {
    /// The line between two rows; nothing before the first.
    fn divide(&mut self) {
        if self.any {
            let (line, _) = self.ui.allocate_exact_size(vec2(self.ui.available_width(), 1.0), Sense::hover());
            self.ui.painter().hline(line.x_range(), line.center().y, Stroke::new(1.0, self.palette.outline));
        }
        self.any = true;
    }

    /// A setting: its name (and a quieter line or two under it) on the left, its control on the right.
    // Human: the text wraps before the control instead of running under it. How wide the control is, is only
    // known once it is laid out, so the row remembers that from the frame before.
    // Agent: WRITES one number per row to egui's temporary memory.
    pub fn row(&mut self, name: &str, more: Option<&str>, lead: Lead<'_>, control: impl FnOnce(&mut Ui)) {
        self.divide();
        let (ui, palette, pad) = (&mut *self.ui, self.palette, self.pad);
        let width = ui.available_width();
        let memory = ui.scope_id().with(("row-room", name));
        let room: f32 = ui.data(|data| data.get_temp(memory)).unwrap_or(SETTINGS_ROW_CONTROL_ROOM);
        let text_width = (width - 2.0 * pad - lead.room() - room - 16.0).max(80.0);
        let font = match lead {
            Lead::Strong(_) => strong(14.0),
            _ if self.mono => FontId::monospace(12.5),
            _ => FontId::proportional(SIDEBAR_TEXT_SIZE),
        };
        let named = ui.painter().layout(name.to_owned(), font, palette.foreground, text_width);
        let said = more.map(|more| ui.painter().layout(more.to_owned(), FontId::proportional(12.0), palette.muted, text_width));
        let text_height = named.size().y + said.as_ref().map_or(0.0, |said| 3.0 + said.size().y);
        let height = (text_height + 24.0).max(SETTINGS_ROW);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let tile = Rect::from_center_size(pos2(rect.left() + pad + SETTINGS_TILE / 2.0, rect.center().y), Vec2::splat(SETTINGS_TILE));
        lead.paint(ui, tile, palette);
        let left = rect.left() + pad + lead.room();
        let top = rect.center().y - text_height / 2.0;
        let under = top + named.size().y + 3.0;
        ui.painter().galley(pos2(left, top), named, palette.foreground);
        if let Some(said) = said {
            ui.painter().galley(pos2(left, under), said, palette.muted);
        }
        // Human: a child placed by hand, so the row is exactly as tall as its text asks for.
        let inner = rect.shrink2(vec2(pad, 0.0));
        let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::right_to_left(Align::Center)).id_salt(("row", name)));
        child.spacing_mut().item_spacing.x = 6.0;
        control(&mut child);
        let used = child.min_rect().width();
        ui.data_mut(|data| data.insert_temp(memory, used));
    }

    /// A row that leads somewhere: a square, a name with an optional `tag` after it, a quieter line, and an
    /// arrow. The caller paints a state before the arrow, at [`link_state`].
    pub fn link(&mut self, lead: Lead<'_>, name: &str, more: &str, tag: Option<&str>) -> Response {
        self.divide();
        let (ui, palette) = (&mut *self.ui, self.palette);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), SETTINGS_LINK_ROW), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(rect.shrink(4.0), CHIP_RADIUS, palette.hover.gamma_multiply(0.6));
        }
        let tile = Rect::from_center_size(pos2(rect.left() + SETTINGS_ROW_PAD + SETTINGS_TILE / 2.0, rect.center().y), Vec2::splat(SETTINGS_TILE));
        lead.paint(ui, tile, palette);
        let left = tile.right() + 12.0;
        let line = if more.is_empty() { rect.center().y } else { rect.center().y - 9.0 };
        let named = ui.painter().text(pos2(left, line), Align2::LEFT_CENTER, name, FontId::proportional(SIDEBAR_TEXT_SIZE), palette.foreground);
        if let Some(tag) = tag {
            let font = FontId::proportional(11.0);
            let width = ui.painter().layout_no_wrap(tag.to_owned(), font.clone(), palette.muted).size().x;
            let chip = Rect::from_min_size(pos2(named.right() + 8.0, line - 10.0), vec2(width + 14.0, 20.0));
            ui.painter().rect_filled(chip, 6.0, palette.hover);
            ui.painter().text(chip.center(), Align2::CENTER_CENTER, tag, font, palette.muted);
        }
        if !more.is_empty() {
            let galley = one_line(ui, more, rect.right() - left - 170.0, palette);
            ui.painter().galley(pos2(left, rect.center().y + 9.0 - galley.size().y / 2.0), galley, palette.muted);
        }
        icons::paint(
            ui.painter(),
            Rect::from_center_size(pos2(rect.right() - 22.0, rect.center().y), Vec2::splat(13.0)),
            "chevron-right",
            palette.muted,
        );
        response
    }

    /// A last, short row that shows more of the card ("Show all 18 tools").
    pub fn more(&mut self, text: &str) -> Response {
        self.divide();
        let (ui, palette) = (&mut *self.ui, self.palette);
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
        let color = if response.hovered() { palette.foreground } else { palette.muted };
        let said =
            ui.painter().text(pos2(rect.left() + SETTINGS_ROW_PAD, rect.center().y), Align2::LEFT_CENTER, text, FontId::proportional(12.5), color);
        icons::paint(ui.painter(), Rect::from_center_size(pos2(said.right() + 12.0, rect.center().y), Vec2::splat(13.0)), "chevron-down", color);
        response
    }

    /// Content of its own inside the card, with the card's padding around it.
    pub fn block(&mut self, add: impl FnOnce(&mut Ui)) {
        self.divide();
        egui::Frame::new().inner_margin(Margin::same(16)).show(self.ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    }
}

/// A setting's row on its own, outside a card and as wide as the space it is in: its name (and a quieter line
/// or two) on the left, its control on the right.
pub(crate) fn row(ui: &mut Ui, name: &str, more: Option<&str>, palette: Palette, control: impl FnOnce(&mut Ui)) {
    ui.spacing_mut().item_spacing.y = 0.0;
    Rows { ui, palette, any: false, mono: false, pad: 0.0 }.row(name, more, Lead::Nothing, control);
}

/// Where a link row's state pill ends: before its arrow.
pub(super) fn link_state(row: Rect) -> Pos2 {
    pos2(row.right() - 40.0, row.center().y)
}

/// `text` on one line of at most `width`, ending with "…" when it is longer.
fn one_line(ui: &Ui, text: &str, width: f32, palette: Palette) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), FontId::proportional(12.0), palette.muted);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(40.0));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// A tile to click in a grid: a square, a name, a quieter line, and a plus. `filled` tiles are the known
/// providers; the others are the user's own kinds.
pub(super) fn tile_button(ui: &mut Ui, lead: Lead<'_>, name: &str, more: &str, filled: bool, palette: Palette) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), SETTINGS_TILE_BUTTON), Sense::click());
    let fill = match (filled, response.hovered()) {
        (_, true) => palette.hover.gamma_multiply(0.6),
        (true, false) => palette.card,
        (false, false) => Color32::TRANSPARENT,
    };
    ui.painter().rect(rect, 10.0, fill, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    let tile = Rect::from_center_size(pos2(rect.left() + 12.0 + SETTINGS_TILE / 2.0, rect.center().y), Vec2::splat(SETTINGS_TILE));
    lead.paint(ui, tile, palette);
    let left = tile.right() + 10.0;
    let named = one_line_in(ui, name, FontId::proportional(SIDEBAR_TEXT_SIZE), palette.foreground, rect.right() - left - 34.0);
    ui.painter().galley(pos2(left, rect.center().y - 8.0 - named.size().y / 2.0), named, palette.foreground);
    let galley = one_line(ui, more, rect.right() - left - 34.0, palette);
    ui.painter().galley(pos2(left, rect.center().y + 9.0 - galley.size().y / 2.0), galley, palette.muted);
    icons::paint(ui.painter(), Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), Vec2::splat(13.0)), "plus", palette.muted);
    response
}

/// `text` in `font` on one line of at most `width`, ending with "…" when it is longer.
fn one_line_in(ui: &Ui, text: &str, font: FontId, color: Color32, width: f32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(40.0));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// An on/off switch.
pub(crate) fn switch(ui: &mut Ui, on: &mut bool, palette: Palette) {
    let (rect, response) = ui.allocate_exact_size(Vec2::from(SWITCH_SIZE), Sense::click());
    if response.clicked() {
        *on = !*on;
    }
    let radius = rect.height() / 2.0;
    let knob = radius - 2.0;
    if *on {
        ui.painter().rect_filled(rect, radius, palette.primary);
        ui.painter().circle_filled(pos2(rect.right() - radius, rect.center().y), knob, palette.primary_text);
    } else {
        ui.painter().rect_filled(rect, radius, palette.input);
        ui.painter().circle_filled(pos2(rect.left() + radius, rect.center().y), knob, Color32::WHITE);
    }
}

/// One choice of a [`segmented`] control.
#[derive(Clone, Copy)]
pub(super) struct Segment<'a> {
    /// What the setting holds when this is chosen.
    pub value: &'a str,
    /// What the control shows for it.
    pub shown: &'a str,
    /// An icon before that, if any.
    pub icon: Option<&'static str>,
}

impl<'a> Segment<'a> {
    /// A choice shown as the setting holds it.
    pub fn plain(value: &'a str) -> Segment<'a> {
        Segment { value, shown: value, icon: None }
    }
}

/// A few choices side by side, the chosen one raised. `salt` tells the control apart from others.
pub(super) fn segmented(ui: &mut Ui, salt: &str, value: &mut String, options: &[Segment<'_>], palette: Palette) {
    let font = FontId::proportional(12.5);
    let widths: Vec<f32> = options
        .iter()
        .map(|option| {
            let text = ui.painter().layout_no_wrap(option.shown.to_owned(), font.clone(), palette.foreground).size().x;
            text + 20.0 + if option.icon.is_some() { 19.0 } else { 0.0 }
        })
        .collect();
    // Human: 2 between two choices and 2 around them all: each choice brings 2, and 2 more for the far end.
    let width = widths.iter().map(|width| width + 2.0).sum::<f32>() + 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, FIELD_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, CHIP_RADIUS, palette.hover);
    let mut left = rect.left() + 2.0;
    for (option, width) in options.iter().zip(widths) {
        let segment = Rect::from_min_size(pos2(left, rect.center().y - SETTINGS_SEGMENT / 2.0), vec2(width, SETTINGS_SEGMENT));
        left = segment.right() + 2.0;
        let response = ui.interact(segment, ui.scope_id().with(("segment", salt, option.value)), Sense::click());
        let chosen = value.as_str() == option.value;
        if chosen {
            ui.painter().rect(segment, 6.0, palette.card, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
        }
        let color = if chosen || response.hovered() { palette.foreground } else { palette.muted };
        let mut text = segment.left() + 10.0;
        if let Some(icon) = option.icon {
            icons::paint(ui.painter(), Rect::from_center_size(pos2(text + 6.5, segment.center().y), Vec2::splat(13.0)), icon, color);
            text += 19.0;
        }
        ui.painter().text(pos2(text, segment.center().y), Align2::LEFT_CENTER, option.shown, font.clone(), color);
        if response.clicked() {
            option.value.clone_into(value);
        }
    }
}

/// An outlined button showing `value`, as wide as that; a click offers `options`.
pub(super) fn choice(ui: &mut Ui, salt: &str, value: &mut String, options: &[&str], palette: Palette) {
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE);
    let width = ui.painter().layout_no_wrap(value.clone(), font, palette.foreground).size().x;
    let (rect, _) = ui.allocate_exact_size(vec2(width + 40.0, FIELD_HEIGHT), Sense::hover());
    choice_at(ui, rect, salt, value, options, palette);
}

/// A choice as wide and tall as a text field.
pub(super) fn wide_choice(ui: &mut Ui, value: &mut String, options: &[&str], palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), FIELD_HEIGHT), Sense::hover());
    choice_at(ui, rect, "wide", value, options, palette);
}

/// A choice drawn in `rect`, showing `value`; a click offers `options`. `salt` tells it apart from others.
pub(crate) fn choice_at(ui: &mut Ui, rect: Rect, salt: &str, value: &mut String, options: &[&str], palette: Palette) {
    let response = ui.interact(rect, ui.scope_id().with(("choice", salt)), Sense::click());
    let fill = if response.hovered() { palette.hover.gamma_multiply(0.6) } else { Color32::TRANSPARENT };
    ui.painter().rect(rect, CHIP_RADIUS, fill, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    ui.painter().text(
        pos2(rect.left() + 11.0, rect.center().y),
        Align2::LEFT_CENTER,
        value.as_str(),
        FontId::proportional(SIDEBAR_TEXT_SIZE),
        palette.foreground,
    );
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(rect.right() - 14.0, rect.center().y), Vec2::splat(12.0)),
        "chevrons-up-down",
        palette.muted,
    );
    egui::Popup::menu(&response).show(|ui| {
        for option in options {
            if ui.selectable_label(value.as_str() == *option, *option).clicked() {
                (*option).clone_into(value);
            }
        }
    });
}

/// How much a button stands out.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    /// The one thing a form is there for.
    Primary,
    /// An ordinary action.
    Outline,
    /// A way out, or something rarely wanted.
    Quiet,
}

/// A button as wide as its text (and icon). One that isn't `enabled` is pale and takes no click.
pub(super) fn button(ui: &mut Ui, text: &str, icon: Option<&'static str>, kind: Kind, enabled: bool, palette: Palette) -> Response {
    let font = FontId::proportional(12.5);
    let text_width = ui.painter().layout_no_wrap(text.to_owned(), font.clone(), palette.foreground).size().x;
    let icon_room = if icon.is_some() { 19.0 } else { 0.0 };
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, response) = ui.allocate_exact_size(vec2(text_width + icon_room + 24.0, FIELD_HEIGHT), sense);
    let hovered = enabled && response.hovered();
    let (fill, edge, color) = match kind {
        Kind::Primary => (palette.primary.gamma_multiply(if hovered { 0.85 } else { 1.0 }), Color32::TRANSPARENT, palette.primary_text),
        Kind::Outline => (if hovered { palette.hover.gamma_multiply(0.6) } else { Color32::TRANSPARENT }, palette.outline, palette.foreground),
        Kind::Quiet => {
            let fill = if hovered { palette.hover.gamma_multiply(0.6) } else { Color32::TRANSPARENT };
            (fill, Color32::TRANSPARENT, if hovered { palette.foreground } else { palette.muted })
        }
    };
    let pale = if enabled { 1.0 } else { 0.45 };
    ui.painter().rect(rect, CHIP_RADIUS, fill.gamma_multiply(pale), Stroke::new(1.0, edge), StrokeKind::Inside);
    let mut left = rect.left() + 12.0;
    if let Some(icon) = icon {
        icons::paint(ui.painter(), Rect::from_center_size(pos2(left + 6.5, rect.center().y), Vec2::splat(13.0)), icon, color.gamma_multiply(pale));
        left += icon_room;
    }
    ui.painter().text(pos2(left, rect.center().y), Align2::LEFT_CENTER, text, font, color.gamma_multiply(pale));
    response
}

/// The small name over a form field.
pub(crate) fn caption(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add(egui::Label::new(RichText::new(text).size(12.0).color(palette.foreground)).selectable(false));
    ui.add_space(6.0);
}

/// The frame of a text field.
fn field_frame(palette: Palette) -> egui::Frame {
    egui::Frame::new().stroke(Stroke::new(1.0, palette.outline)).corner_radius(CHIP_RADIUS).inner_margin(Margin::symmetric(11, 7))
}

/// An outlined text field as wide as the space it is in.
pub(super) fn field(ui: &mut Ui, text: &mut String, hint: &str, mono: bool, secret: bool, palette: Palette) -> Response {
    let width = ui.available_width();
    sized_field(ui, text, hint, width, mono, secret, palette)
}

/// An outlined text field `width` wide.
pub(super) fn sized_field(ui: &mut Ui, text: &mut String, hint: &str, width: f32, mono: bool, secret: bool, palette: Palette) -> Response {
    let font = if mono { egui::TextStyle::Monospace } else { egui::TextStyle::Body };
    let hint = RichText::new(hint).text_style(font.clone()).color(palette.muted.gamma_multiply(0.75));
    ui.add(egui::TextEdit::singleline(text).frame(field_frame(palette)).hint_text(hint).font(font).password(secret).desired_width(width))
}

/// A text field with a magnifying glass in it, as wide as the space it is in.
pub(super) fn search_field(ui: &mut Ui, text: &mut String, hint: &str, palette: Palette) -> Response {
    let frame = field_frame(palette).fill(palette.card).inner_margin(Margin { left: 30, right: 10, top: 7, bottom: 7 });
    let hint = RichText::new(hint).size(12.5).color(palette.muted);
    let response = ui.add(egui::TextEdit::singleline(text).frame(frame).hint_text(hint).desired_width(ui.available_width()));
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(response.rect.left() + 16.0, response.rect.center().y), Vec2::splat(13.0)),
        "search",
        palette.muted,
    );
    response
}

/// What a state pill is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Tone {
    /// It works.
    Good,
    /// It is being found out.
    Busy,
    /// It doesn't work, or isn't wanted.
    Quiet,
}

/// A state pill ("Ready") with its right edge at `right`.
pub(super) fn pill_at(ui: &Ui, right: Pos2, words: &str, tone: Tone, palette: Palette) {
    let width = pill_width(ui, words);
    paint_pill(ui, Rect::from_min_size(pos2(right.x - width, right.y - 11.0), vec2(width, 22.0)), words, tone, palette);
}

/// A state pill laid out like a control.
pub(super) fn pill(ui: &mut Ui, words: &str, tone: Tone, palette: Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(pill_width(ui, words), 22.0), Sense::hover());
    paint_pill(ui, rect, words, tone, palette);
}

/// How wide the pill saying `words` is.
fn pill_width(ui: &Ui, words: &str) -> f32 {
    ui.painter().layout_no_wrap(words.to_owned(), FontId::proportional(11.5), Color32::PLACEHOLDER).size().x + 30.0
}

/// Paints a state pill in `rect`.
fn paint_pill(ui: &Ui, rect: Rect, words: &str, tone: Tone, palette: Palette) {
    let (text, dot, edge, fill) = match tone {
        Tone::Quiet => (palette.muted, palette.muted.gamma_multiply(0.5), palette.outline, Color32::TRANSPARENT),
        Tone::Busy => (CHECKING, CHECKING, CHECKING.gamma_multiply(0.5), CHECKING.gamma_multiply(0.14)),
        Tone::Good => (CONNECTED, CONNECTED, CONNECTED.gamma_multiply(0.5), CONNECTED.gamma_multiply(0.14)),
    };
    ui.painter().rect(rect, 11.0, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
    ui.painter().circle_filled(pos2(rect.left() + 12.0, rect.center().y), 3.0, dot);
    ui.painter().text(pos2(rect.left() + 21.0, rect.center().y), Align2::LEFT_CENTER, words, FontId::proportional(11.5), text);
}
