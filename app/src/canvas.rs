//! The canvas: drawing the design, and pointer interaction — select, move, reorder, resize, create, pan, zoom.

use egui::{Color32, CursorIcon, Pos2, Rect as ScreenRect, Sense, Stroke, StrokeKind};
use pen_doc::edit;
use pen_doc::geom::{Affine, Point, Rect};
use pen_doc::layout::{self, NOTE_MIN_HEIGHT, NOTE_MIN_WIDTH};
use pen_doc::model::{Kind, LayoutMode, Node};
use pen_doc::scene::SceneNode;
use pen_doc::value::Num;
use pen_doc::vars::Vars;

use crate::editor::{Camera, Editor, FIT_PADDING, MAX_ZOOM, MIN_ZOOM, Role, Tool};
use crate::ops::{is_free, set_position, stored_position};
use crate::render::{DisplayList, doc_point, screen};
use crate::snap::{Lines, line_up, pulled};
use crate::theme::{
    self, ACCENT, COMPONENT, EDGE_GRAB, EYEDROPPER_OFFSET, EYEDROPPER_SWATCH, FRAME_NAME_GAP, FRAME_NAME_LEAST, FRAME_NAME_MORPH, FRAME_NAME_PILL,
    FRAME_NAME_PILL_RADIUS, FRAME_NAME_PLATE, FRAME_NAME_PLATE_RADIUS, FRAME_NAME_TEXT, HANDLE_SIZE, INSTANCE, PEN_ANCHOR, PEN_HANDLE, PEN_REACH,
    RADIUS_HANDLE, RADIUS_HANDLE_INSET, RADIUS_HANDLE_ROOM, ROTATE_DOT, ROTATE_DOT_GAP, ROTATE_DOT_GRAB, ROTATE_REACH, SIZE_LABEL_TEXT, SNAP_GAP_BAR,
    SNAP_GUIDE, SNAP_GUIDE_OVER, SNAP_REACH, WIRE_REACH,
};

/// Wheel and trackpad panning: screen points per scroll point.
const PAN_SPEED: f32 = 1.5;
/// Largest pan step per event, screen points (the delta is scaled down past 120).
const PAN_STEP_LIMIT: f32 = 120.0;
/// Least screen size of a text-tool drag that makes a sized text box (width, height).
const TEXT_BOX_MIN: (f32, f32) = (30.0, 10.0);
/// Font size of text placed with the text tool.
const TEXT_TOOL_FONT_SIZE: f32 = 16.0;
/// Fill of new rectangles and ellipses.
const SHAPE_FILL: &str = "#CCCCCC";
/// Corners of a newly drawn polygon.
const POLYGON_CORNERS: f32 = 3.0;
/// The step, in degrees, a turn snaps to with ⇧ held.
const ROTATE_STEP: f32 = 15.0;
/// Side of a newly placed icon.
const ICON_TOOL_SIZE: f32 = 24.0;
/// Fill of new frames.
pub const FRAME_FILL: &str = "#FFFFFF";
/// Wheel zoom: largest wheel step counted, with ⌘ and with ctrl.
const ZOOM_STEP_LIMIT_CMD: f32 = 15.0;
/// See [`ZOOM_STEP_LIMIT_CMD`].
const ZOOM_STEP_LIMIT_CTRL: f32 = 30.0;
/// Wheel zoom: zoom change per scroll point.
const ZOOM_PER_SCROLL: f32 = 0.012;
/// Screen points per wheel "line" for mice that scroll by lines (estimated).
const LINE_POINTS: f32 = 40.0;
/// Drags shorter than this many screen points count as clicks.
const DRAG_THRESHOLD: f32 = 3.0;

/// One of the eight resize handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Handle {
    /// −1 left edge, 0 not horizontal, 1 right edge.
    x: i8,
    /// −1 top edge, 0 not vertical, 1 bottom edge.
    y: i8,
}

/// What a layer that a drag took out of its parent's layout had stored before: its place and its size.
struct Lifted {
    id: String,
    x: Option<serde_json::Number>,
    y: Option<serde_json::Number>,
    width: Option<Num>,
    height: Option<Num>,
}

/// What a pointer drag is doing.
enum Drag {
    Pan,
    Move {
        start: Point,
        origins: Vec<(String, f32, f32)>,
        moved: bool,
        /// The box around the moved layers as the drag began, in the document, for snapping; `None` when it
        /// isn't known (the copies of a duplicate went elsewhere).
        bounds: Option<Rect>,
        /// The moved layers this drag took out of their parents' layouts, as they were stored before.
        lifted: Vec<Lifted>,
        /// Where the one moved layer would go back into its parent's layout if it were let go now: the
        /// parent and the place among its children. `None` unless ⌘ is held over a parent with a layout.
        slot: Option<(String, usize)>,
    },
    /// Pulling a handle: the layer's box as it was, the transform from the document to the layer's own axes
    /// (the pull is measured along them, so a turned layer resizes along its own sides), and the one from those
    /// axes to its parent's (for moving the layer's corner).
    Resize {
        id: String,
        handle: Handle,
        start: Point,
        original: Rect,
        to_local: Affine,
        from_local: Affine,
        /// The layer's selection box as the drag began, in the document, for snapping; `None` for a layer
        /// whose sides don't run along the document's (turned, mirrored, or in such a parent).
        shown: Option<Rect>,
    },
    /// Turning the selection: the layers as they were, the angle of the pointer from the turning point then,
    /// and how far it has been turned.
    Rotate {
        from: crate::ops::RotateStart,
        start_angle: f32,
        turned: f32,
    },
    Create {
        tool: Tool,
        start: Point,
    },
    /// The pen pulling the handles out of the point it just placed.
    Pen,
    /// The connector tool pulling a connector out of where the button went down (see `Canvas::wire_draft`).
    Wire,
    /// Pulling a corner-radius handle: the layer, its box, and which way is inwards from the pulled corner.
    Radius {
        id: String,
        bounds: Rect,
        inward: (f32, f32),
    },
    /// The Scale tool pulling a handle: the document as it was, and the handle.
    Scale {
        from: Box<crate::ops::ScaleStart>,
        handle: Handle,
    },
    Select {
        start: Point,
        base: Vec<String>,
    },
}

/// The user's canvas settings the pointer handling follows; set from the settings each frame.
#[derive(Clone, Copy, Debug, Default)]
#[expect(clippy::struct_excessive_bools, reason = "each is one switch of the settings, as the user set it")]
pub struct Options {
    /// New and drawn things land on whole pixels.
    pub snap_to_pixels: bool,
    /// A layer moved or resized lines up with the edges and middles of the layers beside it.
    pub snap_to_objects: bool,
    /// The wheel zooms without ⌘ or ctrl.
    pub wheel_zooms: bool,
    /// Zooming goes the other way.
    pub invert_zoom: bool,
    /// How large an AI model's cursor is drawn (1 = small).
    pub agent_cursor: f32,
}

/// The canvas's own state for one window.
#[derive(Default)]
pub struct Canvas {
    /// The user's canvas settings.
    pub options: Options,
    /// The user's "Animations & effects" setting: things on the canvas move to their new place and look.
    pub animations: bool,
    /// Where each board's name was drawn in the last frame, by the board's id and in the order drawn: a press
    /// on a name takes its board.
    names: Vec<(String, ScreenRect)>,
    /// The lines through what the dragged layer lined up with in this frame.
    guides: Vec<crate::snap::Guide>,
    /// The equal gaps the dragged layer stands at in this frame, each as a line across it.
    gaps: Vec<crate::snap::Guide>,
    list: DisplayList,
    /// The canvas background this frame (for the text tool's colour choice).
    background: Color32,
    drag: Option<Drag>,
    /// The text node being edited in place.
    pub editing: Option<String>,
    /// What a drag holds of a path whose points are edited.
    path_grab: Option<crate::path_edit::Grab>,
    /// The handle of the selected connector a drag holds.
    wire_grab: Option<crate::wire_edit::Grab>,
    /// The connector being drawn with the connector tool.
    wire_draft: Option<crate::wire_edit::Draft>,
}

impl Canvas {
    /// Draws and handles the canvas for `editor` in the remaining space of `ui`.
    // Agent: MAIN-THREAD; WRITES editor.doc (through checkpoint/touched), editor.selection, editor.camera.
    pub fn show(&mut self, ui: &mut egui::Ui, editor: &mut Editor, dark: bool) {
        let (view, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let painter = ui.painter_at(view);
        self.background = theme::palette(dark).canvas;
        painter.rect_filled(view, 0.0, self.background);
        if editor.take_fit_request() {
            fit(editor, view, editor.content_bounds(), 1.0);
        }
        // Human: "Go to component" brings the component into view the way zoom-to-fit does (40-point margin, at
        // most 100 %).
        if let Some(bounds) = editor.take_focus() {
            fit(editor, view, Some(bounds), 1.0);
        }
        self.zoom_and_pan(ui, editor, view, &response);
        let folder = editor.folder().map(std::path::Path::to_path_buf);
        let visible = crate::render::view_to_doc(&editor.camera, view);
        // Human: what a model changed is shown a step at a time: the steps whose turn has come are taken now,
        // and while more are waiting the canvas shows the document as it was after the last one taken.
        editor.activity.advance(std::time::Instant::now(), editor.generation);
        let (stage, stamp) = editor.activity.stage();
        self.list.update(
            stage.unwrap_or(&editor.scene),
            editor.generation,
            editor.camera.zoom,
            visible,
            &editor.doc,
            ui.ctx(),
            folder.as_deref(),
            self.background,
            editor.activity.hidden(),
            stamp,
            // A stage of a model's change is a scene of its own, without stamps: it is built whole.
            if stage.is_some() { &[] } else { editor.stamps() },
        );
        self.list.draw(&painter, &editor.camera, view);
        // Human: a path being drawn becomes a layer when the tool changes (Escape goes back to Move) or with
        // Enter, which also goes back to Move.
        if editor.tool != Tool::Pen {
            finish_pen(editor, false);
        } else if !editor.pen.is_empty() && !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            finish_pen(editor, false);
            editor.tool = Tool::Move;
        }
        self.path_keys(ui, editor);
        self.pointer(ui, editor, view, &response);
        self.overlay(ui, &painter, editor, view, dark);
        spotlight(&painter, editor, view);
        crate::path_edit::overlay(ui, &painter, editor, view, self.path_grab.is_some());
        crate::wire_edit::overlay(ui, &painter, editor, view, self.wire_grab.as_ref(), self.wire_draft.as_ref());
        let pulling = matches!(self.drag, Some(Drag::Pen));
        pen_overlay(ui, &painter, editor, view, pulling, self.options.snap_to_pixels);
        if editor.activity.posed {
            // A picture of the app shows the flash a quarter of the way through, whenever it is taken.
            let now = std::time::Instant::now();
            editor.activity.flash(&editor.selection.clone(), now.checked_sub(theme::AGENT_FLASH_TIME / 4).unwrap_or(now));
            editor.activity.point(editor.selection_bounds(), "Claude", now);
        }
        editor.activity.draw(ui, &painter, editor, view, self.options.agent_cursor);
        self.text_editor(ui, editor, view);
    }

    /// The keys of path editing: Enter starts it for the selected path and ends it again; in it X takes the cut
    /// and V the pointer back.
    fn path_keys(&mut self, ui: &egui::Ui, editor: &mut Editor) {
        editor.keep_path_edit();
        if editor.path_edit.is_none() {
            self.path_grab = None;
        }
        if ui.ctx().egui_wants_keyboard_input() || editor.tool != Tool::Move || ui.input(|i| i.modifiers.any()) {
            return;
        }
        let pressed = |key| ui.input(|i| i.key_pressed(key));
        if pressed(egui::Key::Enter) {
            if editor.path_edit.take().is_none()
                && let [id] = editor.selection.as_slice()
            {
                editor.edit_path(id.clone());
            }
        } else if let Some(editing) = &mut editor.path_edit {
            if pressed(egui::Key::X) {
                (editing.cutting, editing.selected) = (true, Vec::new());
            } else if pressed(egui::Key::V) {
                editing.cutting = false;
            }
        }
    }

    fn zoom_and_pan(&mut self, ui: &egui::Ui, editor: &mut Editor, view: ScreenRect, response: &egui::Response) {
        if !response.hovered() && self.drag.is_none() {
            return;
        }
        let (events, pointer) = ui.input(|i| (i.events.clone(), i.pointer.hover_pos()));
        let anchor = pointer.unwrap_or(view.center());
        for event in events {
            match event {
                // Human: with ⌘ or ctrl the wheel zooms by
                // clamp(deltaY, ±15 or ±30) × −0.012 of the current zoom; otherwise it pans 1.5× the delta, at most
                // 120 points a step. The deltas are counted the other way from egui's, hence the minus signs.
                egui::Event::MouseWheel { unit, delta, modifiers, .. } => {
                    let scale = match unit {
                        egui::MouseWheelUnit::Point => 1.0,
                        egui::MouseWheelUnit::Line => LINE_POINTS,
                        egui::MouseWheelUnit::Page => view.height(),
                    };
                    let (dx, dy) = (-delta.x * scale, -delta.y * scale);
                    if modifiers.ctrl || modifiers.command || self.options.wheel_zooms {
                        let limit = if modifiers.mac_cmd { ZOOM_STEP_LIMIT_CMD } else { ZOOM_STEP_LIMIT_CTRL };
                        let direction = if self.options.invert_zoom { 1.0 } else { -1.0 };
                        let change = dy.clamp(-limit, limit) * direction * ZOOM_PER_SCROLL;
                        zoom_at(editor, view, anchor, editor.camera.zoom + change * editor.camera.zoom);
                    } else {
                        let (mut px, mut py) = (dx * PAN_SPEED, dy * PAN_SPEED);
                        let length = (px * px + py * py).sqrt();
                        if length > PAN_STEP_LIMIT {
                            px *= PAN_STEP_LIMIT / length;
                            py *= PAN_STEP_LIMIT / length;
                        }
                        if !cfg!(target_os = "macos") && modifiers.shift {
                            std::mem::swap(&mut px, &mut py);
                        }
                        editor.camera.center.x += px / editor.camera.zoom;
                        editor.camera.center.y += py / editor.camera.zoom;
                    }
                }
                // A trackpad pinch.
                egui::Event::Zoom(factor) => {
                    let factor = if self.options.invert_zoom { 1.0 / factor.max(f32::EPSILON) } else { factor };
                    zoom_at(editor, view, anchor, editor.camera.zoom * factor);
                }
                _ => {}
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "the pointer state machine reads best as one match")]
    fn pointer(&mut self, ui: &egui::Ui, editor: &mut Editor, view: ScreenRect, response: &egui::Response) {
        let space = ui.input(|i| i.key_down(egui::Key::Space)) && !ui.ctx().egui_wants_keyboard_input();
        let middle = ui.input(|i| i.pointer.middle_down());
        let modifiers = ui.input(|i| i.modifiers);
        let snap = self.options.snap_to_pixels;
        self.guides.clear();
        self.gaps.clear();
        let pointer = response.interact_pointer_pos().or_else(|| ui.input(|i| i.pointer.hover_pos()));
        if editor.tool == Tool::Hand || space || middle || matches!(self.drag, Some(Drag::Pan)) {
            ui.ctx().set_cursor_icon(if response.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab });
        } else if matches!(
            editor.tool,
            Tool::Frame
                | Tool::Rectangle
                | Tool::Ellipse
                | Tool::Polygon
                | Tool::Icon
                | Tool::Text
                | Tool::Note
                | Tool::Pen
                | Tool::Connector
                | Tool::Eyedropper
        ) {
            ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
        }
        // Human: while a path's points are edited the pointer works on them; panning stays as it is.
        if editor.path_edit.is_some() && editor.tool == Tool::Move && !space && !middle && self.drag.is_none() {
            crate::path_edit::pointer(ui, editor, view, response, &mut self.path_grab, snap);
            return;
        }
        // Human: a connector that is selected alone has handles of its own; a press on one of them is theirs, and
        // anything else goes on as usual.
        if editor.tool == Tool::Move
            && !space
            && !middle
            && self.drag.is_none()
            && crate::wire_edit::pointer(ui, editor, view, response, &mut self.wire_grab, snap)
        {
            return;
        }
        // Double click: edit text, the points of a path, or step into the clicked container.
        if response.double_clicked()
            && let Some(p) = pointer
        {
            let chain = hit_free(&editor.scene, doc_point(&editor.camera, view, p), Vars::new(&editor.doc), WIRE_REACH / editor.camera.zoom);
            let current = editor.selection.first().and_then(|s| chain.iter().position(|c| c == s));
            match current {
                Some(i) if chain.get(i + 1).is_some() => editor.selection = vec![chain[i + 1].clone()],
                Some(i) if editor.scene_node(&chain[i]).is_some_and(|s| matches!(s.node.kind, Kind::Text | Kind::Note)) => {
                    editor.checkpoint();
                    self.editing = Some(chain[i].clone());
                }
                Some(i) if editor.can_edit_path(&chain[i]) => editor.edit_path(chain[i].clone()),
                _ => {}
            }
            return;
        }
        if response.drag_started() {
            let Some(p) = pointer else { return };
            let at = doc_point(&editor.camera, view, p);
            // Human: a drag is only known to be one after the pointer has moved a little; what was grabbed is
            // decided by where the button went down, or a small handle would be missed by those few points.
            let grab = ui.input(|i| i.pointer.press_origin()).unwrap_or(p);
            self.editing = None;
            if editor.tool == Tool::Hand || space || middle {
                self.drag = Some(Drag::Pan);
            } else if editor.tool == Tool::Eyedropper {
                // Human: the eyedropper takes a colour with a click; a pull with it moves nothing.
            } else if editor.tool == Tool::Pen {
                // Human: a press and pull places a point where the button went down and pulls its handles out.
                if pen_press(editor, doc_point(&editor.camera, view, grab), modifiers.shift, snap) {
                    self.drag = Some(Drag::Pen);
                }
            } else if editor.tool == Tool::Connector {
                // Human: a connector is pulled from where the button went down to where it is let go.
                self.wire_draft = Some(crate::wire_edit::Draft::start(doc_point(&editor.camera, view, grab)));
                self.drag = Some(Drag::Wire);
            } else if matches!(editor.tool, Tool::Frame | Tool::Rectangle | Tool::Ellipse | Tool::Polygon | Tool::Text) {
                // Human: snap to the pixel grid, on by default.
                self.drag = Some(Drag::Create { tool: editor.tool, start: snapped(at, snap) });
            } else if rotate_dot(editor, view).is_some_and(|(_, dot)| dot.distance(grab) <= ROTATE_DOT_GRAB)
                && let Some(from) = editor.rotate_start()
            {
                // Human: the round handle above the selection turns it around its middle (⇧: steps of 15°).
                editor.checkpoint();
                let start_angle = (at.y - from.centre.y).atan2(at.x - from.centre.x);
                self.drag = Some(Drag::Rotate { from, start_angle, turned: 0.0 });
            } else if let Some((id, handle)) = handle_at(editor, view, grab) {
                // Human: with the Scale tool a handle scales the layer and everything in it, in proportion,
                // away from the opposite side.
                if editor.tool == Tool::Scale {
                    if let Some(from) = editor.scale_start() {
                        editor.checkpoint();
                        self.drag = Some(Drag::Scale { from: Box::new(from), handle });
                    }
                } else if let Some(scene) = editor.scene_node(&id) {
                    let to_parent = parent_transform(editor, &id).invert().unwrap_or(Affine::IDENTITY);
                    let (to_local, from_local) = (scene.to_doc.invert().unwrap_or(Affine::IDENTITY), to_parent.then(&scene.to_doc));
                    let original = scene.local;
                    let shown = upright(&scene.to_doc).then(|| selection_box(scene));
                    editor.checkpoint();
                    self.drag = Some(Drag::Resize { id, handle, start: at, original, to_local, from_local, shown });
                }
            } else if rotate_zone(editor, view, grab)
                && let Some(from) = editor.rotate_start()
            {
                // Human: a press just outside a corner of the selection box turns the layer around its middle
                // (⇧ turns in steps of 15°).
                editor.checkpoint();
                let start_angle = (at.y - from.centre.y).atan2(at.x - from.centre.x);
                self.drag = Some(Drag::Rotate { from, start_angle, turned: 0.0 });
            } else if let Some((id, bounds, inward)) = radius_handle_at(editor, view, grab) {
                editor.checkpoint();
                self.drag = Some(Drag::Radius { id, bounds, inward });
            } else if let Some(id) = self.named_at(editor, grab).or_else(|| pick(editor, at, modifiers.command)) {
                if !editor.selection.contains(&id) {
                    if modifiers.shift {
                        editor.selection.push(id.clone());
                    } else {
                        editor.selection = vec![id.clone()];
                    }
                }
                self.drag = Some(start_move(editor, &id, at, modifiers.alt));
            } else {
                let base = if modifiers.shift { editor.selection.clone() } else { Vec::new() };
                editor.selection.clone_from(&base);
                self.drag = Some(Drag::Select { start: at, base });
            }
        }
        if response.dragged() {
            let delta = response.drag_delta();
            let Some(p) = pointer else { return };
            let at = doc_point(&editor.camera, view, p);
            match &mut self.drag {
                Some(Drag::Pan) => {
                    editor.camera.center.x -= delta.x / editor.camera.zoom;
                    editor.camera.center.y -= delta.y / editor.camera.zoom;
                }
                Some(Drag::Move { start, origins, moved, bounds, lifted, slot }) => {
                    if !*moved && (p - screen(&editor.camera, view, *start)).length() < DRAG_THRESHOLD {
                        return;
                    }
                    if !*moved {
                        editor.checkpoint();
                        *lifted = lift(editor, origins);
                        *moved = true;
                    }
                    let (mut dx, mut dy) = (at.x - start.x, at.y - start.y);
                    // Human: near a neighbour's edge or middle the layers go the rest of the way and a line says so.
                    if self.options.snap_to_objects
                        && let Some(bounds) = bounds
                    {
                        let reach = SNAP_REACH / editor.camera.zoom;
                        let lined = line_up(&bounds.translate(dx, dy), &snap_targets(editor, view), reach, Lines::ALL, Lines::ALL, false);
                        (dx, dy) = (dx + lined.dx, dy + lined.dy);
                        (self.guides, self.gaps) = (lined.guides, lined.gaps);
                    }
                    for (id, x, y) in origins.iter() {
                        let local = linear_inverse(&parent_transform(editor, id), dx, dy);
                        if let Some(node) = edit::find_mut(&mut editor.doc, id) {
                            set_position(node, x + local.0, y + local.1);
                        }
                    }
                    // Human: with ⌘ held a line shows where the layer would go back into its parent's layout.
                    *slot = match origins.as_slice() {
                        [(id, ..)] if modifiers.command => {
                            flex_parent(editor, id).and_then(|parent| reorder_target(editor, &parent, id, at).map(|target| (parent, target)))
                        }
                        _ => None,
                    };
                    editor.touched();
                }
                Some(Drag::Resize { id, handle, start, original, to_local, from_local, shown }) => {
                    let (mut dx, mut dy) = (at.x - start.x, at.y - start.y);
                    // Human: the pulled edge lines up like a moved layer does. Not while ⇧ keeps the proportions:
                    // an edge that jumped would take the other side's size with it.
                    if self.options.snap_to_objects
                        && !modifiers.shift
                        && let Some(shown) = shown
                    {
                        let (xs, ys) = (Lines::pulled(handle.x), Lines::pulled(handle.y));
                        let reach = SNAP_REACH / editor.camera.zoom;
                        let lined = line_up(&pulled(shown, xs, ys, dx, dy), &snap_targets(editor, view), reach, xs, ys, true);
                        (dx, dy) = (dx + lined.dx, dy + lined.dy);
                        self.guides = lined.guides;
                    }
                    let pull = linear(to_local, dx, dy);
                    resize(editor, id, *handle, *original, pull, modifiers.shift, from_local);
                }
                Some(Drag::Rotate { from, start_angle, turned }) => {
                    // Human: on screen y points down, so the pointer's angle grows clockwise; the file's
                    // rotation is counter-clockwise, hence the minus.
                    let angle = (at.y - from.centre.y).atan2(at.x - from.centre.x);
                    let mut degrees = -(angle - *start_angle).to_degrees();
                    if modifiers.shift {
                        degrees = (degrees / ROTATE_STEP).round() * ROTATE_STEP;
                    }
                    *turned = degrees;
                    editor.rotate_from(from, degrees);
                    editor.touched();
                }
                Some(Drag::Radius { id, bounds, inward }) => {
                    // Human: the radius is how far the pointer is in from the corner, on the nearer axis, up to
                    // half the shorter side (a larger radius draws no differently).
                    let corner =
                        Point::new(if inward.0 > 0.0 { bounds.x } else { bounds.right() }, if inward.1 > 0.0 { bounds.y } else { bounds.bottom() });
                    let pulled = ((at.x - corner.x) * inward.0).min((at.y - corner.y) * inward.1);
                    let radius = pulled.clamp(0.0, bounds.w.min(bounds.h) / 2.0).round();
                    editor.change(id, &|node| node.corner_radius = (radius > 0.0).then(|| pen_doc::model::Corners::All(Num::new(radius))));
                    editor.touched();
                }
                Some(Drag::Scale { from, handle }) => {
                    let b = from.bounds;
                    let anchor = Point::new(b.x + b.w * (1.0 - f32::from(handle.x)) / 2.0, b.y + b.h * (1.0 - f32::from(handle.y)) / 2.0);
                    let along =
                        |pulled: i8, at: f32, anchor: f32, size: f32| (pulled != 0 && size > 0.0).then(|| (at - anchor) * f32::from(pulled) / size);
                    let factor =
                        [along(handle.x, at.x, anchor.x, b.w), along(handle.y, at.y, anchor.y, b.h)].into_iter().flatten().fold(0.0, f32::max);
                    editor.scale_from(from, factor, anchor);
                    editor.touched();
                }
                Some(Drag::Select { start, base }) => {
                    let area = Rect::from_corners(*start, at);
                    let mut selection = base.clone();
                    for node in selectable_level(editor) {
                        if layout::doc_bounds(node).intersects(&area)
                            && let Some(id) = &node.doc_id
                            && !selection.contains(id)
                            && !editor.locked(id)
                        {
                            selection.push(id.clone());
                        }
                    }
                    editor.selection = selection;
                }
                Some(Drag::Pen) => editor.pen.pull(snapped(at, snap), modifiers.shift),
                Some(Drag::Create { .. } | Drag::Wire) | None => {}
            }
        }
        if response.drag_stopped() {
            match self.drag.take() {
                Some(Drag::Wire) => {
                    if let (Some(draft), Some(p)) = (self.wire_draft.take(), pointer) {
                        draft.finish(editor, doc_point(&editor.camera, view, p), snap);
                    }
                }
                Some(Drag::Create { tool, start }) => {
                    let end = snapped(pointer.map_or(start, |p| doc_point(&editor.camera, view, p)), snap);
                    let area = Rect::from_corners(start, end);
                    if tool == Tool::Text {
                        let (sw, sh) = (area.w * editor.camera.zoom, area.h * editor.camera.zoom);
                        let sized = sw > TEXT_BOX_MIN.0 && sh > TEXT_BOX_MIN.1;
                        let at = if sized { Point::new(area.x, area.y) } else { start };
                        self.editing = Some(create_text(editor, at, sized.then_some((area.w, area.h)), self.background));
                    } else if area.w > 0.0 && area.h > 0.0 {
                        // Human: shapes come only from a drag; a size of 0 creates nothing.
                        create(editor, tool, area);
                    }
                }
                // Human: let go with ⌘ held, the layer goes into its parent's layout where the line showed.
                Some(Drag::Move { origins, lifted, slot: Some((parent, target)), .. }) if modifiers.command => {
                    if let [(id, ..)] = origins.as_slice() {
                        drop_in_flex(editor, id, &parent, target, &lifted);
                    }
                }
                // Human: the next scaling starts from the layer as it is now.
                Some(Drag::Scale { .. }) => editor.scaled = (1.0, Vec::new()),
                Some(
                    Drag::Move { .. } | Drag::Pan | Drag::Pen | Drag::Resize { .. } | Drag::Rotate { .. } | Drag::Radius { .. } | Drag::Select { .. },
                )
                | None => {}
            }
        }
        if response.clicked() && self.drag.is_none() {
            let Some(p) = pointer else { return };
            let at = doc_point(&editor.camera, view, p);
            match editor.tool {
                Tool::Text => self.editing = Some(create_text(editor, snapped(at, snap), None, self.background)),
                Tool::Note => self.editing = Some(create_note(editor, snapped(at, snap))),
                // Human: a click without a drag draws nothing with the shape tools.
                Tool::Frame | Tool::Rectangle | Tool::Ellipse | Tool::Polygon | Tool::Hand | Tool::Connector => {}
                Tool::Icon => create_icon(editor, snapped(at, snap), self.background),
                // Human: a click places a corner point; on the first point it closes the path.
                Tool::Pen => {
                    pen_press(editor, at, modifiers.shift, snap);
                }
                // Human: the colour under the pointer goes to the selected layers, as their fill, or with ⌥ as
                // their stroke; with nothing selected it goes to the clipboard as a hex colour. Then the pointer
                // is back. A click on nothing that has a colour of its own leaves the tool on.
                Tool::Eyedropper => {
                    if let Some(color) = color_under(editor, at) {
                        if editor.selection.is_empty() {
                            ui.ctx().copy_text(crate::ops::hex_of(color));
                        } else {
                            editor.drop_color(color, modifiers.alt);
                        }
                        editor.tool = Tool::Move;
                    }
                }
                Tool::Move | Tool::Scale => {
                    self.editing = None;
                    match self.named_at(editor, screen(&editor.camera, view, at)).or_else(|| pick(editor, at, modifiers.command)) {
                        Some(id) if modifiers.shift => {
                            if let Some(i) = editor.selection.iter().position(|s| *s == id) {
                                editor.selection.remove(i);
                            } else {
                                editor.selection.push(id);
                            }
                        }
                        Some(id) => editor.selection = vec![id],
                        None if !modifiers.shift => editor.selection.clear(),
                        None => {}
                    }
                }
            }
        }
    }

    /// The board whose name is at `on` on the screen, if any.
    // Human: a board's name is the handle one reaches for to take the whole board, the more so on a board full
    // of layers, where a click inside takes one of them. A press on it comes before what [`pick`] finds, as
    // the name lies over the design; the name drawn last lies on top and wins.
    fn named_at(&self, editor: &Editor, on: Pos2) -> Option<String> {
        let named = self.names.iter().rev().find(|(id, name)| name.contains(on) && editor.scene_node(id).is_some());
        named.map(|(id, _)| id.clone())
    }

    /// Draws the top-level frames' names above their boards, placed by [`place_names`]; `over` is the pointer in
    /// the document. With `animate` a name moves to a new place and turns into the pill and back over
    /// `FRAME_NAME_MORPH` seconds; without, at once. Returns where each name shown stands, by its board's id, for
    /// [`Self::named_at`].
    // Human: a name that jumped to its place above the handle when its board was clicked read as two things, one
    // gone and one new. It is one thing that goes there: it slides to the middle and up while the pill grows
    // around it and the colours turn, and the same way back when the board is let go.
    // Agent: MAIN-THREAD; the motion is kept by egui under the board's id, which asks for frames while it runs.
    // How far the pill has come is a share of the way, so a zoom or pan under way doesn't leave the name behind.
    fn board_names(
        &self,
        painter: &egui::Painter,
        editor: &Editor,
        view: ScreenRect,
        palette: &theme::Palette,
        over: Option<Point>,
        covering: Covering,
    ) -> Vec<(String, ScreenRect)> {
        let cam = &editor.camera;
        let font = egui::FontId::proportional(FRAME_NAME_TEXT);
        let (mut boards, mut drawn) = (Vec::new(), Vec::new());
        for node in editor.activity.shown(&editor.scene) {
            if node.node.kind != Kind::Frame {
                continue;
            }
            let b = layout::doc_bounds(node);
            let board = ScreenRect::from_min_max(screen(cam, view, Point::new(b.x, b.y)), screen(cam, view, Point::new(b.right(), b.bottom())));
            let selected = node.doc_id.as_ref().is_some_and(|id| editor.selection.contains(id));
            // Human: the pointer on a board's name counts as on the board, so the name stays whole under it.
            let on_name = over.map(|p| screen(cam, view, p)).is_some_and(|p| self.names.iter().any(|(id, name)| *id == node.id && name.contains(p)));
            let whole = selected || on_name || over.is_some_and(|p| b.contains(p));
            // Agent: a board off the window is left out before its name is measured; a large design has thousands.
            if !whole && !view.intersects(board.expand(2.0 * FRAME_NAME_TEXT)) {
                continue;
            }
            let (ink, tint) = if node.node.reusable == Some(true) { (COMPONENT, COMPONENT) } else { (palette.muted, ACCENT) };
            let galley = painter.layout_no_wrap(node.node.display_name(), font.clone(), Color32::PLACEHOLDER);
            boards.push(NamedBoard { board, whole, selected, text: galley.size() });
            drawn.push((galley, ink, tint, &node.id));
        }
        let places = place_names(&boards, covering);
        let (ctx, time) = (painter.ctx(), if self.animations { FRAME_NAME_MORPH } else { 0.0 });
        // The names that are whole or on their way go over the cut ones.
        let (mut over_the_rest, mut shown) = (Vec::new(), Vec::new());
        for ((place, named), (galley, ink, tint, id)) in places.iter().zip(&boards).zip(&drawn) {
            let rest = name_rest(named);
            let (lift, pill) = match place {
                NamePlace::Whole { text, pill, .. } => (text.min.y - rest.y, *pill),
                NamePlace::Hidden | NamePlace::Cut(_) => (0.0, false),
            };
            let share = ctx.animate_bool_with_time_and_easing(egui::Id::unique(("board-name-pill", id)), pill, time, egui::emath::easing::cubic_out);
            let lift = ctx.animate_value_with_time(egui::Id::unique(("board-name-lift", id)), lift, time);
            if share > 0.0 || lift.abs() > 0.01 {
                // Human: on its way, the name is where its share of the way puts it, and looks that share like the pill.
                let min = rest + egui::vec2(share * (named.board.width() - named.text.x) / 2.0, lift);
                let pad = FRAME_NAME_PLATE + (FRAME_NAME_PILL - FRAME_NAME_PLATE) * share;
                // A name that ends up not shown at all goes out as it gets there.
                let fade = if *place == NamePlace::Hidden { share } else { 1.0 };
                let plate = palette.canvas.lerp_to_gamma(*tint, share).gamma_multiply(fade);
                let radius = FRAME_NAME_PLATE_RADIUS + (FRAME_NAME_PILL_RADIUS - FRAME_NAME_PLATE_RADIUS) * share;
                over_the_rest.push((
                    *id,
                    ScreenRect::from_min_size(min, named.text),
                    Some((pad, radius, plate)),
                    galley,
                    ink.lerp_to_gamma(Color32::WHITE, share).gamma_multiply(fade),
                ));
                continue;
            }
            match place {
                NamePlace::Hidden => {}
                NamePlace::Cut(text) => {
                    let galley = if galley.size().x > text.width() {
                        let mut job = egui::text::LayoutJob::simple_singleline(galley.text().to_owned(), font.clone(), Color32::PLACEHOLDER);
                        job.wrap = egui::text::TextWrapping::truncate_at_width(text.width());
                        painter.layout_job(job)
                    } else {
                        galley.clone()
                    };
                    painter.galley(text.min, galley, *ink);
                    shown.push(((*id).clone(), text.expand2(FRAME_NAME_PLATE)));
                }
                NamePlace::Whole { text, plate, .. } => {
                    over_the_rest.push((*id, *text, plate.then_some((FRAME_NAME_PLATE, FRAME_NAME_PLATE_RADIUS, palette.canvas)), galley, *ink));
                }
            }
        }
        for (id, text, plate, galley, ink) in over_the_rest {
            if let Some((pad, radius, fill)) = plate {
                painter.rect_filled(text.expand2(pad), radius, fill);
            }
            painter.galley(text.min, galley.clone(), ink);
            shown.push((id.clone(), text.expand2(plate.map_or(FRAME_NAME_PLATE, |(pad, _, _)| pad))));
        }
        shown
    }

    fn overlay(&mut self, ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, dark: bool) {
        let cam = &editor.camera;
        let palette = theme::palette(dark);
        let over = if self.drag.is_none() { ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p)) } else { None };
        let over = over.map(|p| doc_point(cam, view, p));
        // Human: the turning handle and the size label lie over the names; the names make way for them. The
        // handle counts while a drag hides it too, so a board's name doesn't drop and rise as the board is moved.
        let stem = rotate_dot(editor, view).map(|(foot, dot)| ScreenRect::from_two_pos(foot, dot).expand(ROTATE_DOT + 1.5));
        let badge = self.size_badge(editor, view).map(|(at, text)| (size_label_rect(painter, at, &text), text));
        let covering = Covering { stem, badge: badge.as_ref().map(|(rect, _)| *rect) };
        self.names = self.board_names(painter, editor, view, &palette, over, covering);
        // Human: a component's origin is marked with a magenta box, and its name above it when it isn't a board
        // that is named already — but only while it is selected or the pointer is over it; left alone the
        // design shows clean.
        for top in &editor.scene {
            top.walk(
                &mut |node, depth| {
                    let Some(id) = node.doc_id.as_ref().filter(|_| node.node.reusable == Some(true)) else { return };
                    if !editor.selection.contains(id) && !over.is_some_and(|p| layout::doc_bounds(node).contains(p)) {
                        return;
                    }
                    outline(painter, cam, view, node, Stroke::new(1.0, COMPONENT.gamma_multiply(0.7)));
                    if depth > 0 || node.node.kind != Kind::Frame {
                        let pos = screen(cam, view, node.to_doc.apply(Point::new(0.0, 0.0))) - egui::vec2(0.0, FRAME_NAME_GAP);
                        painter.text(
                            pos,
                            egui::Align2::LEFT_BOTTOM,
                            node.node.display_name(),
                            egui::FontId::proportional(FRAME_NAME_TEXT),
                            COMPONENT,
                        );
                    }
                },
                0,
            );
        }
        let tint = |id: &str| match editor.role(id) {
            Role::Component => COMPONENT,
            Role::Instance | Role::Inside => INSTANCE,
            Role::Plain => ACCENT,
        };
        // Hover outline.
        if self.drag.is_none()
            && let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p))
            && let Some(id) = self.named_at(editor, p).or_else(|| pick(editor, doc_point(cam, view, p), ui.input(|i| i.modifiers.command)))
            && !editor.selection.contains(&id)
            && let Some(node) = editor.scene_node(&id)
        {
            outline(painter, cam, view, node, Stroke::new(1.0, tint(&id)));
        }
        // Human: one layer is outlined itself. Several get one box around them all, with the size of that box
        // under it, and no boxes of their own (the user's wish). Each selected layer gets a thin line along its
        // own shape instead, so it is plain which layers inside the box are the selected ones while their
        // colours stay as they are.
        let several = editor.selection.len() > 1;
        if several {
            for node in editor.selection.iter().filter_map(|id| editor.scene_node(id)) {
                trace_shape(painter, cam, view, node);
            }
        }
        // Human: a path whose points are edited shows those instead of its box and handles.
        if editor.path_edit.is_some() {
            return;
        }
        if let [id] = editor.selection.as_slice()
            && let Some(node) = editor.scene_node(id)
        {
            outline(painter, cam, view, node, Stroke::new(1.5, tint(id)));
        }
        if several && let Some(all) = editor.selection.iter().filter_map(|id| editor.scene_node(id)).map(selection_box).reduce(|a, b| a.union(&b)) {
            let (min, max) = (screen(cam, view, Point::new(all.x, all.y)), screen(cam, view, Point::new(all.right(), all.bottom())));
            painter.rect_stroke(ScreenRect::from_min_max(min, max), 0.0, Stroke::new(1.5, ACCENT), StrokeKind::Inside);
        }
        self.handles(ui, painter, editor, view);
        guide_lines(painter, cam, view, &self.guides, &self.gaps);
        eyedropper_swatch(ui, painter, editor, view, &palette);
        if let Some((rect, text)) = badge {
            size_label(painter, rect, &text);
        }
        match &self.drag {
            Some(Drag::Select { start, .. }) => {
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let r = ScreenRect::from_two_pos(screen(cam, view, *start), p);
                    painter.rect(r, 0.0, ACCENT.gamma_multiply(0.1), Stroke::new(1.0, ACCENT), StrokeKind::Inside);
                }
            }
            Some(Drag::Create { start, .. }) => {
                if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                    let r = ScreenRect::from_two_pos(screen(cam, view, *start), p);
                    painter.rect(r, 0.0, Color32::TRANSPARENT, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
                }
            }
            Some(Drag::Move { slot: Some((parent, target)), .. }) => {
                if let Some(line) = reorder_line(editor, parent, *target) {
                    painter.line_segment([screen(cam, view, line.0), screen(cam, view, line.1)], Stroke::new(2.0, ACCENT));
                }
            }
            Some(
                Drag::Pan
                | Drag::Pen
                | Drag::Wire
                | Drag::Move { .. }
                | Drag::Resize { .. }
                | Drag::Rotate { .. }
                | Drag::Radius { .. }
                | Drag::Scale { .. },
            )
            | None => {}
        }
    }

    /// The selected layer's handles and the label under it.
    fn handles(&self, ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect) {
        let cam = &editor.camera;
        // Human: a round handle on a short stem above the selection: drag it to turn the selection.
        if !matches!(self.drag, Some(Drag::Move { .. } | Drag::Select { .. } | Drag::Create { .. } | Drag::Pan))
            && let Some((foot, dot)) = rotate_dot(editor, view)
        {
            painter.line_segment([foot, dot], Stroke::new(1.0, ACCENT));
            painter.circle(dot, ROTATE_DOT, Color32::WHITE, Stroke::new(1.5, ACCENT));
            if self.drag.is_none() && ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| dot.distance(p) <= ROTATE_DOT_GRAB) {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
            }
        }
        if let [id] = editor.selection.as_slice()
            && let Some(node) = editor.scene_node(id)
            && resizable(node)
            && editor.role(id) != Role::Inside
            && !editor.locked(id)
        {
            // Human: a square handle at each corner; the edges resize too, without a handle.
            // The handles sit on the box's own corners, so they turn with a turned layer.
            for corner in tight(node).corners() {
                let r = ScreenRect::from_center_size(screen(cam, view, node.to_doc.apply(corner)), egui::vec2(HANDLE_SIZE, HANDLE_SIZE));
                painter.rect(r, 1.0, Color32::WHITE, Stroke::new(1.0, ACCENT), StrokeKind::Inside);
            }
            // Human: just outside a corner the pointer turns the layer; a small turning arrow by the pointer says so.
            if self.drag.is_none()
                && let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p))
                && rotate_zone(editor, view, p)
            {
                ui.ctx().set_cursor_icon(CursorIcon::Grab);
                crate::icons::paint(painter, ScreenRect::from_center_size(p + egui::vec2(14.0, 14.0), egui::Vec2::splat(13.0)), "rotate-cw", ACCENT);
            }
            for (centre, _) in radius_handles(editor, view, id, node).into_iter().flatten() {
                painter.circle(centre, RADIUS_HANDLE, Color32::WHITE, Stroke::new(1.0, ACCENT));
            }
        }
    }

    /// The label under the selection: the point it is centred under, and what it says. One layer's label
    /// measures the box it sits under and says how far while the layer is turned; several layers' label
    /// measures the box around them all. `None` when no label is shown.
    fn size_badge(&self, editor: &Editor, view: ScreenRect) -> Option<(Pos2, String)> {
        if editor.path_edit.is_some() {
            return None;
        }
        let cam = &editor.camera;
        let turned = match &self.drag {
            Some(Drag::Rotate { turned, .. }) => Some(*turned),
            _ => None,
        };
        if let [id] = editor.selection.as_slice() {
            let node = editor.scene_node(id).filter(|node| resizable(node) && editor.role(id) != Role::Inside)?;
            let (b, measured) = (selection_box(node), tight(node));
            let text = turned.map_or_else(|| format!("{} × {}", trim(measured.w), trim(measured.h)), |turned| format!("{}°", trim(turned)));
            return Some((screen(cam, view, Point::new(b.center().x, b.bottom())), text));
        }
        if turned.is_some() {
            return None;
        }
        let all = editor.selection.iter().filter_map(|id| editor.scene_node(id)).map(selection_box).reduce(|a, b| a.union(&b))?;
        Some((screen(cam, view, Point::new(all.center().x, all.bottom())), format!("{} × {}", trim(all.w), trim(all.h))))
    }

    /// The in-place text editor over a text or note being edited.
    fn text_editor(&mut self, ui: &mut egui::Ui, editor: &mut Editor, view: ScreenRect) {
        let Some(id) = self.editing.clone() else { return };
        let Some(scene) = editor.scene_node(&id) else {
            self.editing = None;
            return;
        };
        let b = layout::doc_bounds(scene);
        let font = layout::text_style(&scene.node, Vars::new(&editor.doc), &scene.theme).size * editor.camera.zoom;
        let (mut min, mut size) = (
            screen(&editor.camera, view, Point::new(b.x, b.y)),
            egui::vec2((b.w * editor.camera.zoom).max(120.0), (b.h * editor.camera.zoom).max(28.0)),
        );
        // Human: a text set on a circle or along a line has that shape's box as its own; an editor that large
        // would hide the very letters being typed. Its words are edited in one line in the middle of the box,
        // and the text on its curve follows each key.
        if crate::render::runs_along(&scene.node) {
            size = egui::vec2((size.x * 0.7).max(120.0), (font.clamp(8.0, 96.0) * 1.4).max(28.0));
            let centre = screen(&editor.camera, view, b.center());
            min = egui::pos2(centre.x - size.x / 2.0, centre.y - size.y / 2.0);
        }
        // Human: the text shown, which for a text inside an instance is the component's with the instance's override.
        let mut text = scene.node.content.clone().unwrap_or_default();
        let area = egui::Area::new(egui::Id::unique(("text-edit", &id))).fixed_pos(min).order(egui::Order::Foreground);
        let output = area.show(ui.ctx(), |ui| {
            egui::Frame::new().fill(ui.visuals().window_fill.gamma_multiply(0.92)).stroke(Stroke::new(1.0, ACCENT)).show(ui, |ui| {
                ui.add_sized(
                    size,
                    egui::TextEdit::multiline(&mut text).font(egui::FontId::proportional(font.clamp(8.0, 96.0))).frame(egui::Frame::NONE),
                )
            })
        });
        let edit_response = output.inner.inner;
        if !edit_response.has_focus() && !edit_response.lost_focus() {
            edit_response.request_focus();
        }
        if edit_response.changed() && editor.change(&id, &|node| node.content = Some(text.clone())) {
            editor.touched();
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) || edit_response.lost_focus() {
            self.editing = None;
            // Human: a text left empty is deleted when editing ends.
            if edit::find(&editor.doc, &id).is_some_and(|n| n.kind == Kind::Text && n.content.as_deref().unwrap_or("").is_empty()) {
                edit::remove(&mut editor.doc, &id);
                editor.selection.retain(|s| *s != id);
                editor.touched();
            }
            editor.tool = Tool::Move;
        }
    }

    /// The canvas's drawing of the open document, which exports are made with.
    #[must_use]
    pub fn list(&self) -> &DisplayList {
        &self.list
    }

    /// Whether a drag on the canvas is in progress.
    #[must_use]
    pub fn busy(&self) -> bool {
        self.drag.is_some() || self.editing.is_some() || self.wire_grab.is_some()
    }

    /// Whether the user is dragging something on the canvas right now.
    #[must_use]
    pub fn dragging(&self) -> bool {
        self.drag.is_some() || self.wire_grab.is_some()
    }
}

/// The next zoom level for ⌘+ / ⌘−: the next power of two up or down:
/// `2 ** (floor(log2(zoom)) + 1)` and `2 ** (ceil(log2(zoom)) - 1)`.
#[must_use]
pub fn zoom_step(zoom: f32, up: bool) -> f32 {
    let level = zoom.max(MIN_ZOOM).log2();
    let exponent = if up { level.floor() + 1.0 } else { level.ceil() - 1.0 };
    exponent.exp2().clamp(MIN_ZOOM, MAX_ZOOM)
}

/// Zooms so `bounds` fills the part of the view the floating islands leave free ([`Editor::covered`]), with a
/// 40-point margin, never beyond `max_zoom`.
// Human: the design is kept out from under the tool rail, the message bar and the properties island (the
// user's wish).
pub fn fit(editor: &mut Editor, view: ScreenRect, bounds: Option<Rect>, max_zoom: f32) {
    let Some(b) = bounds else {
        editor.camera = Camera::default();
        return;
    };
    let [left, top, right, bottom] = editor.covered;
    let free = ScreenRect::from_min_max(view.min + egui::vec2(left, top), view.max - egui::vec2(right, bottom));
    // Agent: a window too small to leave free room falls back to the whole view.
    let free = if free.width() > 4.0 * FIT_PADDING && free.height() > 4.0 * FIT_PADDING { free } else { view };
    let zx = (free.width() - 2.0 * FIT_PADDING).max(1.0) / b.w.max(1.0);
    let zy = (free.height() - 2.0 * FIT_PADDING).max(1.0) / b.h.max(1.0);
    let zoom = zx.min(zy).min(max_zoom).clamp(MIN_ZOOM, MAX_ZOOM);
    // The design's middle lands on the middle of the free part, not of the whole view.
    let shift = (free.center() - view.center()) / zoom;
    editor.camera = Camera { center: Point::new(b.center().x - shift.x, b.center().y - shift.y), zoom };
}

/// Zooms to `zoom` keeping the document point under `anchor` (screen) still.
pub fn zoom_at(editor: &mut Editor, view: ScreenRect, anchor: Pos2, zoom: f32) {
    let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    let before = doc_point(&editor.camera, view, anchor);
    editor.camera.zoom = zoom;
    let after = doc_point(&editor.camera, view, anchor);
    editor.camera.center.x += before.x - after.x;
    editor.camera.center.y += before.y - after.y;
}

/// A board as its name is placed: its box on screen, whether the whole name is wanted (the board is selected
/// or under the pointer), whether it is selected, and the size of the whole name as text.
#[derive(Clone, Copy, Debug)]
struct NamedBoard {
    board: ScreenRect,
    whole: bool,
    selected: bool,
    text: egui::Vec2,
}

/// What lies over the names: the box of the turning handle with its stem, and that of the size label under
/// the selection.
#[derive(Clone, Copy, Debug, Default)]
struct Covering {
    stem: Option<ScreenRect>,
    badge: Option<ScreenRect>,
}

/// What is shown of a board's name.
#[derive(Clone, Copy, Debug, PartialEq)]
enum NamePlace {
    /// Nothing.
    Hidden,
    /// The name in this box, cut with "…" at its width.
    Cut(ScreenRect),
    /// The whole name in this box: as a pill in the selection colour for a selected board, else on a plate of
    /// the canvas colour when it lies over other things.
    Whole { text: ScreenRect, plate: bool, pill: bool },
}

/// Where a board's name starts when nothing moves it: above the board's top left corner.
fn name_rest(named: &NamedBoard) -> Pos2 {
    named.board.left_top() - egui::vec2(0.0, FRAME_NAME_GAP + named.text.y)
}

/// Where each board's name goes, in the order of `boards`.
// Human: zoomed out, names of a fixed size ran into each other and into the boards around them. So a name
// gets only its board's width, and none at all over a board narrower than `FRAME_NAME_LEAST`, or where it
// would lie on another board, under the size label or under a whole name. The board under the pointer keeps
// the whole name. A selected board's name is a pill in the selection colour over the middle of the board,
// the counterpart of the size label under it. A whole name the turning handle would cross stands above the
// handle; that is every selected board's when it is the only thing selected.
// Agent: boards that don't overlap can't have their cut names collide sideways; the other rules cover the rest.
fn place_names(boards: &[NamedBoard], covering: Covering) -> Vec<NamePlace> {
    let mut places: Vec<NamePlace> = boards
        .iter()
        .map(|named| {
            let (rest, room) = (name_rest(named), named.board.width());
            if named.whole {
                let (min, pad) = if named.selected {
                    (egui::pos2(named.board.center().x - named.text.x / 2.0, rest.y), FRAME_NAME_PILL)
                } else {
                    (rest, FRAME_NAME_PLATE)
                };
                let text = ScreenRect::from_min_size(min, named.text);
                match covering.stem.filter(|stem| stem.intersects(text.expand2(pad))) {
                    Some(stem) => {
                        let lift = stem.top() - FRAME_NAME_GAP - pad.y - text.bottom();
                        NamePlace::Whole { text: text.translate(egui::vec2(0.0, lift)), plate: true, pill: named.selected }
                    }
                    None => NamePlace::Whole { text, plate: named.text.x > room, pill: named.selected },
                }
            } else if room < FRAME_NAME_LEAST {
                NamePlace::Hidden
            } else {
                NamePlace::Cut(ScreenRect::from_min_size(rest, egui::vec2(named.text.x.min(room), named.text.y)))
            }
        })
        .collect();
    let covers: Vec<ScreenRect> = places
        .iter()
        .filter_map(|place| match place {
            NamePlace::Whole { text, pill, .. } => Some(text.expand2(if *pill { FRAME_NAME_PILL } else { FRAME_NAME_PLATE })),
            NamePlace::Hidden | NamePlace::Cut(_) => None,
        })
        .chain(covering.badge)
        .collect();
    for (index, place) in places.iter_mut().enumerate() {
        if let NamePlace::Cut(text) = *place {
            // A hair smaller, so a board or label that only touches the name doesn't count.
            let text = text.shrink(0.5);
            let covered = covers.iter().any(|cover| cover.intersects(text));
            if covered || boards.iter().enumerate().any(|(other, named)| other != index && named.board.intersects(text)) {
                *place = NamePlace::Hidden;
            }
        }
    }
    places
}

/// The box of the blue label under a selection box, centred under `at`.
fn size_label_rect(painter: &egui::Painter, at: Pos2, text: &str) -> ScreenRect {
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(SIZE_LABEL_TEXT), Color32::WHITE);
    egui::Align2::CENTER_TOP.anchor_size(at + egui::vec2(0.0, 7.0), galley.size() + egui::vec2(12.0, 5.0))
}

/// Draws the blue label under a selection box in `rect` (from [`size_label_rect`]).
fn size_label(painter: &egui::Painter, rect: ScreenRect, text: &str) {
    let galley = painter.layout_no_wrap(text.to_owned(), egui::FontId::proportional(SIZE_LABEL_TEXT), Color32::WHITE);
    painter.rect_filled(rect, 5.0, ACCENT);
    painter.galley(rect.min + egui::vec2(6.0, 2.5), galley, Color32::WHITE);
}

fn trim(v: f32) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    if rounded.fract() == 0.0 { format!("{rounded:.0}") } else { format!("{rounded}") }
}

/// Draws a thin line in the selection colour along a layer's own shape (the ellipse or polygon itself, else its
/// box), to mark it as one of several selected layers without covering its colours.
fn trace_shape(painter: &egui::Painter, cam: &Camera, view: ScreenRect, node: &SceneNode) {
    if node.node.kind == Kind::Connector {
        trace_wire(painter, cam, view, node, Stroke::new(1.0, ACCENT));
        return;
    }
    let points = shape_points(node).unwrap_or_else(|| Rect::new(0.0, 0.0, node.local.w, node.local.h).corners().to_vec());
    let on_screen: Vec<Pos2> = points.into_iter().map(|p| screen(cam, view, node.to_doc.apply(p))).collect();
    painter.add(egui::Shape::closed_line(on_screen, Stroke::new(1.0, ACCENT)));
}

/// Draws a line along a connector, which has no box worth showing.
fn trace_wire(painter: &egui::Painter, cam: &Camera, view: ScreenRect, node: &SceneNode, stroke: Stroke) {
    let on_screen: Vec<Pos2> = crate::wire_edit::line(node).into_iter().map(|p| screen(cam, view, p)).collect();
    painter.add(egui::Shape::line(on_screen, stroke));
}

/// Outlines a layer: its box, and for an ellipse or polygon the shape itself too. A connector is traced.
fn outline(painter: &egui::Painter, cam: &Camera, view: ScreenRect, node: &SceneNode, stroke: Stroke) {
    if node.node.kind == Kind::Connector {
        trace_wire(painter, cam, view, node, stroke);
        return;
    }
    let corners = tight(node).corners().map(|c| screen(cam, view, node.to_doc.apply(c)));
    painter.add(egui::Shape::closed_line(corners.to_vec(), stroke));
    if let Some(points) = shape_points(node) {
        painter.add(egui::Shape::closed_line(points.into_iter().map(|p| screen(cam, view, node.to_doc.apply(p))).collect(), stroke));
    }
}

/// The box drawn around a selected layer, in the layer's own coordinates: its own box, or for a polygon the
/// box that just holds its corners.
// Human: a polygon's corners lie on the ellipse of its box, so a triangle leaves a quarter of the box empty
// below its base. The user wants the selection to fit the shape, so the box, its handles and its size label
// follow the polygon itself.
fn tight(node: &SceneNode) -> Rect {
    let whole = Rect::new(0.0, 0.0, node.local.w, node.local.h);
    if node.node.kind != Kind::Polygon {
        return whole;
    }
    let Some(points) = shape_points(node).filter(|points| !points.is_empty()) else { return whole };
    let (mut min, mut max) = (points[0], points[0]);
    for p in &points {
        (min, max) = (Point::new(min.x.min(p.x), min.y.min(p.y)), Point::new(max.x.max(p.x), max.y.max(p.y)));
    }
    Rect::new(min.x, min.y, max.x - min.x, max.y - min.y)
}

/// The part of a layer's own box its selection box takes up, each side as a fraction of the box (see [`tight`]):
/// the whole of it, except for a polygon.
#[must_use]
pub fn fitted_part(node: &SceneNode) -> Rect {
    if node.local.w <= 0.0 || node.local.h <= 0.0 {
        return Rect::new(0.0, 0.0, 1.0, 1.0);
    }
    let t = tight(node);
    Rect::new(t.x / node.local.w, t.y / node.local.h, (t.w / node.local.w).max(1e-3), (t.h / node.local.h).max(1e-3))
}

/// With the eyedropper, shows the colour it would take in a round swatch beside the pointer.
fn eyedropper_swatch(ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, palette: &theme::Palette) {
    if editor.tool == Tool::Eyedropper
        && let Some(p) = ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p))
        && let Some(color) = color_under(editor, doc_point(&editor.camera, view, p))
    {
        let middle = p + egui::vec2(EYEDROPPER_OFFSET, EYEDROPPER_OFFSET);
        painter.circle(middle, EYEDROPPER_SWATCH + 1.0, Color32::TRANSPARENT, Stroke::new(1.0, palette.muted));
        painter.circle(middle, EYEDROPPER_SWATCH, crate::render::color32(color), Stroke::new(2.0, Color32::WHITE));
    }
}

/// Draws the lines through what the dragged layer lined up with, each a little longer than what it joins, and
/// a line with a bar at each end across every gap of `gaps`, which are the equal ones.
fn guide_lines(painter: &egui::Painter, cam: &Camera, view: ScreenRect, guides: &[crate::snap::Guide], gaps: &[crate::snap::Guide]) {
    let ends = |guide: &crate::snap::Guide| {
        let (from, to) = if guide.vertical {
            (Point::new(guide.at, guide.from), Point::new(guide.at, guide.to))
        } else {
            (Point::new(guide.from, guide.at), Point::new(guide.to, guide.at))
        };
        (screen(cam, view, from), screen(cam, view, to))
    };
    let stroke = Stroke::new(1.0, SNAP_GUIDE);
    for guide in guides {
        let (from, to) = ends(guide);
        let over = if guide.vertical { egui::vec2(0.0, SNAP_GUIDE_OVER) } else { egui::vec2(SNAP_GUIDE_OVER, 0.0) };
        painter.line_segment([from - over, to + over], stroke);
    }
    for gap in gaps {
        let (from, to) = ends(gap);
        let bar = if gap.vertical { egui::vec2(SNAP_GAP_BAR, 0.0) } else { egui::vec2(0.0, SNAP_GAP_BAR) };
        painter.line_segment([from, to], stroke);
        for end in [from, to] {
            painter.line_segment([end - bar, end + bar], stroke);
        }
    }
}

/// The selection box of a layer in document coordinates (see [`tight`]).
fn selection_box(node: &SceneNode) -> Rect {
    node.to_doc.bounds(&tight(node))
}

/// The outline of a layer whose shape isn't its box (an ellipse or a polygon), in the layer's own coordinates.
// Agent: reads plain numbers only; a corner count or radius bound to a variable falls back to 3 corners, sharp.
fn shape_points(node: &SceneNode) -> Option<Vec<Point>> {
    /// Straight pieces an ellipse's outline is drawn with.
    const ELLIPSE_STEPS: u8 = 72;
    let (w, h) = (node.local.w, node.local.h);
    match node.node.kind {
        Kind::Ellipse => Some(
            (0..ELLIPSE_STEPS)
                .map(|i| {
                    let angle = std::f32::consts::TAU * f32::from(i) / f32::from(ELLIPSE_STEPS);
                    Point::new(w / 2.0 + w / 2.0 * angle.cos(), h / 2.0 + h / 2.0 * angle.sin())
                })
                .collect(),
        ),
        Kind::Polygon => {
            let count = node.node.polygon_count.as_ref().and_then(Num::literal).unwrap_or(3.0).clamp(3.0, 64.0);
            let radius = match &node.node.corner_radius {
                Some(pen_doc::model::Corners::All(r)) => r.literal(),
                Some(pen_doc::model::Corners::Each(each)) => each.first().and_then(Num::literal),
                None => None,
            };
            Some(crate::render::polygon_points(w, h, count, radius.unwrap_or(0.0).max(0.0)).into_iter().map(|[x, y]| Point::new(x, y)).collect())
        }
        _ => None,
    }
}

/// Whether `p` is inside the closed line `points` (crossing count).
fn inside_outline(points: &[Point], p: Point) -> bool {
    let mut inside = false;
    for (index, a) in points.iter().enumerate() {
        let b = points[(index + 1) % points.len()];
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// The corner-radius handles of the selected layer `id`: where each is on screen, and which way is inwards from
/// its corner. `None` unless the layer is an upright rectangle or frame large enough on screen to hold them.
// Human: a round handle shows inside each corner of a selected rectangle; pulling one towards the
// middle rounds all four corners. A handle sits at its corner's radius, but never nearer the corner than 12 points.
fn radius_handles(editor: &Editor, view: ScreenRect, id: &str, node: &SceneNode) -> Option<[(Pos2, (f32, f32)); 4]> {
    if editor.locked(id) {
        return None;
    }
    let cam = &editor.camera;
    let b = layout::doc_bounds(node);
    let fits = b.w.min(b.h) * cam.zoom >= RADIUS_HANDLE_ROOM;
    let upright = node.to_doc.is_axis_aligned() && resizable(node);
    if !matches!(node.node.kind, Kind::Rectangle | Kind::Frame) || !upright || !fits || editor.role(id) == Role::Inside {
        return None;
    }
    let radius = match &node.node.corner_radius {
        Some(pen_doc::model::Corners::All(r)) => r.literal(),
        Some(pen_doc::model::Corners::Each(each)) => each.first().and_then(Num::literal),
        None => None,
    };
    let inset = (radius.unwrap_or(0.0) * cam.zoom).clamp(RADIUS_HANDLE_INSET, b.w.min(b.h) * cam.zoom / 2.0);
    Some([(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)].map(|inward: (f32, f32)| {
        let corner = Point::new(if inward.0 > 0.0 { b.x } else { b.right() }, if inward.1 > 0.0 { b.y } else { b.bottom() });
        (screen(cam, view, corner) + egui::vec2(inward.0 * inset, inward.1 * inset), inward)
    }))
}

/// The corner-radius handle under `p`: the layer, its box, and which way is inwards from that corner.
fn radius_handle_at(editor: &Editor, view: ScreenRect, p: Pos2) -> Option<(String, Rect, (f32, f32))> {
    let [id] = editor.selection.as_slice() else { return None };
    let node = editor.scene_node(id)?;
    let handles = radius_handles(editor, view, id, node)?;
    let (_, inward) = handles.into_iter().find(|(centre, _)| centre.distance(p) <= RADIUS_HANDLE + 3.0)?;
    Some((id.clone(), layout::doc_bounds(node), inward))
}

/// Whether a layer has handles: any layer that isn't mirrored (a turned one included). A connector has its
/// own, at its ends and waypoints (`wire_edit`).
fn resizable(node: &SceneNode) -> bool {
    let [a, b, c, d, _, _] = node.to_doc.m;
    a * d - b * c > 0.0 && node.node.kind != Kind::Connector
}

/// Where the pointer is on the selected layer's box, in the layer's own coordinates: the layer, its box, the
/// point, and how many of the layer's units one screen point is.
#[expect(clippy::many_single_char_names, reason = "a, b, c, d are the matrix entries' standard names; p the pointer")]
fn on_selection(editor: &Editor, view: ScreenRect, p: Pos2) -> Option<(String, Rect, Point, f32)> {
    let [id] = editor.selection.as_slice() else { return None };
    let node = editor.scene_node(id).filter(|n| resizable(n) && editor.role(id) != Role::Inside)?;
    let local = layout::to_local(node, doc_point(&editor.camera, view, p))?;
    let [a, b, c, d, _, _] = node.to_doc.m;
    let unit = 1.0 / (editor.camera.zoom * (a * d - b * c).abs().sqrt().max(1e-6));
    Some((id.clone(), tight(node), local, unit))
}

/// The rotation handle of the selection: where its stem starts on the selection box, and where the dot is.
/// One layer has it above the middle of its top side, turned with the layer; several have it above the middle
/// of the selection. `None` when nothing that can be turned is selected.
fn rotate_dot(editor: &Editor, view: ScreenRect) -> Option<(Pos2, Pos2)> {
    if holds_locked(editor) {
        return None;
    }
    let cam = &editor.camera;
    match editor.selection.as_slice() {
        [] => None,
        [id] => {
            let node = editor.scene_node(id).filter(|_| matches!(editor.role(id), Role::Plain | Role::Component | Role::Instance))?;
            // A connector lies where its ends are and can't be turned.
            if node.node.kind == Kind::Connector {
                return None;
            }
            let frame = tight(node);
            let top = Point::new(frame.x + frame.w / 2.0, frame.y);
            let foot = screen(cam, view, node.to_doc.apply(top));
            // Outwards is away from the box's middle, whichever way the layer is turned or mirrored.
            let middle = screen(cam, view, node.to_doc.apply(frame.center()));
            let away = foot - middle;
            let away = if away.length() > 1e-3 { away.normalized() } else { egui::vec2(0.0, -1.0) };
            Some((foot, foot + away * ROTATE_DOT_GAP))
        }
        _ => {
            let b = editor.selection_bounds()?;
            let foot = screen(cam, view, Point::new(b.center().x, b.y));
            Some((foot, foot - egui::vec2(0.0, ROTATE_DOT_GAP)))
        }
    }
}

/// Whether `p` is where a press turns the selected layer: outside its box, near one of its corners.
fn rotate_zone(editor: &Editor, view: ScreenRect, p: Pos2) -> bool {
    if holds_locked(editor) {
        return false;
    }
    let Some((_, frame, local, unit)) = on_selection(editor, view, p) else { return false };
    let outside = !frame.contains(local);
    let grip = (HANDLE_SIZE / 2.0 + 3.0) * unit;
    frame.corners().iter().any(|c| {
        let (dx, dy) = ((local.x - c.x).abs(), (local.y - c.y).abs());
        outside && dx.hypot(dy) <= ROTATE_REACH * unit && (dx > grip || dy > grip)
    })
}

fn handle_at(editor: &Editor, view: ScreenRect, p: Pos2) -> Option<(String, Handle)> {
    if holds_locked(editor) {
        return None;
    }
    // Human: worked out in the layer's own coordinates, so a turned layer's handles are where they are drawn.
    let (id, frame, local, unit) = on_selection(editor, view, p)?;
    let grip = (HANDLE_SIZE / 2.0 + 3.0) * unit;
    let side = |at: f32, low: f32, high: f32, reach: f32| -> Option<i8> {
        if (at - low).abs() <= reach {
            Some(-1)
        } else if (at - high).abs() <= reach {
            Some(1)
        } else {
            None
        }
    };
    if let (Some(x), Some(y)) = (side(local.x, frame.x, frame.right(), grip), side(local.y, frame.y, frame.bottom(), grip)) {
        return Some((id, Handle { x, y }));
    }
    // Human: an edge has no handle; a press on the edge itself resizes by it.
    let edge = EDGE_GRAB * unit;
    let (across, down) = (local.x >= frame.x && local.x <= frame.right(), local.y >= frame.y && local.y <= frame.bottom());
    let handle = match (side(local.x, frame.x, frame.right(), edge), side(local.y, frame.y, frame.bottom(), edge)) {
        (Some(x), _) if down => Handle { x, y: 0 },
        (_, Some(y)) if across => Handle { x: 0, y },
        _ => return None,
    };
    Some((id, handle))
}

/// The transform of the parent of the document node `id` (identity at the top level).
pub(crate) fn parent_transform(editor: &Editor, id: &str) -> Affine {
    edit::slot(&editor.doc, id).and_then(|s| s.parent).and_then(|p| editor.scene_node(&p)).map_or(Affine::IDENTITY, |p| p.to_doc)
}

/// A document-space vector in a parent's coordinates (only the linear part of the transform).
fn linear_inverse(parent: &Affine, dx: f32, dy: f32) -> (f32, f32) {
    parent.invert().map_or((dx, dy), |inv| linear(&inv, dx, dy))
}

#[expect(clippy::many_single_char_names, reason = "a, b, c, d are the matrix entries' standard names")]
fn linear(t: &Affine, dx: f32, dy: f32) -> (f32, f32) {
    let [a, b, c, d, _, _] = t.m;
    (a * dx + c * dy, b * dx + d * dy)
}

/// The layers under `p`, outermost first, by scene id: an instance is followed by the layers inside it.
// Human: a connector is hit on its line, within `reach` of it (in the document's units), never on the box
// around it, which would cover the layers it runs between. With a reach of 0 connectors are passed over.
pub(crate) fn hit_chain(scene: &[SceneNode], p: Point, vars: Vars<'_>, reach: f32) -> Vec<String> {
    hit(scene, p, vars, reach, false)
}

/// [`hit_chain`] for picking a layer up: a locked layer, with everything in it, is not there.
fn hit_free(scene: &[SceneNode], p: Point, vars: Vars<'_>, reach: f32) -> Vec<String> {
    hit(scene, p, vars, reach, true)
}

fn hit(scene: &[SceneNode], p: Point, vars: Vars<'_>, reach: f32, free: bool) -> Vec<String> {
    fn visit(node: &SceneNode, p: Point, vars: Vars<'_>, reach: f32, free: bool) -> Option<Vec<String>> {
        if node.node.enabled.as_ref().and_then(|e| vars.flag(e, &node.theme)) == Some(false) || (free && crate::ops::is_locked(&node.node)) {
            return None;
        }
        if node.node.kind == Kind::Connector {
            // The box is looked at first: reading the line is work, and this runs for every connector on
            // every frame the pointer is over the canvas.
            let b = layout::doc_bounds(node);
            let close = p.x >= b.x - reach && p.x <= b.right() + reach && p.y >= b.y - reach && p.y <= b.bottom() + reach;
            return (reach > 0.0 && close && crate::wire_edit::near(node, p, reach)).then(|| vec![node.id.clone()]);
        }
        let local = layout::to_local(node, p);
        let inside = local.is_some_and(|l| l.x >= 0.0 && l.y >= 0.0 && l.x <= node.local.w && l.y <= node.local.h);
        // Human: an ellipse or polygon is hit on its shape, not on the empty corners of its box.
        let on_shape = inside && shape_points(node).is_none_or(|points| local.is_some_and(|l| inside_outline(&points, l)));
        let clips = node.node.clip.as_ref().and_then(|c| vars.flag(c, &node.theme)).unwrap_or(false);
        if !clips || inside {
            for child in node.children.iter().rev() {
                if let Some(mut chain) = visit(child, p, vars, reach, free) {
                    chain.insert(0, node.id.clone());
                    return Some(chain);
                }
            }
        }
        if on_shape && node.node.kind != Kind::Group {
            return Some(vec![node.id.clone()]);
        }
        None
    }
    for node in scene.iter().rev() {
        if let Some(chain) = visit(node, p, vars, reach, free) {
            return chain;
        }
    }
    Vec::new()
}

/// What a click at `p` selects.
// Human: the rules:
// - ⌘ picks the deepest layer.
// - A top-level frame passes the click to the child under the pointer; its empty background picks it only when it
//   has a flex layout or no children (a free board is picked by its name label). A board that is selected is
//   picked itself wherever one of its layers lies, so a drag there moves the whole board.
// - A nested frame, a group and a component instance are picked whole, also while they are selected: a press on
//   a selected one takes all of it, and a drag moves all of it. Only a double click steps inside (the pointer's
//   double-click rule selects the layer under it, one level down).
// - Once a layer inside one of them is selected, a click on another layer inside it picks that layer: the user
//   has stepped in, and stays in until something outside is clicked.
// - Anything else is picked itself.
// Agent: `own` is "this container is selected itself", `inside` "a layer somewhere in it is". The first keeps the
// click on the container; only the second lets it through.
fn pick(editor: &Editor, p: Point, deep: bool) -> Option<String> {
    let chain = hit_free(&editor.scene, p, Vars::new(&editor.doc), WIRE_REACH / editor.camera.zoom);
    if deep {
        return chain.last().cloned();
    }
    let mut i = 0;
    while let Some(id) = chain.get(i) {
        let scene = editor.scene_node(id)?;
        let instance = editor.role(id) == Role::Instance;
        let own = editor.selection.contains(id);
        let mut inside = false;
        scene.walk(&mut |inner, depth| inside |= depth > 0 && editor.selection.contains(&inner.id), 0);
        let entered = inside && !own;
        let deeper = chain.get(i + 1).is_some();
        match scene.node.kind {
            Kind::Frame if i == 0 && !instance => {
                if deeper && own {
                    return Some(id.clone());
                } else if deeper {
                    i += 1;
                } else {
                    let background = scene.node.layout_mode() != LayoutMode::None || scene.children.is_empty();
                    return background.then(|| id.clone());
                }
            }
            _ if instance && entered && deeper => i += 1,
            Kind::Group | Kind::Frame if !instance && entered && deeper => i += 1,
            _ => return Some(id.clone()),
        }
    }
    None
}

/// Whether a locked layer is among the selected ones: the selection then has no handles and isn't turned.
fn holds_locked(editor: &Editor) -> bool {
    editor.selection.iter().any(|id| editor.locked(id))
}

/// The scene nodes a box selection picks from: the selection's siblings, or the top level.
fn selectable_level(editor: &Editor) -> Vec<&SceneNode> {
    let parent = editor.selection.first().and_then(|id| edit::slot(&editor.doc, id)).and_then(|s| s.parent);
    match parent.as_deref().and_then(|p| editor.scene_node(p)) {
        Some(parent) => parent.children.iter().collect(),
        None => editor.scene.iter().collect(),
    }
}

fn start_move(editor: &mut Editor, id: &str, at: Point, alt: bool) -> Drag {
    // A layer inside an instance is placed by its component and isn't dragged about; a connector lies where
    // its ends are.
    let wire = |id: &str| edit::find(&editor.doc, id).is_some_and(|node| node.kind == Kind::Connector);
    if editor.role(id) == Role::Inside || wire(id) || editor.locked(id) {
        return Drag::Move { start: at, origins: Vec::new(), moved: false, bounds: None, lifted: Vec::new(), slot: None };
    }
    // Agent: the scene knows the layers as they are now; a duplicate's copies aren't in it until the next frame.
    // The copies lie on their originals, so the originals' box is theirs; where they don't (a lone board's
    // copy goes beside it), the box isn't known and the drag doesn't snap.
    let before = movable(editor);
    let boxes: Option<Vec<Rect>> = before.iter().map(|(id, ..)| editor.scene_node(id).map(layout::doc_bounds)).collect();
    if alt {
        // Human: ⌥-drag drags off a duplicate.
        editor.duplicate();
    }
    let origins = movable(editor);
    let same = before.len() == origins.len() && before.iter().zip(&origins).all(|(a, b)| (a.1, a.2) == (b.1, b.2));
    let bounds = boxes.filter(|_| same).and_then(|boxes| boxes.into_iter().reduce(|a, b| a.union(&b)));
    Drag::Move { start: at, origins, moved: alt, bounds, lifted: Vec::new(), slot: None }
}

/// The selected layers a drag moves, each with the place it is moved from: its stored place, or for a layer
/// its parent's layout places, the place that layout gave it. Locked layers and connectors stay.
// Human: a layer in a layout is moved too: the drag takes it out of the layout (`lift`) and it stays where it is
// let go. It was only put in another place among its siblings before, which read as "it can't be moved".
// Agent: the place a layout gave is read from the scene; a layer the scene doesn't have yet (the copy of a
// duplicate made this frame) is left out, so a copy of a layer in a layout stays where the layout puts it.
fn movable(editor: &Editor) -> Vec<(String, f32, f32)> {
    let place = |id: &String| {
        let node = edit::find(&editor.doc, id).filter(|node| node.kind != Kind::Connector && !editor.locked(id))?;
        let (x, y) = if is_free(editor, id) { stored_position(node) } else { laid_out_position(editor.scene_node(id)?) };
        Some((id.clone(), x, y))
    };
    editor.selection.iter().filter_map(place).collect()
}

/// The place to store for a layer so that it lies, placed by its own x and y, where its parent's layout put it.
// Agent: a group's box starts at its top-left child, not at its stored place; the difference is taken off.
fn laid_out_position(scene: &SceneNode) -> (f32, f32) {
    fn group_origin(scene: &SceneNode) -> (f32, f32) {
        match scene.children.first().filter(|_| scene.node.kind == Kind::Group) {
            Some(child) => {
                let ((x, y), inner) = (stored_position(&child.node), group_origin(child));
                (x + inner.0 - child.local.x, y + inner.1 - child.local.y)
            }
            None => (0.0, 0.0),
        }
    }
    let origin = group_origin(scene);
    (scene.local.x - origin.0, scene.local.y - origin.1)
}

/// Takes the layers of `origins` that a layout places out of that layout, each to the place given there, and
/// returns what they had stored before.
// Human: such a layer becomes one placed by its own x and y (`layoutPosition: absolute`), so its siblings close
// up. One that was as wide or as high as the layout made it (`fill_container`) keeps that size as a number.
// Agent: WRITES editor.doc; the caller makes the undo step.
fn lift(editor: &mut Editor, origins: &[(String, f32, f32)]) -> Vec<Lifted> {
    let mut lifted = Vec::new();
    for (id, x, y) in origins {
        if is_free(editor, id) {
            continue;
        }
        let size = editor.scene_node(id).map(|scene| (scene.local.w, scene.local.h));
        let Some(node) = edit::find_mut(&mut editor.doc, id) else { continue };
        lifted.push(Lifted { id: id.clone(), x: node.x.clone(), y: node.y.clone(), width: node.width.clone(), height: node.height.clone() });
        node.layout_position = Some("absolute".to_owned());
        set_position(node, *x, *y);
        let fills = |n: &Option<Num>| matches!(n, Some(Num::Text(text)) if text.starts_with("fill_container"));
        if let Some((w, h)) = size {
            if fills(&node.width) {
                node.width = Some(Num::new(w));
            }
            if fills(&node.height) {
                node.height = Some(Num::new(h));
            }
        }
    }
    lifted
}

/// The parent of the layer with `id`, when it has a layout.
fn flex_parent(editor: &Editor, id: &str) -> Option<String> {
    let parent = edit::slot(&editor.doc, id)?.parent?;
    edit::find(&editor.doc, &parent).is_some_and(|node| node.layout_mode() != LayoutMode::None).then_some(parent)
}

/// What a dragged layer may line up with, as boxes in the document: the layers beside the selected ones that
/// are in view, and the frame that holds them.
// Human: a layer lines up with its neighbours and with the frame it is in (its edges and its middle), not with
// layers in other frames or deep inside its neighbours; those would catch it everywhere.
// Agent: worked out each frame of a drag from the scene; the selected layers are left out, so the frame's lag
// behind the document doesn't matter. Only what is in view counts, which also bounds the work on a large design.
fn snap_targets(editor: &Editor, view: ScreenRect) -> Vec<Rect> {
    let visible = crate::render::view_to_doc(&editor.camera, view);
    let parent = editor.selection.first().and_then(|id| edit::slot(&editor.doc, id)).and_then(|s| s.parent);
    let mut targets: Vec<Rect> = selectable_level(editor)
        .into_iter()
        .filter(|node| node.node.kind != Kind::Connector && !node.doc_id.as_ref().is_some_and(|id| editor.selection.contains(id)))
        .map(layout::doc_bounds)
        .filter(|bounds| bounds.intersects(&visible))
        .collect();
    targets.extend(parent.as_deref().and_then(|p| editor.scene_node(p)).map(layout::doc_bounds));
    targets
}

/// Whether a layer's sides run along the document's: neither turned nor mirrored nor scaled, nor in such a parent.
fn upright(to_doc: &Affine) -> bool {
    let ((a, b), (c, d)) = (linear(to_doc, 1.0, 0.0), linear(to_doc, 0.0, 1.0));
    (a - 1.0).abs() < 1e-4 && b.abs() < 1e-4 && c.abs() < 1e-4 && (d - 1.0).abs() < 1e-4
}

/// The index among a flex parent's children where the dragged node would land.
fn reorder_target(editor: &Editor, parent: &str, id: &str, at: Point) -> Option<usize> {
    let scene = editor.scene_node(parent)?;
    let node = edit::find(&editor.doc, parent)?;
    let row = node.layout_mode() == LayoutMode::Horizontal;
    let local = layout::to_local(scene, at)?;
    let mut target = 0;
    for (i, child) in scene.children.iter().enumerate() {
        if child.doc_id.as_deref() == Some(id) || child.node.is_absolute() {
            continue;
        }
        let center = if row { child.local.center().x } else { child.local.center().y };
        if (if row { local.x } else { local.y }) > center {
            target = i + 1;
        }
    }
    Some(target)
}

/// The insertion line for a reorder, in document coordinates.
fn reorder_line(editor: &Editor, parent: &str, target: usize) -> Option<(Point, Point)> {
    let scene = editor.scene_node(parent)?;
    let row = edit::find(&editor.doc, parent)?.layout_mode() == LayoutMode::Horizontal;
    let edge = match (scene.children.get(target), target.checked_sub(1).and_then(|i| scene.children.get(i))) {
        (Some(next), _) => {
            if row {
                next.local.x
            } else {
                next.local.y
            }
        }
        (None, Some(prev)) => {
            if row {
                prev.local.right()
            } else {
                prev.local.bottom()
            }
        }
        (None, None) => 0.0,
    };
    let (a, b) =
        if row { (Point::new(edge, 0.0), Point::new(edge, scene.local.h)) } else { (Point::new(0.0, edge), Point::new(scene.local.w, edge)) };
    Some((scene.to_doc.apply(a), scene.to_doc.apply(b)))
}

/// Puts the layer with `id` into the layout of `parent`, as the child at `target`; `lifted` is what this drag
/// took out of layouts, whose place and size as stored before come back.
// Agent: WRITES editor.doc; the drag has made the undo step.
fn drop_in_flex(editor: &mut Editor, id: &str, parent: &str, target: usize, lifted: &[Lifted]) {
    let Some(slot) = edit::slot(&editor.doc, id) else { return };
    if let Some(node) = edit::find_mut(&mut editor.doc, id) {
        node.layout_position = None;
        if let Some(was) = lifted.iter().find(|was| was.id == id) {
            (node.x, node.y) = (was.x.clone(), was.y.clone());
            (node.width, node.height) = (was.width.clone(), was.height.clone());
        }
    }
    // `target` counts the scene's children, which are the document's children in the same order.
    let index = if target > slot.index { target - 1 } else { target };
    if index != slot.index
        && let Some((node, _)) = edit::remove(&mut editor.doc, id)
    {
        edit::insert(&mut editor.doc, Some(parent), index, node);
    }
    editor.touched();
}

/// Resizes a node from its original box by a pointer movement (parent coordinates).
fn resize(editor: &mut Editor, id: &str, handle: Handle, original: Rect, pull: (f32, f32), keep_ratio: bool, from_local: &Affine) {
    let (dx, dy) = pull;
    // Human: the handles sit on the selection box, which for a polygon is smaller than the layer's own box (see
    // `tight`). The pull is applied to the selection box, and the layer's box follows in proportion, so the
    // handle stays under the pointer and the opposite side stays where it is.
    let part = editor.scene_node(id).map_or(Rect::new(0.0, 0.0, 1.0, 1.0), fitted_part);
    let shown = Rect::new(original.x + part.x * original.w, original.y + part.y * original.h, part.w * original.w, part.h * original.h);
    let mut pulled = shown;
    match handle.x {
        -1 => {
            pulled.x = shown.x + dx.min(shown.w);
            pulled.w = (shown.w - dx).max(0.0);
        }
        1 => pulled.w = (shown.w + dx).max(0.0),
        _ => {}
    }
    match handle.y {
        -1 => {
            pulled.y = shown.y + dy.min(shown.h);
            pulled.h = (shown.h - dy).max(0.0);
        }
        1 => pulled.h = (shown.h + dy).max(0.0),
        _ => {}
    }
    if keep_ratio && shown.w > 0.0 && shown.h > 0.0 {
        let ratio = shown.w / shown.h;
        if handle.x == 0 {
            pulled.w = pulled.h * ratio;
        } else {
            pulled.h = pulled.w / ratio;
        }
    }
    let (w, h) = (pulled.w / part.w, pulled.h / part.h);
    let r = Rect::new(pulled.x - part.x * w, pulled.y - part.y * h, w, h);
    let free = is_free(editor, id);
    let Some(node) = edit::find_mut(&mut editor.doc, id) else { return };
    if handle.x != 0 {
        node.width = Some(Num::new(r.w));
    }
    if handle.y != 0 {
        node.height = Some(Num::new(r.h));
    }
    if node.kind == Kind::Text {
        // Human: resizing text fixes its width (and height when dragged vertically), as the growth modes say.
        let growth = if handle.y != 0 { "fixed-width-height" } else { "fixed-width" };
        if node.text_growth.as_deref() != Some("fixed-width-height") {
            node.text_growth = Some(growth.to_owned());
        }
        if node.width.is_none() {
            node.width = Some(Num::new(r.w));
        }
        if growth == "fixed-width-height" && node.height.is_none() {
            node.height = Some(Num::new(r.h));
        }
    }
    if free {
        // Human: the corner moved along the layer's own sides; its stored place is in the parent's.
        let (sx, sy) = stored_position(node);
        let (px, py) = linear(from_local, r.x - original.x, r.y - original.y);
        set_position(node, sx + px, sy + py);
    }
    editor.touched();
}

/// `p` on the pixel grid when `snap` (on by default), else `p` itself.
fn snapped(p: Point, snap: bool) -> Point {
    if snap { Point::new(p.x.round(), p.y.round()) } else { p }
}

/// Puts `node` into the frame under `at` (or the top level), at `at` in that frame's coordinates, and selects it.
fn insert_at(editor: &mut Editor, mut node: Node, at: Point, centre: Option<(f32, f32)>) -> String {
    let parent = container_at(editor, at);
    let to_local = parent.as_deref().and_then(|p| editor.scene_node(p)).and_then(|p| p.to_doc.invert()).unwrap_or(Affine::IDENTITY);
    let local = to_local.apply(at);
    let (dx, dy) = centre.map_or((0.0, 0.0), |(w, h)| (w / 2.0, h / 2.0));
    set_position(&mut node, local.x - dx, local.y - dy);
    let id = node.id.clone();
    editor.checkpoint();
    edit::insert(&mut editor.doc, parent.as_deref(), usize::MAX, node);
    editor.selection = vec![id.clone()];
    editor.tool = Tool::Move;
    id
}

/// Outlines the layers find layers has found, the one its arrows are on more strongly.
fn spotlight(painter: &egui::Painter, editor: &Editor, view: ScreenRect) {
    let (marked, current) = &editor.spotlight;
    for id in marked {
        if let Some(node) = editor.scene_node(id) {
            let stroke = if current.as_ref() == Some(id) { Stroke::new(2.0, ACCENT) } else { Stroke::new(1.0, ACCENT.gamma_multiply(0.55)) };
            outline(painter, &editor.camera, view, node, stroke);
        }
    }
}

/// A press with the pen at `at`: closes the path when on its first point, else places a point (with `held`, ⇧,
/// on a 45° line from the last one). `true` when a point was placed, whose handles a pull then shapes.
// Human: closing ends that path and leaves the pen in hand for the next one. Every outline is its own layer.
fn pen_press(editor: &mut Editor, at: Point, held: bool, snap: bool) -> bool {
    if editor.pen.closes(at, PEN_REACH / editor.camera.zoom) {
        finish_pen(editor, true);
        return false;
    }
    if editor.pen.is_empty() {
        editor.selection.clear();
    }
    let to = snapped(editor.pen.place(at, held), snap);
    match editor.pen.press(to, 0.0) {
        crate::pen::Pressed::Placed => true,
        crate::pen::Pressed::Closes => {
            finish_pen(editor, true);
            false
        }
    }
}

/// Turns the path the pen has drawn into a layer in the frame under its first point, and selects it. Fewer
/// than two points make nothing.
fn finish_pen(editor: &mut Editor, closed: bool) {
    let anchors = editor.pen.take();
    let Some(first) = anchors.first() else { return };
    let parent = container_at(editor, first.at);
    let to_parent = parent.as_deref().and_then(|p| editor.scene_node(p)).and_then(|p| p.to_doc.invert()).unwrap_or(Affine::IDENTITY);
    let id = editor.fresh_id();
    let Some((mut node, origin)) = crate::pen::layer(&anchors, closed, &to_parent, id.clone()) else { return };
    set_position(&mut node, origin.x, origin.y);
    editor.checkpoint();
    edit::insert(&mut editor.doc, parent.as_deref(), usize::MAX, node);
    editor.selection = vec![id];
}

/// Draws the path the pen is drawing: its lines, the line the next click would add, the last point's handles
/// and every point's dot (the first one larger while a click would close the path).
fn pen_overlay(ui: &egui::Ui, painter: &egui::Painter, editor: &Editor, view: ScreenRect, pulling: bool, snap: bool) {
    let anchors = editor.pen.anchors();
    let (Some(first), Some(last)) = (anchors.first(), anchors.last()) else { return };
    let cam = &editor.camera;
    let place = |p: Point| screen(cam, view, p);
    let line = Stroke::new(1.0, ACCENT);
    for pair in anchors.windows(2) {
        crate::path_edit::stretch_line(painter, place(pair[0].at), pair[0].after.map(place), pair[1].before.map(place), place(pair[1].at), line);
    }
    let hover = if pulling { None } else { ui.input(|i| i.pointer.hover_pos()).filter(|p| view.contains(*p)) };
    let at = hover.map(|p| doc_point(cam, view, p));
    let closing = at.is_some_and(|p| editor.pen.closes(p, PEN_REACH / cam.zoom));
    if let Some(p) = at {
        let next = if closing { first.at } else { snapped(editor.pen.place(p, ui.input(|i| i.modifiers.shift)), snap) };
        crate::path_edit::stretch_line(
            painter,
            place(last.at),
            last.after.map(place),
            None,
            place(next),
            Stroke::new(1.0, ACCENT.gamma_multiply(0.6)),
        );
    }
    for handle in [last.before, last.after].into_iter().flatten().map(place) {
        painter.line_segment([place(last.at), handle], line);
        let half = PEN_HANDLE / 2.0;
        let diamond =
            vec![handle + egui::vec2(0.0, -half), handle + egui::vec2(half, 0.0), handle + egui::vec2(0.0, half), handle + egui::vec2(-half, 0.0)];
        painter.add(egui::Shape::convex_polygon(diamond, Color32::WHITE, line));
    }
    for (index, anchor) in anchors.iter().enumerate() {
        let radius = if index == 0 && closing { PEN_ANCHOR / 2.0 + 1.5 } else { PEN_ANCHOR / 2.0 };
        painter.circle(place(anchor.at), radius, Color32::WHITE, line);
    }
}

/// Adds a frame, rectangle or ellipse drawn over `area` (document coordinates) and selects it.
// Human: rectangles and ellipses are filled #CCCCCC; frames are white, clip their content,
// have no flex layout, a fixed size and the name "Frame N" (one more than the highest such name). Afterwards the
// tool goes back to Move and the new layer is selected; a new frame takes in the layers it covers.
fn create(editor: &mut Editor, tool: Tool, area: Rect) {
    let id = editor.fresh_id();
    let solid = |c: &str| Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid(c.to_owned())));
    let mut node = match tool {
        Tool::Frame => {
            let mut n = Node::new(Kind::Frame, id);
            n.name = Some(format!("Frame {}", crate::ops::next_frame_number(&editor.doc.children)));
            n.layout = Some("none".to_owned());
            n.clip = Some(pen_doc::value::Flag::Value(true));
            n.fill = solid(FRAME_FILL);
            n
        }
        Tool::Rectangle | Tool::Ellipse => {
            let mut n = Node::new(if tool == Tool::Rectangle { Kind::Rectangle } else { Kind::Ellipse }, id);
            n.fill = solid(SHAPE_FILL);
            n
        }
        Tool::Polygon => {
            let mut n = Node::new(Kind::Polygon, id);
            n.fill = solid(SHAPE_FILL);
            n.polygon_count = Some(Num::new(POLYGON_CORNERS));
            n
        }
        Tool::Text | Tool::Note | Tool::Icon | Tool::Move | Tool::Scale | Tool::Hand | Tool::Pen | Tool::Connector | Tool::Eyedropper => return,
    };
    node.width = Some(Num::new(area.w));
    node.height = Some(Num::new(area.h));
    let frame_id = insert_at(editor, node, Point::new(area.x, area.y), None);
    if tool == Tool::Frame {
        adopt_covered(editor, &frame_id, area);
    }
}

/// Moves the new frame's siblings that lie wholly inside it into it, keeping where they are on the canvas.
fn adopt_covered(editor: &mut Editor, frame_id: &str, area: Rect) {
    // Human: the laid-out scene still shows the canvas from before the frame was added: the layers' boxes there are
    // the ones to test.
    let Some(slot) = edit::slot(&editor.doc, frame_id) else { return };
    let siblings: Vec<String> = match slot.parent.as_deref().and_then(|p| editor.scene_node(p)) {
        Some(parent) => parent.children.iter().filter_map(|c| c.doc_id.clone()).collect(),
        None => editor.scene.iter().filter_map(|c| c.doc_id.clone()).collect(),
    };
    // Human: the frame and the layers share a parent, so a layer's position inside the frame is its position
    // minus the frame's.
    let (fx, fy) = edit::find(&editor.doc, frame_id).map_or((0.0, 0.0), stored_position);
    for id in siblings.into_iter().filter(|s| s != frame_id) {
        let Some(bounds) = editor.scene_node(&id).map(layout::doc_bounds) else { continue };
        let inside = bounds.x >= area.x && bounds.y >= area.y && bounds.right() <= area.right() && bounds.bottom() <= area.bottom();
        if inside && let Some((mut node, _)) = edit::remove(&mut editor.doc, &id) {
            let (x, y) = stored_position(&node);
            set_position(&mut node, x - fx, y - fy);
            edit::insert(&mut editor.doc, Some(frame_id), usize::MAX, node);
        }
    }
    editor.relayout();
}

/// Places an empty text with the text tool and returns its id for editing.
// Human: font size 16, empty content, and a fill of black or white, whichever stands out more against what is
// under the click: the solid fill of the top layer there, or the canvas background. A drag of at least
// 30 × 10 screen points sets its box.
fn create_text(editor: &mut Editor, at: Point, size: Option<(f32, f32)>, background: Color32) -> String {
    let under = color_under(editor, at).unwrap_or_else(|| {
        let [r, g, b, a] = background.to_srgba_unmultiplied();
        pen_doc::color::Rgba { r, g, b, a }
    });
    let ink = if under.contrast(pen_doc::color::Rgba::BLACK) > under.contrast(pen_doc::color::Rgba::WHITE) { "#000000" } else { "#ffffff" };
    let mut node = Node::new(Kind::Text, editor.fresh_id());
    node.content = Some(String::new());
    node.fill = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid(ink.to_owned())));
    node.font_size = Some(Num::new(TEXT_TOOL_FONT_SIZE));
    if let Some((w, h)) = size {
        node.width = Some(Num::new(w));
        node.height = Some(Num::new(h));
    }
    insert_at(editor, node, at, None)
}

/// Places an icon centred on the click, black or white by what is under it: a 24 × 24
/// Lucide "heart", coloured like new text.
fn create_icon(editor: &mut Editor, at: Point, background: Color32) {
    let under = color_under(editor, at).unwrap_or_else(|| {
        let [r, g, b, a] = background.to_srgba_unmultiplied();
        pen_doc::color::Rgba { r, g, b, a }
    });
    let ink = if under.contrast(pen_doc::color::Rgba::BLACK) > under.contrast(pen_doc::color::Rgba::WHITE) { "#000000" } else { "#ffffff" };
    let mut node = Node::new(Kind::Icon, editor.fresh_id());
    (node.library, node.icon) = (Some("lucide".to_owned()), Some("heart".to_owned()));
    (node.width, node.height) = (Some(Num::new(ICON_TOOL_SIZE)), Some(Num::new(ICON_TOOL_SIZE)));
    node.fill = Some(pen_doc::model::Paints::One(pen_doc::model::Paint::Solid(ink.to_owned())));
    insert_at(editor, node, at, Some((ICON_TOOL_SIZE, ICON_TOOL_SIZE)));
}

/// The solid fill of the top layer under `at`, if it has one.
fn color_under(editor: &Editor, at: Point) -> Option<pen_doc::color::Rgba> {
    let vars = Vars::new(&editor.doc);
    hit_chain(&editor.scene, at, vars, 0.0).iter().rev().find_map(|id| {
        let scene = editor.scene_node(id)?;
        match scene.node.fill.as_ref()?.as_slice().last()? {
            pen_doc::model::Paint::Solid(c) => vars.color(c, &scene.theme),
            pen_doc::model::Paint::Object(_) => None,
        }
    })
}

/// Places a sticky note centred on the click, in the frame under it, and returns its id for editing
/// (250 × 219 at `x - 250/2, y - 219/2`).
fn create_note(editor: &mut Editor, at: Point) -> String {
    let mut node = Node::new(Kind::Note, editor.fresh_id());
    node.width = Some(Num::new(NOTE_MIN_WIDTH));
    node.height = Some(Num::new(NOTE_MIN_HEIGHT));
    insert_at(editor, node, at, Some((NOTE_MIN_WIDTH, NOTE_MIN_HEIGHT)))
}

/// The deepest document frame under `p` that new nodes go into (none: the top level).
pub(crate) fn container_at(editor: &Editor, p: Point) -> Option<String> {
    let chain = hit_chain(&editor.scene, p, Vars::new(&editor.doc), 0.0);
    chain.into_iter().rev().find(|id| edit::find(&editor.doc, id).is_some_and(|n| n.kind == Kind::Frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The size of the window the pointer tests draw the canvas in; the canvas fills it.
    const WINDOW: egui::Vec2 = egui::vec2(1000.0, 800.0);

    /// Draws the canvas for one frame with `events`.
    fn frame(canvas: &mut Canvas, ctx: &egui::Context, editor: &mut Editor, events: Vec<egui::Event>) {
        held(canvas, ctx, editor, events, egui::Modifiers::NONE);
    }

    /// A frame of the canvas with `modifiers` held down.
    fn held(canvas: &mut Canvas, ctx: &egui::Context, editor: &mut Editor, mut events: Vec<egui::Event>, modifiers: egui::Modifiers) {
        events.insert(0, egui::Event::ModifiersChanged(modifiers));
        let input = egui::RawInput { screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, WINDOW)), events, focused: true, ..Default::default() };
        editor.refresh(&|_| (0.0, 0.0));
        let mut drawn = ctx.run_ui(input, |ui| canvas.show(ui, editor, false));
        // The font picture this frame made is for a screen; there is none here.
        drawn.textures_delta.clear();
    }

    /// A board at `x`, `y` on screen, `w` wide and 200 high, whose whole name is 90 by 14.
    fn named(x: f32, y: f32, w: f32, whole: bool) -> NamedBoard {
        NamedBoard { board: ScreenRect::from_min_size(egui::pos2(x, y), egui::vec2(w, 200.0)), whole, selected: false, text: egui::vec2(90.0, 14.0) }
    }

    /// The same board, selected.
    fn selected(x: f32, y: f32, w: f32) -> NamedBoard {
        NamedBoard { selected: true, ..named(x, y, w, true) }
    }

    #[test]
    fn a_boards_name_gets_its_boards_width_and_none_over_a_narrow_board() {
        let boards = [named(0.0, 100.0, 60.0, false), named(80.0, 100.0, 300.0, false), named(400.0, 100.0, 30.0, false)];
        let places = place_names(&boards, Covering::default());
        // Cut to the board's 60, just above it.
        assert_eq!(places[0], NamePlace::Cut(ScreenRect::from_min_size(egui::pos2(0.0, 100.0 - FRAME_NAME_GAP - 14.0), egui::vec2(60.0, 14.0))));
        // Room enough: the name's own width.
        assert!(matches!(places[1], NamePlace::Cut(text) if (text.width() - 90.0).abs() < 1e-3));
        assert_eq!(places[2], NamePlace::Hidden);
    }

    #[test]
    fn a_name_that_would_lie_on_another_board_or_under_the_size_label_is_left_out() {
        // The second board ends 10 above the first: less than a name's height.
        let stacked = place_names(&[named(0.0, 300.0, 100.0, false), named(0.0, 90.0, 100.0, false)], Covering::default());
        assert_eq!(stacked[0], NamePlace::Hidden);
        assert!(matches!(stacked[1], NamePlace::Cut(_)));
        // With room between them both names show.
        let apart = place_names(&[named(0.0, 330.0, 100.0, false), named(0.0, 90.0, 100.0, false)], Covering::default());
        assert!(apart.iter().all(|place| matches!(place, NamePlace::Cut(_))));
        let badge = Some(ScreenRect::from_min_size(egui::pos2(40.0, 75.0), egui::vec2(80.0, 20.0)));
        assert_eq!(place_names(&[named(0.0, 100.0, 100.0, false)], Covering { stem: None, badge }), [NamePlace::Hidden]);
    }

    #[test]
    fn the_board_under_the_pointer_keeps_its_whole_name_over_its_neighbours() {
        // 60 wide with a name of 90: it reaches over the neighbour's name, which is left out.
        let places =
            place_names(&[named(0.0, 100.0, 60.0, true), named(70.0, 100.0, 60.0, false), named(200.0, 100.0, 60.0, false)], Covering::default());
        let text = ScreenRect::from_min_size(egui::pos2(0.0, 82.0), egui::vec2(90.0, 14.0));
        assert_eq!(places[0], NamePlace::Whole { text, plate: true, pill: false });
        assert_eq!(places[1], NamePlace::Hidden);
        assert!(matches!(places[2], NamePlace::Cut(_)));
        // On a wide board it needs no plate.
        assert_eq!(place_names(&[named(0.0, 100.0, 300.0, true)], Covering::default()), [NamePlace::Whole { text, plate: false, pill: false }]);
    }

    #[test]
    fn a_selected_boards_name_is_a_pill_over_the_boards_middle_and_above_the_turning_handle() {
        // No handle on this board (several are selected): over the middle, where the name would stand.
        let text = ScreenRect::from_min_size(egui::pos2(105.0, 82.0), egui::vec2(90.0, 14.0));
        assert_eq!(place_names(&[selected(0.0, 100.0, 300.0)], Covering::default()), [NamePlace::Whole { text, plate: false, pill: true }]);
        // The handle stands on the middle of the board's top: the pill goes above it.
        let stem = Some(ScreenRect::from_min_size(egui::pos2(143.0, 67.0), egui::vec2(14.0, 33.0)));
        let lifted = text.translate(egui::vec2(0.0, 67.0 - FRAME_NAME_GAP - FRAME_NAME_PILL.y - 96.0));
        assert_eq!(
            place_names(&[selected(0.0, 100.0, 300.0)], Covering { stem, badge: None }),
            [NamePlace::Whole { text: lifted, plate: true, pill: true }]
        );
    }

    #[test]
    fn a_press_on_a_boards_name_takes_the_board_and_a_drag_from_it_moves_the_board() {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[{"type":"frame","id":"home","name":"Home","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#ffffff","children":[
            {"type":"rectangle","id":"a","x":0,"y":0,"width":400,"height":300,"fill":"#333333"}]}]}"##,
        )
        .unwrap();
        let (ctx, mut canvas, mut editor) = (egui::Context::default(), Canvas::default(), Editor::new(doc, None, 7));
        frame(&mut canvas, &ctx, &mut editor, Vec::new());
        // On the name, 10 points right of and above the board's corner; a click inside takes the layer there.
        let on_name = |editor: &Editor| Point::new(10.0 / editor.camera.zoom, -10.0 / editor.camera.zoom);
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(200.0, 150.0)]);
        assert_eq!(editor.selection, ["a"]);
        let at = on_name(&editor);
        drag(&mut canvas, &ctx, &mut editor, &[at]);
        assert_eq!(editor.selection, ["home"]);
        // Beside the name nothing is taken.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(at.x + 300.0, at.y)]);
        assert_eq!(editor.selection, Vec::<String>::new());
        // Pulled by its name, the board comes along: as far as the pointer goes on from where the pull was seen
        // to be one (the first step here), as with any layer.
        let at = on_name(&editor);
        drag(&mut canvas, &ctx, &mut editor, &[at, Point::new(at.x + 30.0, at.y), Point::new(at.x + 60.0, at.y + 20.0)]);
        let moved = &editor.doc.children[0];
        assert_eq!((editor.selection.clone(), stored_position(moved)), (vec!["home".to_owned()], (30.0, 20.0)));
    }

    /// Draws the canvas at `time` seconds and says where the name "Home" stands and what colour its letters have.
    fn name_at(canvas: &mut Canvas, ctx: &egui::Context, editor: &mut Editor, time: f64) -> (Pos2, Color32) {
        fn find(shape: &egui::Shape) -> Option<(Pos2, Color32)> {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().find_map(find),
                egui::Shape::Text(text) if text.galley.text() == "Home" => Some((text.pos, text.fallback_color)),
                _ => None,
            }
        }
        let input = egui::RawInput {
            screen_rect: Some(ScreenRect::from_min_size(Pos2::ZERO, WINDOW)),
            time: Some(time),
            focused: true,
            ..Default::default()
        };
        editor.refresh(&|_| (0.0, 0.0));
        let mut drawn = ctx.run_ui(input, |ui| canvas.show(ui, editor, false));
        // The font picture this frame made is for a screen; there is none here.
        drawn.textures_delta.clear();
        drawn.shapes.iter().find_map(|clipped| find(&clipped.shape)).unwrap_or((Pos2::ZERO, Color32::TRANSPARENT))
    }

    #[test]
    fn a_boards_name_goes_to_its_place_as_a_pill_and_back_over_a_moment() {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[{"type":"frame","id":"home","name":"Home","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#ffffff","children":[]}]}"##,
        )
        .unwrap();
        let (ctx, mut canvas, mut editor) = (egui::Context::default(), Canvas::default(), Editor::new(doc, None, 7));
        canvas.animations = true;
        let (rest, ink) = name_at(&mut canvas, &ctx, &mut editor, 1.0);
        // Selected: a third of the way in the name is between its two places and its two colours.
        editor.selection = vec!["home".to_owned()];
        name_at(&mut canvas, &ctx, &mut editor, 1.1);
        let (part, mixed) = name_at(&mut canvas, &ctx, &mut editor, 1.1 + f64::from(FRAME_NAME_MORPH) / 3.0);
        let (there, white) = name_at(&mut canvas, &ctx, &mut editor, 3.0);
        assert!(rest.x < part.x && part.x < there.x && there.y < part.y && part.y < rest.y, "{rest:?} {part:?} {there:?}");
        assert!(white == Color32::WHITE && mixed != ink && mixed != white, "{ink:?} {mixed:?} {white:?}");
        // Above the turning handle, over the board's middle.
        assert!(rest.y - there.y > ROTATE_DOT_GAP && name_at(&mut canvas, &ctx, &mut editor, 4.0) == (there, white));
        // Let go: the same way back.
        editor.selection.clear();
        name_at(&mut canvas, &ctx, &mut editor, 5.0);
        let (back, _) = name_at(&mut canvas, &ctx, &mut editor, 5.0 + f64::from(FRAME_NAME_MORPH) / 3.0);
        assert!(rest.y > back.y && back.y > there.y && name_at(&mut canvas, &ctx, &mut editor, 7.0) == (rest, ink));
        // With animations off the name is there at once.
        canvas.animations = false;
        editor.selection = vec!["home".to_owned()];
        assert_eq!(name_at(&mut canvas, &ctx, &mut editor, 8.0), (there, white));
    }

    /// Presses at the first of `points` (in the document), moves through the others and lets go at the last.
    fn drag(canvas: &mut Canvas, ctx: &egui::Context, editor: &mut Editor, points: &[Point]) {
        drag_holding(canvas, ctx, editor, points, egui::Modifiers::NONE);
    }

    /// [`drag`], with `modifiers` held from the first move after the press until the button is let go.
    fn drag_holding(canvas: &mut Canvas, ctx: &egui::Context, editor: &mut Editor, points: &[Point], modifiers: egui::Modifiers) {
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        let at = |editor: &Editor, p: Point| screen(&editor.camera, view, p);
        let button = |pos: Pos2, pressed: bool, modifiers: egui::Modifiers| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers,
        };
        let first = at(editor, points[0]);
        frame(canvas, ctx, editor, vec![egui::Event::PointerMoved(first)]);
        frame(canvas, ctx, editor, vec![button(first, true, egui::Modifiers::NONE)]);
        for point in &points[1..] {
            let here = at(editor, *point);
            held(canvas, ctx, editor, vec![egui::Event::PointerMoved(here)], modifiers);
        }
        let last = at(editor, points[points.len() - 1]);
        held(canvas, ctx, editor, vec![button(last, false, modifiers)], modifiers);
    }

    #[test]
    fn a_drag_takes_a_layer_out_of_its_parents_layout_and_ctrl_drops_it_back_in() {
        let json = r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":600,"height":520,"layout":"vertical","gap":20,"padding":20,"fill":"#ffffff","children":[
            {"type":"rectangle","id":"red","width":200,"height":100,"fill":"#ff0000"},
            {"type":"rectangle","id":"wide","width":"fill_container","height":100,"fill":"#00aa00"},
            {"type":"rectangle","id":"blue","width":200,"height":100,"fill":"#0000ff"}]}]}"##;
        let mut editor = Editor::new(serde_json::from_str::<pen_doc::Document>(json).unwrap(), None, 7);
        let (mut canvas, ctx) = (Canvas::default(), egui::Context::default());
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        editor.refresh(&|_| (0.0, 0.0));
        fit(&mut editor, view, Some(Rect::new(0.0, 0.0, 600.0, 520.0)), 1.0);
        let node = |editor: &Editor, id: &str| edit::find(&editor.doc, id).cloned().unwrap();
        let shown = |editor: &Editor, id: &str| editor.scene_node(id).map(|scene| scene.local).unwrap();
        let order = |editor: &Editor| editor.doc.children[0].kids().iter().map(|n| n.id.clone()).collect::<Vec<_>>();
        // A press alone changes nothing: the layer stays in the layout.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(120.0, 70.0)]);
        assert!(!node(&editor, "red").is_absolute() && !editor.dirty);
        // Dragged, it goes with the pointer and stays where it is let go, out of the layout; the others close up.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(120.0, 70.0), Point::new(130.0, 80.0), Point::new(420.0, 270.0)]);
        editor.refresh(&|_| (0.0, 0.0));
        let red = node(&editor, "red");
        assert!(red.is_absolute() && stored_position(&red) == (310.0, 210.0), "{:?} {:?}", red.layout_position, stored_position(&red));
        assert_eq!((shown(&editor, "red").x, shown(&editor, "red").y, shown(&editor, "wide").y), (310.0, 210.0, 20.0));
        assert_eq!(order(&editor), ["red", "wide", "blue"]);
        // A layer as wide as its parent's layout made it keeps that width as a number.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(300.0, 70.0), Point::new(300.0, 80.0), Point::new(300.0, 400.0)]);
        editor.refresh(&|_| (0.0, 0.0));
        let wide = node(&editor, "wide");
        assert!(wide.is_absolute() && wide.width == Some(Num::new(560.0)) && stored_position(&wide) == (20.0, 340.0), "{wide:?}");
        // One step back and it is in the layout again, as wide as the layout makes it.
        editor.undo();
        editor.refresh(&|_| (0.0, 0.0));
        let wide = node(&editor, "wide");
        assert!(!wide.is_absolute() && wide.width == Some(Num::Text("fill_container".to_owned())));
        // With ctrl held as it is let go, the layer goes back into the layout, where the line shows: after "blue".
        let ctrl = egui::Modifiers { ctrl: true, command: true, ..egui::Modifiers::NONE };
        drag_holding(&mut canvas, &ctx, &mut editor, &[Point::new(420.0, 270.0), Point::new(410.0, 280.0), Point::new(120.0, 400.0)], ctrl);
        editor.refresh(&|_| (0.0, 0.0));
        let red = node(&editor, "red");
        assert!(!red.is_absolute() && order(&editor) == ["wide", "blue", "red"], "{:?} {:?}", red.layout_position, order(&editor));
        assert_eq!((shown(&editor, "red").x, shown(&editor, "red").y), (20.0, 260.0));
    }

    #[test]
    fn a_connector_is_drawn_with_the_tool_and_edited_by_its_handles() {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":800,"height":600,"layout":"none","fill":"#ffffff","children":[
            {"type":"rectangle","id":"a","x":50,"y":100,"width":100,"height":60,"fill":"#333333"},
            {"type":"rectangle","id":"b","x":500,"y":100,"width":100,"height":60,"fill":"#333333"},
            {"type":"rectangle","id":"c","x":500,"y":400,"width":100,"height":60,"fill":"#333333"}]}]}"##,
        )
        .unwrap();
        let (ctx, mut canvas, mut editor) = (egui::Context::default(), Canvas::default(), Editor::new(doc, None, 7));
        frame(&mut canvas, &ctx, &mut editor, Vec::new());
        let wire = |editor: &Editor| editor.doc.children[0].kids().iter().find(|n| n.kind == Kind::Connector).cloned();
        let key = |editor: &Editor, key: &str| wire(editor).and_then(|w| w.extra.get(key).cloned());
        // The tool pulls a connector from the right edge of a to the left edge of b, and gives the pointer back.
        editor.tool = Tool::Connector;
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(145.0, 130.0), Point::new(300.0, 130.0), Point::new(505.0, 130.0)]);
        assert_eq!(key(&editor, "from"), Some(serde_json::json!({ "id": "a", "side": "right" })));
        assert_eq!(key(&editor, "to"), Some(serde_json::json!({ "id": "b", "side": "left" })));
        let id = wire(&editor).unwrap().id;
        assert_eq!((editor.selection.clone(), editor.tool, canvas.busy()), (vec![id.clone()], Tool::Move, false));
        // Its last end, at (500, 130), is dragged onto the top of c; a drag of the line itself moves nothing.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(500.0, 130.0), Point::new(520.0, 300.0), Point::new(550.0, 404.0)]);
        assert_eq!(key(&editor, "to"), Some(serde_json::json!({ "id": "c", "side": "top" })));
        // ... and off into the open, where it stays a free point.
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(550.0, 400.0), Point::new(400.0, 300.0), Point::new(300.0, 500.0)]);
        assert_eq!(key(&editor, "to"), Some(serde_json::json!({ "x": 300.0, "y": 500.0 })));
        // The dot in the middle of the line is pulled aside: a waypoint, which a second drag moves.
        frame(&mut canvas, &ctx, &mut editor, Vec::new());
        let line = crate::wire_edit::line(editor.scene_node(&id).unwrap());
        let lengths: Vec<f32> = line.windows(2).map(|ends| (ends[1].x - ends[0].x).hypot(ends[1].y - ends[0].y)).collect();
        let mut left = lengths.iter().sum::<f32>() / 2.0;
        let mut middle = line[0];
        for (ends, length) in line.windows(2).zip(&lengths) {
            if left <= *length {
                middle = Point::new(ends[0].x + (ends[1].x - ends[0].x) * left / length, ends[0].y + (ends[1].y - ends[0].y) * left / length);
                break;
            }
            left -= length;
        }
        drag(&mut canvas, &ctx, &mut editor, &[middle, Point::new(400.0, 250.0), Point::new(420.0, 220.0)]);
        assert_eq!(key(&editor, "via"), Some(serde_json::json!([{ "x": 420.0, "y": 220.0 }])));
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(420.0, 220.0), Point::new(430.0, 230.0), Point::new(440.0, 240.0)]);
        assert_eq!(key(&editor, "via"), Some(serde_json::json!([{ "x": 440.0, "y": 240.0 }])));
        // Each of those was one step to undo.
        editor.undo();
        assert_eq!(key(&editor, "via"), Some(serde_json::json!([{ "x": 420.0, "y": 220.0 }])));
        editor.undo();
        assert_eq!(key(&editor, "via"), None);
        // A double click on a waypoint takes it out again.
        frame(&mut canvas, &ctx, &mut editor, Vec::new());
        drag(&mut canvas, &ctx, &mut editor, &[middle, Point::new(400.0, 250.0), Point::new(420.0, 220.0)]);
        assert!(key(&editor, "via").is_some());
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        let on = screen(&editor.camera, view, Point::new(420.0, 220.0));
        for pressed in [true, false, true, false] {
            let click = egui::Event::PointerButton { pos: on, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
            frame(&mut canvas, &ctx, &mut editor, vec![click]);
        }
        assert_eq!((key(&editor, "via"), editor.selection.clone()), (None, vec![id]));
    }

    #[test]
    fn the_pen_draws_a_path_into_the_frame_under_its_first_point() {
        let doc: pen_doc::Document = serde_json::from_str(
            r##"{"version":"2.22","children":[{"type":"frame","id":"f","x":100,"y":200,"width":400,"height":300,"layout":"none","fill":"#ffffff"}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        editor.tool = Tool::Pen;
        let drawn = |editor: &Editor| editor.doc.children[0].children.clone().unwrap_or_default();
        // Three clicks, the third pulled into a curve; ⌘Z takes a fourth, mistaken point away again.
        assert!(pen_press(&mut editor, Point::new(120.2, 230.4), false, true));
        assert!(pen_press(&mut editor, Point::new(220.0, 230.0), false, true));
        assert!(pen_press(&mut editor, Point::new(220.0, 280.0), false, true));
        editor.pen.pull(Point::new(220.0, 300.0), false);
        assert!(pen_press(&mut editor, Point::new(400.0, 400.0), false, true));
        editor.undo();
        assert_eq!((editor.pen.anchors().len(), drawn(&editor).len()), (3, 0));
        // A click on the first point closes the path: one layer, in the frame, in the frame's coordinates.
        assert!(!pen_press(&mut editor, Point::new(121.0, 231.0), false, true));
        assert!(editor.pen.is_empty());
        let made = drawn(&editor);
        let path = &made[0];
        assert_eq!((&path.kind, stored_position(path)), (&Kind::Path, (20.0, 30.0)));
        assert_eq!(path.geometry.as_deref(), Some("M0 0 L100 0 C100 0 100 30 100 50 C100 70 0 0 0 0 Z"));
        assert_eq!((editor.selection.clone(), editor.tool), (vec![path.id.clone()], Tool::Pen));
        // One undo takes the whole path away.
        editor.undo();
        assert_eq!(drawn(&editor), Vec::new());
        // A single point makes no layer when the path is ended.
        pen_press(&mut editor, Point::new(150.0, 250.0), false, true);
        finish_pen(&mut editor, false);
        assert!(editor.pen.is_empty() && drawn(&editor).is_empty());
    }

    #[test]
    fn a_triangle_is_boxed_and_resized_by_its_own_corners() {
        let doc: pen_doc::Document = serde_json::from_str(
            r#"{"version":"2.22","children":[{"type":"polygon","id":"p","x":0,"y":0,"width":298,"height":298,"polygonCount":3}]}"#,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        // The selection box holds the triangle, not the quarter of the layer's box under its base.
        let shown = selection_box(editor.scene_node("p").unwrap());
        assert!(
            (shown.x - 19.96).abs() < 0.05 && shown.y.abs() < 0.05 && (shown.w - 258.07).abs() < 0.05 && (shown.h - 223.5).abs() < 0.05,
            "{shown:?}"
        );
        // Pulling the bottom right handle by 10 and 20 grows that box by as much and keeps its top left corner.
        resize(&mut editor, "p", Handle { x: 1, y: 1 }, Rect::new(0.0, 0.0, 298.0, 298.0), (10.0, 20.0), false, &Affine::IDENTITY);
        editor.refresh(&|_| (0.0, 0.0));
        let grown = selection_box(editor.scene_node("p").unwrap());
        assert!((grown.x - shown.x).abs() < 0.05 && (grown.y - shown.y).abs() < 0.05, "{grown:?}");
        assert!((grown.w - shown.w - 10.0).abs() < 0.05 && (grown.h - shown.h - 20.0).abs() < 0.05, "{grown:?}");
        // A rectangle's selection box is its own box.
        let mut plain = editor.scene_node("p").unwrap().clone();
        plain.node.kind = Kind::Rectangle;
        assert_eq!(tight(&plain), Rect::new(0.0, 0.0, plain.local.w, plain.local.h));
    }

    #[test]
    fn a_turned_layer_resizes_along_its_own_sides() {
        let doc: pen_doc::Document = serde_json::from_str(
            r#"{"version":"2.22","children":[{"type":"rectangle","id":"r","x":100,"y":100,"width":40,"height":30,"rotation":30}]}"#,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        let scene = editor.scene_node("r").unwrap().clone();
        let right_edge = |editor: &Editor| {
            let scene = editor.scene_node("r").unwrap();
            scene.to_doc.apply(Point::new(scene.local.w, 0.0))
        };
        let fixed = right_edge(&editor);
        // Pulling the left side 10 outwards (along the layer's own x) widens it and leaves the right side in place.
        resize(&mut editor, "r", Handle { x: -1, y: 0 }, scene.local, (-10.0, 0.0), false, &scene.to_doc);
        editor.refresh(&|_| (0.0, 0.0));
        let after = right_edge(&editor);
        assert_eq!(edit::find(&editor.doc, "r").unwrap().width.as_ref().and_then(Num::literal), Some(50.0));
        assert!((after.x - fixed.x).abs() < 1e-3 && (after.y - fixed.y).abs() < 1e-3, "{after:?} {fixed:?}");
        // Its handles are found on its turned corners, and just outside one the pointer turns it.
        editor.selection = vec!["r".to_owned()];
        let view = ScreenRect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let scene = editor.scene_node("r").unwrap();
        let corner = screen(&editor.camera, view, scene.to_doc.apply(Point::new(scene.local.w, scene.local.h)));
        assert_eq!(handle_at(&editor, view, corner).map(|(_, h)| (h.x, h.y)), Some((1, 1)));
        let outward = screen(&editor.camera, view, scene.to_doc.apply(Point::new(scene.local.w + 12.0, scene.local.h + 12.0)));
        assert!(rotate_zone(&editor, view, outward) && handle_at(&editor, view, outward).is_none());
        let middle = screen(&editor.camera, view, scene.to_doc.apply(Point::new(scene.local.w / 2.0, scene.local.h / 2.0)));
        assert!(!rotate_zone(&editor, view, middle));
        // The rotation dot stands off the middle of the top side, turned with the layer.
        let (foot, dot) = rotate_dot(&editor, view).unwrap();
        let top = screen(&editor.camera, view, scene.to_doc.apply(Point::new(scene.local.w / 2.0, 0.0)));
        assert!(foot.distance(top) < 1e-3 && (dot.distance(foot) - ROTATE_DOT_GAP).abs() < 1e-3);
        assert!(dot.distance(middle) > foot.distance(middle) && (dot.x - foot.x).abs() > 1.0);
        editor.selection.clear();
        assert!(rotate_dot(&editor, view).is_none());
    }

    #[test]
    fn a_moved_or_resized_layer_lines_up_with_its_neighbours_when_the_setting_is_on() {
        /// The line segments painted in the guide colour.
        fn lines(shape: &egui::Shape, into: &mut Vec<[Pos2; 2]>) {
            match shape {
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| lines(shape, into)),
                egui::Shape::LineSegment { points, stroke } if stroke.color == SNAP_GUIDE => into.push(*points),
                _ => {}
            }
        }
        let json = r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":800,"height":600,"layout":"none","fill":"#ffffff","children":[
            {"type":"rectangle","id":"a","x":100,"y":100,"width":100,"height":80,"fill":"#cccccc"},
            {"type":"rectangle","id":"b","x":400,"y":300,"width":60,"height":40,"fill":"#cccccc"}]}]}"##;
        let place = |editor: &Editor, id: &str| edit::find(&editor.doc, id).map(stored_position).unwrap();
        let open = || {
            let mut editor = Editor::new(serde_json::from_str::<pen_doc::Document>(json).unwrap(), None, 7);
            editor.refresh(&|_| (0.0, 0.0));
            fit(&mut editor, ScreenRect::from_min_size(Pos2::ZERO, WINDOW), Some(Rect::new(0.0, 0.0, 800.0, 600.0)), 1.0);
            editor.selection = vec!["b".to_owned()];
            editor
        };
        let ctx = egui::Context::default();
        // Dragged 297 left and 37 down, "b" would stand at 103 / 337: 3 off the left edge of "a", its top far from
        // everything. It lands on that edge and a line runs down it.
        let (mut canvas, mut editor) = (Canvas::default(), open());
        canvas.options.snap_to_objects = true;
        // A drag is known as one a few points after the press; from there the pointer goes 297 left and 37 down.
        let (from, known, to) = (Point::new(430.0, 320.0), Point::new(420.0, 320.0), Point::new(123.0, 357.0));
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        let at = |editor: &Editor, p: Point| screen(&editor.camera, view, p);
        let button = |pos: Pos2, pressed: bool| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let (first, second, last) = (at(&editor, from), at(&editor, known), at(&editor, to));
        frame(&mut canvas, &ctx, &mut editor, vec![egui::Event::PointerMoved(first)]);
        frame(&mut canvas, &ctx, &mut editor, vec![button(first, true)]);
        frame(&mut canvas, &ctx, &mut editor, vec![egui::Event::PointerMoved(second)]);
        frame(&mut canvas, &ctx, &mut editor, vec![egui::Event::PointerMoved(last)]);
        assert_eq!(canvas.guides.len(), 1, "{:?}", canvas.guides);
        assert!(canvas.guides[0].vertical && (canvas.guides[0].at - 100.0).abs() < 1e-3);
        // The line is painted in the guide colour down the shared edge, from above "a" to below "b".
        editor.refresh(&|_| (0.0, 0.0));
        let input = egui::RawInput { screen_rect: Some(view), events: vec![egui::Event::PointerMoved(last)], focused: true, ..Default::default() };
        let drawn = ctx.run_ui(input, |ui| canvas.show(ui, &mut editor, false));
        let mut painted = Vec::new();
        drawn.shapes.iter().for_each(|clipped| lines(&clipped.shape, &mut painted));
        let (top, foot) = (at(&editor, Point::new(100.0, 100.0)), at(&editor, Point::new(100.0, 377.0)));
        assert_eq!(painted.len(), 1, "{painted:?}");
        let [start, end] = painted[0];
        assert!((start.x - top.x).abs() < 1e-3 && (end.x - top.x).abs() < 1e-3, "{start:?} {end:?} {top:?}");
        assert!(
            (start.y - (top.y - SNAP_GUIDE_OVER)).abs() < 1e-3 && (end.y - (foot.y + SNAP_GUIDE_OVER)).abs() < 1e-3,
            "{start:?} {end:?} {top:?} {foot:?}"
        );
        frame(&mut canvas, &ctx, &mut editor, vec![button(last, false)]);
        let (x, y) = place(&editor, "b");
        assert!((x - 100.0).abs() < 1e-3 && (y - 337.0).abs() < 0.6, "{x} {y}");
        // The lines go when the layer is let go.
        frame(&mut canvas, &ctx, &mut editor, vec![]);
        assert_eq!(canvas.guides, []);
        // With the setting off the same drag leaves it where the pointer put it.
        let (mut canvas, mut editor) = (Canvas::default(), open());
        drag(&mut canvas, &ctx, &mut editor, &[from, known, to]);
        assert!((place(&editor, "b").0 - 103.0).abs() < 0.6, "{:?}", place(&editor, "b"));
        // Pulling the left side of "b" from 400 to 197 brings it to the right edge of "a" at 200; its right side stays.
        let (mut canvas, mut editor) = (Canvas::default(), open());
        canvas.options.snap_to_objects = true;
        drag(&mut canvas, &ctx, &mut editor, &[Point::new(400.0, 320.0), Point::new(390.0, 320.0), Point::new(187.0, 320.0)]);
        let width = edit::find(&editor.doc, "b").and_then(|node| node.width.as_ref().and_then(Num::literal)).unwrap();
        assert!((place(&editor, "b").0 - 200.0).abs() < 1e-3 && (width - 260.0).abs() < 1e-3, "{:?} {width}", place(&editor, "b"));
    }

    #[test]
    fn a_click_goes_through_a_locked_layer_and_the_eyedropper_takes_the_colour_under_it() {
        let json = r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":400,"height":300,"layout":"none","fill":"#ffffff","children":[
            {"type":"rectangle","id":"under","x":50,"y":50,"width":200,"height":100,"fill":"#00aa00"},
            {"type":"rectangle","id":"over","x":100,"y":70,"width":60,"height":40,"fill":"#ff0000","locked":true},
            {"type":"rectangle","id":"other","x":300,"y":200,"width":40,"height":40,"fill":"#0000ff"}]}]}"##;
        let mut editor = Editor::new(serde_json::from_str::<pen_doc::Document>(json).unwrap(), None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        let on_both = Point::new(120.0, 90.0);
        // The locked layer isn't picked, not even with the key that picks the deepest; the one under it is.
        assert_eq!(pick(&editor, on_both, false).as_deref(), Some("under"));
        assert_eq!(pick(&editor, on_both, true).as_deref(), Some("under"));
        // Selected from the layers list it has no handles and no turning handle.
        editor.selection = vec!["over".to_owned()];
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        let corner = screen(&editor.camera, view, Point::new(160.0, 110.0));
        assert!(handle_at(&editor, view, corner).is_none() && rotate_dot(&editor, view).is_none());
        // The eyedropper sees it all the same: its colour goes to the selected layer with a click, and the
        // pointer is back.
        let (mut canvas, ctx) = (Canvas::default(), egui::Context::default());
        fit(&mut editor, view, Some(Rect::new(0.0, 0.0, 400.0, 300.0)), 1.0);
        editor.selection = vec!["other".to_owned()];
        editor.tool = Tool::Eyedropper;
        let at = screen(&editor.camera, view, on_both);
        let button =
            |pressed: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        frame(&mut canvas, &ctx, &mut editor, vec![egui::Event::PointerMoved(at)]);
        frame(&mut canvas, &ctx, &mut editor, vec![button(true)]);
        frame(&mut canvas, &ctx, &mut editor, vec![button(false)]);
        let written = pen_doc::write(&editor.doc).unwrap();
        assert!(editor.tool == Tool::Move && written.matches("#ff0000").count() == 2, "{written}");
    }

    #[test]
    fn a_selected_container_is_taken_whole_and_only_a_double_click_steps_inside() {
        let json = r##"{"version":"2.22","children":[{"type":"frame","id":"board","x":0,"y":0,"width":600,"height":400,"layout":"none","fill":"#ffffff","children":[
            {"type":"group","id":"pair","x":100,"y":100,"children":[
                {"type":"rectangle","id":"left","x":0,"y":0,"width":80,"height":60,"fill":"#ff0000"},
                {"type":"rectangle","id":"right","x":120,"y":0,"width":80,"height":60,"fill":"#0000ff"}]}]}]}"##;
        let mut editor = Editor::new(serde_json::from_str::<pen_doc::Document>(json).unwrap(), None, 7);
        editor.refresh(&|_| (0.0, 0.0));
        let on_left = Point::new(140.0, 130.0);
        let picked = |editor: &Editor, p: Point| pick(editor, p, false);
        // A click on a layer of the group takes the group, and goes on taking it while the group is selected.
        assert_eq!(picked(&editor, on_left).as_deref(), Some("pair"));
        editor.selection = vec!["pair".to_owned()];
        assert_eq!(picked(&editor, on_left).as_deref(), Some("pair"));
        // A selected board is taken where its layers lie; its empty ground still lets go of it.
        editor.selection = vec!["board".to_owned()];
        assert_eq!(picked(&editor, on_left).as_deref(), Some("board"));
        assert_eq!(picked(&editor, Point::new(500.0, 350.0)), None);
        // A drag on a layer of the selected group moves the whole group, not the layer.
        let (mut canvas, ctx) = (Canvas::default(), egui::Context::default());
        let view = ScreenRect::from_min_size(Pos2::ZERO, WINDOW);
        fit(&mut editor, view, Some(Rect::new(0.0, 0.0, 600.0, 400.0)), 1.0);
        editor.selection = vec!["pair".to_owned()];
        drag(&mut canvas, &ctx, &mut editor, &[on_left, Point::new(150.0, 130.0), Point::new(190.0, 150.0)]);
        let place = |editor: &Editor, id: &str| edit::find(&editor.doc, id).map(stored_position).unwrap();
        assert_eq!((editor.selection.clone(), place(&editor, "pair"), place(&editor, "left")), (vec!["pair".to_owned()], (140.0, 120.0), (0.0, 0.0)));
        // A double click on a layer of the selected group steps in and selects that layer.
        editor.refresh(&|_| (0.0, 0.0));
        let at = screen(&editor.camera, view, Point::new(300.0, 150.0));
        let button =
            |pressed: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE };
        frame(&mut canvas, &ctx, &mut editor, vec![egui::Event::PointerMoved(at)]);
        for _ in 0..2 {
            frame(&mut canvas, &ctx, &mut editor, vec![button(true)]);
            frame(&mut canvas, &ctx, &mut editor, vec![button(false)]);
        }
        assert_eq!(editor.selection, vec!["right".to_owned()]);
        // Inside now, a click on the other layer picks that one.
        assert_eq!(picked(&editor, Point::new(180.0, 150.0)).as_deref(), Some("left"));
    }

    #[test]
    fn zoom_steps_snap_to_powers_of_two() {
        assert!((zoom_step(1.0, true) - 2.0).abs() < 1e-6);
        assert!((zoom_step(1.0, false) - 0.5).abs() < 1e-6);
        assert!((zoom_step(1.5, true) - 2.0).abs() < 1e-6);
        assert!((zoom_step(1.5, false) - 1.0).abs() < 1e-6);
        assert!((zoom_step(MAX_ZOOM, true) - MAX_ZOOM).abs() < 1e-6);
    }
}
