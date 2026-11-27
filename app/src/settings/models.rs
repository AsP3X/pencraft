//! The window a provider's models are chosen in: the list the provider answered with, a field that finds a
//! model by a few of its letters, and a tick before every model the chat is to offer.

use std::collections::BTreeSet;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{Align, Align2, Color32, FontId, Key, Layout, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};

use super::parts::{Kind, button, search_field, strong};
use crate::fuzzy;
use crate::icons;
use crate::sidebar::icon_button;
use crate::theme::{
    self, CHECKING, DIALOG_PADDING, DIALOG_TITLE_SIZE, FIELD_HEIGHT, MODEL_PICKER_BACKDROP, MODEL_PICKER_ROW, MODEL_PICKER_SIZE, Palette,
    SETTINGS_CARD_RADIUS, SETTINGS_FOOTER, SETTINGS_RADIUS,
};

/// Models shown by `--settings models`, where no provider is asked: a picture of the window needs a list.
const SAMPLES: [&str; 14] = [
    "deepseek-v4-pro",
    "deepseek-v4.1-flash",
    "gemma4:31b",
    "glm-5.2",
    "glm-5.3",
    "glm-5.3-flash",
    "gpt-oss:120b",
    "gpt-oss:20b",
    "kimi-k2.7-code",
    "kimi-k3",
    "kimi-k3.1-pro",
    "minimax-m3",
    "mistral-large-4",
    "nemotron-3-ultra",
];

/// How far the list is.
#[derive(Clone, Debug, PartialEq, Eq)]
enum State {
    /// The provider has been asked and hasn't answered yet.
    Loading,
    /// The provider's models are there.
    Listed,
    /// The provider couldn't be asked or refused; what to tell the user.
    Failed(String),
}

/// What the user decided in the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Answer {
    /// Take the ticked models into the form.
    Apply,
    /// Leave the form as it was.
    Cancel,
    /// Ask the provider for its models again.
    Reload,
}

/// The window, while it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Picker {
    /// The provider's name, for the window's second line.
    provider: String,
    /// What is typed into the search field.
    query: String,
    /// The provider's models, in the order it gave them.
    offered: Vec<String>,
    /// The models that are ticked.
    chosen: BTreeSet<String>,
    state: State,
    /// The models the search shows, best first: each one's place in `offered` and the letters found in it.
    shown: Vec<(usize, Vec<usize>)>,
    /// The query `shown` was worked out for; `None` when it has to be worked out anew.
    ranked: Option<String>,
    /// The row the arrow keys are on.
    cursor: usize,
    /// The arrow keys moved, so the list follows the row they are on.
    moved: bool,
    /// The arrow keys have been used, so the row they are on shows also without a search.
    keyed: bool,
    /// The window has just opened, so the search field takes the keyboard.
    fresh: bool,
}

impl Picker {
    /// The window for the provider called `provider`, with the models it `offered` so far and the ones already
    /// in the form (`chosen`) ticked.
    pub(super) fn new(provider: &str, offered: &[String], chosen: impl IntoIterator<Item = String>) -> Picker {
        Picker {
            provider: if provider.trim().is_empty() { "the provider".to_owned() } else { provider.trim().to_owned() },
            query: String::new(),
            offered: offered.to_vec(),
            chosen: chosen.into_iter().collect(),
            state: State::Listed,
            shown: Vec::new(),
            ranked: None,
            cursor: 0,
            moved: false,
            keyed: false,
            fresh: true,
        }
    }

    /// The window as `--settings models` shows it: a list that asks no provider, and a search in it when
    /// `query` isn't empty.
    pub(super) fn sample(query: &str) -> Picker {
        let offered: Vec<String> = SAMPLES.iter().map(|id| (*id).to_owned()).collect();
        let mut picker = Picker::new("Ollama Cloud", &offered, ["glm-5.3", "kimi-k3"].map(str::to_owned));
        query.clone_into(&mut picker.query);
        picker
    }

    /// The provider is being asked.
    pub(super) fn loading(&mut self) {
        self.state = State::Loading;
    }

    /// Whether the window waits for the provider's answer.
    pub(super) fn is_loading(&self) -> bool {
        self.state == State::Loading
    }

    /// The provider answered with the models `offered`.
    pub(super) fn listed(&mut self, offered: Vec<String>) {
        self.offered = offered;
        self.state = State::Listed;
        self.ranked = None;
        self.cursor = 0;
    }

    /// The provider couldn't be asked, or refused: `why`, as the user is told.
    pub(super) fn failed(&mut self, why: String) {
        self.state = State::Failed(why);
    }

    /// The provider's models, as the window has them.
    pub(super) fn offered(&self) -> &[String] {
        &self.offered
    }

    /// The models that are ticked.
    pub(super) fn chosen(&self) -> &BTreeSet<String> {
        &self.chosen
    }

    /// How many of the provider's models are ticked.
    fn ticked(&self) -> usize {
        self.offered.iter().filter(|id| self.chosen.contains(*id)).count()
    }

    /// Works out which models the search shows, when what is typed has changed.
    fn rank(&mut self) {
        if self.ranked.as_deref() != Some(self.query.as_str()) {
            self.shown = fuzzy::rank(&self.query, &self.offered);
            self.ranked = Some(self.query.clone());
            self.cursor = 0;
        }
    }

    /// Ticks the model at `row` of the shown ones, or takes its tick away.
    fn toggle(&mut self, row: usize) {
        let Some(id) = self.shown.get(row).and_then(|(place, _)| self.offered.get(*place)) else { return };
        if !self.chosen.remove(id) {
            self.chosen.insert(id.clone());
        }
    }

    /// Takes the arrow keys and Return: the arrows move through the shown models, Return ticks the one they
    /// are on. Returns whether Return was pressed.
    fn keys(&mut self, ui: &Ui) -> bool {
        let (up, down, enter) = ui.input(|input| (input.key_pressed(Key::ArrowUp), input.key_pressed(Key::ArrowDown), input.key_pressed(Key::Enter)));
        let last = self.shown.len().saturating_sub(1);
        if down && self.cursor < last {
            self.cursor += 1;
        }
        if up {
            self.cursor = self.cursor.saturating_sub(1);
        }
        self.cursor = self.cursor.min(last);
        self.moved = up || down;
        self.keyed |= self.moved;
        if enter && self.state == State::Listed {
            self.toggle(self.cursor);
        }
        enter
    }

    /// Draws the window over everything else. Returns what the user decided, once they have.
    // Human: Escape and a click beside the window leave it like Cancel; the settings window under it stays.
    // Agent: MAIN-THREAD; WRITES only this window's own state.
    pub(super) fn show(&mut self, ctx: &egui::Context, palette: Palette) -> Option<Answer> {
        self.rank();
        let screen = ctx.content_rect();
        let size = vec2(
            MODEL_PICKER_SIZE[0].min(screen.width() - theme::MODEL_PICKER_MARGIN),
            MODEL_PICKER_SIZE[1].min(screen.height() - theme::MODEL_PICKER_MARGIN),
        );
        let frame = egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(SETTINGS_RADIUS);
        let id = egui::Id::unique("model-picker");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(MODEL_PICKER_BACKDROP))
            .show(ctx, |ui| {
                let (window, _) = ui.allocate_exact_size(size.max(Vec2::ZERO), Sense::hover());
                self.draw(ui, window, palette)
            });
        if modal.should_close() { Some(Answer::Cancel) } else { modal.inner }
    }

    /// Draws the window's parts in `window`: its head, the search field, the list and the bar of buttons.
    fn draw(&mut self, ui: &mut Ui, window: Rect, palette: Palette) -> Option<Answer> {
        let pad = f32::from(DIALOG_PADDING);
        let inner = window.shrink2(vec2(pad, 0.0));
        let mut answer = self.head(
            ui,
            Rect::from_min_max(pos2(inner.left(), window.top() + pad), pos2(inner.right(), window.top() + pad + theme::MODEL_PICKER_HEAD[0])),
            palette,
        );
        let search = Rect::from_min_size(pos2(inner.left(), window.top() + pad + theme::MODEL_PICKER_HEAD[1]), vec2(inner.width(), FIELD_HEIGHT));
        let field = ui.scope_builder(UiBuilder::new().max_rect(search), |ui| search_field(ui, &mut self.query, "Search models", palette)).inner;
        if std::mem::take(&mut self.fresh) {
            field.request_focus();
        }
        self.rank();
        // Human: Return ticks a model and the field keeps the keyboard, so the next search can be typed at once.
        if self.keys(ui) {
            field.request_focus();
        }
        let bar = Rect::from_min_max(pos2(window.left() + 1.0, window.bottom() - SETTINGS_FOOTER), window.max - vec2(1.0, 1.0));
        let list = Rect::from_min_max(
            pos2(inner.left(), search.bottom() + theme::MODEL_PICKER_HEAD[2]),
            pos2(inner.right(), bar.top() - theme::MODEL_PICKER_HEAD[3]),
        );
        ui.painter().rect(list, SETTINGS_CARD_RADIUS, palette.card, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
        let within = list.shrink(1.0);
        ui.scope_builder(UiBuilder::new().max_rect(within), |ui| {
            ui.set_clip_rect(within.intersect(ui.clip_rect()));
            match self.state.clone() {
                State::Loading => self.waiting(ui, within, palette),
                State::Failed(why) => {
                    if said(ui, within, "triangle-alert", CHECKING, &why, Some("Try again"), palette) {
                        answer = Some(Answer::Reload);
                    }
                }
                State::Listed if self.shown.is_empty() => {
                    let none = format!("No model matches “{}”.", self.query.trim());
                    if said(ui, within, "search", palette.muted, &none, Some("Clear search"), palette) {
                        self.query.clear();
                        field.request_focus();
                    }
                }
                State::Listed => self.rows(ui, palette),
            }
        });
        ui.painter().hline(bar.x_range(), bar.top(), Stroke::new(1.0, palette.outline));
        let buttons = Rect::from_min_max(pos2(inner.left(), bar.top()), pos2(inner.right(), bar.bottom()));
        answer = self.foot(ui, buttons, palette).or(answer);
        answer
    }

    /// The window's head: what it is for, whose models these are, and at the right the ways to load the list
    /// again and to close.
    fn head(&self, ui: &mut Ui, rect: Rect, palette: Palette) -> Option<Answer> {
        ui.painter().text(
            rect.left_top() + vec2(0.0, theme::MODEL_PICKER_TITLE[0]),
            Align2::LEFT_CENTER,
            "Choose models",
            strong(DIALOG_TITLE_SIZE),
            palette.foreground,
        );
        let under = match &self.state {
            State::Loading => format!("Asking {}…", self.provider),
            State::Failed(_) => format!("{} didn't give its models", self.provider),
            State::Listed => format!("{} · {}", self.provider, super::counted(self.offered.len(), "model")),
        };
        ui.painter().text(
            rect.left_top() + vec2(0.0, theme::MODEL_PICKER_TITLE[1]),
            Align2::LEFT_CENTER,
            under,
            FontId::proportional(theme::MODEL_PICKER_TITLE[2]),
            palette.muted,
        );
        let mut answer = None;
        let corner = Rect::from_min_max(
            pos2(rect.right() - theme::MODEL_PICKER_CORNER[0], rect.top() - theme::MODEL_PICKER_CORNER[1]),
            pos2(rect.right() + theme::MODEL_PICKER_CORNER[1], rect.top() + theme::MODEL_PICKER_CORNER[2]),
        );
        ui.scope_builder(UiBuilder::new().max_rect(corner).layout(Layout::right_to_left(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = theme::MODEL_PICKER_CORNER[3];
            if icon_button(ui, "x", Vec2::splat(theme::MODEL_PICKER_CORNER[4]), false, "Close without changes", palette).clicked() {
                answer = Some(Answer::Cancel);
            }
            // Human: a provider's list changes (a model pulled, a new release); this asks for it again.
            if self.state != State::Loading
                && icon_button(ui, "refresh-cw", Vec2::splat(theme::MODEL_PICKER_CORNER[4]), false, "Load the list again", palette).clicked()
            {
                answer = Some(Answer::Reload);
            }
        });
        answer
    }

    /// What the list's place shows while the provider is asked.
    fn waiting(&self, ui: &mut Ui, within: Rect, palette: Palette) {
        let middle = within.center();
        ui.put(
            Rect::from_center_size(middle - vec2(0.0, theme::MODEL_PICKER_WAIT[0]), Vec2::splat(theme::MODEL_PICKER_WAIT[1])),
            egui::Spinner::new().size(theme::MODEL_PICKER_WAIT[1]).color(palette.muted),
        );
        let words = format!("Asking {} for its models…", self.provider);
        ui.painter().text(
            middle + vec2(0.0, theme::MODEL_PICKER_WAIT[2]),
            Align2::CENTER_CENTER,
            words,
            FontId::proportional(theme::SETTINGS_CONTROL_TEXT),
            palette.muted,
        );
    }

    /// The shown models, one row each: a tick box, and the model's name with the letters the search found
    /// standing out.
    fn rows(&mut self, ui: &mut Ui, palette: Palette) {
        let searching = !self.query.trim().is_empty();
        let mut clicked = None;
        egui::ScrollArea::vertical().id_salt("model-picker-rows").auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(theme::SETTINGS_HINT_GAP);
            for (row, (place, found)) in self.shown.iter().enumerate() {
                let Some(id) = self.offered.get(*place) else { continue };
                let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), MODEL_PICKER_ROW), Sense::click());
                if response.clicked() {
                    clicked = Some(row);
                }
                if self.moved && row == self.cursor {
                    response.scroll_to_me(None);
                }
                if !ui.is_rect_visible(rect) {
                    continue;
                }
                let ticked = self.chosen.contains(id);
                // Human: the row the arrow keys are on is filled like a hovered one, a little stronger.
                let fill = match (row == self.cursor && (searching || self.keyed), response.hovered()) {
                    (true, _) => palette.hover,
                    (false, true) => palette.hover.gamma_multiply(theme::OUTLINE_SOFT),
                    (false, false) => Color32::TRANSPARENT,
                };
                ui.painter().rect_filled(
                    rect.shrink2(vec2(theme::MODEL_PICKER_ROW_LOOK[0], theme::MODEL_PICKER_ROW_LOOK[1])),
                    theme::MODEL_PICKER_ROW_LOOK[2],
                    fill,
                );
                let middle = rect.center().y;
                tick_box(
                    ui,
                    Rect::from_center_size(pos2(rect.left() + theme::MODEL_PICKER_ROW_LOOK[3], middle), Vec2::splat(theme::MODEL_PICKER_TICK[0])),
                    ticked,
                    palette,
                );
                let left = rect.left() + theme::MODEL_PICKER_ROW_LOOK[4];
                let name = name(ui, id, found, searching, rect.right() - left - theme::MODEL_PICKER_ROW_LOOK[5], palette);
                ui.painter().galley(pos2(left, middle - name.size().y / 2.0), name, palette.foreground);
            }
            ui.add_space(theme::SETTINGS_HINT_GAP);
        });
        if let Some(row) = clicked {
            self.cursor = row;
            self.toggle(row);
        }
    }

    /// The bar under the list: how many models are ticked and the ways to tick all shown or none on the left,
    /// Cancel and the button that takes the choice on the right.
    fn foot(&mut self, ui: &mut Ui, rect: Rect, palette: Palette) -> Option<Answer> {
        let mut answer = None;
        let ticked = self.ticked();
        let listed = self.state == State::Listed;
        ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::left_to_right(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = theme::SETTINGS_HINT_GAP;
            let words = if ticked == 0 { "None selected".to_owned() } else { format!("{ticked} selected") };
            ui.add(egui::Label::new(egui::RichText::new(words).size(theme::SETTINGS_CONTROL_TEXT).color(palette.muted)).selectable(false));
            ui.add_space(theme::SETTINGS_LINE_GAP);
            let all = if self.query.trim().is_empty() { "Select all" } else { "Select shown" };
            let open = self.shown.iter().filter_map(|(place, _)| self.offered.get(*place)).any(|id| !self.chosen.contains(id));
            if button(ui, all, None, Kind::Quiet, listed && open, palette).clicked() {
                self.chosen.extend(self.shown.iter().filter_map(|(place, _)| self.offered.get(*place)).cloned());
            }
            if button(ui, "Clear", None, Kind::Quiet, listed && ticked > 0, palette).clicked() {
                self.chosen.retain(|id| !self.offered.contains(id));
            }
        });
        ui.scope_builder(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)), |ui| {
            ui.spacing_mut().item_spacing.x = theme::SETTINGS_INLINE_GAP;
            let take = if ticked == 0 { "Done".to_owned() } else { format!("Use {}", super::counted(ticked, "model")) };
            if button(ui, &take, None, Kind::Primary, listed, palette).clicked() {
                answer = Some(Answer::Apply);
            }
            if button(ui, "Cancel", None, Kind::Outline, true, palette).clicked() {
                answer = Some(Answer::Cancel);
            }
        });
        answer
    }
}

/// A tick box in `rect`: filled with a tick when `ticked`, an empty outline otherwise.
fn tick_box(ui: &Ui, rect: Rect, ticked: bool, palette: Palette) {
    if ticked {
        ui.painter().rect_filled(rect, theme::MODEL_PICKER_TICK[1], palette.primary);
        icons::paint(ui.painter(), Rect::from_center_size(rect.center(), Vec2::splat(theme::MODEL_PICKER_TICK[2])), "check", palette.primary_text);
    } else {
        ui.painter().rect(
            rect,
            theme::MODEL_PICKER_TICK[1],
            palette.card,
            Stroke::new(1.0, palette.muted.gamma_multiply(theme::MODEL_PICKER_TICK[3])),
            StrokeKind::Inside,
        );
    }
}

/// The parts of a name `id` as a list shows them: runs of letters, each either one the search found or not.
fn runs(id: &str, found: &[usize]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for (place, letter) in id.chars().enumerate() {
        let hit = found.binary_search(&place).is_ok();
        match out.last_mut() {
            Some((run, same)) if *same == hit => run.push(letter),
            _ => out.push((letter.to_string(), hit)),
        }
    }
    out
}

/// A model's name on one line of at most `width`: while a search is on, the letters it `found` are in the
/// main colour and the rest is quiet; without one the whole name is in the main colour.
fn name(ui: &Ui, id: &str, found: &[usize], searching: bool, width: f32, palette: Palette) -> std::sync::Arc<egui::Galley> {
    let font = FontId::monospace(theme::SETTINGS_ROW_NAME[1]);
    let mut job = LayoutJob::default();
    for (run, hit) in runs(id, found) {
        let color = if hit || !searching { palette.foreground } else { palette.muted };
        job.append(&run, 0.0, TextFormat::simple(font.clone(), color));
    }
    job.wrap = TextWrapping::truncate_at_width(width.max(theme::SETTINGS_LINE_LEAST));
    ui.fonts_mut(|fonts| fonts.layout_job(job))
}

/// Something said in the middle of the list's place: an icon, a sentence, and a button under it when there is
/// something to do. Returns whether the button was clicked.
fn said(ui: &mut Ui, within: Rect, icon: &'static str, tint: Color32, words: &str, action: Option<&str>, palette: Palette) -> bool {
    let width = (within.width() - theme::MODEL_PICKER_EMPTY[0]).max(theme::MODEL_PICKER_EMPTY[1]);
    let text = ui.painter().layout(words.to_owned(), FontId::proportional(theme::SETTINGS_CONTROL_TEXT), palette.muted, width);
    let height = theme::MODEL_PICKER_EMPTY[2]
        + theme::MODEL_PICKER_EMPTY[3]
        + text.size().y
        + if action.is_some() { theme::MODEL_PICKER_EMPTY[4] + FIELD_HEIGHT } else { 0.0 };
    let top = within.center().y - height / 2.0;
    icons::paint(
        ui.painter(),
        Rect::from_center_size(pos2(within.center().x, top + theme::MODEL_PICKER_EMPTY[5]), Vec2::splat(theme::MODEL_PICKER_EMPTY[6])),
        icon,
        tint,
    );
    let at = pos2(within.center().x - text.size().x / 2.0, top + theme::MODEL_PICKER_EMPTY[7]);
    let under = at.y + text.size().y + theme::MODEL_PICKER_EMPTY[4];
    ui.painter().galley(at, text, palette.muted);
    let Some(action) = action else { return false };
    let place = Rect::from_min_size(pos2(within.left(), under), vec2(within.width(), FIELD_HEIGHT));
    ui.scope_builder(UiBuilder::new().max_rect(place).layout(Layout::top_down(Align::Center)), |ui| {
        button(ui, action, None, Kind::Outline, true, palette).clicked()
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn the_search_shows_the_likely_models_and_ticks_stay_through_it() {
        let mut picker =
            Picker::new(" Mine ", &ids(&["kimi-k3", "glm-5.3", "kimi-k2.7-code", "gpt-oss:20b"]), ["glm-5.3".to_owned(), "typed-by-hand".to_owned()]);
        picker.rank();
        assert_eq!((picker.provider.as_str(), picker.shown.len(), picker.ticked()), ("Mine", 4, 1));
        // A search narrows the list and says where its letters are; the tick set is untouched.
        "kk3".clone_into(&mut picker.query);
        picker.rank();
        assert_eq!(picker.shown, [(0, vec![0, 5, 6])]);
        picker.toggle(0);
        picker.toggle(7);
        assert!(picker.chosen.contains("kimi-k3") && picker.ticked() == 2);
        // What is typed anew starts at the first row again.
        picker.cursor = 3;
        "kimi".clone_into(&mut picker.query);
        picker.rank();
        assert_eq!((picker.shown.iter().map(|(place, _)| *place).collect::<Vec<_>>(), picker.cursor), (vec![0, 2], 0));
        picker.toggle(0);
        assert!(!picker.chosen.contains("kimi-k3") && picker.chosen.contains("typed-by-hand"));
        // A new answer of the provider replaces the list and is ranked again.
        picker.loading();
        assert!(picker.is_loading());
        picker.listed(ids(&["kimi-k3.1-pro"]));
        picker.rank();
        assert_eq!((picker.shown.len(), picker.offered(), picker.is_loading()), (1, ids(&["kimi-k3.1-pro"]).as_slice(), false));
        picker.failed("No.".to_owned());
        assert_eq!(picker.state, State::Failed("No.".to_owned()));
    }

    #[test]
    fn a_name_is_cut_into_found_and_other_letters() {
        assert_eq!(
            runs("gpt-4o", &[0, 1, 2, 4]),
            [("gpt".to_owned(), true), ("-".to_owned(), false), ("4".to_owned(), true), ("o".to_owned(), false)]
        );
        assert_eq!(runs("glm", &[]), [("glm".to_owned(), false)]);
        assert_eq!(runs("", &[]), []);
        // The icons the window draws are bundled.
        for icon in ["check", "search", "refresh-cw", "triangle-alert", "x", "list-checks"] {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }

    #[test]
    fn the_window_draws_in_every_state() {
        let ctx = egui::Context::default();
        let draw = |picker: &mut Picker| {
            let mut answer = None;
            let mut drawn = ctx.run_ui(egui::RawInput::default(), |ui| answer = picker.show(ui.ctx(), crate::theme::palette(true)));
            // The font picture this frame made is for a screen; there is none here.
            drawn.textures_delta.clear();
            answer
        };
        let mut picker = Picker::sample("");
        assert_eq!((draw(&mut picker), picker.shown.len(), picker.ticked()), (None, SAMPLES.len(), 2));
        let mut picker = Picker::sample("kimi");
        assert_eq!((draw(&mut picker), picker.shown.len()), (None, 3));
        let mut picker = Picker::sample("no such model");
        assert_eq!((draw(&mut picker), picker.shown.len()), (None, 0));
        picker.loading();
        assert_eq!(draw(&mut picker), None);
        picker.failed("The provider couldn't be reached.".to_owned());
        assert_eq!(draw(&mut picker), None);
    }
}
