//! The action search: every command of the app in one list, found by typing (the command palette,
//! opened with ⌘P or the rail's "…").

use egui::{Align2, Color32, FontId, Key, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::editor::Tool;
use crate::icons;
use crate::ops::{AlignTo, Order};
use crate::theme::{self, ACCENT, ACTIONS_HEIGHT, ACTIONS_RADIUS, ACTIONS_ROW, ACTIONS_TOP, ACTIONS_WIDTH};

/// Something the app can do, as the action search and the keyboard ask for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Back to the home screen with the files.
    Dashboard,
    /// Save the document.
    Save,
    /// Save the document under a new name.
    SaveAs,
    /// Save the document under a new name in the other format: a `.pen` file as a bundle, a bundle as a `.pen` file.
    SaveAsOther,
    /// A new, empty document.
    NewFile,
    /// Open a document from disk.
    OpenFile,
    /// Close the document's tab.
    CloseTab,
    /// Add a picture or SVG file as a layer.
    Import,
    /// Open the "Generate Image…" dialog.
    GenerateImage,
    /// Step back.
    Undo,
    /// Step forward again.
    Redo,
    /// Copy the selection in place.
    Duplicate,
    /// Delete the selection.
    Delete,
    /// Copy the selection.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste what was copied.
    Paste,
    /// Group the selection.
    Group,
    /// Put the selection in a frame.
    FrameSelection,
    /// Take a group or frame apart.
    Ungroup,
    /// Align the selection.
    Align(AlignTo),
    /// Mirror the selection; `true` is left to right.
    Flip(bool),
    /// Turn the selection by this many degrees, counter-clockwise.
    Rotate(f32),
    /// Move the selection in the stack.
    Reorder(Order),
    /// Lay the selection out with flex.
    AddLayout,
    /// Take the flex layout off.
    RemoveLayout,
    /// Align text.
    TextAlign(&'static str),
    /// Make the selection a component.
    CreateComponent,
    /// Make a component a plain layer again.
    DetachComponent,
    /// Turn an instance into plain layers.
    DetachInstance,
    /// Select the component an instance shows.
    GoToComponent,
    /// Select nothing.
    Deselect,
    /// Select everything beside the selection.
    SelectAll,
    /// Select the selection's parent.
    SelectParent,
    /// Select the selection's children.
    EnterSelection,
    /// Export the selection as a file of this format.
    Export(crate::export::Format),
    /// Zoom in a step.
    ZoomIn,
    /// Zoom out a step.
    ZoomOut,
    /// Zoom to this many times life size.
    ZoomTo(f32),
    /// Fit the whole design in the view.
    ZoomToFit,
    /// Fit the selection in the view.
    ZoomToSelection,
    /// Show a view of the sidebar, by its name.
    Show(&'static str),
    /// Choose a tool.
    Tool(Tool),
    /// Open the settings.
    Settings,
    /// Look for a newer version of the app now.
    CheckForUpdates,
    /// Open the recent file at this place in the list.
    OpenRecent(usize),
    /// Bring the tab at this place to the front.
    ShowTab(usize),
    /// The tab before the one in front.
    PreviousTab,
    /// The tab after the one in front.
    NextTab,
    /// Move the tab in front one place; `true` is to the left.
    MoveTab(bool),
    /// Switch between the light and the dark look.
    ToggleTheme,
    /// Fill the screen with the window, or stop doing so.
    FullScreen,
    /// Open the action search.
    Actions,
    /// Open find layers.
    FindLayers,
    /// Hide or show everything but the canvas.
    ToggleInterface,
}

/// One line of the action search.
pub struct Action {
    /// What it does.
    pub command: Command,
    /// Its icon.
    pub icon: &'static str,
    /// Its name.
    pub label: &'static str,
    /// The heading it is listed under.
    pub category: &'static str,
    /// Its keyboard shortcut, key by key; empty when it has none.
    pub keys: &'static [&'static str],
    /// More words it is found by.
    pub words: &'static str,
}

const fn action(
    command: Command,
    icon: &'static str,
    label: &'static str,
    category: &'static str,
    keys: &'static [&'static str],
    words: &'static str,
) -> Action {
    Action { command, icon, label, category, keys, words }
}

/// Every action, in the order and under the headings the search lists them.
// Agent: icons MUST be bundled Lucide names; a test checks it.
pub const ACTIONS: [Action; 77] = [
    action(Command::Dashboard, "layout-grid", "Open dashboard", "File", &[], "home files projects recent"),
    action(Command::Save, "save", "Save", "File", &["⌘", "S"], "write store disk"),
    action(Command::SaveAs, "save-all", "Save as…", "File", &["⌘", "⇧", "S"], "copy duplicate file disk"),
    action(Command::SaveAsOther, "package", "Save as bundle or .pen file…", "File", &[], "pencraft package zip pictures one file convert send"),
    action(Command::NewFile, "file-plus", "New file", "File", &["⌘", "⇧", "N"], "create document blank"),
    action(Command::OpenFile, "folder-open", "Open file…", "File", &["⌘", "O"], "browse pen file document"),
    action(Command::CloseTab, "file-x", "Close tab", "File", &["⌘", "W"], "close file document window"),
    action(Command::Undo, "undo-2", "Undo", "Edit", &["⌘", "Z"], "revert back"),
    action(Command::Redo, "redo-2", "Redo", "Edit", &["⌘", "⇧", "Z"], "again forward"),
    action(Command::Duplicate, "copy", "Duplicate", "Edit", &["⌘", "D"], "clone copy"),
    action(Command::Delete, "trash", "Delete", "Edit", &["⌫"], "remove erase"),
    action(Command::Copy, "copy", "Copy", "Edit", &["⌘", "C"], "clipboard"),
    action(Command::Cut, "scissors", "Cut", "Edit", &["⌘", "X"], "clipboard"),
    action(Command::Paste, "clipboard", "Paste", "Edit", &["⌘", "V"], "clipboard insert"),
    action(Command::Import, "image", "Import image or SVG…", "Edit", &[], "place insert picture photo file upload"),
    action(Command::GenerateImage, "image-plus", "Generate image…", "Edit", &[], "ai picture photo illustration make create prompt"),
    action(Command::Group, "group", "Group selection", "Arrange", &["⌘", "G"], "combine"),
    action(Command::FrameSelection, "frame", "Frame selection", "Arrange", &["⌥", "⌘", "G"], "wrap container"),
    action(Command::Ungroup, "ungroup", "Ungroup", "Arrange", &["⌘", "⇧", "G"], "split release"),
    action(Command::Align(AlignTo::Left), "align-start-vertical", "Align left", "Arrange", &["⌥", "A"], ""),
    action(Command::Align(AlignTo::Center), "align-center-vertical", "Align center", "Arrange", &["⌥", "H"], "horizontal"),
    action(Command::Align(AlignTo::Right), "align-end-vertical", "Align right", "Arrange", &["⌥", "D"], ""),
    action(Command::Align(AlignTo::Top), "align-start-horizontal", "Align top", "Arrange", &["⌥", "W"], ""),
    action(Command::Align(AlignTo::Middle), "align-center-horizontal", "Align middle", "Arrange", &["⌥", "V"], "vertical"),
    action(Command::Align(AlignTo::Bottom), "align-end-horizontal", "Align bottom", "Arrange", &["⌥", "S"], ""),
    action(Command::Flip(true), "square-centerline-dashed-vertical", "Flip horizontal", "Arrange", &["⇧", "H"], "mirror"),
    action(Command::Flip(false), "square-centerline-dashed-horizontal", "Flip vertical", "Arrange", &["⇧", "V"], "mirror"),
    action(Command::Rotate(-90.0), "rotate-cw", "Rotate 90° right", "Arrange", &[], "turn clockwise quarter"),
    action(Command::Rotate(90.0), "rotate-ccw", "Rotate 90° left", "Arrange", &[], "turn counter-clockwise quarter"),
    action(Command::Reorder(Order::Front), "bring-to-front", "Bring to Front", "Arrange", &["]"], "order z-index raise top"),
    action(Command::Reorder(Order::Forward), "bring-to-front", "Bring forward", "Arrange", &["⌘", "]"], "order z-index raise"),
    action(Command::Reorder(Order::Backward), "send-to-back", "Send backward", "Arrange", &["⌘", "["], "order z-index lower"),
    action(Command::Reorder(Order::Back), "send-to-back", "Send to Back", "Arrange", &["["], "order z-index lower bottom"),
    action(Command::AddLayout, "layout-panel-left", "Add flex layout", "Layout", &["⇧", "A"], "auto layout stack"),
    action(Command::RemoveLayout, "layout-panel-left", "Remove flex layout", "Layout", &["⌥", "⇧", "A"], "auto layout freeform"),
    action(Command::TextAlign("left"), "text-align-start", "Text align left", "Text", &["⌥", "⌘", "L"], ""),
    action(Command::TextAlign("center"), "text-align-center", "Text align center", "Text", &["⌥", "⌘", "T"], ""),
    action(Command::TextAlign("right"), "text-align-end", "Text align right", "Text", &["⌥", "⌘", "R"], ""),
    action(Command::TextAlign("justify"), "text-align-justify", "Text align justified", "Text", &["⌥", "⌘", "J"], ""),
    action(Command::CreateComponent, "component", "Create Component", "Components", &["⌥", "⌘", "K"], "reusable symbol main component"),
    action(Command::DetachComponent, "component", "Detach Component", "Components", &[], "reusable unlink"),
    action(Command::DetachInstance, "component", "Detach Instance", "Components", &["⌥", "⌘", "X"], "unlink break"),
    action(Command::GoToComponent, "locate-fixed", "Go to component", "Components", &[], "main component source reveal"),
    action(Command::Deselect, "square-dashed", "Deselect", "Selection", &["esc"], "none clear"),
    action(Command::SelectAll, "square-dashed", "Select all", "Selection", &["⌘", "A"], "everything"),
    action(Command::SelectParent, "square-dashed", "Select parent", "Selection", &["⇧", "↵"], "up container"),
    action(Command::EnterSelection, "square-dashed", "Edit selection", "Selection", &["↵"], "children inside enter"),
    action(Command::Export(crate::export::Format::Png), "download", "Export selection as PNG", "Export", &[], "image picture bitmap"),
    action(Command::Export(crate::export::Format::Jpeg), "download", "Export selection as JPEG", "Export", &[], "image picture jpg photo"),
    action(Command::Export(crate::export::Format::WebP), "download", "Export selection as WebP", "Export", &[], "image picture bitmap"),
    action(Command::Export(crate::export::Format::Pdf), "download", "Export selection as PDF", "Export", &[], "document pages print"),
    action(Command::Export(crate::export::Format::Svg), "download", "Export selection as SVG", "Export", &[], "vector shapes"),
    action(Command::ZoomIn, "zoom-in", "Zoom in", "View", &["⌘", "+"], "magnify closer"),
    action(Command::ZoomOut, "zoom-out", "Zoom out", "View", &["⌘", "−"], "smaller further"),
    action(Command::ZoomTo(1.0), "search", "Zoom to 100%", "View", &["⌘", "0"], "actual size reset"),
    action(Command::ZoomTo(0.5), "search", "Zoom to 50%", "View", &[], "half"),
    action(Command::ZoomTo(2.0), "search", "Zoom to 200%", "View", &[], "double"),
    action(Command::ZoomToFit, "maximize", "Zoom to fit", "View", &["⇧", "1"], "all everything"),
    action(Command::ZoomToSelection, "maximize", "Zoom to selection", "View", &["⇧", "2"], "focus"),
    action(Command::Show("layers"), "layers", "Show layers", "View", &[], "sidebar tree"),
    action(Command::FindLayers, "search", "Find layers", "View", &["⌘", "F"], "search filter text"),
    action(Command::Show("components"), "component", "Show components", "View", &[], "sidebar symbols"),
    action(Command::Tool(Tool::Move), "mouse-pointer-2", "Move tool", "Tools", &["V"], "select pointer cursor arrow"),
    action(Command::Tool(Tool::Hand), "hand", "Hand tool", "Tools", &["H"], "pan drag"),
    action(Command::Tool(Tool::Scale), "scaling", "Scale tool", "Tools", &["K"], "resize proportional"),
    action(Command::Tool(Tool::Frame), "frame", "Frame tool", "Tools", &["F"], "artboard board container"),
    action(Command::Tool(Tool::Text), "type", "Text tool", "Tools", &["T"], "type write"),
    action(Command::Tool(Tool::Note), "sticky-note", "Sticky note tool", "Tools", &["N"], "comment memo"),
    action(Command::Tool(Tool::Pen), "pen-tool", "Pen tool", "Tools", &["P"], "path vector curve bezier draw"),
    action(Command::Tool(Tool::Connector), "spline", "Connector tool", "Tools", &["C"], "wire line arrow link edge node graph flow"),
    action(Command::Tool(Tool::Rectangle), "square", "Rectangle tool", "Tools", &["R"], "box shape"),
    action(Command::Tool(Tool::Ellipse), "circle", "Ellipse tool", "Tools", &["O"], "oval shape"),
    action(Command::Tool(Tool::Polygon), "triangle", "Polygon tool", "Tools", &[], "triangle shape"),
    action(Command::Tool(Tool::Icon), "sparkle", "Icon tool", "Tools", &[], "symbol glyph"),
    action(Command::Settings, "settings", "Settings…", "Application", &["⌘", ","], "preferences options"),
    action(Command::CheckForUpdates, "refresh-cw", "Check for updates…", "Application", &[], "version release new upgrade download"),
    action(Command::ToggleInterface, "panel-left", "Hide interface", "Application", &["⌘", "\\"], "ui panels show"),
];

/// Every action of the search, in listing order.
pub fn actions() -> impl Iterator<Item = &'static Action> {
    ACTIONS.iter()
}

/// The actions matching `query`: every word typed must be in the action's name, heading or extra words.
#[must_use]
pub fn matching(query: &str) -> Vec<&'static Action> {
    let query = query.to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    actions()
        .filter(|a| {
            let text = format!("{} {} {}", a.label, a.category, a.words).to_lowercase();
            words.iter().all(|word| text.contains(word))
        })
        .collect()
}

/// The action search's state.
#[derive(Default)]
pub struct Palette {
    open: bool,
    query: String,
    /// The highlighted line among those matching.
    cursor: usize,
}

impl Palette {
    /// Opens the search, or closes it when it is open.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.query.clear();
        self.cursor = 0;
    }

    /// Draws the search over everything when it is open. Returns the command chosen, which also closes it.
    // Human: a field on top, the actions under their headings, the highlighted one in the
    // accent colour; ↑ and ↓ move the highlight, ↵ runs it, esc or a click elsewhere closes.
    // Agent: MAIN-THREAD; takes keyboard focus while open.
    pub fn show(&mut self, ctx: &egui::Context, dark: bool) -> Option<Command> {
        if !self.open {
            return None;
        }
        let palette = theme::palette(dark);
        let screen = ctx.content_rect();
        let width = ACTIONS_WIDTH.min(screen.width() - 40.0);
        let found = matching(&self.query);
        let (up, down, enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(egui::Modifiers::NONE, Key::ArrowUp),
                i.consume_key(egui::Modifiers::NONE, Key::ArrowDown),
                i.consume_key(egui::Modifiers::NONE, Key::Enter),
                i.consume_key(egui::Modifiers::NONE, Key::Escape),
            )
        });
        if down {
            self.cursor = (self.cursor + 1).min(found.len().saturating_sub(1));
        }
        if up {
            self.cursor = self.cursor.saturating_sub(1);
        }
        self.cursor = self.cursor.min(found.len().saturating_sub(1));
        let mut chosen = if enter { found.get(self.cursor).map(|a| a.command) } else { None };
        let area = egui::Area::new(egui::Id::unique("action-search"))
            .fixed_pos(pos2(screen.center().x - width / 2.0, screen.top() + ACTIONS_TOP))
            .order(egui::Order::Foreground)
            .fade_in(false)
            .show(ctx, |ui| {
                egui::Frame::new().fill(palette.island).stroke(Stroke::new(1.0, palette.outline)).corner_radius(ACTIONS_RADIUS).show(ui, |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    let (bar, _) = ui.allocate_exact_size(vec2(width, 48.0), Sense::hover());
                    icons::paint(
                        ui.painter(),
                        Rect::from_center_size(pos2(bar.left() + 25.0, bar.center().y), egui::Vec2::splat(15.0)),
                        "search",
                        palette.muted,
                    );
                    let field = Rect::from_min_max(pos2(bar.left() + 43.0, bar.top()), pos2(bar.right() - 14.0, bar.bottom()));
                    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(field).layout(egui::Layout::left_to_right(egui::Align::Center)));
                    let edit = inner.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .frame(egui::Frame::new())
                            .background_color(Color32::TRANSPARENT)
                            .hint_text("Search actions…")
                            .desired_width(field.width()),
                    );
                    if edit.changed() {
                        self.cursor = 0;
                    }
                    edit.request_focus();
                    ui.painter().hline(bar.x_range(), bar.bottom(), Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
                    egui::ScrollArea::vertical().max_height(ACTIONS_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
                        ui.add_space(6.0);
                        if found.is_empty() {
                            let (row, _) = ui.allocate_exact_size(vec2(width, ACTIONS_ROW), Sense::hover());
                            ui.painter().text(row.center(), Align2::CENTER_CENTER, "No actions found", FontId::proportional(12.5), palette.muted);
                        }
                        let mut heading = "";
                        for (index, found_action) in found.iter().enumerate() {
                            if found_action.category != heading {
                                heading = found_action.category;
                                let (row, _) = ui.allocate_exact_size(vec2(width, 26.0), Sense::hover());
                                ui.painter().text(
                                    pos2(row.left() + 17.0, row.center().y + 2.0),
                                    Align2::LEFT_CENTER,
                                    heading,
                                    FontId::proportional(11.0),
                                    palette.muted,
                                );
                            }
                            let lit = index == self.cursor;
                            let response = line(ui, found_action, lit, width, palette);
                            if response.hovered() && ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                self.cursor = index;
                            }
                            if lit && (up || down) {
                                response.scroll_to_me(None);
                            }
                            if response.clicked() {
                                chosen = Some(found_action.command);
                            }
                        }
                        ui.add_space(6.0);
                    });
                });
            });
        let clicked_away = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p)));
        if chosen.is_some() || escape || clicked_away {
            self.open = false;
        }
        chosen
    }
}

/// One action's line: its icon and name, and its shortcut as key caps at the right; `lit` is the highlighted one.
fn line(ui: &mut egui::Ui, action: &Action, lit: bool, width: f32, palette: theme::Palette) -> egui::Response {
    let (row, response) = ui.allocate_exact_size(vec2(width, ACTIONS_ROW), Sense::click());
    let plate = row.shrink2(vec2(7.0, 0.0));
    if lit {
        ui.painter().rect_filled(plate, 7.0, ACCENT);
    }
    let ink = if lit { Color32::WHITE } else { palette.foreground };
    icons::paint(ui.painter(), Rect::from_center_size(pos2(plate.left() + 18.0, row.center().y), egui::Vec2::splat(14.0)), action.icon, ink);
    ui.painter().text(pos2(plate.left() + 36.0, row.center().y), Align2::LEFT_CENTER, action.label, FontId::proportional(13.0), ink);
    let mut right = plate.right() - 10.0;
    // Human: a cap is as wide as its key's name needs: one sign on a Mac, a word like "Ctrl" elsewhere.
    for key in crate::keys::caps(action.keys).iter().rev() {
        let ink = if lit { Color32::WHITE } else { palette.muted };
        let name = ui.painter().layout_no_wrap((*key).to_owned(), FontId::proportional(10.5), ink);
        let width = (name.size().x + 8.0).max(19.0);
        let cap = Rect::from_min_size(pos2(right - width, row.center().y - 9.0), vec2(width, 18.0));
        right = cap.left() - 3.0;
        let (fill, edge) =
            if lit { (Color32::from_white_alpha(40), Color32::TRANSPARENT) } else { (palette.hover.gamma_multiply(0.6), palette.outline) };
        ui.painter().rect(cap, 4.0, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
        ui.painter().galley(cap.center() - name.size() / 2.0, name, ink);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_a_bundled_icon_and_a_heading_listed_in_one_run() {
        let mut seen: Vec<&str> = Vec::new();
        for a in actions() {
            assert!(icons::outline("lucide", a.icon).is_some(), "{}", a.icon);
            if seen.last() != Some(&a.category) {
                assert!(!seen.contains(&a.category), "{} is listed twice", a.category);
                seen.push(a.category);
            }
        }
        assert_eq!(seen, ["File", "Edit", "Arrange", "Layout", "Text", "Components", "Selection", "Export", "View", "Tools", "Application"]);
    }

    #[test]
    fn actions_are_found_by_name_heading_or_extra_words() {
        assert_eq!(matching("").len(), ACTIONS.len());
        assert_eq!(matching("save").iter().map(|a| a.label).collect::<Vec<_>>(), ["Save", "Save as…", "Save as bundle or .pen file…"]);
        assert!(matching("proportional").iter().any(|a| a.command == Command::Tool(Tool::Scale)));
        assert!(matching("zoom fit").iter().any(|a| a.command == Command::ZoomToFit));
        assert_eq!(matching("no such thing").len(), 0);
    }
}
