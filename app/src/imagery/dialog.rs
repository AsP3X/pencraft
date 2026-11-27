//! The "Generate Image…" window: a description, the pictures to send with it, a service, a shape and a size, and
//! a button. With a picture marked "Change this" it is the "Edit image" window.

use std::path::PathBuf;

use egui::{Align2, Color32, FontId, Key, Margin, Modifiers, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2, pos2, vec2};
use serde_json::{Value, json};

use super::service::ASPECTS;
use super::sources::MOST_REFERENCES;
use crate::icons;
use crate::settings::{caption, choice_at, row, switch};
use crate::theme::{
    ACCENT, ACCENT_PLATE, CHIP_RADIUS, DIALOG_BACKDROP, DIALOG_BUTTON, DIALOG_GAP, DIALOG_PADDING, DIALOG_TITLE_SIZE, FIELD_HEIGHT, FIELD_MARGIN,
    GENERATE_LINES, GENERATE_WIDTH, ON_ACCENT, PIECE_CHIP_PADDING, PIECE_CROSS, PIECE_CROSS_OUT, PIECE_GAP, PIECE_SIDE, PIECE_TEXT, PIECE_TOGGLE,
    PROBLEM, Palette, SETTINGS_RADIUS,
};

/// The size choice's words for a picture of about 1024 pixels, and of about 2048.
const NORMAL: &str = "Normal";
/// See [`NORMAL`].
const LARGE: &str = "Large";
/// The Shape choice that keeps the shape of the picture to change; offered, and first, while editing.
pub const SAME: &str = "Same as the picture";
/// The most pictures in the row: one to change and the references.
const MOST_PIECES: usize = MOST_REFERENCES + 1;
/// Said in place of the form when there are pictures to send and no service that can take them.
const NO_EDITING_HERE: &str = "None of your image services can edit pictures yet. Add a Google Gemini service in Settings → Images.";

/// The sentence for a reference too many.
fn too_many_references() -> String {
    format!("At most {MOST_REFERENCES} reference pictures can be sent.")
}

/// What the person did in the dialog this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Did {
    /// Nothing that concerns the app.
    Nothing,
    /// Pressed "Generate" (or "Edit"), or ⌘/Ctrl+Enter in the description.
    Generate,
    /// Pressed "Cancel" or "Close", or closed the window (Escape, a click beside it) while no picture was being
    /// made.
    Cancel,
    /// Pressed "Open Settings" where there is no service.
    OpenSettings,
    /// Pressed "Add file…": the app asks for reference pictures.
    AddFile,
}

/// What Escape or a click beside the window does: it closes the window, but while a picture is being made
/// (`busy`) only the Cancel button gives it up.
// Human: a picture costs money; a stray key or click must not throw one away that is nearly there.
fn on_close(busy: bool) -> Did {
    if busy { Did::Nothing } else { Did::Cancel }
}

/// An image service as the dialog offers it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Offered {
    /// What the user calls it.
    pub name: String,
    /// Whether it can change a picture sent to it.
    pub edits: bool,
}

/// A picture in the dialog's row.
pub struct Piece {
    /// The layer it is, or the file.
    pub what: PieceOf,
    /// The name under it.
    pub name: String,
    /// "Change this" is marked on it.
    pub change: bool,
    /// Its small picture, once made (`None` in tests, and while a file's is being made).
    pub thumb: Option<egui::TextureHandle>,
    /// Its small picture was asked for: made, being made, or found impossible (the picture icon then stays).
    pub looked: bool,
}

impl Piece {
    /// The layer `id`, called `name`; `has_picture` when its fill has a picture.
    #[must_use]
    pub fn layer(id: &str, name: &str, has_picture: bool) -> Piece {
        Piece { what: PieceOf::Layer { id: id.to_owned(), has_picture }, name: name.to_owned(), change: false, thumb: None, looked: false }
    }

    /// The picture file at `path`, named after the file.
    #[must_use]
    pub fn file(path: PathBuf) -> Piece {
        let name = path.file_name().map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned());
        Piece { what: PieceOf::File(path), name, change: false, thumb: None, looked: false }
    }

    /// Whether this is a layer whose fill has a picture: the one kind whose picture can be replaced.
    fn has_picture(&self) -> bool {
        matches!(self.what, PieceOf::Layer { has_picture: true, .. })
    }
}

/// What a picture of the row is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PieceOf {
    /// A layer of the document; `has_picture` when its fill has a picture, which is sent as its file. Any other
    /// layer is sent as it looks.
    Layer {
        /// The layer's id.
        id: String,
        /// Its fill has a picture.
        has_picture: bool,
    },
    /// A picture file, a reference only.
    File(PathBuf),
}

/// What a click in the row asks.
enum Act {
    /// Take the piece out.
    Remove(usize),
    /// Mark or unmark it.
    Mark(usize),
}

/// The dialog's state.
#[derive(Default)]
#[expect(clippy::struct_excessive_bools, reason = "each is one independent part of the window's state, set by the app and its tests")]
pub struct Dialog {
    /// Whether the window shows.
    pub open: bool,
    /// The description, or what to change.
    pub prompt: String,
    /// The chosen service's name; empty when there is none.
    pub service: String,
    /// The chosen shape, one of [`ASPECTS`], or [`SAME`] while editing.
    pub aspect: String,
    /// About 2048 pixels instead of about 1024.
    pub large: bool,
    /// The layer that was selected when the dialog opened and can take a picture.
    pub target: Option<String>,
    /// Give the picture to [`Dialog::target`] instead of adding a layer.
    pub fill: bool,
    /// Put the changed picture on the marked layer in place of its picture, instead of beside it.
    pub replace: bool,
    /// A picture is being made.
    pub busy: bool,
    /// Another picture of the person's is being made (from the properties panel): this one waits for it.
    pub other: bool,
    /// What went wrong the last time.
    pub problem: Option<String>,
    /// The document the dialog was opened for is closed: nothing can be generated for it, only closed.
    pub orphaned: bool,
    /// The pictures to send.
    pieces: Vec<Piece>,
    /// The services that worked when the dialog opened.
    services: Vec<Offered>,
    /// The settings' default service.
    default: String,
    /// The description is to take the keyboard next time it is drawn.
    focus: bool,
}

impl Dialog {
    /// Opens the dialog. `target` is the selected layer when it can take a picture; `services` are the ones
    /// that work and `default` the settings' choice among them. The row of pictures starts empty
    /// ([`Dialog::set_pieces`] fills it).
    // Human: while a picture is being made the dialog is shown again as it is: the picture is still for the
    // layer and the service it was asked for, and the person can wait for it or cancel it.
    pub fn open_for(&mut self, target: Option<String>, services: &[Offered], default: &str) {
        self.open = true;
        if self.busy {
            return;
        }
        self.problem = None;
        self.focus = true;
        self.fill = target.is_some();
        self.replace = true;
        self.target = target;
        self.services = services.to_vec();
        default.clone_into(&mut self.default);
        self.pieces.clear();
        self.settle();
    }

    /// Puts the selected layers into the row. A lone layer with an image fill is marked as the one to change.
    /// More than [`MOST_PIECES`] are cut to that many, which `problem` says. Nothing changes while a picture is
    /// being made.
    pub fn set_pieces(&mut self, mut pieces: Vec<Piece>) {
        if self.busy {
            return;
        }
        if pieces.len() > MOST_PIECES {
            pieces.truncate(MOST_PIECES);
            self.problem = Some(format!(
                "Only the first {MOST_PIECES} selected layers are in the row: one picture to change and {MOST_REFERENCES} references can be sent."
            ));
        }
        for piece in &mut pieces {
            piece.change = false;
        }
        if let [only] = pieces.as_mut_slice()
            && only.has_picture()
        {
            only.change = true;
        }
        self.pieces = pieces;
        if self.editing() {
            SAME.clone_into(&mut self.aspect);
        }
        self.settle();
    }

    /// The pictures of the row, to give them their small pictures.
    pub fn pieces_mut(&mut self) -> &mut [Piece] {
        &mut self.pieces
    }

    /// Whether there are pictures to send.
    #[must_use]
    pub fn has_pieces(&self) -> bool {
        !self.pieces.is_empty()
    }

    /// How many pictures are references (not marked).
    fn references(&self) -> usize {
        self.pieces.iter().filter(|piece| !piece.change).count()
    }

    /// Marks piece `index` as the one to change, or unmarks it when it was; no other stays marked. A file is a
    /// reference only. Unmarking is refused when it would leave more references than can be sent.
    pub fn mark(&mut self, index: usize) {
        let Some(piece) = self.pieces.get(index) else { return };
        if matches!(piece.what, PieceOf::File(_)) {
            return;
        }
        let was = self.editing();
        if piece.change {
            if self.references() + 1 > MOST_REFERENCES {
                self.problem = Some(too_many_references());
                return;
            }
            self.pieces[index].change = false;
        } else {
            for (at, piece) in self.pieces.iter_mut().enumerate() {
                piece.change = at == index;
            }
        }
        self.problem = None;
        self.switched(was);
    }

    /// Adds a file as a reference, or says why not (the limit, or a path that can't be sent).
    ///
    /// # Errors
    /// The sentence to show.
    pub fn add_file(&mut self, path: PathBuf) -> Result<(), String> {
        if self.references() >= MOST_REFERENCES {
            return Err(too_many_references());
        }
        // Human: a path is checked by the same reading the assistant's calls go through, so a network share is
        // refused here as it is there, before anything is read from it.
        super::named(&json!({ "references": [path.to_string_lossy()] }))?;
        let was = self.editing();
        self.pieces.push(Piece::file(path));
        self.problem = None;
        self.switched(was);
        Ok(())
    }

    /// Takes piece `index` out of the row.
    pub fn remove(&mut self, index: usize) {
        if index >= self.pieces.len() {
            return;
        }
        let was = self.editing();
        self.pieces.remove(index);
        self.problem = None;
        self.switched(was);
    }

    /// "Sends 3 pictures to Gemini." or `None` when nothing is sent, or when there are more references than
    /// can be sent ([`Dialog::too_many`] says so instead).
    #[must_use]
    pub fn sends(&self) -> Option<String> {
        if self.too_many().is_some() {
            return None;
        }
        match self.pieces.len() {
            0 => None,
            1 => Some(format!("Sends 1 picture to {}.", self.service)),
            many => Some(format!("Sends {many} pictures to {}.", self.service)),
        }
    }

    /// The sentence shown in place of [`Dialog::sends`] while the row holds more references than can be sent;
    /// nothing can be sent then.
    #[must_use]
    pub fn too_many(&self) -> Option<String> {
        (self.references() > MOST_REFERENCES).then(|| format!("{} Mark one as the picture to change, or take one out.", too_many_references()))
    }

    /// Adds picture files as references, as many as fit, and says in `problem` how many were added and why the
    /// others weren't (nothing when all were).
    pub fn add_files(&mut self, paths: Vec<PathBuf>) {
        let total = paths.len();
        let mut added = 0;
        let mut why: Vec<String> = Vec::new();
        for path in paths {
            match self.add_file(path) {
                Ok(()) => added += 1,
                Err(sentence) if !why.contains(&sentence) => why.push(sentence),
                Err(_) => {}
            }
        }
        self.problem = match (why.is_empty(), added) {
            (true, _) => None,
            (false, 0) => Some(why.join(" ")),
            (false, 1) => Some(format!("1 of the {total} files was added. {}", why.join(" "))),
            (false, added) => Some(format!("{added} of the {total} files were added. {}", why.join(" "))),
        };
    }

    /// Whether this is an edit (a piece is marked) rather than a generation.
    #[must_use]
    pub fn editing(&self) -> bool {
        self.pieces.iter().any(|piece| piece.change)
    }

    /// The marked piece, when it is a layer with a picture: the one whose picture can be replaced.
    fn replaceable(&self) -> bool {
        self.pieces.iter().any(|piece| piece.change && piece.has_picture())
    }

    /// The services offered: all of them, or with pictures to send only those that can edit.
    fn offered(&self) -> Vec<&str> {
        let editing = self.has_pieces();
        self.services.iter().filter(|service| !editing || service.edits).map(|service| service.name.as_str()).collect()
    }

    /// The Shape choices: [`SAME`] first while editing, then the shapes.
    fn shapes(&self) -> Vec<&'static str> {
        if self.editing() { std::iter::once(SAME).chain(ASPECTS).collect() } else { ASPECTS.to_vec() }
    }

    /// After a change of the row: Shape keeps the picture's shape when the dialog becomes an edit, and leaves it
    /// when it stops being one; the service and shape are ones offered.
    fn switched(&mut self, was: bool) {
        let now = self.editing();
        if now && !was {
            SAME.clone_into(&mut self.aspect);
        }
        self.settle();
    }

    /// Makes the chosen service and shape ones that are offered: the one chosen, else the default, else the first.
    fn settle(&mut self) {
        let offered = self.offered();
        if !offered.contains(&self.service.as_str()) {
            let chosen = offered.iter().find(|name| **name == self.default).or_else(|| offered.first()).map(|name| (*name).to_owned());
            self.service = chosen.unwrap_or_default();
        }
        let shapes = self.shapes();
        if !shapes.contains(&self.aspect.as_str()) {
            shapes[0].clone_into(&mut self.aspect);
        }
    }

    /// What the dialog asks for: with pictures to send in the form of an `edit_image` call's arguments, else
    /// in that of a `generate_image` call's.
    // Human: the dialog asks in the tools' own words, so the app turns them into a request through the same
    // functions a model's call goes through, and the two can't drift apart.
    #[must_use]
    pub fn asked(&self) -> Value {
        let size = if self.large { "large" } else { "normal" };
        if self.pieces.is_empty() {
            return json!({ "prompt": self.prompt, "service": self.service, "aspect": self.aspect, "size": size });
        }
        let mut asked = json!({ "instruction": self.prompt, "service": self.service, "size": size });
        let mut references = Vec::new();
        for piece in &self.pieces {
            match (&piece.what, piece.change) {
                (PieceOf::Layer { id, has_picture: true }, true) => asked["layer"] = json!(id),
                (PieceOf::Layer { id, has_picture: false }, true) => asked["layers"] = json!([id]),
                (PieceOf::Layer { id, .. }, false) => references.push(json!(id)),
                (PieceOf::File(path), _) => references.push(json!(path.to_string_lossy())),
            }
        }
        asked["references"] = Value::Array(references);
        if self.aspect != SAME {
            asked["aspect"] = json!(self.aspect);
        }
        asked
    }

    /// Draws the window when it is open; what the person did in it.
    // Agent: MAIN-THREAD.
    pub fn show(&mut self, ctx: &egui::Context, palette: Palette) -> Did {
        if !self.open {
            return Did::Nothing;
        }
        let mut did = Did::Nothing;
        let frame = egui::Frame::new()
            .fill(palette.island)
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(SETTINGS_RADIUS)
            .inner_margin(Margin::same(DIALOG_PADDING));
        // Human: the window appears at once, as the settings do; a fade would show the canvas through it.
        let id = egui::Id::unique("generate-image");
        let modal = egui::Modal::new(id)
            .area(egui::Modal::default_area(id).fade_in(false))
            .frame(frame)
            .backdrop_color(Color32::from_black_alpha(DIALOG_BACKDROP))
            .show(ctx, |ui| self.contents(ui, palette, &mut did));
        if modal.should_close() && did == Did::Nothing {
            did = on_close(self.busy);
        }
        did
    }

    /// Whether the form only offers to close: the picture's document is gone and none is being made.
    fn only_close(&self) -> bool {
        self.orphaned && !self.busy
    }

    /// The window's content: the form, or what to do when there is no service.
    fn contents(&mut self, ui: &mut Ui, palette: Palette, did: &mut Did) {
        ui.set_width(GENERATE_WIDTH);
        let title = if self.editing() { "Edit image" } else { "Generate image" };
        ui.add(egui::Label::new(RichText::new(title).size(DIALOG_TITLE_SIZE).color(palette.foreground)).selectable(false));
        ui.add_space(DIALOG_GAP);
        // Human: the form is locked while its picture is made, while another of the person's is, and when its
        // document is gone.
        let locked = self.busy || self.other || self.only_close();
        if self.services.is_empty() || self.offered().is_empty() {
            // Human: with pictures to send and no service that can take them, the row stays, so a picture can be
            // taken out and the dialog becomes the plain one again.
            if self.has_pieces() {
                self.row_of_pieces(ui, palette, locked, did);
                ui.add_space(DIALOG_GAP);
            }
            let why = if self.services.is_empty() { super::NO_SERVICE } else { NO_EDITING_HERE };
            ui.add(egui::Label::new(RichText::new(why).color(palette.muted)).wrap());
            ui.add_space(DIALOG_GAP);
            // Human: in a row, as the form's buttons are, so each word sits in its button's middle. Settings
            // first: it is the way on.
            ui.horizontal(|ui| {
                if ui.add(primary("Open Settings", palette)).clicked() {
                    *did = Did::OpenSettings;
                }
                if ui.add(button("Close")).clicked() {
                    *did = Did::Cancel;
                }
            });
            return;
        }
        caption(ui, "Description", palette);
        let field = egui::Frame::new()
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CHIP_RADIUS)
            .inner_margin(Margin::symmetric(FIELD_MARGIN[0], FIELD_MARGIN[1]));
        let hint = if self.editing() {
            "What to change: the light, the colours, what to add or take away"
        } else {
            "What the picture shows: subject, setting, light, style"
        };
        let hint = RichText::new(hint).color(palette.muted);
        let description =
            egui::TextEdit::multiline(&mut self.prompt).frame(field).hint_text(hint).desired_rows(GENERATE_LINES).desired_width(f32::INFINITY);
        let description = ui.add_enabled(!locked, description);
        // Human: the description takes the keyboard once when the dialog opens, so the person can type at
        // once; after that a click elsewhere keeps it elsewhere.
        if !locked && std::mem::take(&mut self.focus) {
            description.request_focus();
        }
        // Human: never with more references than can be sent: the line under the row says why.
        let can_generate = !locked && !self.prompt.trim().is_empty() && self.too_many().is_none();
        // Human: ⌘/Ctrl+Enter in the description is Generate; Enter alone is a new line, as in other fields.
        if can_generate && description.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter)) {
            *did = Did::Generate;
        }
        if self.has_pieces() {
            ui.add_space(DIALOG_GAP);
            self.row_of_pieces(ui, palette, locked, did);
            if let Some(too_many) = self.too_many() {
                ui.add_space(DIALOG_GAP / 2.0);
                ui.add(egui::Label::new(RichText::new(too_many).color(PROBLEM)).wrap().selectable(false));
            } else if let Some(sends) = self.sends() {
                ui.add_space(DIALOG_GAP / 2.0);
                ui.add(egui::Label::new(RichText::new(sends).color(palette.muted)).selectable(false));
            }
        }
        ui.add_space(DIALOG_GAP);
        self.choices(ui, palette, locked);
        if self.other {
            ui.add_space(DIALOG_GAP);
            let waiting = "A picture is being changed from the properties panel. This one can be asked for when it is done.";
            ui.add(egui::Label::new(RichText::new(waiting).color(palette.muted)).wrap());
        }
        if let Some(problem) = &self.problem {
            ui.add_space(DIALOG_GAP);
            ui.add(egui::Label::new(RichText::new(problem).color(PROBLEM)).wrap());
        }
        ui.add_space(DIALOG_GAP);
        ui.horizontal(|ui| {
            if self.only_close() {
                // Human: the picture's document is gone, so Generate could only fail; closing is the way out.
                if ui.add(button("Close")).clicked() {
                    *did = Did::Cancel;
                }
                return;
            }
            let label = match (self.editing(), self.busy) {
                (true, true) => "Editing…",
                (true, false) => "Edit",
                (false, true) => "Generating…",
                (false, false) => "Generate",
            };
            if ui.add_enabled(can_generate, primary(label, palette)).clicked() {
                *did = Did::Generate;
            }
            if ui.add(button("Cancel")).clicked() {
                *did = Did::Cancel;
            }
            if self.busy {
                ui.spinner();
                ui.label(RichText::new("This can take a minute or two.").color(palette.muted));
            }
        });
    }

    /// The service, shape and size, and the switch that says where the picture goes.
    fn choices(&mut self, ui: &mut Ui, palette: Palette, locked: bool) {
        let mut size = if self.large { LARGE } else { NORMAL }.to_owned();
        ui.add_enabled_ui(!locked, |ui| {
            let services: Vec<String> = self.offered().into_iter().map(str::to_owned).collect();
            let services: Vec<&str> = services.iter().map(String::as_str).collect();
            let shapes = self.shapes();
            // Human: the service is only offered when there is a choice.
            let several = services.len() > 1;
            ui.columns(if several { 3 } else { 2 }, |columns| {
                let mut at = 0;
                if several {
                    labelled(&mut columns[0], "Service", &mut self.service, &services, palette);
                    at = 1;
                }
                labelled(&mut columns[at], "Shape", &mut self.aspect, &shapes, palette);
                labelled(&mut columns[at + 1], "Size", &mut size, &[NORMAL, LARGE], palette);
            });
            if self.replaceable() {
                ui.add_space(DIALOG_GAP);
                let otherwise = "When off, the result is a new layer beside it.";
                ui.scope(|ui| row(ui, "Replace the picture", Some(otherwise), palette, |ui| switch(ui, &mut self.replace, palette)));
            } else if self.target.is_some() && !self.editing() {
                ui.add_space(DIALOG_GAP);
                let otherwise = "When off, the picture is a new layer in the middle of the view.";
                ui.scope(|ui| row(ui, "Use as the fill of the selection", Some(otherwise), palette, |ui| switch(ui, &mut self.fill, palette)));
            }
        });
        self.large = size == LARGE;
    }

    /// The row of pictures to send: each with its name, a cross to take it out and its role under it; then
    /// "Add file…" while another reference fits.
    fn row_of_pieces(&mut self, ui: &mut Ui, palette: Palette, locked: bool, did: &mut Did) {
        let font = FontId::proportional(PIECE_TEXT);
        let name_height = PIECE_TEXT + 6.0;
        // Human: the crosses reach a little past their pictures' corners, so the row starts that much lower.
        let height = PIECE_CROSS_OUT + PIECE_SIDE + 2.0 + name_height + PIECE_TOGGLE;
        let (area, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let top = area.top() + PIECE_CROSS_OUT;
        let mut act = None;
        let mut left = area.left();
        for (index, piece) in self.pieces.iter().enumerate() {
            let tile = Rect::from_min_size(pos2(left, top), Vec2::splat(PIECE_SIDE));
            left = tile.right() + PIECE_GAP;
            ui.painter().rect_filled(tile, CHIP_RADIUS, palette.hover.gamma_multiply(0.6));
            match &piece.thumb {
                Some(thumb) => {
                    // Human: the whole picture shows, its shape kept, in the middle of its square.
                    let size = thumb.size_vec2();
                    let scale = (PIECE_SIDE / size.x.max(1.0)).min(PIECE_SIDE / size.y.max(1.0));
                    let shown = Rect::from_center_size(tile.center(), size * scale);
                    egui::Image::from_texture((thumb.id(), size)).corner_radius(CHIP_RADIUS).paint_at(ui, shown);
                }
                None => icons::paint(ui.painter(), Rect::from_center_size(tile.center(), Vec2::splat(PIECE_CROSS + 4.0)), "image", palette.muted),
            }
            let edge = if piece.change { Stroke::new(2.0, ACCENT) } else { Stroke::new(1.0, palette.outline) };
            ui.painter().rect_stroke(tile, CHIP_RADIUS, edge, StrokeKind::Inside);
            // The cross, on the top right corner, reaching past it, so it covers as little of the picture as it can.
            let corner = pos2(tile.right() + PIECE_CROSS_OUT - PIECE_CROSS / 2.0, tile.top() - PIECE_CROSS_OUT + PIECE_CROSS / 2.0);
            let cross = Rect::from_center_size(corner, Vec2::splat(PIECE_CROSS));
            let sense = if locked { Sense::hover() } else { Sense::click() };
            let response = ui.interact(cross, ui.scope_id().with(("piece-cross", index)), sense);
            ui.painter().circle(cross.center(), PIECE_CROSS / 2.0, palette.island, Stroke::new(1.0, palette.outline));
            let color = if response.hovered() && !locked { palette.foreground } else { palette.muted };
            icons::paint(ui.painter(), cross.shrink(4.0), "x", color);
            if response.on_hover_text("Take it out").clicked() {
                act = Some(Act::Remove(index));
            }
            // Its name, cut to fit.
            let name = Rect::from_min_size(pos2(tile.left(), tile.bottom() + 2.0), vec2(PIECE_SIDE, name_height));
            // Human: a child placed by hand, so the row keeps the height it took above.
            let mut under = ui
                .new_child(egui::UiBuilder::new().max_rect(name).layout(egui::Layout::top_down(egui::Align::Center)).id_salt(("piece-name", index)));
            under.add(egui::Label::new(RichText::new(&piece.name).font(font.clone()).color(palette.foreground)).truncate().selectable(false));
            // Its role: a layer can be marked as the one to change, so its role is a chip that switches; a file is
            // a reference only, so its role is plain words.
            let markable = matches!(piece.what, PieceOf::Layer { .. });
            let (words, ink) = if piece.change { ("Change this", ON_ACCENT) } else { ("Reference", palette.muted) };
            let galley = ui.painter().layout_no_wrap(words.to_owned(), font.clone(), ink);
            // Human: the chip may reach a little into the gap beside its picture, so its words keep clear of its
            // edge; never far enough to meet the next one's.
            let chip_width = (galley.size().x + 2.0 * PIECE_CHIP_PADDING).min(PIECE_SIDE + PIECE_GAP - 2.0);
            let toggle = Rect::from_center_size(pos2(tile.center().x, name.bottom() + PIECE_TOGGLE / 2.0), vec2(chip_width, PIECE_TOGGLE));
            let response =
                ui.interact(toggle, ui.scope_id().with(("piece-mark", index)), if markable && !locked { Sense::click() } else { Sense::hover() });
            if piece.change {
                ui.painter().rect_filled(toggle, CHIP_RADIUS, ACCENT_PLATE);
            } else if markable {
                if !locked && response.hovered() {
                    ui.painter().rect_filled(toggle, CHIP_RADIUS, palette.hover);
                }
                ui.painter().rect_stroke(toggle, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
            }
            ui.painter().galley(toggle.center() - galley.size() / 2.0, galley, ink);
            let hint = match (markable, piece.change) {
                (false, _) => "A picture file is sent as a reference",
                (true, true) => "The picture to change. Click to make it a reference",
                (true, false) => "Sent as a reference. Click to make it the picture to change",
            };
            if response.on_hover_text(hint).clicked() && markable && !locked {
                act = Some(Act::Mark(index));
            }
        }
        if self.pieces.len() < MOST_PIECES && self.references() < MOST_REFERENCES {
            let tile = Rect::from_min_size(pos2(left, top), Vec2::splat(PIECE_SIDE));
            let sense = if locked { Sense::hover() } else { Sense::click() };
            let response = ui.interact(tile, ui.scope_id().with("piece-add"), sense);
            if response.hovered() && !locked {
                ui.painter().rect_filled(tile, CHIP_RADIUS, palette.hover.gamma_multiply(0.4));
            }
            ui.painter().rect_stroke(tile, CHIP_RADIUS, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
            let color = if locked { palette.muted.gamma_multiply(0.6) } else { palette.foreground };
            icons::paint(ui.painter(), Rect::from_center_size(tile.center() - vec2(0.0, 8.0), Vec2::splat(PIECE_CROSS)), "plus", color);
            ui.painter().text(tile.center() + vec2(0.0, 12.0), Align2::CENTER_CENTER, "Add file…", font, color);
            if response.on_hover_text("Add a picture file as a reference").clicked() && !locked {
                *did = Did::AddFile;
            }
        }
        match act {
            Some(Act::Remove(index)) => self.remove(index),
            Some(Act::Mark(index)) => self.mark(index),
            None => {}
        }
    }
}

/// A plain button of the window, as the settings' forms have them.
fn button(text: &str) -> egui::Button<'_> {
    egui::Button::new(text).corner_radius(CHIP_RADIUS).min_size(vec2(DIALOG_BUTTON, FIELD_HEIGHT))
}

/// The window's main button, in the primary colour.
fn primary(text: &str, palette: Palette) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(palette.primary_text))
        .fill(palette.primary)
        .corner_radius(CHIP_RADIUS)
        .min_size(vec2(DIALOG_BUTTON, FIELD_HEIGHT))
}

/// A choice as wide as its column with its name over it; the name also tells it apart from the others.
fn labelled(ui: &mut Ui, name: &str, value: &mut String, options: &[&str], palette: Palette) {
    caption(ui, name, palette);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), FIELD_HEIGHT), Sense::hover());
    choice_at(ui, rect, name, value, options, palette);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Services that can't edit, by name.
    fn plain(names: &[&str]) -> Vec<Offered> {
        names.iter().map(|name| Offered { name: (*name).to_owned(), edits: false }).collect()
    }

    /// OpenAI-like "Studio", which can't edit, and "Gemini", which can.
    fn both() -> Vec<Offered> {
        vec![Offered { name: "Studio".to_owned(), edits: false }, Offered { name: "Gemini".to_owned(), edits: true }]
    }

    /// A picture file's full path on this system.
    fn full(name: &str) -> PathBuf {
        PathBuf::from(if cfg!(windows) { format!("C:/pictures/{name}") } else { format!("/pictures/{name}") })
    }

    #[test]
    fn the_dialog_asks_in_the_tools_own_form() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &plain(&["Studio", "Gemini"]), "Gemini");
        assert!(dialog.open && dialog.service == "Gemini" && dialog.fill && dialog.target.as_deref() == Some("hero"));
        dialog.prompt = "A red fox".to_owned();
        dialog.aspect = "16:9".to_owned();
        dialog.large = true;
        assert_eq!(dialog.asked(), json!({ "prompt": "A red fox", "service": "Gemini", "aspect": "16:9", "size": "large" }));
        assert_eq!(dialog.sends(), None);
    }

    #[test]
    fn opening_again_keeps_the_description_and_forgets_the_last_problem() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &plain(&["Studio"]), "");
        assert!(dialog.service == "Studio" && !dialog.fill && dialog.aspect == "1:1");
        dialog.prompt = "A fox".to_owned();
        dialog.problem = Some("no".to_owned());
        dialog.open = false;
        dialog.open_for(None, &plain(&["Studio"]), "");
        assert!(dialog.prompt == "A fox" && dialog.problem.is_none());
        dialog.open_for(None, &[], "");
        assert_eq!(dialog.service, "");
    }

    #[test]
    fn opening_while_a_picture_is_made_shows_the_dialog_as_it_is() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &plain(&["Studio"]), "");
        dialog.prompt = "A fox".to_owned();
        dialog.busy = true;
        dialog.open = false;
        // Human: the picture being made is for "hero"; opening again with another selection changes nothing.
        dialog.open_for(None, &[], "");
        dialog.set_pieces(vec![Piece::layer("other", "Other", true)]);
        assert!(dialog.open && dialog.busy && dialog.fill && dialog.target.as_deref() == Some("hero") && dialog.service == "Studio");
        assert!(!dialog.has_pieces());
    }

    #[test]
    fn escape_or_a_click_beside_closes_only_when_no_picture_is_being_made() {
        assert_eq!(on_close(false), Did::Cancel);
        assert_eq!(on_close(true), Did::Nothing);
    }

    #[test]
    fn the_description_takes_the_keyboard_once_when_the_dialog_opens() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &plain(&["Studio"]), "");
        assert!(dialog.focus);
        dialog.focus = false;
        dialog.busy = true;
        dialog.open_for(None, &plain(&["Studio"]), "");
        assert!(!dialog.focus, "a dialog shown again while its picture is made has its field locked");
    }

    #[test]
    fn a_dialog_whose_document_is_gone_only_offers_to_close() {
        let mut dialog = Dialog { orphaned: true, ..Dialog::default() };
        assert!(dialog.only_close());
        // Human: while its picture is still being made, Cancel stays; the picture is kept when it comes.
        dialog.busy = true;
        assert!(!dialog.only_close());
    }

    #[test]
    fn a_lone_layer_with_a_picture_is_marked_and_several_layers_are_references() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &both(), "Studio");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true)]);
        assert!(dialog.editing() && dialog.pieces[0].change);
        // Human: editing keeps the picture's shape, and only a service that can edit is offered.
        assert_eq!((dialog.aspect.as_str(), dialog.service.as_str()), (SAME, "Gemini"));
        assert_eq!(dialog.shapes()[0], SAME);
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true), Piece::layer("logo", "Logo", true)]);
        assert!(!dialog.editing() && dialog.aspect == "1:1");
        // A lone layer without a picture is a reference too: there is nothing to replace.
        dialog.set_pieces(vec![Piece::layer("card", "Card", false)]);
        assert!(!dialog.editing());
    }

    #[test]
    fn marking_one_unmarks_the_other_and_a_file_is_only_a_reference() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("a", "A", true), Piece::layer("b", "B", false)]);
        dialog.mark(0);
        assert!(dialog.pieces[0].change && dialog.aspect == SAME);
        dialog.mark(1);
        assert!(!dialog.pieces[0].change && dialog.pieces[1].change);
        dialog.mark(1);
        assert!(!dialog.editing() && dialog.aspect == "1:1", "unmarked, the shape is a generation's again");
        dialog.add_file(full("a.png")).unwrap();
        dialog.mark(2);
        assert!(!dialog.editing());
        assert_eq!(dialog.pieces[2].name, "a.png");
        dialog.remove(0);
        assert_eq!(dialog.pieces.len(), 2);
        dialog.remove(9);
        assert_eq!(dialog.pieces.len(), 2);
    }

    #[test]
    fn at_most_four_references_and_one_to_change() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true)]);
        for name in ["a.png", "b.png", "c.png", "d.png"] {
            dialog.add_file(full(name)).unwrap();
        }
        assert_eq!(dialog.add_file(full("e.png")), Err("At most 4 reference pictures can be sent.".to_owned()));
        assert_eq!(dialog.pieces.len(), 5);
        // Human: unmarking the picture to change would make it a fifth reference.
        dialog.mark(0);
        assert!(dialog.editing());
        assert_eq!(dialog.problem.as_deref(), Some("At most 4 reference pictures can be sent."));
        // Paths that can't be sent are refused as a model's are.
        dialog.remove(4);
        assert_eq!(dialog.add_file(PathBuf::from(r"\\server\pictures\a.png")), Err(super::super::ON_A_SHARE.to_owned()));
    }

    #[test]
    fn more_than_five_layers_are_cut_to_five_and_said_so() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(["a", "b", "c", "d", "e", "f"].iter().map(|id| Piece::layer(id, id, false)).collect());
        assert_eq!(dialog.pieces.len(), 5);
        assert_eq!(
            dialog.problem.as_deref(),
            Some("Only the first 5 selected layers are in the row: one picture to change and 4 references can be sent.")
        );
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(["a", "b", "c", "d", "e"].iter().map(|id| Piece::layer(id, id, false)).collect());
        // Human: five references: the line under the row says so in place of what is sent, and nothing is sent.
        assert_eq!(
            dialog.too_many().as_deref(),
            Some("At most 4 reference pictures can be sent. Mark one as the picture to change, or take one out.")
        );
        assert_eq!(dialog.sends(), None);
        dialog.mark(0);
        assert!(dialog.problem.is_none() && dialog.too_many().is_none());
        assert_eq!(dialog.sends().as_deref(), Some("Sends 5 pictures to Gemini."));
    }

    #[test]
    fn the_line_never_says_more_pictures_than_are_asked_for() {
        // Human: how many pictures the arguments name: the one to change (a layer's, or layers drawn as one) and
        // the references.
        let named = |asked: &Value| {
            usize::from(asked.get("layer").is_some() || asked.get("layers").is_some()) + asked["references"].as_array().map_or(0, Vec::len)
        };
        let said = |dialog: &Dialog| dialog.sends().and_then(|line| line.split(' ').nth(1).and_then(|n| n.parse::<usize>().ok()));
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        let rows: [Vec<Piece>; 4] = [
            vec![Piece::layer("hero", "Hero", true)],
            vec![Piece::layer("hero", "Hero", true), Piece::layer("card", "Card", false)],
            vec![Piece::layer("card", "Card", false)],
            ["a", "b", "c", "d"].iter().map(|id| Piece::layer(id, id, false)).collect(),
        ];
        for row in rows {
            dialog.set_pieces(row);
            for marked in [None, Some(0)] {
                if let Some(index) = marked {
                    dialog.mark(index);
                }
                assert_eq!(said(&dialog), Some(named(&dialog.asked())), "{}", dialog.asked());
            }
        }
        dialog.add_file(full("a.png")).unwrap();
        assert_eq!(said(&dialog), Some(named(&dialog.asked())));
    }

    #[test]
    fn of_several_files_chosen_those_that_fit_are_added_and_the_rest_explained() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true), Piece::layer("card", "Card", false)]);
        dialog.mark(0);
        dialog.add_files(["a.png", "b.png", "c.png", "d.png", "e.png"].iter().map(|name| full(name)).collect());
        assert_eq!(dialog.pieces.len(), 5);
        assert_eq!(dialog.problem.as_deref(), Some("3 of the 5 files were added. At most 4 reference pictures can be sent."));
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.add_files(vec![full("a.png"), PathBuf::from(r"\\server\pictures\b.png")]);
        assert_eq!(dialog.problem.as_deref(), Some(format!("1 of the 2 files was added. {}", super::super::ON_A_SHARE).as_str()));
        dialog.add_files(vec![PathBuf::from(r"\\server\pictures\c.png")]);
        assert_eq!(dialog.problem.as_deref(), Some(super::super::ON_A_SHARE));
        dialog.add_files(vec![full("c.png")]);
        assert!(dialog.problem.is_none() && dialog.pieces.len() == 2);
    }

    #[test]
    fn the_line_about_what_is_sent_counts_the_pictures_and_names_the_service() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true)]);
        assert_eq!(dialog.sends().as_deref(), Some("Sends 1 picture to Gemini."));
        dialog.set_pieces(vec![Piece::layer("a", "A", true), Piece::layer("b", "B", true), Piece::layer("c", "C", false)]);
        assert_eq!(dialog.sends().as_deref(), Some("Sends 3 pictures to Gemini."));
    }

    #[test]
    fn with_pictures_only_services_that_edit_are_offered() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &plain(&["Studio"]), "");
        assert_eq!(dialog.offered(), ["Studio"]);
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true)]);
        assert!(dialog.offered().is_empty() && dialog.service.is_empty());
        // Taking the picture out makes it the plain dialog again.
        dialog.remove(0);
        assert_eq!((dialog.offered(), dialog.service.as_str()), (vec!["Studio"], "Studio"));
    }

    #[test]
    fn with_pictures_the_dialog_asks_in_edit_images_form() {
        let logo = full("logo.png");
        let shown = logo.to_string_lossy().into_owned();
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true), Piece::layer("card", "Card", false)]);
        dialog.add_file(logo).unwrap();
        dialog.mark(0);
        dialog.prompt = "Make it dusk".to_owned();
        assert_eq!(
            dialog.asked(),
            json!({ "instruction": "Make it dusk", "service": "Gemini", "size": "normal", "layer": "hero", "references": ["card", shown] })
        );
        // A layer without a picture is changed as it looks; a shape asked for is sent.
        dialog.mark(1);
        "16:9".clone_into(&mut dialog.aspect);
        dialog.large = true;
        assert_eq!(
            dialog.asked(),
            json!({ "instruction": "Make it dusk", "service": "Gemini", "size": "large", "layers": ["card"], "references": ["hero", shown], "aspect": "16:9" })
        );
        // References alone: a generation guided by them, with the shape chosen.
        dialog.mark(1);
        assert_eq!(dialog.asked()["aspect"], json!("16:9"));
        assert!(dialog.asked().get("layer").is_none() && dialog.asked().get("layers").is_none());
        // Human: what the dialog asks is read by the very function a model's call goes through.
        assert!(super::super::named(&dialog.asked()).is_ok());
    }

    #[test]
    fn replacing_is_offered_on_and_only_for_a_marked_layer_with_a_picture() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &both(), "");
        dialog.set_pieces(vec![Piece::layer("hero", "Hero", true), Piece::layer("card", "Card", false)]);
        assert!(dialog.replace && !dialog.replaceable());
        dialog.mark(0);
        assert!(dialog.replaceable());
        dialog.mark(1);
        assert!(!dialog.replaceable());
    }
}
