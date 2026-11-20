//! The assistant menu of the message box: who can answer in the chat, grouped by who provides them, with a
//! field that finds one by a few of its letters.
//!
//! An assistant is named "Provider · Model" unless the user gave it a name of their own. The menu lists the
//! models under their provider; while a search is on it is one list, the best find first, with the provider
//! beside each model.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align2, Color32, FontId, Key, Margin, Rect, RichText, Sense, Stroke, Ui, Vec2, pos2, vec2};

use crate::chat::Choice;
use crate::fuzzy;
use crate::icons;
use crate::theme::{self, ASSISTANT_MENU_ROW, ASSISTANT_MENU_ROWS, ASSISTANT_MENU_WIDTH, CHIP_RADIUS, Palette, SIDEBAR_TEXT_SIZE};

/// What stands between the provider and the model in an assistant's name.
const BETWEEN: &str = " · ";

/// What the user did in the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Chose the assistant with this id.
    Pick(String),
    /// Wants to set assistants up: the settings open on their page.
    Manage,
}

/// What the menu remembers while it is open.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Menu {
    /// What is typed into the search field.
    query: String,
    /// The assistant the arrow keys are on, counted along the assistants shown.
    cursor: usize,
    /// The menu has just opened, so the search field takes the keyboard.
    fresh: bool,
    /// The arrow keys moved, so the list follows the row they are on.
    moved: bool,
    /// The menu is shown open whatever was clicked, for a picture of it (`--sidebar assistants`).
    held: bool,
}

/// One line of the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Row<'a> {
    /// A provider's name over its models.
    Head(&'a str),
    /// An assistant to choose.
    Assistant {
        /// Its place among the assistants the app offers.
        place: usize,
        /// The model's name, or the name the user gave the assistant.
        name: &'a str,
        /// Who provides it; shown beside it while a search is on. Empty for a name of the user's own.
        provider: &'a str,
        /// Where in `name` the search's letters are.
        found: Vec<usize>,
    },
}

/// The provider and the model in an assistant's `label`; no provider for a name of the user's own.
fn parts(label: &str) -> (&str, &str) {
    label.split_once(BETWEEN).unwrap_or(("", label))
}

/// The lines of the menu for the assistants called `labels` while `query` is searched for.
// Human: without a search the models stand under their provider, the providers in the order the app offers
// them, and assistants the user named themselves come first, without a heading. A search looks in the whole
// name, so "ollama" finds every model of Ollama's; its finds are one list, the best first.
fn rows<'a, Label: AsRef<str>>(labels: &'a [Label], query: &str) -> Vec<Row<'a>> {
    let assistant = |place: usize, found: &[usize]| {
        let label = labels[place].as_ref();
        let (provider, name) = parts(label);
        // The search's letters are counted along the whole name; the model's part starts after the provider.
        let before = label.chars().count() - name.chars().count();
        Row::Assistant { place, name, provider, found: found.iter().filter(|at| **at >= before).map(|at| at - before).collect() }
    };
    if !query.trim().is_empty() {
        return fuzzy::rank(query, labels).iter().map(|(place, found)| assistant(*place, found)).collect();
    }
    let mut providers: Vec<&str> = Vec::new();
    for label in labels {
        let provider = parts(label.as_ref()).0;
        if !providers.contains(&provider) {
            providers.push(provider);
        }
    }
    providers.sort_by_key(|provider| !provider.is_empty());
    let mut out = Vec::new();
    for provider in providers {
        if !provider.is_empty() {
            out.push(Row::Head(provider));
        }
        out.extend((0..labels.len()).filter(|place| parts(labels[*place].as_ref()).0 == provider).map(|place| assistant(place, &[])));
    }
    out
}

/// Where the assistant called `current` stands among the assistants called `labels` as the menu lists them
/// without a search; the first place when it isn't there.
fn place_of(labels: &[&str], current: &str) -> usize {
    let listed = rows(labels, "");
    let assistants = listed.iter().filter_map(|row| if let Row::Assistant { place, .. } = row { Some(*place) } else { None });
    assistants.into_iter().position(|place| labels[place] == current).unwrap_or(0)
}

impl Menu {
    /// The menu opens over the assistants `all`: nothing is searched for, and the arrow keys start on the
    /// assistant in use, the one called `current`.
    pub fn opened(&mut self, all: &[Choice], current: &str) {
        let labels: Vec<&str> = all.iter().map(|choice| choice.label.as_str()).collect();
        *self = Menu { query: String::new(), cursor: place_of(&labels, current), fresh: true, moved: true, held: self.held };
    }

    /// Shows the menu open from now on, for a picture of it.
    pub fn hold_open(&mut self) {
        self.held = true;
    }

    /// Whether the menu is shown open for a picture.
    #[must_use]
    pub fn held_open(&self) -> bool {
        self.held
    }

    /// Draws the menu's inside for the assistants `all`, of which the one called `current` is in use. Returns
    /// what the user did.
    // Agent: MAIN-THREAD; drawn inside the popup of the message box's assistant button.
    pub fn show(&mut self, ui: &mut Ui, all: &[Choice], current: &str, palette: Palette) -> Option<Act> {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        ui.set_width(ASSISTANT_MENU_WIDTH);
        let labels: Vec<&str> = all.iter().map(|choice| choice.label.as_str()).collect();
        // Human: the list is as high as all the assistants need, up to a few rows, and stays so while a search
        // is on: the menu doesn't shrink and grow under the pointer with every letter.
        #[expect(clippy::cast_precision_loss, reason = "a handful of rows")]
        let height = rows(&labels, "").len().clamp(2, ASSISTANT_MENU_ROWS) as f32 * ASSISTANT_MENU_ROW;
        let field = self.field(ui, palette);
        if std::mem::take(&mut self.fresh) {
            field.request_focus();
        }
        let shown = rows(&labels, &self.query);
        let offered: Vec<usize> = shown.iter().filter_map(|row| if let Row::Assistant { place, .. } = row { Some(*place) } else { None }).collect();
        let mut act = self.keys(ui, &offered).and_then(|place| all.get(place)).map(|choice| Act::Pick(choice.id.clone()));
        ui.add_space(4.0);
        let searching = !self.query.trim().is_empty();
        egui::ScrollArea::vertical().id_salt("assistant-menu").max_height(height).min_scrolled_height(height).auto_shrink([false, false]).show(
            ui,
            |ui| {
                ui.set_min_height(height);
                if all.is_empty() {
                    said(ui, "No assistant is set up. Install Claude Code, or add a provider with a key.", palette);
                } else if shown.is_empty() {
                    said(ui, &format!("No assistant matches “{}”.", self.query.trim()), palette);
                }
                let mut at = 0;
                for row in &shown {
                    match row {
                        Row::Head(provider) => head(ui, provider, palette),
                        Row::Assistant { place, name, provider, found } => {
                            let on = at == self.cursor;
                            let look =
                                Line { name, beside: if searching { provider } else { "" }, found, searching, on, ticked: labels[*place] == current };
                            let response = line(ui, &look, palette);
                            if on && self.moved {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                act = all.get(*place).map(|choice| Act::Pick(choice.id.clone()));
                            }
                            at += 1;
                        }
                    }
                }
            },
        );
        self.moved = false;
        let counted = searching.then(|| format!("{} of {}", offered.len(), all.len()));
        if foot(ui, counted.as_deref(), palette) {
            act = Some(Act::Manage);
        }
        act
    }

    /// The search field at the top of the menu.
    fn field(&mut self, ui: &mut Ui, palette: Palette) -> egui::Response {
        let frame = egui::Frame::new()
            .fill(palette.background)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CHIP_RADIUS)
            .inner_margin(Margin { left: 28, right: 10, top: 6, bottom: 6 });
        let hint = RichText::new("Search assistants").size(SIDEBAR_TEXT_SIZE - 1.0).color(palette.muted);
        let edit = egui::TextEdit::singleline(&mut self.query)
            .frame(frame)
            .hint_text(hint)
            .font(FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0))
            .desired_width(ui.available_width());
        let response = ui.add(edit);
        let glass = Rect::from_center_size(pos2(response.rect.left() + 15.0, response.rect.center().y), Vec2::splat(12.0));
        icons::paint(ui.painter(), glass, "search", palette.muted);
        if response.changed() {
            (self.cursor, self.moved) = (0, true);
        }
        response
    }

    /// Takes the arrow keys and Return. Returns the place of the assistant Return chose, among `offered`.
    fn keys(&mut self, ui: &Ui, offered: &[usize]) -> Option<usize> {
        let pressed = |key: Key| ui.input(|input| input.key_pressed(key));
        self.cursor = self.cursor.min(offered.len().saturating_sub(1));
        if pressed(Key::ArrowDown) && self.cursor + 1 < offered.len() {
            (self.cursor, self.moved) = (self.cursor + 1, true);
        }
        if pressed(Key::ArrowUp) && self.cursor > 0 {
            (self.cursor, self.moved) = (self.cursor - 1, true);
        }
        if pressed(Key::Enter) { offered.get(self.cursor).copied() } else { None }
    }
}

/// A sentence in the list's place, when there is nothing to list.
fn said(ui: &mut Ui, words: &str, palette: Palette) {
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.add_space(10.0);
        ui.label(RichText::new(words).size(SIDEBAR_TEXT_SIZE - 1.0).color(palette.muted));
    });
}

/// A provider's name over its models.
fn head(ui: &mut Ui, provider: &str, palette: Palette) {
    let (row, _) = ui.allocate_exact_size(vec2(ASSISTANT_MENU_WIDTH, ASSISTANT_MENU_ROW), Sense::hover());
    let at = pos2(row.left() + 10.0, row.bottom() - 9.0);
    ui.painter().text(at, Align2::LEFT_CENTER, provider, FontId::proportional(SIDEBAR_TEXT_SIZE - 2.5), palette.muted.gamma_multiply(0.8));
}

/// How one assistant's line looks.
struct Line<'a> {
    name: &'a str,
    /// The provider's name at the line's right; empty when the line stands under its provider's heading.
    beside: &'a str,
    /// Where in `name` the search's letters are.
    found: &'a [usize],
    searching: bool,
    /// The arrow keys are on it.
    on: bool,
    /// It is the assistant in use.
    ticked: bool,
}

/// One assistant: its name, with the letters a search found standing out, and at the right a tick when it is
/// the one in use, or its provider while a search is on.
fn line(ui: &mut Ui, look: &Line<'_>, palette: Palette) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(vec2(ASSISTANT_MENU_WIDTH, ASSISTANT_MENU_ROW), Sense::click());
    if !ui.is_rect_visible(row) {
        return response;
    }
    let fill = match (look.on, response.hovered()) {
        (true, _) => palette.hover,
        (false, true) => palette.hover.gamma_multiply(0.6),
        (false, false) => Color32::TRANSPARENT,
    };
    ui.painter().rect_filled(row.shrink2(vec2(0.0, 1.0)), 6.0, fill);
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 1.0);
    let mut right = row.right() - 10.0;
    if look.ticked {
        icons::paint(ui.painter(), Rect::from_center_size(pos2(right - 6.0, row.center().y), Vec2::splat(12.0)), "check", theme::AGENT);
        right -= 20.0;
    }
    if !look.beside.is_empty() {
        let small = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.5);
        let beside = ui.painter().layout_no_wrap(look.beside.to_owned(), small, palette.muted.gamma_multiply(0.8));
        // The provider gives way to a long model name.
        if beside.size().x < row.width() * 0.4 {
            right -= beside.size().x;
            ui.painter().galley(pos2(right, row.center().y - beside.size().y / 2.0), beside, palette.muted);
            right -= 8.0;
        }
    }
    let left = row.left() + 10.0;
    let mut job = LayoutJob::default();
    for (place, letter) in look.name.chars().enumerate() {
        let bright = !look.searching || look.found.binary_search(&place).is_ok();
        let color = if bright { palette.foreground } else { palette.muted };
        job.append(letter.encode_utf8(&mut [0; 4]), 0.0, TextFormat::simple(font.clone(), color));
    }
    job.wrap = TextWrapping::truncate_at_width((right - left).max(40.0));
    let name = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.painter().galley(pos2(left, row.center().y - name.size().y / 2.0), name, palette.foreground);
    response
}

/// The line under the list: the way to the settings' assistants, or while a search is on how many assistants it
/// found (`counted`); at the right the keys that work here. Returns whether the way to the settings was taken.
fn foot(ui: &mut Ui, counted: Option<&str>, palette: Palette) -> bool {
    ui.add_space(4.0);
    let (bar, _) = ui.allocate_exact_size(vec2(ASSISTANT_MENU_WIDTH, ASSISTANT_MENU_ROW + 2.0), Sense::hover());
    ui.painter().hline(bar.x_range(), bar.top(), Stroke::new(1.0, palette.outline));
    let font = FontId::proportional(SIDEBAR_TEXT_SIZE - 2.5);
    let middle = bar.center().y + 2.0;
    let quiet = palette.muted.gamma_multiply(0.8);
    ui.painter().text(pos2(bar.right() - 10.0, middle), Align2::RIGHT_CENTER, "↑ ↓ to move  ·  Return to choose", font.clone(), quiet);
    if let Some(counted) = counted {
        ui.painter().text(pos2(bar.left() + 10.0, middle), Align2::LEFT_CENTER, counted, font, quiet);
        return false;
    }
    let words = ui.painter().layout_no_wrap("Manage…".to_owned(), font, palette.muted);
    let link = Rect::from_min_size(pos2(bar.left() + 4.0, middle - 11.0), vec2(words.size().x + 30.0, 22.0));
    let response = ui.interact(link, ui.scope_id().with("manage-assistants"), Sense::click());
    let colour = if response.hovered() { palette.foreground } else { palette.muted };
    if response.hovered() {
        ui.painter().rect_filled(link, 6.0, palette.hover.gamma_multiply(0.6));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(link.left() + 12.0, middle), Vec2::splat(11.0)), "settings", colour);
    ui.painter().galley(pos2(link.left() + 22.0, middle - words.size().y / 2.0), words, colour);
    response.on_hover_text("Set up assistants in the settings").clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABELS: [&str; 6] = [
        "Claude Code · Opus 5.5",
        "Claude Code · Fable 5.2",
        "My fast one",
        "Ollama Cloud · kimi-k3.1-pro",
        "Ollama Cloud · deepseek-v4-pro",
        "Grok · Grok 4.6",
    ];

    /// The lines as words: a heading as its provider in brackets, an assistant as its name.
    fn told(rows: &[Row<'_>]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Head(provider) => format!("[{provider}]"),
                Row::Assistant { name, .. } => (*name).to_owned(),
            })
            .collect()
    }

    #[test]
    fn the_models_stand_under_their_provider_and_the_users_own_names_come_first() {
        assert_eq!(
            told(&rows(&LABELS, "  ")),
            ["My fast one", "[Claude Code]", "Opus 5.5", "Fable 5.2", "[Ollama Cloud]", "kimi-k3.1-pro", "deepseek-v4-pro", "[Grok]", "Grok 4.6"]
        );
        assert_eq!((parts("Ollama Cloud · gpt-oss:120b"), parts("Mine")), (("Ollama Cloud", "gpt-oss:120b"), ("", "Mine")));
        assert_eq!(told(&rows(&[] as &[&str], "")), [] as [String; 0]);
    }

    #[test]
    fn a_search_is_one_list_with_the_best_find_first_and_its_letters_marked_in_the_model() {
        let found = rows(&LABELS, "kpro");
        assert_eq!(told(&found), ["kimi-k3.1-pro", "deepseek-v4-pro"]);
        let Row::Assistant { place, provider, found: letters, .. } = &found[0] else { panic!("an assistant") };
        // A "k" of the model's name and the "pro" at its end, counted along the model's name alone.
        assert_eq!((*place, *provider, letters.as_slice()), (3, "Ollama Cloud", [5, 10, 11, 12].as_slice()));
        // A provider's name finds all its models; its letters are not in the model's name.
        let ollama = rows(&LABELS, "ollama");
        assert_eq!(told(&ollama), ["kimi-k3.1-pro", "deepseek-v4-pro"]);
        assert!(matches!(&ollama[0], Row::Assistant { found, .. } if found.is_empty()));
        assert_eq!(told(&rows(&LABELS, "zzz")), [] as [String; 0]);
    }

    #[test]
    fn the_arrow_keys_start_on_the_assistant_in_use() {
        // The menu lists the user's own name first, so the fourth assistant offered stands fifth... and the
        // third, the user's own, first.
        assert_eq!((place_of(&LABELS, "Ollama Cloud · kimi-k3.1-pro"), place_of(&LABELS, "My fast one"), place_of(&LABELS, "gone")), (3, 0, 0));
        assert_eq!(place_of(&LABELS, "Claude Code · Opus 5.5"), 1);
        let mut menu = Menu { query: "old".to_owned(), cursor: 7, fresh: false, moved: false, held: false };
        menu.opened(&[], "");
        assert_eq!(menu, Menu { query: String::new(), cursor: 0, fresh: true, moved: true, held: false });
        menu.hold_open();
        menu.opened(&[], "");
        assert!(menu.held_open());
    }
}
