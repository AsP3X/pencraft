//! The panels around the canvas: the layers list of the sidebar, and the properties island on the right, which
//! follows the user's properties design: a header, then sections with a title row each, divided by lines.

use egui::{Align, Align2, Color32, FontId, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, pos2, vec2};
use pen_doc::color::Rgba;
use pen_doc::edit;
use pen_doc::model::{Corners, Kind, Node, Paint, Paints, StrokeWidth};
use pen_doc::scene::SceneNode;
use pen_doc::value::{Flag, Num, Sizing};
use pen_doc::vars::Vars;
use serde_json::Value;

use crate::editor::{Editor, Role};
use crate::effects::{self, EffectKind};
use crate::export::{self, Format, Options};
use crate::icons;
use crate::material::{self, Appearance, Depth, Knob, Material, Thickness, Variant};
use crate::ops::{AlignTo, Order};
use crate::render::color32;
use crate::shader;
use crate::theme::{
    self, ACCENT, PROBLEM, PROPS_CANCEL, PROPS_GAP, PROPS_ICON, PROPS_MENU_WIDTH, PROPS_PADDING, PROPS_RADIUS, PROPS_ROW, PROPS_SMALL_TEXT,
    PROPS_TEXT, Palette, SEND_ARROW, SEND_BUTTON,
};

/// A short symbol for each kind of layer in the layers list.
fn kind_symbol(kind: &Kind) -> &'static str {
    match kind {
        Kind::Frame => "#",
        Kind::Group => "▣",
        Kind::Rectangle => "▭",
        Kind::Ellipse => "◯",
        Kind::Polygon => "△",
        Kind::Path | Kind::Line => "∿",
        Kind::Connector => "⤳",
        Kind::Text => "T",
        Kind::Note => "▤",
        Kind::Icon | Kind::IconFont => "★",
        Kind::Script => "ƒ",
        Kind::Browser => "◎",
        Kind::Ref => "◇",
        Kind::Other(_) => "?",
    }
}

/// One property change from the panel, applied to the node after it is drawn.
type Change = Box<dyn Fn(&mut Node)>;

/// What a fill or stroke editor decided.
#[derive(Clone)]
enum PaintEdit {
    /// The paints are now these.
    Set(Paints),
    /// The last paint was taken off.
    Clear,
}

impl PaintEdit {
    fn paints(&self) -> Option<Paints> {
        match self {
            PaintEdit::Set(paints) => Some(paints.clone()),
            PaintEdit::Clear => None,
        }
    }
}

/// What the properties island shows of changing a picture by description (the quick field and "Edit with AI…"
/// on an image fill), and what the person asked of it.
#[derive(Default)]
pub struct Pictures {
    /// The service the quick field sends to: one that can edit; `None` when there is none.
    pub service: Option<String>,
    /// The quick field's words.
    pub text: String,
    /// What went wrong the last time.
    pub problem: Option<String>,
    /// A picture of the person's is being made (by the dialog or the field): the field is off.
    pub busy: bool,
    /// The field itself is having it made: progress and Cancel show.
    pub ours: bool,
    /// The name of the layer the quick field is having a picture made for, when it is doing so (for this layer
    /// or another).
    pub making_for: Option<String>,
    /// The dark look is on.
    pub dark: bool,
    /// What the person asked this frame.
    pub asked: Option<PictureAsk>,
}

/// What the person asked of a picture in the properties island.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PictureAsk {
    /// "Edit with AI…": the dialog, with the layer marked.
    Dialog,
    /// Return or the quick field's button: change the picture as its words say.
    Quick,
    /// Cancel: give up the quick field's picture.
    Cancel,
}

/// The layers list: the document tree, with show/hide toggles.
pub fn layers(ui: &mut Ui, editor: &mut Editor, dark: bool) {
    if editor.doc.children.is_empty() {
        ui.label(RichText::new("Empty canvas.").color(theme::palette(dark).muted));
        return;
    }
    let mut action = None;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        let nodes = editor.doc.children.clone();
        for node in nodes.iter().rev() {
            layer_row(ui, editor, node, 0, dark, &mut action);
        }
    });
    if let Some(action) = action {
        apply_layer_action(editor, action);
    }
}

enum LayerAction {
    Select { id: String, add: bool },
    ToggleHidden(String),
}

/// One line of the layers list.
struct Row<'a> {
    /// The layer's id: a document id, or `instance/inner` inside an instance.
    id: &'a str,
    name: String,
    kind: &'a Kind,
    hidden: bool,
    /// The colour of a component's name, or of an instance's and what is inside it.
    linked: Option<Color32>,
    has_kids: bool,
    /// Top-level layers start open.
    top: bool,
}

/// Draws one line of the layers list; returns whether its children are shown.
fn row(ui: &mut Ui, editor: &Editor, line: &Row, depth: usize, dark: bool, action: &mut Option<LayerAction>) -> bool {
    let palette = theme::palette(dark);
    let id = line.id;
    let selected = editor.selection.iter().any(|s| s == id);
    let open_id = ui.scope_id().with(("layer-open", id));
    // Human: containers start closed below the top level, unless they hold the selection. A layer inside an
    // instance is held by the instance, and by everything the instance is in.
    let holds_selection = editor.selection.iter().any(|s| {
        let inside = s.strip_prefix(id).is_some_and(|rest| rest.starts_with('/'));
        let root = s.split('/').next().unwrap_or(s);
        inside || (s != id && edit::ancestry(&editor.doc, root).iter().any(|a| a == id))
    });
    let mut open = ui.data_mut(|d| *d.get_temp_mut_or(open_id, line.top));
    if holds_selection {
        open = true;
    }
    let shown = ui.horizontal(|ui| {
        ui.add_space(f32::from(u8::try_from(depth.min(20)).unwrap_or(20)) * 12.0);
        if line.has_kids {
            if ui.add(egui::Button::new(if open { "▾" } else { "▸" }).frame(false).small()).clicked() {
                open = !open;
            }
        } else {
            ui.add_space(14.0);
        }
        let text_color = if line.hidden {
            palette.muted
        } else if let Some(color) = line.linked {
            color
        } else {
            palette.foreground
        };
        ui.label(RichText::new(kind_symbol(line.kind)).color(palette.muted).monospace());
        let label = ui.add(egui::Label::new(RichText::new(&line.name).color(text_color)).truncate().sense(egui::Sense::click()));
        if label.clicked() {
            *action = Some(LayerAction::Select { id: id.to_owned(), add: ui.input(|i| i.modifiers.shift || i.modifiers.command) });
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let eye = if line.hidden { "○" } else { "●" };
            if ui
                .add(egui::Button::new(RichText::new(eye).small().color(palette.muted)).frame(false))
                .on_hover_text(if line.hidden { "Show" } else { "Hide" })
                .clicked()
            {
                *action = Some(LayerAction::ToggleHidden(id.to_owned()));
            }
        });
    });
    if selected {
        ui.painter().rect_filled(shown.response.rect.expand2(egui::vec2(4.0, 1.0)), 4.0, ACCENT.gamma_multiply(0.18));
    }
    if holds_selection == open || !holds_selection {
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }
    open && line.has_kids
}

fn layer_row(ui: &mut Ui, editor: &Editor, node: &Node, depth: usize, dark: bool, action: &mut Option<LayerAction>) {
    // Human: an instance has no children of its own in the document; its line unfolds to what it shows, which
    // comes from the component.
    let instance = if node.kind == Kind::Ref { editor.scene_node(&node.id) } else { None };
    let line = Row {
        id: &node.id,
        name: instance.map_or_else(|| node.display_name(), |shown| shown.node.display_name()),
        kind: &node.kind,
        hidden: matches!(node.enabled, Some(Flag::Value(false))),
        linked: if node.kind == Kind::Ref {
            Some(theme::INSTANCE)
        } else if node.reusable == Some(true) {
            Some(theme::COMPONENT)
        } else {
            None
        },
        has_kids: instance.map_or(!node.kids().is_empty(), |shown| !shown.children.is_empty()),
        top: depth == 0,
    };
    if !row(ui, editor, &line, depth, dark, action) {
        return;
    }
    match instance {
        Some(shown) => {
            for child in shown.children.iter().rev() {
                inside_row(ui, editor, child, depth + 1, dark, action);
            }
        }
        None => {
            for child in node.kids().iter().rev() {
                layer_row(ui, editor, child, depth + 1, dark, action);
            }
        }
    }
}

/// The line of a layer inside an instance, and those of its children.
fn inside_row(ui: &mut Ui, editor: &Editor, shown: &SceneNode, depth: usize, dark: bool, action: &mut Option<LayerAction>) {
    let line = Row {
        id: &shown.id,
        name: shown.node.display_name(),
        kind: &shown.node.kind,
        hidden: matches!(shown.node.enabled, Some(Flag::Value(false))),
        linked: Some(theme::INSTANCE),
        has_kids: !shown.children.is_empty(),
        top: false,
    };
    if row(ui, editor, &line, depth, dark, action) {
        for child in shown.children.iter().rev() {
            inside_row(ui, editor, child, depth + 1, dark, action);
        }
    }
}

fn apply_layer_action(editor: &mut Editor, action: LayerAction) {
    match action {
        LayerAction::Select { id, add } => {
            if add {
                if let Some(i) = editor.selection.iter().position(|s| *s == id) {
                    editor.selection.remove(i);
                } else {
                    editor.selection.push(id);
                }
            } else {
                editor.selection = vec![id];
            }
        }
        LayerAction::ToggleHidden(id) => {
            editor.checkpoint();
            // Human: hiding a layer inside an instance is how an instance "deletes" it (an `enabled` override);
            // showing it again writes `true`, since leaving the key out can't undo an override.
            let inside = editor.role(&id) != Role::Plain && editor.role(&id) != Role::Component;
            editor.change(&id, &|node| {
                node.enabled = match node.enabled {
                    Some(Flag::Value(false)) if inside => Some(Flag::Value(true)),
                    Some(Flag::Value(false)) => None,
                    _ => Some(Flag::Value(false)),
                };
            });
        }
    }
}

/// Said by the code section's controls: nothing can be exported as code yet.
const NO_CODE: &str = "Code export isn't available yet.";

/// The properties island for the selection. Returns a sentence to tell the user, if a control asks for one.
// Agent: MAIN-THREAD; WRITES editor.doc through one checkpoint per edit (a drag merges into its first step).
pub fn properties(ui: &mut Ui, editor: &mut Editor, dark: bool, pictures: &mut Pictures) -> Option<&'static str> {
    let mut notice = None;
    egui::ScrollArea::vertical().id_salt("properties").auto_shrink([false, false]).show(ui, |ui| {
        let content = ui.max_rect().shrink2(vec2(PROPS_PADDING, 0.0));
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.style_mut().override_font_id = Some(FontId::proportional(PROPS_TEXT));
            ui.add_space(8.0);
            if editor.tool == crate::editor::Tool::Scale && !editor.selection.is_empty() {
                scale_section(ui, editor, theme::palette(dark));
            }
            match editor.selection.as_slice() {
                [] => document_panel(ui, editor, dark),
                [id] => {
                    let id = id.clone();
                    notice = node_panel(ui, editor, &id, dark, pictures);
                }
                many => {
                    let palette = theme::palette(dark);
                    let count = many.len();
                    title_row(ui, &format!("{count} layers"), None, palette);
                    end(ui, false);
                    rule(ui, palette);
                    title_row(ui, "Alignment", None, palette);
                    if let Some(to) = alignment(ui, palette) {
                        editor.align(to);
                    }
                    end(ui, true);
                    if let Some(chosen) = export_controls(ui, editor.export.0, count, palette) {
                        editor.export = chosen;
                    }
                }
            }
        });
    });
    notice
}

/// The Scale tool's own section, on top of the island: how far the selection is scaled, to type or drag.
// Human: a factor between 0.01 and 100 that scales the selection with everything in it
// around its middle; it starts again at 1 when the selection changes. Pulling a handle on the canvas scales
// away from the opposite side instead.
fn scale_section(ui: &mut Ui, editor: &mut Editor, palette: Palette) {
    title_row(ui, "Scale", None, palette);
    ui.add_space(PROPS_GAP);
    let now = if editor.scaled.1 == editor.selection { editor.scaled.0 } else { 1.0 };
    let [left, _] = pair(ui);
    if let Some(factor) = number(ui, left, Tag::Icon("scaling"), Some(now), None, 0.01, palette)
        && (factor - now).abs() > f32::EPSILON
    {
        // One undo step for a drag of the field, like the other number fields.
        let dragging = ui.ctx().dragged_id().is_some();
        if dragging && ui.data(|d| d.get_temp::<bool>(egui::Id::unique("scale-drag")).unwrap_or(false)) {
            editor.touched();
        } else {
            editor.checkpoint();
        }
        ui.data_mut(|d| d.insert_temp(egui::Id::unique("scale-drag"), dragging));
        editor.scale_selection_to(factor);
    } else if ui.ctx().dragged_id().is_none() {
        ui.data_mut(|d| d.insert_temp(egui::Id::unique("scale-drag"), false));
    }
    ui.add_space(4.0);
    small(ui, "Or pull a handle on the canvas.", palette);
    end(ui, true);
    rule(ui, palette);
}

/// What the island's sections read: the selected node and how it is laid out.
struct Shown<'a> {
    node: &'a Node,
    vars: Vars<'a>,
    theme: &'a pen_doc::vars::ThemeContext,
    /// Its laid-out size.
    size: (f32, f32),
    /// The part of its own box its selection box takes up (all of it, except for a polygon).
    part: pen_doc::geom::Rect,
    /// Positioned by x and y, not by its parent's layout.
    free: bool,
    /// What the layer is to components.
    role: Role,
    /// A plain layer somewhere inside a component.
    in_component: bool,
    /// For a frame inside an instance: what the instance can put into it.
    slot: Option<SlotInfo>,
    /// What the Export section is set to make.
    export: Options,
    /// The folder of the document's file, where a shader's path starts.
    folder: Option<std::path::PathBuf>,
    palette: Palette,
}

/// A frame inside an instance, as a place for the instance's own content.
struct SlotInfo {
    /// The components to offer, id and name: the slot's recommended ones, or every component of the document.
    choices: Vec<(String, String)>,
    /// Whether the instance has put content of its own into it.
    filled: bool,
}

/// What the Slot section asks for.
enum SlotEdit {
    /// Put an instance of the component with this id into the slot.
    Insert(String),
    /// Drop the instance's own content, so the slot is the component's again.
    Clear,
}

/// The components the slot `node` offers. `instance` is the instance the slot is in, whose own component is
/// left out (it would have to hold itself).
// Human: a slot lists the ids of the components recommended for it (`slot` in the file); an empty list
// recommends none, and then every component of the document is offered.
fn slot_info(editor: &Editor, id: &str, node: &Node) -> Option<SlotInfo> {
    const MOST: usize = 12;
    let (instance, key) = edit::owner(&editor.doc, id)?;
    let own = edit::find(&editor.doc, &instance)?.reference.clone();
    let named = |id: &str| edit::find(&editor.doc, id).filter(|n| n.reusable == Some(true)).map(|n| (n.id.clone(), n.display_name()));
    let mut choices: Vec<(String, String)> =
        node.extra.get("slot").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).filter_map(named).collect();
    if choices.is_empty() {
        fn collect(nodes: &[Node], out: &mut Vec<(String, String)>) {
            for node in nodes {
                if node.reusable == Some(true) {
                    out.push((node.id.clone(), node.display_name()));
                }
                collect(node.kids(), out);
            }
        }
        collect(&editor.doc.children, &mut choices);
    }
    choices.retain(|(id, _)| Some(id) != own.as_ref());
    choices.truncate(MOST);
    Some(SlotInfo { choices, filled: edit::override_children(&editor.doc, &instance, &key).is_some() })
}

/// What the island's sections decide; applied after the island is drawn.
#[derive(Default)]
struct Edits {
    /// A change to the node's own properties.
    change: Option<Change>,
    /// A command that restructures the document.
    command: Option<fn(&mut Editor)>,
    /// A change to what an instance holds in a slot.
    slot: Option<SlotEdit>,
    /// What the Export section is set to now, and whether its button was pressed.
    export: Option<(Options, bool)>,
    /// Whether the command also serves a layer inside an instance (most rearrange the document, which such a
    /// layer isn't part of).
    anywhere: bool,
    /// A sentence for the user.
    notice: Option<&'static str>,
    /// The quick field and "Edit with AI…" on an image fill: what they show, and what was asked of them.
    pictures: Pictures,
}

impl Edits {
    fn set(&mut self, change: impl Fn(&mut Node) + 'static) {
        self.change = Some(Box::new(change));
    }
}

fn node_panel(ui: &mut Ui, editor: &mut Editor, id: &str, dark: bool, pictures: &mut Pictures) -> Option<&'static str> {
    let role = editor.role(id);
    let scene = editor.scene_node(id).cloned();
    // Human: an instance and the layers inside it show what the component gives them, with the instance's
    // overrides; that is what the island reads and what an edit starts from.
    let node = match role {
        Role::Instance | Role::Inside => scene.as_ref().map(|s| s.node.clone())?,
        Role::Plain | Role::Component => edit::find(&editor.doc, id).cloned()?,
    };
    let in_component = role == Role::Plain && edit::ancestry(&editor.doc, id).iter().any(|a| a != id && editor.role(a) == Role::Component);
    let theme_context = scene.as_ref().map(|s| s.theme.clone()).unwrap_or_default();
    let shown = Shown {
        node: &node,
        vars: Vars::new(&editor.doc),
        theme: &theme_context,
        size: scene.as_ref().map_or((0.0, 0.0), |s| (s.local.w, s.local.h)),
        part: scene.as_ref().map_or(pen_doc::geom::Rect::new(0.0, 0.0, 1.0, 1.0), crate::canvas::fitted_part),
        // Human: inside an instance a layer is placed by x and y when the frame it is in has no layout.
        free: crate::ops::is_free(editor, id)
            || (role == Role::Inside && editor.scene_parent(id).is_some_and(|p| p.node.layout_mode() == pen_doc::model::LayoutMode::None)),
        role,
        in_component,
        slot: if role == Role::Inside && node.kind == Kind::Frame { slot_info(editor, id, &node) } else { None },
        export: editor.export.0,
        folder: editor.path.as_deref().and_then(std::path::Path::parent).map(std::path::Path::to_path_buf),
        palette: theme::palette(dark),
    };
    let mut edits = Edits { pictures: std::mem::take(pictures), ..Edits::default() };
    header(ui, &shown, &mut edits);
    for section in [slot, context, align, position, layout, connector, appearance, material, fill, stroke, effects, text, code, export] {
        section(ui, &shown, &mut edits);
    }
    *pictures = std::mem::take(&mut edits.pictures);
    if let Some(chosen) = edits.export {
        editor.export = chosen;
    }
    ui.add_space(12.0);

    match edits.slot {
        Some(SlotEdit::Insert(component)) => {
            editor.fill_slot_with(id, &component);
            return edits.notice;
        }
        Some(SlotEdit::Clear) => {
            editor.clear_slot(id);
            return edits.notice;
        }
        None => {}
    }
    if let Some(command) = edits.command {
        if role == Role::Inside && !edits.anywhere {
            return Some("A layer inside an instance can't be rearranged. Detach the instance first.");
        }
        command(editor);
        return edits.notice;
    }
    if let Some(change) = edits.change {
        // Human: one undo step per edit; dragging a number merges into the step its first change made.
        let dragging = ui.ctx().dragged_id().is_some();
        if dragging && ui.data(|d| d.get_temp::<bool>(egui::Id::unique("prop-drag")).unwrap_or(false)) {
            editor.touched();
        } else {
            editor.checkpoint();
        }
        ui.data_mut(|d| d.insert_temp(egui::Id::unique("prop-drag"), dragging));
        editor.change(id, &*change);
    } else if ui.ctx().dragged_id().is_none() {
        ui.data_mut(|d| d.insert_temp(egui::Id::unique("prop-drag"), false));
    }
    edits.notice
}

/// The Lucide icon that stands for a kind of layer, in the island's header and the message bar's chip.
pub fn kind_icon(kind: &Kind) -> &'static str {
    match kind {
        Kind::Frame => "frame",
        Kind::Group => "group",
        Kind::Rectangle | Kind::Browser | Kind::Other(_) => "square",
        Kind::Ellipse => "circle",
        Kind::Polygon => "triangle",
        Kind::Path | Kind::Line => "pen-tool",
        Kind::Connector => "spline",
        Kind::Text => "type",
        Kind::Note => "sticky-note",
        Kind::Icon | Kind::IconFont => "star",
        Kind::Script => "code",
        Kind::Ref => "component",
    }
}

/// The header: the layer's kind and name, and the component button under it.
fn header(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let icon = Rect::from_center_size(pos2(row.left() + 8.0, row.center().y), Vec2::splat(PROPS_ICON));
    icons::paint(ui.painter(), icon, kind_icon(&node.kind), palette.muted);
    // Human: a text field's line sits a little high for its box; 3 points lower puts it level with the icon.
    let name_box = Rect::from_min_max(pos2(icon.right() + 6.0, row.top() + 3.0), pos2(row.right() - 22.0, row.bottom() + 3.0));
    let mut name_ui = ui.new_child(UiBuilder::new().max_rect(name_box).layout(Layout::left_to_right(Align::Center)).id_salt("name"));
    let mut name = node.name.clone().unwrap_or_default();
    let field = egui::TextEdit::singleline(&mut name)
        .frame(egui::Frame::new())
        .background_color(Color32::TRANSPARENT)
        .hint_text(RichText::new(node.display_name()).color(palette.foreground))
        .desired_width(name_box.width());
    if name_ui.add(field).changed() {
        edits.set(move |n| n.name = (!name.is_empty()).then(|| name.clone()));
    }
    if corner_button(ui, row, "panel-right", "Fold the panel", palette).clicked() {
        edits.notice = Some("Folding the properties away isn't available yet.");
    }
    ui.add_space(8.0);
    // Human: the component button says what the layer can become: a plain layer a component, a
    // component a plain layer again, an instance a detached copy. An instance also leads back to its component.
    match shown.role {
        Role::Plain if wide_button(ui, Some("component"), "Create Component", palette).clicked() => edits.command = Some(Editor::create_component),
        Role::Component if wide_button(ui, Some("component"), "Detach Component", palette).clicked() => edits.set(|n| n.reusable = None),
        Role::Instance if wide_button(ui, Some("component"), "Detach Instance", palette).clicked() => edits.command = Some(Editor::detach_instances),
        Role::Plain | Role::Component | Role::Instance => {}
        Role::Inside => small(ui, "Inside an instance: changes here apply to this instance only.", palette),
    }
    if matches!(shown.role, Role::Instance | Role::Inside) {
        ui.add_space(PROPS_GAP);
        if wide_button(ui, Some("locate-fixed"), "Go to component", palette).clicked() {
            edits.command = Some(Editor::go_to_component);
            edits.anywhere = true;
        }
    }
    // Human: "Make slot": an empty frame inside a component can be marked as the place where an
    // instance puts content of its own (`slot` in the file).
    if shown.in_component && node.kind == Kind::Frame && node.kids().is_empty() {
        let is_slot = node.extra.get("slot").is_some_and(Value::is_array);
        ui.add_space(PROPS_GAP);
        if wide_button(ui, Some("square-dashed"), if is_slot { "Remove slot" } else { "Make slot" }, palette).clicked() {
            edits.set(move |n| {
                n.extra.remove("slot");
                if !is_slot {
                    n.extra.insert("slot".to_owned(), Value::Array(Vec::new()));
                }
            });
        }
    }
    end(ui, true);
}

/// Slot: for a frame inside an instance, the components the instance can put into it, and a way to give it back.
// Human: this is how an instance gets content of its own: a click adds an instance of that component to the
// frame; pasting with the frame selected adds what was copied. "Reset" drops all of it, and the frame shows
// what the component has there again.
fn slot(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let Some(info) = &shown.slot else { return };
    let palette = shown.palette;
    rule(ui, palette);
    if title_row(ui, "Slot", info.filled.then_some(("rotate-ccw", "Reset to the component")), palette) {
        edits.slot = Some(SlotEdit::Clear);
    }
    ui.add_space(4.0);
    small(ui, if info.choices.is_empty() { "Paste layers here to fill it." } else { "Add a component, or paste layers here." }, palette);
    for (id, name) in &info.choices {
        ui.add_space(PROPS_GAP);
        let (cell, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
        if plate_button(ui, cell, id, Some("plus"), name, palette).clicked() {
            edits.slot = Some(SlotEdit::Insert(id.clone()));
        }
    }
    end(ui, true);
}

/// Context: a note for assistants, folded away until its title is clicked.
fn context(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    rule(ui, shown.palette);
    let open_id = egui::Id::unique("props-context-open");
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(shown.node.context.is_some());
    let arrow = if open { "chevron-up" } else { "chevron-down" };
    if title_row(ui, "Context", Some((arrow, "Context for assistants")), shown.palette) {
        open = !open;
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));
    if open {
        ui.add_space(PROPS_GAP);
        let mut context = shown.node.context.clone().unwrap_or_default();
        let field = egui::TextEdit::multiline(&mut context).hint_text("Context information").desired_rows(2).desired_width(f32::INFINITY);
        if ui.add(field).changed() {
            edits.set(move |n| n.context = (!context.is_empty()).then(|| context.clone()));
        }
    }
    end(ui, open);
}

/// The six alignment buttons in two groups; the direction of the one clicked.
fn alignment(ui: &mut Ui, palette: Palette) -> Option<AlignTo> {
    ui.add_space(PROPS_GAP);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let width = (row.width() - PROPS_GAP) / 2.0;
    let across = [
        ("align-start-vertical", "Align left (⌥A)", AlignTo::Left),
        ("align-center-vertical", "Align center (⌥H)", AlignTo::Center),
        ("align-end-vertical", "Align right (⌥D)", AlignTo::Right),
    ];
    let down = [
        ("align-start-horizontal", "Align top (⌥W)", AlignTo::Top),
        ("align-center-horizontal", "Align middle (⌥V)", AlignTo::Middle),
        ("align-end-horizontal", "Align bottom (⌥S)", AlignTo::Bottom),
    ];
    let left = Rect::from_min_size(row.min, vec2(width, row.height()));
    let right = Rect::from_min_size(pos2(row.right() - width, row.top()), vec2(width, row.height()));
    let mut clicked = None;
    for (group, buttons) in [(left, across), (right, down)] {
        let icons: Vec<(&'static str, &str)> = buttons.iter().map(|(icon, hint, _)| (*icon, *hint)).collect();
        if let Some(index) = icon_group(ui, group, &icons, palette) {
            clicked = buttons.get(index).map(|(.., to)| *to);
        }
    }
    clicked
}

fn align(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    // A connector lies where its ends are: there is nothing to align, place or lay out.
    if shown.node.kind == Kind::Connector {
        return;
    }
    rule(ui, shown.palette);
    title_row(ui, "Alignment", None, shown.palette);
    edits.command = match alignment(ui, shown.palette) {
        Some(AlignTo::Left) => Some(|e| e.align(AlignTo::Left)),
        Some(AlignTo::Center) => Some(|e| e.align(AlignTo::Center)),
        Some(AlignTo::Right) => Some(|e| e.align(AlignTo::Right)),
        Some(AlignTo::Top) => Some(|e| e.align(AlignTo::Top)),
        Some(AlignTo::Middle) => Some(|e| e.align(AlignTo::Middle)),
        Some(AlignTo::Bottom) => Some(|e| e.align(AlignTo::Bottom)),
        None => edits.command,
    };
    end(ui, true);
}

/// Position: x and y, rotation, and bring to front and the two flips.
fn position(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    if node.kind == Kind::Connector {
        return;
    }
    rule(ui, palette);
    title_row(ui, "Position", None, palette);
    ui.add_space(PROPS_GAP);
    if shown.free {
        let (x, y) = crate::ops::stored_position(node);
        let [left, right] = pair(ui);
        if let Some(v) = number(ui, left, Tag::Letter("X"), Some(x), None, 1.0, palette) {
            edits.set(move |n| n.x = Some(pen_doc::value::number(v)));
        }
        if let Some(v) = number(ui, right, Tag::Letter("Y"), Some(y), None, 1.0, palette) {
            edits.set(move |n| n.y = Some(pen_doc::value::number(v)));
        }
    } else {
        small(ui, "Placed by the parent's layout", palette);
    }
    ui.add_space(PROPS_GAP);
    let [left, right] = pair(ui);
    let rotation = node.rotation.as_ref().and_then(|r| shown.vars.number(r, shown.theme));
    let bound = node.rotation.as_ref().and_then(Num::variable);
    if let Some(v) = number(ui, left, Tag::Icon("rotate-cw"), Some(rotation.unwrap_or(0.0)), bound, 1.0, palette) {
        edits.set(move |n| n.rotation = (v.abs() > f32::EPSILON).then(|| Num::new(v)));
    }
    let buttons = [
        ("bring-to-front", "Bring to front (])"),
        ("square-centerline-dashed-horizontal", "Flip vertical (⇧V)"),
        ("square-centerline-dashed-vertical", "Flip horizontal (⇧H)"),
    ];
    edits.command = match icon_group(ui, right, &buttons, palette) {
        Some(0) => Some(|e| e.reorder(Order::Front)),
        Some(1) => Some(|e| e.flip(false)),
        Some(2) => Some(|e| e.flip(true)),
        _ => edits.command,
    };
    end(ui, true);
}

/// Layout: the size, how it fills its parent, and for frames how they lay out what is in them.
fn layout(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    if node.kind == Kind::Connector {
        return;
    }
    rule(ui, palette);
    if title_row(ui, "Layout", Some(("layout-panel-left", "Add flex layout (⇧A)")), palette) {
        edits.command = Some(Editor::add_flex_layout);
    }
    ui.add_space(4.0);
    small(ui, "Dimensions", palette);
    ui.add_space(4.0);
    let cells = pair(ui);
    let sides = [("W", "Fill Width", "Fit Width", true, shown.size.0), ("H", "Fill Height", "Fit Height", false, shown.size.1)];
    for ((letter, ..), cell) in sides.iter().zip(cells) {
        let horizontal = *letter == "W";
        let raw = if horizontal { &node.width } else { &node.height };
        let bound = raw.as_ref().and_then(Num::variable);
        let current = if horizontal { shown.size.0 } else { shown.size.1 };
        // Human: W and H are the size of the selection box, like the label under it. For a polygon that box is
        // smaller than the layer's own (it fits the corners), so the field shows that part, and a typed size
        // sets the layer's box in proportion, keeping the fitted box's top left corner where it is.
        let (share, offset) = if horizontal { (shown.part.w, shown.part.x) } else { (shown.part.h, shown.part.y) };
        if let Some(v) = number(ui, cell, Tag::Letter(letter), Some(current * share), bound, 1.0, palette) {
            let size = v.max(0.0) / share;
            let (shift, free) = (offset * (size - current), shown.free);
            edits.set(move |n| {
                set_side(n, horizontal, Some(Num::new(size)));
                if free && shift != 0.0 {
                    let (x, y) = crate::ops::stored_position(n);
                    crate::ops::set_position(n, if horizontal { x - shift } else { x }, if horizontal { y } else { y - shift });
                }
            });
        }
    }
    ui.add_space(PROPS_GAP);
    let cells = pair(ui);
    for ((_, fill_label, _, horizontal, current), cell) in sides.into_iter().zip(cells) {
        let raw = if horizontal { &node.width } else { &node.height };
        let fills = matches!(raw.as_ref().map(Num::sizing), Some(Sizing::FillContainer(_)));
        // Human: only something laid out by its parent can fill it.
        if check(ui, cell, fill_label, fills, !shown.free, palette) {
            let value = if fills { Num::new(current) } else { Num::Text("fill_container".to_owned()) };
            edits.set(move |n| set_side(n, horizontal, Some(value.clone())));
        }
    }
    if node.kind == Kind::Frame {
        ui.add_space(PROPS_GAP);
        let cells = pair(ui);
        for ((_, _, fit_label, horizontal, current), cell) in sides.into_iter().zip(cells) {
            let raw = if horizontal { &node.width } else { &node.height };
            let fits = matches!(raw.as_ref().map(Num::sizing), Some(Sizing::FitContent(_)) | None);
            if check(ui, cell, fit_label, fits, true, palette) {
                let value = if fits { Num::new(current) } else { Num::Text("fit_content".to_owned()) };
                edits.set(move |n| set_side(n, horizontal, Some(value.clone())));
            }
        }
        frame_layout(ui, shown, edits);
    }
    end(ui, true);
}

fn set_side(node: &mut Node, horizontal: bool, value: Option<Num>) {
    if horizontal { node.width = value } else { node.height = value }
}

/// A frame's own layout: direction, gap, padding, and where its children sit.
fn frame_layout(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    use pen_doc::model::{LayoutMode, Padding};
    let (palette, node) = (shown.palette, shown.node);
    let mode = node.layout_mode();
    ui.add_space(PROPS_GAP);
    let choices = [
        ("Freeform", "none", LayoutMode::None),
        ("Vertical", "vertical", LayoutMode::Vertical),
        ("Horizontal", "horizontal", LayoutMode::Horizontal),
    ];
    let labels: Vec<(&str, bool)> = choices.iter().map(|(label, _, m)| (*label, mode == *m)).collect();
    if let Some(index) = choice_row(ui, "direction", &labels, palette)
        && let Some((_, value, m)) = choices.get(index)
        && mode != *m
    {
        if *m == LayoutMode::None {
            edits.command = Some(Editor::remove_flex_layout);
        } else {
            let value = *value;
            edits.set(move |n| n.layout = Some(value.to_owned()));
        }
    }
    if mode == LayoutMode::None {
        return;
    }
    ui.add_space(PROPS_GAP);
    let [left, right] = pair(ui);
    let gap = node.gap.as_ref().and_then(|g| shown.vars.number(g, shown.theme));
    if let Some(v) =
        number(ui, left, Tag::Icon("between-horizontal-start"), Some(gap.unwrap_or(0.0)), node.gap.as_ref().and_then(Num::variable), 1.0, palette)
    {
        edits.set(move |n| n.gap = Some(Num::new(v.max(0.0))));
    }
    let padding = match &node.padding {
        Some(Padding::All(p)) => shown.vars.number(p, shown.theme),
        Some(Padding::Sides(s)) => s.first().and_then(|p| shown.vars.number(p, shown.theme)),
        None => None,
    };
    if let Some(v) = number(ui, right, Tag::Icon("square-dashed"), Some(padding.unwrap_or(0.0)), None, 1.0, palette) {
        edits.set(move |n| n.padding = Some(Padding::All(Num::new(v.max(0.0)))));
    }
    ui.add_space(PROPS_GAP);
    small(ui, "Justify", palette);
    ui.add_space(4.0);
    let justify = [("Start", "start"), ("Center", "center"), ("End", "end"), ("Between", "space_between"), ("Around", "space_around")];
    let now = node.justify_content.as_deref().unwrap_or("start");
    let labels: Vec<(&str, bool)> = justify.iter().map(|(label, value)| (*label, now == *value)).collect();
    if let Some(index) = choice_row(ui, "justify", &labels, palette)
        && let Some((_, value)) = justify.get(index)
    {
        let value = *value;
        edits.set(move |n| n.justify_content = (value != "start").then(|| value.to_owned()));
    }
    ui.add_space(PROPS_GAP);
    small(ui, "Align", palette);
    ui.add_space(4.0);
    let items = [("Start", "start"), ("Center", "center"), ("End", "end")];
    let now = node.align_items.as_deref().unwrap_or("start");
    let labels: Vec<(&str, bool)> = items.iter().map(|(label, value)| (*label, now == *value)).collect();
    if let Some(index) = choice_row(ui, "align-items", &labels, palette)
        && let Some((_, value)) = items.get(index)
    {
        let value = *value;
        edits.set(move |n| n.align_items = (value != "start").then(|| value.to_owned()));
    }
}

/// Appearance: opacity, corner radius, and whether the layer shows at all.
/// The Connector section: how the line runs, what sits at its ends, and its measures.
// Human: Pencraft's own section. Where the ends are and where the waypoints lie is set on the canvas, with the
// connector's handles; here is everything about the line's look. A value at its default is left out of the file.
fn connector(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    /// The routes, as the section names them and as the file does.
    const ROUTES: [(&str, &str); 3] = [("Curved", "curved"), ("Elbow", "elbow"), ("Straight", "straight")];
    /// The markers, in two rows of three.
    const MARKERS: [[(&str, &str); 3]; 2] =
        [[("None", ""), ("Arrow", "arrow"), ("Bar", "bar")], [("Triangle", "triangle"), ("Dot", "dot"), ("Diamond", "diamond")]];
    let (palette, node) = (shown.palette, shown.node);
    if node.kind != Kind::Connector {
        return;
    }
    rule(ui, palette);
    title_row(ui, "Connector", None, palette);
    ui.add_space(PROPS_GAP);
    let said = |key: &str| node.extra.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    // Sets a key of the connector, or takes it away when the value is the one it has without the key.
    let put = |edits: &mut Edits, key: &'static str, value: Option<Value>| {
        edits.set(move |n| match &value {
            Some(value) => {
                n.extra.insert(key.to_owned(), value.clone());
            }
            None => {
                n.extra.remove(key);
            }
        });
    };
    let route = match said("route").as_str() {
        "elbow" => "elbow",
        "straight" => "straight",
        _ => "curved",
    };
    if let Some(picked) = choice_row(ui, "route", &ROUTES.map(|(label, key)| (label, key == route)), palette) {
        let key = ROUTES[picked].1;
        put(edits, "route", (key != "curved").then(|| Value::from(key)));
    }
    for (title, key) in [("Start", "startMarker"), ("End", "endMarker")] {
        ui.add_space(PROPS_GAP);
        small(ui, title, palette);
        let now = said(key);
        for (line, row) in MARKERS.iter().enumerate() {
            if let Some(picked) = choice_row(ui, &format!("{key}{line}"), &row.map(|(label, marker)| (label, marker == now)), palette) {
                let marker = row[picked].1;
                put(edits, key, (!marker.is_empty()).then(|| Value::from(marker)));
            }
        }
    }
    let number_of = |key: &str| node.extra.get(key).and_then(Value::as_f64).map(pen_doc::value::narrow);
    let stored = |value: f32| Value::from((f64::from(value) * 100.0).round() / 100.0);
    ui.add_space(PROPS_GAP);
    small(ui, if route == "curved" { "Marker size · Curvature" } else { "Marker size · Corner radius" }, palette);
    let [left, right] = pair(ui);
    if let Some(v) = number(ui, left, Tag::Icon("move-diagonal"), Some(number_of("markerSize").unwrap_or(8.0)), None, 0.5, palette) {
        put(edits, "markerSize", Some(stored(v.max(0.0))));
    }
    if route == "curved" {
        if let Some(v) = number(ui, right, Tag::Icon("spline"), Some(number_of("curvature").unwrap_or(1.0)), None, 0.05, palette) {
            let v = v.max(0.0);
            put(edits, "curvature", ((v - 1.0).abs() > 0.005).then(|| stored(v)));
        }
    } else {
        let radius = match &node.corner_radius {
            Some(Corners::All(radius)) => shown.vars.number(radius, shown.theme).unwrap_or(0.0),
            _ => 0.0,
        };
        if let Some(v) = number(ui, right, Tag::Icon("scan"), Some(radius), None, 0.5, palette) {
            edits.set(move |n| n.corner_radius = (v > 0.0).then(|| Corners::All(Num::new(v))));
        }
    }
    // Human: one dash length here; a list of lengths written into the file by hand shows its first.
    let dash = match node.extra.get("dash") {
        Some(Value::Array(lengths)) => lengths.first().and_then(Value::as_f64).map(pen_doc::value::narrow),
        other => other.and_then(Value::as_f64).map(pen_doc::value::narrow),
    };
    ui.add_space(PROPS_GAP);
    small(ui, if route == "elbow" { "Dash · Run-out" } else { "Dash" }, palette);
    let [left, right] = pair(ui);
    if let Some(v) = number(ui, left, Tag::Icon("ellipsis"), Some(dash.unwrap_or(0.0)), None, 0.5, palette) {
        put(edits, "dash", (v > 0.01).then(|| stored(v)));
    }
    if route == "elbow"
        && let Some(v) = number(ui, right, Tag::Icon("corner-down-right"), Some(number_of("stub").unwrap_or(16.0)), None, 0.5, palette)
    {
        put(edits, "stub", Some(stored(v.max(0.0))));
    }
    if node.extra.get("via").and_then(Value::as_array).is_some_and(|via| !via.is_empty()) {
        ui.add_space(PROPS_GAP);
        if wide_button(ui, Some("x"), "Remove waypoints", palette).clicked() {
            put(edits, "via", None);
        }
    }
    end(ui, true);
}

fn appearance(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    rule(ui, palette);
    let hidden = matches!(node.enabled, Some(Flag::Value(false)));
    if title_row(ui, "Appearance", Some((if hidden { "eye-off" } else { "eye" }, if hidden { "Show" } else { "Hide" })), palette) {
        edits.set(move |n| n.enabled = if hidden { None } else { Some(Flag::Value(false)) });
    }
    ui.add_space(PROPS_GAP);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let width = (row.width() - 2.0 * PROPS_GAP - PROPS_ROW) / 2.0;
    let first = Rect::from_min_size(row.min, vec2(width, row.height()));
    let second = first.translate(vec2(width + PROPS_GAP, 0.0));
    let opacity = node.opacity.as_ref().and_then(|o| shown.vars.number(o, shown.theme)).unwrap_or(1.0);
    if let Some(percent) = number(ui, first, Tag::Icon("blend"), Some((opacity * 100.0).round()), None, 1.0, palette) {
        let percent = percent.clamp(0.0, 100.0);
        edits.set(move |n| n.opacity = (percent < 99.95).then(|| Num::new(percent / 100.0)));
    }
    if matches!(node.kind, Kind::Frame | Kind::Rectangle | Kind::Browser | Kind::Polygon) {
        let stored = corner_values(node);
        let radii = stored.clone().map(|r| shown.vars.number(&r, shown.theme).unwrap_or(0.0));
        // Human: corners that differ show as "Mixed" in the one field; a number typed there sets all four
        // (mixed unless all four are equal, written as `[r, r, r, r]`).
        let mixed = radii.iter().any(|r| (r - radii[0]).abs() > f32::EPSILON);
        if let Some(v) = number_or_mixed(ui, second, Tag::Icon("scan"), Some(radii[0]), None, 0.5, mixed, palette) {
            let all = std::array::from_fn(|_| Num::new(v.max(0.0)));
            edits.set(move |n| n.corner_radius = joined_corners(&all, &n.kind));
        }
        // Human: a polygon has one radius for all its corners; the others can be set one by one (with a
        // button that stays lit while the four fields show).
        if node.kind != Kind::Polygon {
            let id = ui.scope_id().with("corners one by one");
            // Human: closed until the button is pressed, except for a layer whose corners already differ: its
            // four values would otherwise hide behind "Mixed".
            let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(mixed);
            if open {
                let lit = Rect::from_center_size(pos2(row.right() - 9.0, row.center().y), Vec2::splat(20.0));
                ui.painter().rect_filled(lit, PROPS_RADIUS, ACCENT.gamma_multiply(0.25));
            }
            if corner_button(ui, row, "maximize", "Edit corners individually", palette).clicked() {
                open = !open;
                ui.data_mut(|d| d.insert_temp(id, open));
            }
            if open {
                each_corner(ui, shown, &stored, radii, edits);
            }
        }
    }
    if node.kind == Kind::Frame {
        ui.add_space(PROPS_GAP);
        let [left, _] = pair(ui);
        let clips = node.clip.as_ref().and_then(|c| shown.vars.flag(c, shown.theme)).unwrap_or(false);
        if check(ui, left, "Clip Content", clips, true, palette) {
            edits.set(move |n| n.clip = (!clips).then_some(Flag::Value(true)));
        }
    }
    end(ui, true);
}

/// The Material section: what the layer is made of. None, frosted (a blur with a tint, in four thicknesses) or
/// liquid glass (regular or clear), then the few choices a material has, and under "Advanced" its numbers.
// Human: Pencraft's own section. A layer says only what differs from its
// material's preset, so the numbers under Advanced show the preset's until one is changed, and the reset
// button takes the layer's own numbers out again.
fn material(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    if !matches!(node.kind, Kind::Frame | Kind::Rectangle | Kind::Ellipse | Kind::Browser | Kind::Polygon | Kind::Path) {
        return;
    }
    rule(ui, palette);
    let stored = node.material.clone().unwrap_or(Value::Null);
    let now = Material::read(&stored);
    let _ = title_row(ui, "Material", None, palette);
    ui.add_space(PROPS_GAP);
    let store = |edits: &mut Edits, value: Option<Value>| edits.set(move |n| n.material.clone_from(&value));
    let put = |edits: &mut Edits, key: &'static str, value: Option<&str>| {
        let value = value.map(str::to_owned);
        let changed = material::with(&stored, |keys| match &value {
            Some(value) => {
                keys.insert(key.to_owned(), Value::String(value.clone()));
            }
            None => {
                keys.remove(key);
            }
        });
        store(edits, Some(changed));
    };
    let glass = now.as_ref().map(Material::is_glass);
    match choice_row(ui, "material", &[("None", glass.is_none()), ("Frosted", glass == Some(false)), ("Liquid glass", glass == Some(true))], palette)
    {
        Some(0) => store(edits, None),
        Some(index) if glass != Some(index == 2) => store(edits, Some(material::fresh(index == 2))),
        _ => {}
    }
    let Some(now) = now else {
        end(ui, true);
        return;
    };
    ui.add_space(PROPS_GAP);
    match now.kind {
        material::Kind::Frosted(thickness) => {
            let labels = ["Ultra thin", "Thin", "Regular", "Thick"];
            let choices: Vec<(&str, bool)> = Thickness::ALL.iter().zip(labels).map(|(t, label)| (label, *t == thickness)).collect();
            if let Some(index) = choice_row(ui, "material-thickness", &choices, palette) {
                put(edits, "thickness", Thickness::ALL.get(index).map(|t| t.key()));
            }
        }
        material::Kind::Glass(variant) => {
            let choices = [("Regular", variant == Variant::Regular), ("Clear", variant == Variant::Clear)];
            if let Some(index) = choice_row(ui, "material-variant", &choices, palette) {
                put(edits, "variant", Variant::ALL.get(index).map(|v| v.key()));
            }
        }
    }
    ui.add_space(PROPS_GAP);
    small(ui, "Appearance", palette);
    ui.add_space(4.0);
    let choices: Vec<(&str, bool)> = Appearance::ALL.iter().zip(["Auto", "Light", "Dark"]).map(|(a, label)| (label, *a == now.appearance)).collect();
    if let Some(index) = choice_row(ui, "material-appearance", &choices, palette) {
        put(edits, "appearance", Appearance::ALL.get(index).filter(|a| **a != Appearance::Auto).map(|a| a.key()));
    }
    if now.is_glass() {
        ui.add_space(PROPS_GAP);
        small(ui, "Depth", palette);
        ui.add_space(4.0);
        let depths = Depth::ALL.iter().zip(["Control", "Bar", "Panel"]).map(|(d, label)| (label, Some(*d) == now.depth));
        let choices: Vec<(&str, bool)> = std::iter::once(("By size", now.depth.is_none())).chain(depths).collect();
        if let Some(index) = choice_row(ui, "material-depth", &choices, palette) {
            put(edits, "depth", index.checked_sub(1).and_then(|i| Depth::ALL.get(i)).map(|d| d.key()));
        }
    }
    ui.add_space(PROPS_GAP);
    let [left, _] = pair(ui);
    let tinted = now.tint.is_some();
    if check(ui, left, "Tint", tinted, true, palette) {
        put(edits, "tint", (!tinted).then_some("#0a84ff"));
    }
    if let Some(tint) = &now.tint {
        ui.add_space(4.0);
        if tint.starts_with('$') {
            small(ui, tint, palette);
        } else if let Some(c) = color_row(ui, "material-tint", Rgba::parse(tint).unwrap_or(Rgba::WHITE), palette) {
            put(edits, "tint", Some(&c.to_hex()));
        }
    }
    material_numbers(ui, shown, &stored, &now, edits);
    end(ui, true);
}

/// The numbers of a material under "Advanced": each shows what the material comes to on this layer, and
/// changing one writes it into the layer's `advanced`.
fn material_numbers(ui: &mut Ui, shown: &Shown, stored: &Value, now: &Material, edits: &mut Edits) {
    let palette = shown.palette;
    let open_id = ui.scope_id().with(("material-advanced", &shown.node.id));
    let mut open = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
    ui.add_space(PROPS_GAP);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let title = Rect::from_min_max(row.min, pos2(row.right() - PROPS_ROW, row.bottom()));
    let response = ui.interact(title, ui.scope_id().with("material-advanced-title"), Sense::click());
    let color = if response.hovered() { palette.foreground } else { palette.muted };
    let arrow = Rect::from_center_size(pos2(row.left() + 6.0, row.center().y), Vec2::splat(PROPS_ICON));
    icons::paint(ui.painter(), arrow, if open { "chevron-down" } else { "chevron-right" }, color);
    ui.painter().text(pos2(arrow.right() + 4.0, row.center().y), Align2::LEFT_CENTER, "Advanced", FontId::proportional(PROPS_SMALL_TEXT), color);
    if response.clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }
    if !now.advanced.is_empty() && corner_button(ui, row, "rotate-ccw", "Back to the material's own numbers", palette).clicked() {
        let changed = material::with(stored, |keys| {
            keys.remove("advanced");
        });
        edits.set(move |n| n.material = Some(changed.clone()));
    }
    if !open {
        return;
    }
    let size = shown.size;
    let number = |name: &str| shown.vars.number(&Num::Text(format!("${name}")), shown.theme);
    let color = |text: &str| shown.vars.color(text, shown.theme);
    let params = now.resolve(size, now.appearance == Appearance::Dark, &material::Lookup { number: &number, color: &color });
    let knobs: Vec<Knob> = Knob::ALL.into_iter().filter(|knob| now.is_glass() || knob.frosted()).collect();
    for knob in knobs {
        ui.add_space(4.0);
        let [left, right] = pair(ui);
        ui.painter().text(left.left_center(), Align2::LEFT_CENTER, knob.label(), FontId::proportional(PROPS_SMALL_TEXT), palette.muted);
        let bound = now.advanced.get(knob.key()).and_then(Value::as_str).filter(|text| text.starts_with('$'));
        let (least, most) = knob.range();
        let speed = f64::from(most - least) / 200.0;
        if let Some(new) = number_or_mixed(ui, right, Tag::Letter(""), Some(params.number(knob)), bound, speed, false, palette) {
            let (key, value) = (knob.key(), effects::number(new.clamp(least, most)));
            let changed = material::with(stored, |keys| {
                let advanced = keys.entry("advanced").or_insert_with(|| Value::Object(serde_json::Map::new()));
                if let Some(advanced) = advanced.as_object_mut() {
                    advanced.insert(key.to_owned(), value);
                }
            });
            edits.set(move |n| n.material = Some(changed.clone()));
        }
    }
}

/// The four corner fields: top left and top right, then bottom left and bottom right under them.
fn each_corner(ui: &mut Ui, shown: &Shown, stored: &[Num; 4], radii: [f32; 4], edits: &mut Edits) {
    // Agent: `cornerRadius` lists the corners clockwise from the top left, so the bottom row is read backwards.
    for row in [[0, 1], [3, 2]] {
        ui.add_space(PROPS_GAP);
        for (cell, corner) in pair(ui).into_iter().zip(row) {
            let bound = stored[corner].variable();
            if let Some(v) = number_or_mixed(ui, cell, Tag::Corner(corner), Some(radii[corner]), bound, 0.5, false, shown.palette) {
                let mut all = stored.clone();
                all[corner] = Num::new(v.max(0.0));
                edits.set(move |n| n.corner_radius = joined_corners(&all, &n.kind));
            }
        }
    }
}

/// A layer's four corner radii as the file has them, clockwise from the top left. A browser without any has
/// its own default.
fn corner_values(node: &Node) -> [Num; 4] {
    let same = |r: &Num| [r.clone(), r.clone(), r.clone(), r.clone()];
    match &node.corner_radius {
        Some(Corners::All(r)) => same(r),
        Some(Corners::Each(each)) => match each.as_slice() {
            [a, b, c, d] => [a.clone(), b.clone(), c.clone(), d.clone()],
            [r] => same(r),
            _ => same(&Num::new(0.0)),
        },
        None if node.kind == Kind::Browser => same(&Num::new(crate::render::BROWSER_RADIUS)),
        None => same(&Num::new(0.0)),
    }
}

/// Four corner radii as the file keeps them: one value when all are the same, nothing when that is what the
/// layer has without any (0, or a browser's own default), else all four.
fn joined_corners(all: &[Num; 4], kind: &Kind) -> Option<Corners> {
    if all.iter().any(|r| r != &all[0]) {
        return Some(Corners::Each(all.to_vec()));
    }
    let plain = if *kind == Kind::Browser { crate::render::BROWSER_RADIUS } else { 0.0 };
    let unset = all[0].literal().is_some_and(|r| (r - plain).abs() <= f32::EPSILON);
    (!unset).then(|| Corners::All(all[0].clone()))
}

/// Fill, or a sticky note's colour.
fn fill(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    // A connector is a line: it is drawn by its stroke alone.
    if node.kind == Kind::Connector {
        return;
    }
    if node.kind == Kind::Note {
        rule(ui, palette);
        title_row(ui, "Note color", None, palette);
        ui.add_space(PROPS_GAP);
        let current = node.color.as_deref().and_then(|c| shown.vars.color(c, shown.theme)).unwrap_or(crate::render::NOTE_COLOR);
        if let Some(c) = color_row(ui, "note", current, palette) {
            edits.set(move |n| n.color = Some(c.to_hex()));
        }
        end(ui, true);
        return;
    }
    if matches!(node.kind, Kind::Group | Kind::Ref | Kind::Script) {
        return;
    }
    rule(ui, palette);
    if title_row(ui, "Fill", Some(("plus", "Add fill")), palette) {
        let mut list: Vec<Paint> = node.fill.as_ref().map(|p| p.as_slice().to_vec()).unwrap_or_default();
        list.push(Paint::Solid("#D9D9D9".to_owned()));
        edits.set(move |n| n.fill = paints_of(list.clone()));
    }
    // Human: a picture is changed by description on the layer's own fill: a layer inside an instance, or an
    // instance, is changed through its component.
    let pictures = matches!(shown.role, Role::Plain | Role::Component).then_some(&mut edits.pictures);
    if let Some(edit) = paint_editor(ui, node.fill.as_ref(), shown, "fill", pictures) {
        edits.set(move |n| n.fill = edit.paints());
    }
    end(ui, node.fill.is_some());
}

/// Stroke: its paints and its width.
fn stroke(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    if matches!(node.kind, Kind::Note | Kind::Group | Kind::Ref | Kind::Script) {
        return;
    }
    rule(ui, palette);
    if title_row(ui, "Stroke", Some(("plus", "Add stroke")), palette) {
        let mut list: Vec<Paint> = node.stroke.as_ref().map(|p| p.as_slice().to_vec()).unwrap_or_default();
        list.push(Paint::Solid("#000000".to_owned()));
        edits.set(move |n| n.stroke = paints_of(list.clone()));
    }
    if let Some(edit) = paint_editor(ui, node.stroke.as_ref(), shown, "stroke", None) {
        edits.set(move |n| n.stroke = edit.paints());
    }
    if node.stroke.is_some() {
        let width = match &node.stroke_width {
            Some(StrokeWidth::All(w)) => shown.vars.number(w, shown.theme),
            Some(StrokeWidth::Sides(_)) => None,
            None => Some(1.0),
        };
        ui.add_space(PROPS_GAP);
        let [left, _] = pair(ui);
        if let Some(v) = number(ui, left, Tag::Icon("minus"), Some(width.unwrap_or(0.0)), None, 0.25, palette) {
            edits.set(move |n| n.stroke_width = Some(StrokeWidth::All(Num::new(v.max(0.0)))));
        }
    }
    end(ui, node.stroke.is_some());
}

/// Effects: the layer's shadows and blurs, the newest on top. The plus opens the menu of kinds; a click on an
/// effect opens its values under it.
// Human: rows are listed last first, each with a switch and a remove button, and a newly added effect opens
// at once. Its values unfold in place, under its row.
fn effects(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    rule(ui, palette);
    let list = effects::list(node.effect.as_ref());
    let open_id = ui.scope_id().with(("effect-open", &node.id));
    let mut open = ui.data(|d| d.get_temp::<Option<usize>>(open_id)).flatten().filter(|index| *index < list.len());
    let store = |edits: &mut Edits, list: Vec<Value>| edits.set(move |n| n.effect = effects::stored(list.clone()));

    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    ui.painter().text(row.left_center(), Align2::LEFT_CENTER, "Effects", FontId::proportional(PROPS_TEXT), palette.foreground);
    let add = corner_button(ui, row, "plus", "Add effect", palette);
    egui::Popup::menu(&add).show(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
        for kind in EffectKind::ALL {
            // Human: a layer has one layer blur and one background blur at most.
            let taken = kind.single() && list.iter().any(|effect| EffectKind::of(effect) == Some(kind));
            if menu_item(ui, kind.icon(), kind.label(), !taken, palette).clicked() {
                let mut longer = list.clone();
                longer.push(kind.fresh());
                open = Some(list.len());
                store(edits, longer);
            }
        }
    });

    for (index, effect) in list.iter().enumerate().rev() {
        ui.add_space(PROPS_GAP);
        let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
        let button = PROPS_ROW - 2.0;
        let name = Rect::from_min_max(line.min, pos2(line.right() - 2.0 * button, line.bottom()));
        let response = ui.interact(name, ui.scope_id().with(("effect", index)), Sense::click());
        plate(ui, name, response.hovered(), palette);
        if open == Some(index) {
            ui.painter().rect_stroke(name, PROPS_RADIUS, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
        }
        let kind = EffectKind::of(effect);
        let off = effects::is_off(effect);
        let color = palette.foreground.gamma_multiply(if off { 0.5 } else { 1.0 });
        let icon = Rect::from_center_size(pos2(name.left() + 13.0, name.center().y), Vec2::splat(PROPS_ICON));
        icons::paint(ui.painter(), icon, kind.map_or("circle-dashed", EffectKind::icon), color);
        let label = match kind {
            Some(EffectKind::Shader) => effects::shader_name(effect),
            Some(kind) => kind.label().to_owned(),
            None => "Unknown".to_owned(),
        };
        let words = ui.painter().with_clip_rect(name.shrink2(vec2(4.0, 0.0)));
        words.text(pos2(icon.right() + 6.0, name.center().y), Align2::LEFT_CENTER, label, FontId::proportional(PROPS_TEXT), color);
        let eye = Rect::from_min_max(line.min, pos2(line.right() - button, line.bottom()));
        if corner_button(ui, eye, if off { "eye-off" } else { "eye" }, if off { "Show" } else { "Hide" }, palette).clicked() {
            let mut changed = list.clone();
            changed[index] = effects::with(effect, |keys| {
                // Human: the file says `enabled` only when an effect is off.
                keys.remove("enabled");
                if !off {
                    keys.insert("enabled".to_owned(), Value::Bool(false));
                }
            });
            store(edits, changed);
        }
        if corner_button(ui, line, "minus", "Remove effect", palette).clicked() {
            let mut shorter = list.clone();
            shorter.remove(index);
            open = None;
            store(edits, shorter);
        }
        if response.clicked() {
            open = if open == Some(index) { None } else { Some(index) };
        }
        if open == Some(index)
            && let Some(changed) = effect_fields(ui, shown, effect, kind)
        {
            let mut new = list.clone();
            new[index] = changed;
            store(edits, new);
        }
    }
    ui.data_mut(|d| d.insert_temp(open_id, open));
    end(ui, !list.is_empty());
}

/// A number of an effect as the file has it: its value, and the variable it is bound to, if any.
fn effect_number(shown: &Shown, value: Option<&Value>) -> (f32, Option<String>) {
    match value {
        Some(Value::Number(n)) => (n.as_f64().map_or(0.0, pen_doc::value::narrow), None),
        Some(Value::String(text)) => {
            let num = Num::Text(text.clone());
            (shown.vars.number(&num, shown.theme).unwrap_or(0.0), num.variable().map(str::to_owned))
        }
        _ => (0.0, None),
    }
}

/// The values of the open effect, under its row: a shadow's colour, offset and blur; a blur's radius; a shader's
/// file. Returns the effect with a value changed.
// Human: a shadow has Color, Offset X and Y, and Blur (never below 0, in steps of 0.1); a blur has Blur alone.
// The file leaves out an offset of 0 / 0 and a blur of 0.
fn effect_fields(ui: &mut Ui, shown: &Shown, effect: &Value, kind: Option<EffectKind>) -> Option<Value> {
    let palette = shown.palette;
    let mut changed = None;
    let caption = |ui: &mut Ui, text: &str| {
        ui.add_space(PROPS_GAP);
        small(ui, text, palette);
        ui.add_space(4.0);
    };
    match kind? {
        EffectKind::DropShadow | EffectKind::InnerShadow => {
            caption(ui, "Color");
            let stored = effect.get("color").and_then(Value::as_str).unwrap_or("#000000");
            if stored.starts_with('$') {
                small(ui, stored, palette);
            } else if let Some(c) = color_row(ui, "effect", Rgba::parse(stored).unwrap_or(Rgba::BLACK), palette) {
                changed = Some(effects::with(effect, |keys| {
                    keys.insert("color".to_owned(), Value::String(c.to_hex()));
                }));
            }
            caption(ui, "Offset");
            let offset = effect.get("offset");
            let ((x, x_bound), (y, y_bound)) =
                (effect_number(shown, offset.and_then(|o| o.get("x"))), effect_number(shown, offset.and_then(|o| o.get("y"))));
            let [left, right] = pair(ui);
            let new_x = number(ui, left, Tag::Letter("X"), Some(x), x_bound.as_deref(), 1.0, palette);
            let new_y = number(ui, right, Tag::Letter("Y"), Some(y), y_bound.as_deref(), 1.0, palette);
            if new_x.is_some() || new_y.is_some() {
                // A side bound to a variable keeps its variable.
                let side = |new: Option<f32>, key: &str| {
                    new.map_or_else(|| offset.and_then(|o| o.get(key)).cloned().unwrap_or(effects::number(0.0)), effects::number)
                };
                let (x, y) = (side(new_x, "x"), side(new_y, "y"));
                changed = Some(effects::with(effect, |keys| {
                    if x == effects::number(0.0) && y == effects::number(0.0) {
                        keys.remove("offset");
                    } else {
                        keys.insert("offset".to_owned(), serde_json::json!({ "x": x, "y": y }));
                    }
                }));
            }
            caption(ui, "Blur");
            changed = effect_blur(ui, shown, effect, "blur", true).or(changed);
        }
        EffectKind::LayerBlur | EffectKind::BackgroundBlur => {
            caption(ui, "Blur");
            changed = effect_blur(ui, shown, effect, "radius", false);
        }
        EffectKind::Shader => {
            caption(ui, "Shader file");
            let mut url = effect.get("url").and_then(Value::as_str).unwrap_or("").to_owned();
            if ui.add(egui::TextEdit::singleline(&mut url).hint_text("./effect.glsl").desired_width(f32::INFINITY)).changed() {
                changed = Some(effects::with(effect, |keys| {
                    keys.insert("url".to_owned(), Value::String(url.clone()));
                }));
            }
            changed = shader_fields(ui, shown, effect).or(changed);
        }
    }
    changed
}

/// The effect with its shader uniform `name` set to `value`.
fn with_uniform(effect: &Value, name: &str, value: Value) -> Value {
    effects::with(effect, |keys| {
        let uniforms = keys.entry("uniforms").or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(uniforms) = uniforms.as_object_mut() {
            uniforms.insert(name.to_owned(), value);
        }
    })
}

/// Under a shader effect's file: what is wrong with it, or a control for each uniform the user sets. Returns
/// the effect with a uniform changed.
// Human: the shader says in its comments how each uniform is shown: `@label` names it, `@range` bounds a
// number, `@color` makes a vec3 or vec4 a colour. A sampler is the layer itself, what is behind it, or a picture.
fn shader_fields(ui: &mut Ui, shown: &Shown, effect: &Value) -> Option<Value> {
    let palette = shown.palette;
    let url = effect.get("url").and_then(Value::as_str).unwrap_or("").trim();
    ui.add_space(4.0);
    let note = |ui: &mut Ui, text: &str| {
        ui.add(egui::Label::new(RichText::new(text).size(PROPS_SMALL_TEXT).color(palette.muted)).wrap());
    };
    let program = match crate::images::resolve(shown.folder.as_deref(), url).map(|path| shader::load(&path)) {
        Some(Ok(program)) => program,
        Some(Err(error)) => {
            note(ui, &error);
            return None;
        }
        None if url.is_empty() => {
            note(ui, "Name a WebGL fragment shader (.glsl), from the document's folder.");
            return None;
        }
        None => {
            note(ui, "Save the document first, or give the shader's full path.");
            return None;
        }
    };
    let mut changed = None;
    let variable = |name: &str| shown.vars.lookup(name, shown.theme).cloned();
    for uniform in program.uniforms.iter().filter(|u| u.editable()) {
        let given = effect.get("uniforms").and_then(|u| u.get(&uniform.name));
        let now = shader::numbers(uniform, given, &variable);
        ui.add_space(PROPS_GAP);
        match uniform.ty {
            shader::Type::Bool => {
                let [left, _] = pair(ui);
                if check(ui, left, uniform.title(), now[0] != 0.0, true, palette) {
                    changed = Some(with_uniform(effect, &uniform.name, Value::Bool(now[0] == 0.0)));
                }
            }
            shader::Type::Float | shader::Type::Int => {
                small(ui, uniform.title(), palette);
                ui.add_space(4.0);
                let whole = uniform.ty == shader::Type::Int;
                let bound = given.and_then(Value::as_str).and_then(|text| text.strip_prefix('$'));
                // Fields are told apart by their place in the panel, so two uniforms never share one.
                let [left, _] = pair(ui);
                if let Some(v) = number(ui, left, Tag::Icon("sliders-horizontal"), Some(now[0]), bound, if whole { 1.0 } else { 0.1 }, palette) {
                    let v =
                        v.clamp(uniform.range.0.unwrap_or(f32::MIN), uniform.range.1.unwrap_or(f32::MAX).max(uniform.range.0.unwrap_or(f32::MIN)));
                    changed = Some(with_uniform(effect, &uniform.name, effects::number(if whole { v.round() } else { v })));
                }
            }
            shader::Type::Vec(size) if uniform.color => {
                small(ui, uniform.title(), palette);
                ui.add_space(4.0);
                let unit = |v: f32| crate::render::to_u8(v * 255.0);
                let current = Rgba { r: unit(now[0]), g: unit(now[1]), b: unit(now[2]), a: if size == 4 { unit(now[3]) } else { 255 } };
                if let Some(c) = color_row(ui, &uniform.name, current, palette) {
                    let hex = c.to_hex();
                    let hex = if size == 4 { hex } else { hex.chars().take(7).collect() };
                    changed = Some(with_uniform(effect, &uniform.name, Value::String(hex)));
                }
            }
            shader::Type::Sampler => {
                small(ui, uniform.title(), palette);
                ui.add_space(4.0);
                let source = shader::source(uniform, given);
                let choices = [("Layer", "pencil:object"), ("Background", "pencil:background"), ("Picture", "")];
                let chosen = match source {
                    shader::Source::Object => 0,
                    shader::Source::Background => 1,
                    shader::Source::File(_) | shader::Source::Nothing => 2,
                };
                let labels: Vec<(&str, bool)> = choices.iter().enumerate().map(|(index, (label, _))| (*label, index == chosen)).collect();
                if let Some(index) = choice_row(ui, &uniform.name, &labels, palette)
                    && index != chosen
                    && let Some((_, value)) = choices.get(index)
                {
                    changed = Some(with_uniform(effect, &uniform.name, Value::String((*value).to_owned())));
                }
                if chosen == 2 {
                    ui.add_space(4.0);
                    let mut file = if let shader::Source::File(file) = source { file } else { String::new() };
                    ui.push_id(&uniform.name, |ui| {
                        if ui.add(egui::TextEdit::singleline(&mut file).hint_text("./picture.png").desired_width(f32::INFINITY)).changed() {
                            changed = Some(with_uniform(effect, &uniform.name, Value::String(file.clone())));
                        }
                    });
                }
            }
            shader::Type::Vec(_) | shader::Type::IVec(_) => note(ui, &format!("{}: set in the file's uniforms", uniform.title())),
        }
    }
    changed
}

/// The blur field of an effect, stored under `key`; `optional` leaves the key out of the file at 0.
fn effect_blur(ui: &mut Ui, shown: &Shown, effect: &Value, key: &'static str, optional: bool) -> Option<Value> {
    let (value, bound) = effect_number(shown, effect.get(key));
    let [left, _] = pair(ui);
    let new = number(ui, left, Tag::Icon("circle-dot-dashed"), Some(value), bound.as_deref(), 0.1, shown.palette)?.max(0.0);
    Some(effects::with(effect, |keys| {
        if optional && new == 0.0 {
            keys.remove(key);
        } else {
            keys.insert(key.to_owned(), effects::number(new));
        }
    }))
}

/// One line of a menu: an icon and a name. A disabled one is greyed and can't be clicked.
fn menu_item(ui: &mut Ui, icon: &'static str, label: &str, enabled: bool, palette: Palette) -> Response {
    let (row, response) = ui.allocate_exact_size(vec2(PROPS_MENU_WIDTH, PROPS_ROW + 2.0), if enabled { Sense::click() } else { Sense::hover() });
    if enabled && response.hovered() {
        ui.painter().rect_filled(row, PROPS_RADIUS, palette.hover);
    }
    let color = if enabled { palette.foreground } else { palette.muted.gamma_multiply(0.5) };
    icons::paint(ui.painter(), Rect::from_center_size(pos2(row.left() + 13.0, row.center().y), Vec2::splat(PROPS_ICON)), icon, color);
    ui.painter().text(pos2(row.left() + 28.0, row.center().y), Align2::LEFT_CENTER, label, FontId::proportional(PROPS_TEXT), color);
    response
}

/// Text: what it says and how it is set, for texts and sticky notes.
fn text(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let (palette, node) = (shown.palette, shown.node);
    if !matches!(node.kind, Kind::Text | Kind::Note) {
        return;
    }
    rule(ui, palette);
    title_row(ui, "Text", None, palette);
    ui.add_space(PROPS_GAP);
    let mut content = node.content.clone().unwrap_or_default();
    if ui.add(egui::TextEdit::multiline(&mut content).desired_rows(3).desired_width(f32::INFINITY)).changed() {
        edits.set(move |n| n.content = Some(content.clone()));
    }
    if node.kind != Kind::Text {
        end(ui, true);
        return;
    }
    let style = pen_doc::layout::text_style(node, shown.vars, shown.theme);
    ui.add_space(PROPS_GAP);
    small(ui, &style.family, palette);
    ui.add_space(4.0);
    let [left, right] = pair(ui);
    if let Some(v) = number(ui, left, Tag::Icon("a-large-small"), Some(style.size), node.font_size.as_ref().and_then(Num::variable), 0.5, palette) {
        edits.set(move |n| n.font_size = Some(Num::new(v.max(1.0))));
    }
    let line_height = node.line_height.as_ref().and_then(|l| shown.vars.number(l, shown.theme)).filter(|l| *l != 0.0);
    if let Some(v) = number(ui, right, Tag::Icon("baseline"), Some(line_height.unwrap_or(0.0)), None, 0.01, palette) {
        edits.set(move |n| n.line_height = (v > 0.0).then(|| Num::new(v)));
    }
    ui.add_space(PROPS_GAP);
    let [left, _] = pair(ui);
    if let Some(v) = number(ui, left, Tag::Icon("space"), Some(style.letter_spacing), None, 0.1, palette) {
        edits.set(move |n| n.letter_spacing = (v.abs() > f32::EPSILON).then(|| Num::new(v)));
    }
    ui.add_space(PROPS_GAP);
    let weights = [("Regular", "400"), ("Medium", "500"), ("Bold", "700")];
    let labels: Vec<(&str, bool)> = weights.iter().map(|(label, weight)| (*label, style.weight.to_string() == *weight)).collect();
    if let Some(index) = choice_row(ui, "weight", &labels, palette)
        && let Some((_, weight)) = weights.get(index)
    {
        let weight = *weight;
        edits.set(move |n| n.font_weight = Some(Num::Text(weight.to_owned())));
    }
    ui.add_space(PROPS_GAP);
    let sides = [("Left", "left"), ("Center", "center"), ("Right", "right")];
    let now = node.text_align.as_deref().unwrap_or("left");
    let labels: Vec<(&str, bool)> = sides.iter().map(|(label, value)| (*label, now == *value)).collect();
    if let Some(index) = choice_row(ui, "text-align", &labels, palette)
        && let Some((_, value)) = sides.get(index)
    {
        let value = *value;
        edits.set(move |n| n.text_align = Some(value.to_owned()));
    }
    ui.add_space(PROPS_GAP);
    let growth = [("Auto", "auto"), ("Fixed width", "fixed-width"), ("Fixed", "fixed-width-height")];
    let now = node.text_growth.as_deref().unwrap_or("auto");
    let labels: Vec<(&str, bool)> = growth.iter().map(|(label, value)| (*label, now == *value)).collect();
    if let Some(index) = choice_row(ui, "text-growth", &labels, palette)
        && let Some((_, value)) = growth.get(index)
        && now != *value
    {
        let (value, (w, h)) = (*value, shown.size);
        edits.set(move |n| {
            n.text_growth = Some(value.to_owned());
            if value != "auto" {
                n.width = Some(Num::new(w));
            }
            if value == "fixed-width-height" {
                n.height = Some(Num::new(h));
            }
        });
    }
    end(ui, true);
}

/// Code: turning the layer into code. Nothing can be exported yet; the controls say so.
fn code(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    let palette = shown.palette;
    rule(ui, palette);
    let mut used = title_row(ui, "Code", Some(("settings", "Code settings")), palette);
    ui.add_space(PROPS_GAP);
    let (row, response) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::click());
    plate(ui, row, response.hovered(), palette);
    ui.painter().text(
        pos2(row.left() + 8.0, row.center().y),
        Align2::LEFT_CENTER,
        "HTML + Tailwind",
        FontId::proportional(PROPS_TEXT),
        palette.foreground,
    );
    icons::paint(ui.painter(), Rect::from_center_size(pos2(row.right() - 12.0, row.center().y), Vec2::splat(11.0)), "chevron-down", palette.muted);
    used |= response.clicked();
    ui.add_space(PROPS_GAP);
    let [left, right] = pair(ui);
    used |= plate_button(ui, left, "copy-html", None, "Copy HTML", palette).clicked();
    used |= plate_button(ui, right, "export-html", None, "Export HTML", palette).clicked();
    ui.add_space(PROPS_GAP);
    used |= wide_button(ui, None, "Import Web via Browser", palette).clicked();
    if used {
        edits.notice = Some(NO_CODE);
    }
    end(ui, true);
}

/// Export: the selected layer as a file.
fn export(ui: &mut Ui, shown: &Shown, edits: &mut Edits) {
    edits.export = export_controls(ui, shown.export, 1, shown.palette);
}

/// The Export section for `count` selected layers: the format, for pictures the scale and for JPEG the quality,
/// and the button. Returns the settings when they changed or the button was pressed (then with `true`).
// Human: scale 1x / 2x / 3x, format PNG / JPEG / WEBP / PDF / SVG,
// quality High / Medium / Low for JPEG, a note that a PDF takes one page per layer in the order selected, and
// "Export layer" or "Export layers (n)".
fn export_controls(ui: &mut Ui, now: Options, count: usize, palette: Palette) -> Option<(Options, bool)> {
    rule(ui, palette);
    title_row(ui, "Export", None, palette);
    let mut chosen = now;
    ui.add_space(PROPS_GAP);
    let formats: Vec<(&str, bool)> = Format::ALL.iter().map(|f| (f.label(), *f == now.format)).collect();
    if let Some(index) = choice_row(ui, "export-format", &formats, palette)
        && let Some(format) = Format::ALL.get(index)
    {
        chosen.format = *format;
    }
    if now.format.has_scale() {
        ui.add_space(PROPS_GAP);
        let labels: Vec<String> = export::SCALES.iter().map(|s| format!("{s}x")).collect();
        let scales: Vec<(&str, bool)> = labels.iter().zip(export::SCALES).map(|(label, scale)| (label.as_str(), scale == now.scale)).collect();
        if let Some(index) = choice_row(ui, "export-scale", &scales, palette)
            && let Some(scale) = export::SCALES.get(index)
        {
            chosen.scale = *scale;
        }
    }
    if now.format == Format::Jpeg {
        ui.add_space(PROPS_GAP);
        small(ui, "Quality", palette);
        ui.add_space(4.0);
        let qualities: Vec<(&str, bool)> = export::QUALITIES.iter().map(|(label, quality)| (*label, *quality == now.quality)).collect();
        if let Some(index) = choice_row(ui, "export-quality", &qualities, palette)
            && let Some((_, quality)) = export::QUALITIES.get(index)
        {
            chosen.quality = *quality;
        }
    }
    if count > 1 {
        ui.add_space(PROPS_GAP);
        ui.add(
            egui::Label::new(RichText::new("The selected layers are exported together, as one file.").size(PROPS_SMALL_TEXT).color(palette.muted))
                .wrap(),
        );
    }
    ui.add_space(PROPS_GAP);
    let label = if count == 1 { "Export layer".to_owned() } else { format!("Export layers ({count})") };
    let go = wide_button(ui, Some("download"), &label, palette).clicked();
    end(ui, true);
    (go || chosen != now).then_some((chosen, go))
}

/// The line between two sections, from edge to edge of the island.
fn rule(ui: &mut Ui, palette: Palette) {
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().hline(line.x_range().expand(PROPS_PADDING), line.center().y, Stroke::new(1.0, palette.outline.gamma_multiply(0.7)));
    ui.add_space(8.0);
}

/// The space under a section: more when it has content under its title.
fn end(ui: &mut Ui, content: bool) {
    ui.add_space(if content { 16.0 } else { 8.0 });
}

/// A section's title, with a small icon button at its right end if `action` (icon, hint) is given. Returns
/// whether that button was clicked.
fn title_row(ui: &mut Ui, title: &str, action: Option<(&'static str, &str)>, palette: Palette) -> bool {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    ui.painter().text(row.left_center(), Align2::LEFT_CENTER, title, FontId::proportional(PROPS_TEXT), palette.foreground);
    action.is_some_and(|(icon, hint)| corner_button(ui, row, icon, hint, palette).clicked())
}

/// An icon button at the right end of `row`.
fn corner_button(ui: &mut Ui, row: Rect, icon: &'static str, hint: &str, palette: Palette) -> Response {
    let button = Rect::from_center_size(pos2(row.right() - 9.0, row.center().y), Vec2::splat(20.0));
    let response = ui.interact(button, ui.scope_id().with(("corner", icon, row.top().to_bits())), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(button, PROPS_RADIUS, palette.hover.gamma_multiply(0.5));
    }
    let color = if response.hovered() { palette.foreground } else { palette.muted };
    icons::paint(ui.painter(), Rect::from_center_size(button.center(), Vec2::splat(PROPS_ICON)), icon, color);
    response.on_hover_text(crate::keys::text(hint))
}

/// A small quiet caption.
fn small(ui: &mut Ui, text: &str, palette: Palette) {
    ui.add(egui::Label::new(RichText::new(text).size(PROPS_SMALL_TEXT).color(palette.muted)).selectable(false));
}

/// Paints the rounded backdrop of a field or button.
fn plate(ui: &Ui, rect: Rect, hovered: bool, palette: Palette) {
    ui.painter().rect_filled(rect, PROPS_RADIUS, palette.hover.gamma_multiply(if hovered { 0.7 } else { 0.45 }));
}

/// The next row split into two equal cells.
fn pair(ui: &mut Ui) -> [Rect; 2] {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let width = (row.width() - PROPS_GAP) / 2.0;
    [Rect::from_min_size(row.min, vec2(width, row.height())), Rect::from_min_size(pos2(row.right() - width, row.top()), vec2(width, row.height()))]
}

/// What stands in front of a number: a letter or an icon.
#[derive(Clone, Copy)]
enum Tag {
    Letter(&'static str),
    Icon(&'static str),
    /// One corner of a box, counted clockwise from the top left.
    Corner(usize),
}

/// A number field in `cell`: its tag, then the value, which can be typed or dragged. A value bound to a variable
/// shows the variable's name instead and can't be changed here. Returns the new value when it changed.
fn number(ui: &mut Ui, cell: Rect, tag: Tag, value: Option<f32>, bound: Option<&str>, speed: f64, palette: Palette) -> Option<f32> {
    number_or_mixed(ui, cell, tag, value, bound, speed, false, palette)
}

/// [`number`], which with `mixed` reads "Mixed" until a number is typed or dragged: the layer has several
/// values where the field has room for one.
#[expect(clippy::too_many_arguments, reason = "one more than `number`, which every other field goes through")]
fn number_or_mixed(
    ui: &mut Ui,
    cell: Rect,
    tag: Tag,
    value: Option<f32>,
    bound: Option<&str>,
    speed: f64,
    mixed: bool,
    palette: Palette,
) -> Option<f32> {
    plate(ui, cell, false, palette);
    let salt = match tag {
        Tag::Letter(letter) => {
            ui.painter().text(pos2(cell.left() + 8.0, cell.center().y), Align2::LEFT_CENTER, letter, FontId::proportional(PROPS_TEXT), palette.muted);
            letter
        }
        Tag::Icon(icon) => {
            icons::paint(ui.painter(), Rect::from_center_size(pos2(cell.left() + 13.0, cell.center().y), Vec2::splat(11.0)), icon, palette.muted);
            icon
        }
        Tag::Corner(corner) => {
            corner_mark(ui.painter(), pos2(cell.left() + 13.0, cell.center().y), corner, palette.muted);
            ["top left", "top right", "bottom right", "bottom left"].get(corner).copied().unwrap_or("corner")
        }
    };
    let inner = Rect::from_min_max(pos2(cell.left() + 24.0, cell.top()), pos2(cell.right() - 4.0, cell.bottom()));
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)).id_salt((salt, cell.top().to_bits())));
    if let Some(name) = bound {
        child.add(egui::Label::new(RichText::new(format!("${name}")).color(ACCENT)).truncate());
        return None;
    }
    // Human: the field's own backdrop is the plate, so the number itself has none.
    let widgets = &mut child.visuals_mut().widgets;
    for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
        state.weak_bg_fill = Color32::TRANSPARENT;
        state.bg_fill = Color32::TRANSPARENT;
        state.bg_stroke = Stroke::NONE;
    }
    child.spacing_mut().button_padding = vec2(0.0, 0.0);
    let mut v = value.unwrap_or(0.0);
    let mut field = egui::DragValue::new(&mut v).speed(speed).max_decimals(2);
    if mixed {
        field = field.custom_formatter(|_, _| "Mixed".to_owned());
    }
    child.add(field).changed().then_some(v)
}

/// The small picture in front of a corner's field: two sides of a box meeting in a round corner, the corner
/// (counted clockwise from the top left) where it is on the box.
fn corner_mark(painter: &egui::Painter, centre: egui::Pos2, corner: usize, color: Color32) {
    let (half, round) = (4.5, 3.5);
    // Drawn for the top left corner, then mirrored to where the corner is.
    let (flip_x, flip_y) = (if matches!(corner, 1 | 2) { -1.0 } else { 1.0 }, if matches!(corner, 2 | 3) { -1.0 } else { 1.0 });
    let mut points = vec![vec2(-half, half)];
    points.extend((0..=6u8).map(|step| {
        let (sin, cos) = (f32::from(step) / 6.0 * std::f32::consts::FRAC_PI_2).sin_cos();
        vec2(-half + round - round * cos, -half + round - round * sin)
    }));
    points.push(vec2(half, -half));
    let line = points.into_iter().map(|v| centre + vec2(v.x * flip_x, v.y * flip_y)).collect();
    painter.add(egui::Shape::line(line, Stroke::new(1.2, color)));
}

/// One plate with several icon buttons side by side; the index of the one clicked.
fn icon_group(ui: &mut Ui, group: Rect, buttons: &[(&'static str, &str)], palette: Palette) -> Option<usize> {
    plate(ui, group, false, palette);
    let count = u8::try_from(buttons.len()).map_or(1.0, f32::from).max(1.0);
    let width = group.width() / count;
    let mut clicked = None;
    let mut left = group.left();
    for (index, (icon, hint)) in buttons.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(left, group.top()), vec2(width, group.height()));
        left = cell.right();
        let response = ui.interact(cell, ui.scope_id().with(("group", *icon, group.top().to_bits())), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(cell.shrink(2.0), PROPS_RADIUS - 1.0, palette.hover.gamma_multiply(0.6));
        }
        let color = if response.hovered() { palette.foreground } else { palette.muted };
        icons::paint(ui.painter(), Rect::from_center_size(cell.center(), Vec2::splat(PROPS_ICON)), icon, color);
        if response.on_hover_text(crate::keys::text(hint)).clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// A row of words to choose one from, the chosen one on a plate; the index of the one clicked. Each word gets
/// room in proportion to its length, so long and short ones fit side by side.
fn choice_row(ui: &mut Ui, salt: &str, choices: &[(&str, bool)], palette: Palette) -> Option<usize> {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW - 2.0), Sense::hover());
    plate(ui, row, false, palette);
    let font = FontId::proportional(PROPS_SMALL_TEXT);
    let widths: Vec<f32> =
        choices.iter().map(|(label, _)| ui.painter().layout_no_wrap((*label).to_owned(), font.clone(), palette.muted).size().x + 10.0).collect();
    let scale = row.width() / widths.iter().sum::<f32>().max(1.0);
    let mut clicked = None;
    let mut left = row.left();
    for (index, ((label, chosen), width)) in choices.iter().zip(widths).enumerate() {
        let cell = Rect::from_min_size(pos2(left, row.top()), vec2(width * scale, row.height()));
        left = cell.right();
        let response = ui.interact(cell, ui.scope_id().with(("choice", salt, index)), Sense::click());
        if *chosen {
            ui.painter().rect_filled(cell.shrink(2.0), PROPS_RADIUS - 1.0, palette.hover);
        }
        let color = if *chosen || response.hovered() { palette.foreground } else { palette.muted };
        ui.painter().text(cell.center(), Align2::CENTER_CENTER, *label, font.clone(), color);
        if response.clicked() {
            clicked = Some(index);
        }
    }
    clicked
}

/// A small box to tick, with its label. Returns whether it was clicked; a disabled one never is.
fn check(ui: &mut Ui, cell: Rect, label: &str, checked: bool, enabled: bool, palette: Palette) -> bool {
    let response = ui.interact(cell, ui.scope_id().with(("check", label)), if enabled { Sense::click() } else { Sense::hover() });
    let tick = Rect::from_center_size(pos2(cell.left() + 7.0, cell.center().y), Vec2::splat(12.0));
    let dim = if enabled { 1.0 } else { 0.5 };
    if checked {
        ui.painter().rect_filled(tick, 3.0, ACCENT.gamma_multiply(dim));
        icons::paint(ui.painter(), tick.shrink(2.0), "check", Color32::WHITE);
    } else {
        ui.painter().rect_stroke(tick, 3.0, Stroke::new(1.0, palette.outline), StrokeKind::Inside);
    }
    let color = if response.hovered() && enabled { palette.foreground } else { palette.muted.gamma_multiply(dim) };
    ui.painter().text(pos2(tick.right() + 7.0, cell.center().y), Align2::LEFT_CENTER, label, FontId::proportional(PROPS_SMALL_TEXT), color);
    response.clicked()
}

/// A button on a plate in `cell`, its words centred, an icon before them if given.
fn plate_button(ui: &mut Ui, cell: Rect, salt: &str, icon: Option<&'static str>, label: &str, palette: Palette) -> Response {
    let response = ui.interact(cell, ui.scope_id().with(("plate", salt)), Sense::click());
    plate(ui, cell, response.hovered(), palette);
    let font = FontId::proportional(PROPS_TEXT);
    let words = ui.painter().layout_no_wrap(label.to_owned(), font.clone(), palette.foreground).size().x;
    let room = if icon.is_some() { PROPS_ICON + 5.0 } else { 0.0 };
    let content = room + words;
    let left = cell.center().x - content / 2.0;
    if let Some(icon) = icon {
        icons::paint(
            ui.painter(),
            Rect::from_min_size(pos2(left, cell.center().y - PROPS_ICON / 2.0), Vec2::splat(PROPS_ICON)),
            icon,
            palette.foreground,
        );
    }
    ui.painter().text(pos2(left + room, cell.center().y), Align2::LEFT_CENTER, label, font, palette.foreground);
    response
}

/// A plate button as wide as the island's content.
fn wide_button(ui: &mut Ui, icon: Option<&'static str>, label: &str, palette: Palette) -> Response {
    let (cell, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    plate_button(ui, cell, label, icon, label, palette)
}

/// A list of paints as a fill or stroke stores it: nothing, one, or several.
fn paints_of(mut list: Vec<Paint>) -> Option<Paints> {
    match list.len() {
        0 => None,
        1 => Some(Paints::One(list.remove(0))),
        _ => Some(Paints::Many(list)),
    }
}

/// A colour on a plate: a swatch with a picker, its hex value, and its opacity in percent. Returns a new
/// colour when changed.
#[expect(clippy::many_single_char_names, reason = "r, g, b, a are the colour channels; c the colour")]
fn color_row(ui: &mut Ui, salt: &str, current: Rgba, palette: Palette) -> Option<Rgba> {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width() - 2.0 * (PROPS_ROW - 2.0), PROPS_ROW), Sense::hover());
    plate(ui, row, false, palette);
    let mut result = None;
    let inner = Rect::from_min_max(pos2(row.left() + 4.0, row.top()), pos2(row.right() - 4.0, row.bottom()));
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)).id_salt(("color", salt)));
    child.spacing_mut().item_spacing.x = 4.0;
    child.spacing_mut().interact_size.y = PROPS_ROW - 8.0;
    let mut c = color32(current);
    if egui::color_picker::color_edit_button_srgba(&mut child, &mut c, egui::color_picker::Alpha::OnlyBlend).changed() {
        let [r, g, b, a] = c.to_srgba_unmultiplied();
        result = Some(Rgba { r, g, b, a });
    }
    let mut hex = current.to_hex().trim_start_matches('#').to_owned();
    let field = egui::TextEdit::singleline(&mut hex).frame(egui::Frame::new()).background_color(Color32::TRANSPARENT).desired_width(62.0);
    if child.add(field).changed()
        && let Some(parsed) = Rgba::parse(&format!("#{hex}"))
    {
        result = Some(parsed);
    }
    let widgets = &mut child.visuals_mut().widgets;
    for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
        state.weak_bg_fill = Color32::TRANSPARENT;
        state.bg_fill = Color32::TRANSPARENT;
        state.bg_stroke = Stroke::NONE;
    }
    let mut percent = (f32::from(current.a) / 255.0 * 100.0).round();
    if child.add(egui::DragValue::new(&mut percent).range(0.0..=100.0).max_decimals(0).suffix(" %")).changed() {
        result = Some(Rgba { a: crate::render::to_u8(percent / 100.0 * 255.0), ..current });
    }
    result
}

/// The rows of a fill or stroke: a plain colour can be changed, the others are named. Each row can be removed.
/// With `pictures`, the topmost image paint (the one a change by description replaces) gets "Edit with AI…" and
/// the quick field. Returns the new paints when something changed.
fn paint_editor(ui: &mut Ui, paints: Option<&Paints>, shown: &Shown, what: &'static str, mut pictures: Option<&mut Pictures>) -> Option<PaintEdit> {
    let palette = shown.palette;
    let list: Vec<Paint> = paints.map(|p| p.as_slice().to_vec()).unwrap_or_default();
    let topmost_image =
        list.iter().rposition(|paint| matches!(paint, Paint::Object(object) if object.get("type").and_then(Value::as_str) == Some("image")));
    let mut result = None;
    for (index, paint) in list.iter().enumerate() {
        ui.add_space(PROPS_GAP);
        let top = ui.cursor().top();
        let salt = format!("{what}-{index}");
        match paint {
            Paint::Solid(text) if text.starts_with('$') => {
                let c = shown.vars.color(text, shown.theme).unwrap_or_default();
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width() - 2.0 * (PROPS_ROW - 2.0), PROPS_ROW), Sense::hover());
                plate(ui, row, false, palette);
                let swatch = Rect::from_center_size(pos2(row.left() + 12.0, row.center().y), Vec2::splat(14.0));
                ui.painter().rect_filled(swatch, 3.0, color32(c));
                ui.painter().text(pos2(swatch.right() + 6.0, row.center().y), Align2::LEFT_CENTER, text, FontId::proportional(PROPS_TEXT), ACCENT);
            }
            Paint::Solid(text) => {
                if let Some(c) = color_row(ui, &salt, Rgba::parse(text).unwrap_or_default(), palette) {
                    let mut new = list.clone();
                    new[index] = Paint::Solid(c.to_hex());
                    result = Some(paints_of(new).map_or(PaintEdit::Clear, PaintEdit::Set));
                }
            }
            Paint::Object(object) => {
                let kind = object.get("type").and_then(Value::as_str).unwrap_or("paint");
                let label = match kind {
                    "gradient" => format!("{} gradient", object.get("gradientType").and_then(Value::as_str).unwrap_or("linear")),
                    "image" => "Image".to_owned(),
                    "mesh_gradient" => "Mesh gradient".to_owned(),
                    "shader" => "Shader".to_owned(),
                    "color" => object.get("color").and_then(Value::as_str).unwrap_or("").to_owned(),
                    other => other.to_owned(),
                };
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width() - 2.0 * (PROPS_ROW - 2.0), PROPS_ROW), Sense::hover());
                plate(ui, row, false, palette);
                ui.painter().text(
                    pos2(row.left() + 8.0, row.center().y),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(PROPS_TEXT),
                    palette.foreground,
                );
            }
        }
        // Remove, at the right end of the row just drawn.
        let line = Rect::from_min_size(pos2(ui.max_rect().left(), top), vec2(ui.available_width(), PROPS_ROW));
        if corner_button(ui, line, "minus", "Remove", palette).clicked() {
            let mut new = list.clone();
            new.remove(index);
            result = Some(paints_of(new).map_or(PaintEdit::Clear, PaintEdit::Set));
        }
        if Some(index) == topmost_image
            && let Some(pictures) = pictures.as_deref_mut()
        {
            picture_tools(ui, line, pictures, palette);
        }
    }
    result
}

/// "Edit with AI…" beside an image paint's Remove, and the quick field under its row: words, and a button that
/// sends the picture with them and replaces it with the result. While the field's picture is made, progress
/// and Cancel; with no service that can edit, one line that says where to add one.
// Human: the row of a paint leaves room for two buttons at its right end; Remove takes the outer one.
fn picture_tools(ui: &mut Ui, line: Rect, pictures: &mut Pictures, palette: Palette) {
    if corner_button(ui, line.translate(vec2(-(PROPS_ROW - 2.0), 0.0)), "wand-sparkles", "Edit with AI…", palette).clicked() {
        pictures.asked = Some(PictureAsk::Dialog);
    }
    ui.add_space(PROPS_GAP);
    let Some(service) = pictures.service.clone() else {
        let line = "Add a Google Gemini service in Settings → Images to change pictures by description.";
        ui.add(egui::Label::new(RichText::new(line).size(PROPS_SMALL_TEXT).color(palette.muted)).wrap().selectable(false));
        return;
    };
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), PROPS_ROW), Sense::hover());
    let side = if pictures.ours { PROPS_CANCEL } else { PROPS_ROW };
    let button = Rect::from_min_size(pos2(row.right() - side, row.top()), vec2(side, PROPS_ROW));
    let field = Rect::from_min_max(row.min, pos2(button.left() - PROPS_GAP, row.bottom()));
    // Human: in the dark look the panels' usual plate is too close to the island behind it for a field one types
    // in; the input colour sets it apart.
    if pictures.dark {
        ui.painter().rect_filled(field, PROPS_RADIUS, palette.input);
    } else {
        plate(ui, field, false, palette);
    }
    let inner = field.shrink2(vec2(8.0, 0.0));
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner).layout(Layout::left_to_right(Align::Center)).id_salt("quick-edit"));
    if pictures.ours {
        child.spacing_mut().item_spacing.x = 6.0;
        child.add(egui::Spinner::new().size(PROPS_ICON).color(palette.muted));
        child.add(egui::Label::new(RichText::new("Changing the picture…").color(palette.muted)).truncate().selectable(false));
        if plate_button(ui, button, "quick-cancel", None, "Cancel", palette).on_hover_text("Give up this picture").clicked() {
            pictures.asked = Some(PictureAsk::Cancel);
        }
    } else {
        let hint = match (&pictures.making_for, pictures.busy) {
            (Some(name), _) => format!("A picture is being made for \"{name}\"…"),
            (None, true) => "A picture is being made…".to_owned(),
            (None, false) => "Describe a change…".to_owned(),
        };
        let words = egui::TextEdit::singleline(&mut pictures.text)
            .frame(egui::Frame::new())
            .background_color(Color32::TRANSPARENT)
            .hint_text(RichText::new(hint).color(palette.muted))
            .desired_width(inner.width());
        let typed = child.add_enabled(!pictures.busy, words);
        let ready = !pictures.busy && !pictures.text.trim().is_empty();
        let tip = if pictures.busy {
            "A picture is being made. Wait for it to finish.".to_owned()
        } else {
            format!("Sends this picture to {service} and replaces it with the result.")
        };
        // Human: the field says what Return does too, since Return sends without the button being looked at.
        let typed = typed.on_hover_text(&tip).on_disabled_hover_text(&tip);
        // Human: Return runs it, as the button does.
        if ready && typed.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            pictures.asked = Some(PictureAsk::Quick);
        }
        // Human: the button is the chat's send button (an arrow up, filled once there are words to send), so it
        // isn't taken for "Edit with AI…" beside it, which keeps the wand.
        let response = ui.interact(button, ui.scope_id().with("quick-go"), if ready { Sense::click() } else { Sense::hover() });
        let mark = if ready {
            let fill = if response.hovered() { palette.primary.gamma_multiply(0.85) } else { palette.primary };
            ui.painter().circle_filled(button.center(), SEND_BUTTON / 2.0, fill);
            palette.primary_text
        } else {
            ui.painter().circle_filled(button.center(), SEND_BUTTON / 2.0, palette.input);
            palette.muted
        };
        icons::paint(ui.painter(), Rect::from_center_size(button.center(), Vec2::splat(SEND_ARROW)), "arrow-up", mark);
        if response.on_hover_text(tip).clicked() && ready {
            pictures.asked = Some(PictureAsk::Quick);
        }
    }
    if let Some(problem) = &pictures.problem {
        ui.add_space(4.0);
        ui.add(egui::Label::new(RichText::new(problem).size(PROPS_SMALL_TEXT).color(PROBLEM)).wrap().selectable(false));
    }
}

fn section(ui: &mut Ui, title: &str) {
    ui.add_space(10.0);
    ui.separator();
    ui.label(RichText::new(title).strong().size(12.0));
    ui.add_space(2.0);
}

fn document_panel(ui: &mut Ui, editor: &mut Editor, dark: bool) {
    let palette = theme::palette(dark);
    ui.label(RichText::new("Document").strong());
    ui.label(RichText::new(format!("Schema {}", editor.doc.version)).color(palette.muted).small());
    let axes: Vec<(String, Vec<String>)> = editor
        .doc
        .themes
        .iter()
        .flatten()
        .map(|(axis, values)| (axis.clone(), values.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()))
        .collect();
    if !axes.is_empty() {
        section(ui, "Theme");
        let current = editor.root_theme();
        for (axis, values) in axes {
            ui.horizontal(|ui| {
                ui.label(&axis);
                for value in values {
                    let active = current.get(&axis) == Some(value.as_str());
                    if ui.selectable_label(active, &value).clicked() && !active {
                        editor.theme_choice.retain(|(a, _)| *a != axis);
                        editor.theme_choice.push((axis.clone(), value.clone()));
                        editor.relayout();
                    }
                }
            });
        }
    }
    let theme_context = editor.root_theme();
    let vars = Vars::new(&editor.doc);
    let names: Vec<(String, String)> = editor
        .doc
        .variables
        .iter()
        .flatten()
        .map(|(name, v)| (name.clone(), v.get("type").and_then(Value::as_str).unwrap_or("").to_owned()))
        .collect();
    if !names.is_empty() {
        section(ui, &format!("Variables ({})", names.len()));
        egui::Grid::new("variables").num_columns(2).striped(false).show(ui, |ui| {
            for (name, kind) in names {
                ui.label(RichText::new(&name).small());
                if kind == "color" {
                    let color = vars.color(&format!("${name}"), &theme_context).unwrap_or_default();
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, color32(color));
                } else {
                    let value = vars.lookup(&name, &theme_context).map(ToString::to_string).unwrap_or_default();
                    ui.label(RichText::new(value).small().color(palette.muted));
                }
                ui.end_row();
            }
        });
    }
    section(ui, "Canvas");
    ui.label(RichText::new(format!("{} top-level layers", editor.doc.children.len())).color(palette.muted));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corners_are_kept_as_one_value_until_they_differ() {
        let node = |json: &str| serde_json::from_str::<Node>(json).unwrap();
        let literal = |all: &[Num; 4]| all.clone().map(|r| r.literal().unwrap_or(-1.0));
        // One stored value stands for all four; a browser without any has 16.
        let card = node(r#"{"type":"rectangle","id":"r","cornerRadius":12}"#);
        assert_eq!(literal(&corner_values(&card)), [12.0; 4]);
        assert_eq!(literal(&corner_values(&node(r#"{"type":"browser","id":"b"}"#))), [16.0; 4]);
        // Setting the top right corner alone writes all four, clockwise from the top left.
        let mut all = corner_values(&card);
        all[1] = Num::new(0.0);
        let each = joined_corners(&all, &Kind::Rectangle);
        assert_eq!(serde_json::to_string(&each).unwrap(), "[12,0,12,12]");
        // Set back, it is one value again; all at 0 is no radius at all, which for a browser is 16.
        all[1] = Num::new(12.0);
        assert_eq!(joined_corners(&all, &Kind::Rectangle), Some(Corners::All(Num::new(12.0))));
        let flat: [Num; 4] = std::array::from_fn(|_| Num::new(0.0));
        assert_eq!(joined_corners(&flat, &Kind::Rectangle), None);
        assert_eq!(joined_corners(&flat, &Kind::Browser), Some(Corners::All(Num::new(0.0))));
        let own: [Num; 4] = std::array::from_fn(|_| Num::new(16.0));
        assert_eq!(joined_corners(&own, &Kind::Browser), None);
        // A corner bound to a variable keeps it when another corner is set.
        let bound = node(r#"{"type":"frame","id":"f","cornerRadius":["$radius",4,4,4]}"#);
        let mut all = corner_values(&bound);
        all[2] = Num::new(9.0);
        assert_eq!(serde_json::to_string(&joined_corners(&all, &Kind::Frame)).unwrap(), r#"["$radius",4,9,4]"#);
    }

    #[test]
    fn every_icon_of_the_properties_island_is_bundled() {
        let kinds = [
            Kind::Frame,
            Kind::Group,
            Kind::Rectangle,
            Kind::Ellipse,
            Kind::Polygon,
            Kind::Path,
            Kind::Text,
            Kind::Note,
            Kind::Icon,
            Kind::Script,
            Kind::Ref,
        ];
        let named = [
            "panel-right",
            "component",
            "chevron-up",
            "chevron-down",
            "align-start-vertical",
            "align-center-vertical",
            "align-end-vertical",
            "align-start-horizontal",
            "align-center-horizontal",
            "align-end-horizontal",
            "rotate-cw",
            "bring-to-front",
            "square-centerline-dashed-horizontal",
            "square-centerline-dashed-vertical",
            "layout-panel-left",
            "between-horizontal-start",
            "square-dashed",
            "eye",
            "eye-off",
            "blend",
            "scan",
            "maximize",
            "plus",
            "minus",
            "check",
            "settings",
            "a-large-small",
            "baseline",
            "space",
            "circle-dashed",
            "sliders-horizontal",
            "locate-fixed",
            "rotate-ccw",
            "scaling",
            "download",
            "wand-sparkles",
            "arrow-up",
            "image",
            "x",
        ];
        let effects = EffectKind::ALL.map(EffectKind::icon);
        for icon in kinds.iter().map(kind_icon).chain(named).chain(effects) {
            assert!(icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }

    #[test]
    fn paints_are_stored_as_none_one_or_many() {
        let solid = |hex: &str| Paint::Solid(hex.to_owned());
        assert!(paints_of(Vec::new()).is_none());
        assert!(matches!(paints_of(vec![solid("#000000")]), Some(Paints::One(_))));
        assert!(matches!(paints_of(vec![solid("#000000"), solid("#ffffff")]), Some(Paints::Many(list)) if list.len() == 2));
    }
}
