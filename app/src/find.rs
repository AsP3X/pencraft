//! Find layers: a small search island that looks through the layers' names, texts and ids.
// Human: "Find layers" (⌘F, or the magnifier on the tool rail): a field with a counter and a
// close button, under it the layers that match. ↑ and ↓ step through them and bring each into view on the
// canvas, where all matches are outlined; ↵ selects the current one and closes, esc or a click elsewhere closes.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align2, Color32, FontId, Key, Pos2, Rect, Sense, Stroke, pos2, vec2};
use pen_doc::layout;
use pen_doc::model::Node;

use crate::editor::{Editor, FIT_PADDING};
use crate::icons;
use crate::theme::{self, ACCENT, FIND_BAR, FIND_PADDING, FIND_RADIUS, FIND_ROW, FIND_ROWS, FIND_TEXT, FIND_WIDTH};

/// The longest text searched for (longer input is cut off).
const LONGEST: usize = 200;

/// What of a layer matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// Its name, as the layers list shows it.
    Name,
    /// The text it holds.
    Text,
    /// Its id.
    Id,
}

/// A layer that matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The layer's id.
    pub id: String,
    /// Its kind's icon.
    pub icon: &'static str,
    /// What the line shows: the name, or for a match in the text the text's first line with the match.
    pub label: String,
    /// What matched.
    pub field: Field,
}

/// The layers under `nodes` matching `query`, in the order of the layers list: parents before their children.
/// The search ignores case and looks at the name first, then the text, then the id.
#[must_use]
pub fn search(nodes: &[Node], query: &str) -> Vec<Found> {
    let wanted = query.trim().to_lowercase();
    let mut found = Vec::new();
    if !wanted.is_empty() {
        collect(nodes, &wanted, &mut found);
    }
    found
}

fn collect(nodes: &[Node], wanted: &str, found: &mut Vec<Found>) {
    for node in nodes {
        let name = node.display_name();
        let text = node.content.as_deref().unwrap_or_default();
        let hit = if name.to_lowercase().contains(wanted) {
            Some((Field::Name, name))
        } else if let Some(line) = text.lines().find(|line| line.to_lowercase().contains(wanted)) {
            Some((Field::Text, line.trim().to_owned()))
        } else if node.id.to_lowercase().contains(wanted) {
            Some((Field::Id, name))
        } else {
            None
        };
        if let Some((field, label)) = hit {
            found.push(Found { id: node.id.clone(), icon: crate::panels::kind_icon(&node.kind), label, field });
        }
        collect(node.kids(), wanted, found);
    }
}

/// The search island's state.
#[derive(Default)]
pub struct Find {
    open: bool,
    query: String,
    /// The matches, and the query and document state they were found for.
    results: Vec<Found>,
    searched: Option<(String, u64, usize)>,
    /// The match the arrows are on.
    current: Option<usize>,
}

impl Find {
    /// Opens the search for `editor`'s document; nothing stays selected.
    pub fn open(&mut self, editor: &mut Editor) {
        editor.selection.clear();
        self.open = true;
        self.searched = None;
    }

    /// For a picture of the app (`--find`): the search as it is with `query` typed.
    pub fn pose(&mut self, query: &str) {
        query.clone_into(&mut self.query);
    }

    /// Whether the search is showing.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Closes the search when no document is in front.
    pub fn hide(&mut self) {
        self.open = false;
    }

    /// Closes the search and takes its marks off the canvas.
    pub fn close(&mut self, editor: &mut Editor) {
        self.open = false;
        editor.spotlight = (Vec::new(), None);
    }

    /// Looks again when the query or the document changed; the first match becomes the current one.
    fn refresh(&mut self, editor: &Editor, tab: usize) -> bool {
        let key = (self.query.trim().to_owned(), editor.generation, tab);
        if self.searched.as_ref() == Some(&key) {
            return false;
        }
        let asked_anew = self.searched.as_ref().is_none_or(|(query, _, at)| *query != key.0 || *at != tab);
        let kept = self.current.and_then(|i| self.results.get(i)).map(|f| f.id.clone());
        self.results = search(&editor.doc.children, &key.0);
        self.current =
            if asked_anew { (!self.results.is_empty()).then_some(0) } else { kept.and_then(|id| self.results.iter().position(|f| f.id == id)) };
        self.searched = Some(key);
        asked_anew
    }

    /// Draws the island at `at` when it is open and handles its keys. `view` is the canvas on screen and `tab`
    /// the open document's place among the tabs.
    // Agent: MAIN-THREAD; takes keyboard focus while open; WRITES editor.selection, editor.spotlight and the camera.
    pub fn show(&mut self, ctx: &egui::Context, at: Pos2, view: Rect, editor: &mut Editor, tab: usize, dark: bool) {
        if !self.open {
            return;
        }
        let palette = theme::palette(dark);
        let mut moved = self.refresh(editor, tab);
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        // Human: the arrows go round: down from the last match is the first, up from the first the last.
        let count = self.results.len();
        if count > 0 && (up || down) {
            self.current = Some(match (self.current, down) {
                (Some(i), true) => (i + 1) % count,
                (Some(i), false) => (i + count - 1) % count,
                (None, true) => 0,
                (None, false) => count - 1,
            });
            moved = true;
        }
        let mut close = escape;
        let mut clicked = None;
        let area = egui::Area::new(egui::Id::unique("find-layers")).fixed_pos(at).order(egui::Order::Foreground).fade_in(false).show(ctx, |ui| {
            egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(FIND_RADIUS).show(ui, |ui| {
                ui.set_width(FIND_WIDTH);
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                close |= self.bar(ui, palette);
                ui.painter().hline(ui.max_rect().x_range(), ui.cursor().top(), Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
                clicked = self.body(ui, moved, palette);
            });
        });
        if let Some(index) = clicked {
            self.current = Some(index);
            moved = true;
        }
        editor.spotlight =
            (self.results.iter().map(|f| f.id.clone()).collect(), self.current.and_then(|i| self.results.get(i)).map(|f| f.id.clone()));
        if moved && let Some(id) = editor.spotlight.1.clone() {
            bring_into_view(editor, &id, view);
        }
        // Human: ↵ selects the current match, or the first, and closes.
        if enter && let Some(found) = self.current.and_then(|i| self.results.get(i)).or(self.results.first()) {
            editor.selection = vec![found.id.clone()];
            close = true;
        }
        let away = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p)));
        if close || away {
            self.close(editor);
        }
    }

    /// The field with the magnifier, the counter and the close button; `true` when close was pressed.
    fn bar(&mut self, ui: &mut egui::Ui, palette: theme::Palette) -> bool {
        let (bar, _) = ui.allocate_exact_size(vec2(FIND_WIDTH, FIND_BAR), Sense::hover());
        icons::paint(ui.painter(), Rect::from_center_size(pos2(bar.left() + 23.0, bar.center().y), egui::Vec2::splat(14.0)), "search", palette.muted);
        let button = Rect::from_center_size(pos2(bar.right() - 24.0, bar.center().y), egui::Vec2::splat(24.0));
        let response = ui.interact(button, ui.scope_id().with("close"), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(button, 6.0, palette.hover);
        }
        let ink = if response.hovered() { palette.foreground } else { palette.muted };
        icons::paint(ui.painter(), Rect::from_center_size(button.center(), egui::Vec2::splat(14.0)), "x", ink);
        // The counter: where the arrows are among the matches, or how many there are.
        let counter = match (self.current, self.results.len()) {
            (_, 0) => String::new(),
            (Some(i), total) => format!("{} of {total}", i + 1),
            (None, 1) => "1 match".to_owned(),
            (None, total) => format!("{total} matches"),
        };
        let drawn =
            ui.painter().text(pos2(button.left() - 8.0, bar.center().y), Align2::RIGHT_CENTER, counter, FontId::proportional(11.0), palette.muted);
        let field = Rect::from_min_max(pos2(bar.left() + 40.0, bar.top()), pos2(drawn.left().min(button.left()) - 8.0, bar.bottom()));
        let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(field).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let edit = inner.add(
            egui::TextEdit::singleline(&mut self.query)
                .frame(egui::Frame::new())
                .background_color(Color32::TRANSPARENT)
                .font(FontId::proportional(FIND_TEXT))
                .hint_text("Find layers")
                .char_limit(LONGEST)
                .desired_width(field.width()),
        );
        edit.request_focus();
        response.on_hover_text("Close (esc)").clicked()
    }

    /// What is under the field: the hint, the matches, or that there are none. Gives the match clicked.
    fn body(&self, ui: &mut egui::Ui, moved: bool, palette: theme::Palette) -> Option<usize> {
        let note = |ui: &mut egui::Ui, text: &str| {
            let (row, _) = ui.allocate_exact_size(vec2(FIND_WIDTH, 2.0 * FIND_ROW), Sense::hover());
            let clipped = ui.painter().with_clip_rect(row.shrink2(vec2(16.0, 0.0)));
            clipped.text(row.center(), Align2::CENTER_CENTER, text, FontId::proportional(FIND_TEXT), palette.muted);
        };
        let query = self.query.trim();
        if query.is_empty() {
            note(ui, "Searches names, text, and IDs");
            return None;
        }
        if self.results.is_empty() {
            note(ui, &format!("No layers match “{query}”"));
            return None;
        }
        let mut clicked = None;
        let height = f32::from(FIND_ROWS) * FIND_ROW + 2.0 * FIND_PADDING;
        egui::ScrollArea::vertical().max_height(height).auto_shrink([false, true]).show(ui, |ui| {
            ui.add_space(FIND_PADDING);
            for (index, found) in self.results.iter().enumerate() {
                let lit = self.current == Some(index);
                let response = line(ui, found, query, lit, palette);
                if lit && moved {
                    response.scroll_to_me(None);
                }
                if response.clicked() {
                    clicked = Some(index);
                }
            }
            ui.add_space(FIND_PADDING);
        });
        clicked
    }
}

/// One match's line: the layer's icon, its name or text with the matching part in the accent colour, and for a
/// match in the id the id at the right.
fn line(ui: &mut egui::Ui, found: &Found, query: &str, lit: bool, palette: theme::Palette) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(vec2(FIND_WIDTH, FIND_ROW), Sense::click());
    let plate = row.shrink2(vec2(FIND_PADDING, 1.0));
    if lit || response.hovered() {
        ui.painter().rect_filled(plate, 6.0, if lit { palette.hover } else { palette.hover.gamma_multiply(0.5) });
    }
    icons::paint(ui.painter(), Rect::from_center_size(pos2(plate.left() + 16.0, row.center().y), egui::Vec2::splat(13.0)), found.icon, palette.muted);
    let mut right = plate.right() - 10.0;
    if found.field == Field::Id {
        let id = ui.painter().layout_job(marked(&found.id, query, FontId::monospace(10.5), palette.muted));
        right -= id.size().x;
        ui.painter().galley(pos2(right, row.center().y - id.size().y / 2.0), id, palette.muted);
        right -= 8.0;
    }
    let ink = if found.field == Field::Text { palette.muted } else { palette.foreground };
    let mark = if found.field == Field::Id { "" } else { query };
    let label = ui.painter().layout_job(marked(&found.label, mark, FontId::proportional(FIND_TEXT), ink));
    let left = plate.left() + 32.0;
    let clipped = ui.painter().with_clip_rect(Rect::from_min_max(pos2(left, row.top()), pos2(right, row.bottom())));
    clipped.galley(pos2(left, row.center().y - label.size().y / 2.0), label, ink);
    response
}

/// `text` on one line with the first part that matches `query` (ignoring case) in the accent colour.
fn marked(text: &str, query: &str, font: FontId, ink: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat { font_id: font.clone(), color: ink, ..TextFormat::default() };
    let (start, end) = match_range(text, query).unwrap_or((text.len(), text.len()));
    job.append(&text[..start], 0.0, plain.clone());
    job.append(&text[start..end], 0.0, TextFormat { font_id: font, color: ACCENT, ..TextFormat::default() });
    job.append(&text[end..], 0.0, plain);
    job.wrap.max_rows = 1;
    job.wrap.max_width = f32::INFINITY;
    job
}

/// Where in `text` the first part matching `query` (ignoring case) is, in bytes of `text`.
fn match_range(text: &str, query: &str) -> Option<(usize, usize)> {
    let wanted = query.to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    // Agent: lower-casing can change a letter's length in bytes, so the match is looked for from each letter of
    // `text` itself, never by an offset into the lower-cased copy.
    text.char_indices().find_map(|(start, _)| {
        let mut rest = wanted.as_str();
        for (offset, letter) in text[start..].char_indices() {
            let lower: String = letter.to_lowercase().collect();
            rest = rest.strip_prefix(lower.as_str())?;
            if rest.is_empty() {
                return Some((start, start + offset + letter.len_utf8()));
            }
        }
        None
    })
}

/// Shows the layer with `id` on the canvas when it isn't all in view: the view moves to it, and zooms out to fit
/// it when it is larger than the view.
fn bring_into_view(editor: &mut Editor, id: &str, view: Rect) {
    let Some(bounds) = editor.scene_node(id).map(layout::doc_bounds) else { return };
    if crate::render::view_to_doc(&editor.camera, view).holds(&bounds) {
        return;
    }
    let zoom = editor.camera.zoom;
    if bounds.w * zoom <= view.width() - 2.0 * FIT_PADDING && bounds.h * zoom <= view.height() - 2.0 * FIT_PADDING {
        editor.camera.center = bounds.center();
    } else {
        editor.focus = Some(bounds);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"{"version":"2.22","children":[
        {"type":"frame","id":"hero","name":"Hero","x":0,"y":0,"width":400,"height":300,"layout":"none","children":[
            {"type":"text","id":"t1","content":"Design calm,\nuseful Products."},
            {"type":"rectangle","id":"heroImage","name":"Picture","x":10,"y":10,"width":50,"height":50}]},
        {"type":"frame","id":"f2","name":"Product card","x":3000,"y":0,"width":200,"height":100}]}"#;

    fn found(query: &str) -> Vec<(String, Field, String)> {
        let doc: pen_doc::Document = serde_json::from_str(DOC).unwrap();
        search(&doc.children, query).into_iter().map(|f| (f.id, f.field, f.label)).collect()
    }

    #[test]
    fn names_then_texts_then_ids_match_without_regard_to_case() {
        // A parent comes before its children; a text without a name shows its first line and matches as a name.
        assert_eq!(found(" HERO "), [("hero".to_owned(), Field::Name, "Hero".to_owned()), ("heroImage".to_owned(), Field::Id, "Picture".to_owned())]);
        // A match further down in a text shows the line it is in.
        assert_eq!(
            found("product"),
            [("t1".to_owned(), Field::Text, "useful Products.".to_owned()), ("f2".to_owned(), Field::Name, "Product card".to_owned())]
        );
        assert_eq!(found("calm"), [("t1".to_owned(), Field::Name, "Design calm,".to_owned())]);
        assert_eq!((found("nothing like it"), found("  ")), (Vec::new(), Vec::new()));
    }

    #[test]
    fn the_matching_part_is_found_in_the_text_as_it_is_written() {
        assert_eq!(match_range("Product card", "CARD"), Some((8, 12)));
        // Letters that take more room in lower case than in upper case don't shift the match.
        assert_eq!(match_range("İstanbul Ünite", "ünite"), Some(("İstanbul ".len(), "İstanbul Ünite".len())));
        assert_eq!((match_range("abc", "abcd"), match_range("abc", "")), (None, None));
    }

    #[test]
    fn the_arrows_go_round_and_a_new_search_starts_at_the_first_match() {
        let mut editor = Editor::new(serde_json::from_str(DOC).unwrap(), None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor.selection = vec!["hero".to_owned()];
        let mut find = Find::default();
        find.open(&mut editor);
        assert!(find.is_open() && editor.selection.is_empty());
        find.query = "product".to_owned();
        assert!(find.refresh(&editor, 0));
        assert_eq!((find.results.len(), find.current), (2, Some(0)));
        // The same search again changes nothing; the current match is kept while the document changes.
        find.current = Some(1);
        assert!(!find.refresh(&editor, 0));
        editor.touched();
        editor.refresh(&|_| (0.0, 0.0));
        assert!(!find.refresh(&editor, 0));
        assert_eq!(find.current, Some(1));
        // A match out of view is brought into it without changing the zoom.
        let view = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        bring_into_view(&mut editor, "f2", view);
        assert_eq!((editor.camera.center, editor.camera.zoom), (pen_doc::geom::Point::new(3100.0, 50.0), 1.0));
        find.close(&mut editor);
        assert!(!find.is_open() && editor.spotlight == (Vec::new(), None));
    }
}
