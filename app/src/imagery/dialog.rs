//! The "Generate Image…" window: a description, a service, a shape and a size, and a button.

use egui::{Color32, FontId, Key, Margin, Modifiers, RichText, Sense, Stroke, Ui, vec2};
use serde_json::{Value, json};

use super::service::ASPECTS;
use crate::settings::{caption, choice_at, row, switch};
use crate::theme::{
    CHIP_RADIUS, DIALOG_BACKDROP, DIALOG_BUTTON, DIALOG_GAP, DIALOG_PADDING, DIALOG_TITLE_SIZE, FIELD_HEIGHT, FIELD_MARGIN, GENERATE_LINES,
    GENERATE_WIDTH, PROBLEM, Palette, SETTINGS_RADIUS, SIDEBAR_TEXT_SIZE,
};

/// The size choice's words for a picture of about 1024 pixels, and of about 2048.
const NORMAL: &str = "Normal";
/// See [`NORMAL`].
const LARGE: &str = "Large";

/// What the person did in the dialog this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Did {
    /// Nothing that concerns the app.
    Nothing,
    /// Pressed "Generate", or ⌘/Ctrl+Enter in the description.
    Generate,
    /// Pressed "Cancel" or "Close", or closed the window (Escape, a click beside it) while no picture was being
    /// made.
    Cancel,
    /// Pressed "Open Settings" where there is no service.
    OpenSettings,
}

/// What Escape or a click beside the window does: it closes the window, but while a picture is being made
/// (`busy`) only the Cancel button gives it up.
// Human: a picture costs money; a stray key or click must not throw one away that is nearly there.
fn on_close(busy: bool) -> Did {
    if busy { Did::Nothing } else { Did::Cancel }
}

/// The dialog's state.
#[derive(Default)]
#[expect(clippy::struct_excessive_bools, reason = "each is one independent part of the window's state, set by the app and its tests")]
pub struct Dialog {
    /// Whether the window shows.
    pub open: bool,
    /// The description.
    pub prompt: String,
    /// The chosen service's name; empty when there is none.
    pub service: String,
    /// The chosen shape, one of [`ASPECTS`].
    pub aspect: String,
    /// About 2048 pixels instead of about 1024.
    pub large: bool,
    /// The layer that was selected when the dialog opened and can take a picture.
    pub target: Option<String>,
    /// Give the picture to [`Dialog::target`] instead of adding a layer.
    pub fill: bool,
    /// A picture is being made.
    pub busy: bool,
    /// What went wrong the last time.
    pub problem: Option<String>,
    /// The document the dialog was opened for is closed: nothing can be generated for it, only closed.
    pub orphaned: bool,
    /// The services that worked when the dialog opened.
    services: Vec<String>,
    /// The description is to take the keyboard next time it is drawn.
    focus: bool,
}

impl Dialog {
    /// Opens the dialog. `target` is the selected layer when it can take a picture; `services` are the ones
    /// that work and `default` the settings' choice among them.
    // Human: while a picture is being made the dialog is shown again as it is: the picture is still for the
    // layer and the service it was asked for, and the person can wait for it or cancel it.
    pub fn open_for(&mut self, target: Option<String>, services: &[String], default: &str) {
        self.open = true;
        if self.busy {
            return;
        }
        self.problem = None;
        self.focus = true;
        self.fill = target.is_some();
        self.target = target;
        self.services = services.to_vec();
        if !services.contains(&self.service) {
            self.service = services.iter().find(|name| name.as_str() == default).or_else(|| services.first()).cloned().unwrap_or_default();
        }
        if !ASPECTS.contains(&self.aspect.as_str()) {
            ASPECTS[0].clone_into(&mut self.aspect);
        }
    }

    /// What the dialog asks for, in the form of a `generate_image` call's arguments.
    #[must_use]
    pub fn asked(&self) -> Value {
        json!({ "prompt": self.prompt, "service": self.service, "aspect": self.aspect, "size": if self.large { "large" } else { "normal" } })
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
        ui.add(egui::Label::new(RichText::new("Generate image").size(DIALOG_TITLE_SIZE).color(palette.foreground)).selectable(false));
        ui.add_space(DIALOG_GAP);
        if self.services.is_empty() {
            ui.add(egui::Label::new(RichText::new(super::NO_SERVICE).color(palette.muted)).wrap());
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
        // Human: the form is locked while its picture is made, and when its document is gone.
        let locked = self.busy || self.only_close();
        caption(ui, "Description");
        let field = egui::Frame::new()
            .stroke(Stroke::new(1.0, palette.outline))
            .corner_radius(CHIP_RADIUS)
            .inner_margin(Margin::symmetric(FIELD_MARGIN[0], FIELD_MARGIN[1]));
        let hint = RichText::new("What the picture shows: subject, setting, light, style").color(palette.muted);
        let description =
            egui::TextEdit::multiline(&mut self.prompt).frame(field).hint_text(hint).desired_rows(GENERATE_LINES).desired_width(f32::INFINITY);
        let description = ui.add_enabled(!locked, description);
        // Human: the description takes the keyboard once when the dialog opens, so the person can type at
        // once; after that a click elsewhere keeps it elsewhere.
        if !locked && std::mem::take(&mut self.focus) {
            description.request_focus();
        }
        let can_generate = !locked && !self.prompt.trim().is_empty();
        // Human: ⌘/Ctrl+Enter in the description is Generate; Enter alone is a new line, as in other fields.
        if can_generate && description.has_focus() && ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter)) {
            *did = Did::Generate;
        }
        ui.add_space(DIALOG_GAP);
        let mut size = if self.large { LARGE } else { NORMAL }.to_owned();
        ui.add_enabled_ui(!locked, |ui| {
            let services: Vec<&str> = self.services.iter().map(String::as_str).collect();
            // Human: the service is only offered when there is a choice.
            let several = services.len() > 1;
            ui.columns(if several { 3 } else { 2 }, |columns| {
                let mut at = 0;
                if several {
                    labelled(&mut columns[0], "Service", &mut self.service, &services, palette);
                    at = 1;
                }
                labelled(&mut columns[at], "Shape", &mut self.aspect, &ASPECTS, palette);
                labelled(&mut columns[at + 1], "Size", &mut size, &[NORMAL, LARGE], palette);
            });
            if self.target.is_some() {
                ui.add_space(DIALOG_GAP);
                let otherwise = "When off, the picture is a new layer in the middle of the view.";
                ui.scope(|ui| row(ui, "Use as the fill of the selection", Some(otherwise), palette, |ui| switch(ui, &mut self.fill, palette)));
            }
        });
        self.large = size == LARGE;
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
            let label = if self.busy { "Generating…" } else { "Generate" };
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
    caption(ui, name);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), FIELD_HEIGHT), Sense::hover());
    choice_at(ui, rect, name, value, options, FontId::proportional(SIDEBAR_TEXT_SIZE), palette);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_dialog_asks_in_the_tools_own_form() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &["Studio".to_owned(), "Gemini".to_owned()], "Gemini");
        assert!(dialog.open && dialog.service == "Gemini" && dialog.fill && dialog.target.as_deref() == Some("hero"));
        dialog.prompt = "A red fox".to_owned();
        dialog.aspect = "16:9".to_owned();
        dialog.large = true;
        assert_eq!(dialog.asked(), json!({ "prompt": "A red fox", "service": "Gemini", "aspect": "16:9", "size": "large" }));
    }

    #[test]
    fn opening_again_keeps_the_description_and_forgets_the_last_problem() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &["Studio".to_owned()], "");
        assert!(dialog.service == "Studio" && !dialog.fill && dialog.aspect == "1:1");
        dialog.prompt = "A fox".to_owned();
        dialog.problem = Some("no".to_owned());
        dialog.open = false;
        dialog.open_for(None, &["Studio".to_owned()], "");
        assert!(dialog.prompt == "A fox" && dialog.problem.is_none());
        dialog.open_for(None, &[], "");
        assert_eq!(dialog.service, "");
    }

    #[test]
    fn opening_while_a_picture_is_made_shows_the_dialog_as_it_is() {
        let mut dialog = Dialog::default();
        dialog.open_for(Some("hero".to_owned()), &["Studio".to_owned()], "");
        dialog.prompt = "A fox".to_owned();
        dialog.busy = true;
        dialog.open = false;
        // Human: the picture being made is for "hero"; opening again with another selection changes nothing.
        dialog.open_for(None, &[], "");
        assert!(dialog.open && dialog.busy && dialog.fill && dialog.target.as_deref() == Some("hero") && dialog.service == "Studio");
    }

    #[test]
    fn escape_or_a_click_beside_closes_only_when_no_picture_is_being_made() {
        assert_eq!(on_close(false), Did::Cancel);
        assert_eq!(on_close(true), Did::Nothing);
    }

    #[test]
    fn the_description_takes_the_keyboard_once_when_the_dialog_opens() {
        let mut dialog = Dialog::default();
        dialog.open_for(None, &["Studio".to_owned()], "");
        assert!(dialog.focus);
        dialog.focus = false;
        dialog.busy = true;
        dialog.open_for(None, &["Studio".to_owned()], "");
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
}
