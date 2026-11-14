//! The tools an AI model works on a document with: what they are called, what they take, and what they do.
//!
//! Any model that can call tools can use these: they take and give plain JSON, and a `.pen` layer is written
//! as it is in the file. The tools come to the app through [`crate::link`]; this module is only what they mean.

use pen_doc::edit;
use pen_doc::layout;
use pen_doc::model::{Kind, Node};
use pen_doc::scene::SceneNode;
use serde_json::{Map, Value, json};

use crate::editor::{Editor, Role};

/// How deep `read_layers` goes into children unless asked otherwise.
const DEFAULT_DEPTH: u64 = 2;
/// Most layers `find` lists.
const MOST_FOUND: usize = 200;

/// What a model has to know to write layers: the `.pen` format in short.
// Human: this is the app's own summary of the format it reads and writes (see MECHANICS.md, "File format").
pub const GUIDE: &str = r##"# Working on a Pencraft document

A document is a tree of layers on an endless canvas. You read it with `read_layers`, change it with
`edit_layers`, and look at the result with `screenshot`. Call `get_state` first.

## Layers
Every layer is a JSON object with a `type` and an `id` (made for you when you leave it out; never contains `/`).
Common properties: `name`, `x`, `y`, `width`, `height`, `rotation` (degrees, counter-clockwise, around the top
left corner), `opacity` (0-1), `enabled` (false hides), `fill`, `stroke`, `strokeWidth`, `effect`.

Types:
- `frame`: a container. `children`, `layout` ("none" | "horizontal" | "vertical"; default horizontal), `gap`,
  `padding` (number, [vertical, horizontal] or [top, right, bottom, left]), `justifyContent` ("start" |
  "center" | "end" | "space_between" | "space_around"), `alignItems` ("start" | "center" | "end"),
  `cornerRadius` (number or four numbers), `clip` (true cuts children off at its edge).
- `group`: children without a box of its own.
- `rectangle` (`cornerRadius`), `ellipse` (`innerRadius` 0 to 1 makes it a ring; `startAngle` and
  `sweepAngle` in degrees, counter-clockwise from the right, make it an arc: a spinner, a progress ring),
  `polygon` (`polygonCount`, `cornerRadius`), `line`.
- `path`: `geometry` (SVG path data), `viewBox` [x, y, width, height], `fillRule`.
- `text`: `content`, `fontFamily` (a family installed on this computer, listed at the end of this guide; any
  other is drawn in a stand-in), `fontSize`, `fontWeight` ("100" to "900"; the family's nearest weight is
  drawn), `fontStyle` ("italic"), `underline` and `strikethrough` (true), `lineHeight` (a multiple of the
  font size), `letterSpacing`, `textAlign`, `textAlignVertical` ("top" | "middle" | "bottom", in a text with
  a height of its own), `textGrowth` ("auto": one line that grows; "fixed-width": wraps at `width`;
  "fixed-width-height"). The text colour is its `fill`.
- `icon`: `library` ("lucide"), `icon` (the icon's name), sized by `width` and `height`; coloured by `fill`.
  Only the Lucide set is there, all of it, under its current names; among them `check`, `x`, `chevron-down`, `chevron-right`,
  `chevron-up`, `ellipsis`, `search`, `loader-circle`, `panel-left`, `circle-check`, `info`,
  `triangle-exclamation-point`, `octagon-x`, `plus`, `house`, `trash`. A name that doesn't exist is drawn as a small
  empty box, and `edit_layers` answers with the nearest name.
- `note`: a sticky note with `content` and `color`.
- `ref`: an instance of a component (see below).

## Size and position
`width` and `height` are numbers, or "fill_container" (take the room the parent's layout gives) or
"fit_content" (a frame as large as its children; a frame's default). A layer inside a frame with a `layout` is
placed by that layout: leave `x` and `y` out there. Anywhere else give both `x` and `y`. Prefer layouts over
hand-placed positions; they keep spacing even. A child with `"layoutPosition": "absolute"` leaves its frame's
layout and is placed by its own `x` and `y` inside the frame: a badge on an avatar's corner, a close button on
a dialog, a menu drawn over its page.

## Fill and stroke
A colour is `"#rrggbb"` or `"#rrggbbaa"`. `fill` and `stroke` take one paint or a list (first is at the back).
A paint is a colour, or an object: `{"type":"gradient","gradientType":"linear"|"radial"|"angular",
"rotation":deg,"colors":[{"color":"#..","position":0..1}]}` or `{"type":"image","url":"./picture.png",
"mode":"cover"|"contain"|"stretch"}` (the `url` is a path beside the document or a full path like
`C:/pictures/a.jpg`). A linear gradient with `rotation` 0 runs upward (position 0 at the bottom, 1 at the
top); 180 runs downward. `strokeWidth` is a number or `{"top":..,"right":..,"bottom":..,"left":..}`;
`strokeAlignment` is "inner" | "center" | "outer".

## Effects
`effect` is one effect or a list: `{"type":"shadow","shadowType":"outer"|"inner","color":"#00000040",
"offset":{"x":0,"y":4},"blur":8}`, `{"type":"blur","radius":8}` (blurs the layer),
`{"type":"background_blur","radius":16}` (blurs what is behind it).

## Materials
`material` makes a frame, rectangle or ellipse out of glass: what is behind the layer shows through it.
`"frosted"` blurs and tints it (`{"type":"frosted","thickness":"ultra_thin"|"thin"|"regular"|"thick"}`), for
panels inside the content. `"liquid_glass"` also bends it at the rim like thick glass
(`{"type":"liquid_glass","variant":"regular"|"clear"}`), for bars and controls that float above content. Both
take `appearance` ("auto": light or dark by what is behind it; "light"; "dark") and `tint` (a colour, for the
one primary action); an update replaces the whole `material`, so send all of it; glass takes `depth` ("control" | "bar" | "panel"; by its size without it). Leave `fill`
off such a layer; its children and stroke are drawn sharp on top. A frame with `"glassGroup": <spacing>`
draws its glass children as one piece that flows together where they are closer than the spacing.
A material only works with something behind it, and has rules of its own: read the `liquid-glass` skill
(`read_skill`) before using one, and run `check_glass` on the result.

## Variables and themes
The document can hold `variables` (`{"name":{"type":"color"|"number"|"string"|"boolean","value":...}}`); use one
anywhere a value goes as `"$name"`. Reuse the variables a document already has.
A document can have theme axes, `themes`: `{"Mode":["Light","Dark"]}` (the first value is the default; set them
with `set_variables`). A variable's value can then differ by theme: `"value":[{"value":"#ffffff"},
{"value":"#18181b","theme":{"Mode":"Dark"}}]`. A frame with `"theme":{"Mode":"Dark"}` shows everything in it in
that theme.

## Components
A layer with `"reusable": true` is a component. `{"type":"ref","ref":"<component id>"}` is an instance of it:
it shows the component and has no children of its own. To change an instance, put the changed properties on
the `ref` itself (for the component's root) or under `descendants`, keyed by the id of the layer inside the
component: `"descendants":{"labelId":{"content":"Cancel"}}`. An entry with a `type` replaces that layer; an
entry with `children` gives a frame inside the component content of its own (this is how a slot is filled).
A layer inside an instance is addressed as `instanceId/layerId` in `read_layers` and in an `update`.
Look for existing components before building the same thing twice.

## Working well
- Read before you write: `get_state`, then `read_layers` on the frame you will work in.
- Make one `edit_layers` call per meaningful step, with a `description`; a call is one undo step, and if one
  operation fails nothing of the call is kept.
- In one call a later operation can use the id a former one made, as `"$0"`, `"$1"` (the operation's place).
- After a change, take a `screenshot` of the frame and check it. With the picture comes a `review` of what
  Pencraft counted in those layers (text too faint to read, layers that don't fit their frame, a font that
  isn't installed, sizes and spacings off any scale, the marks of a generated design); put it right.
- For anything with a look of its own, follow "Designing well" below.
- Make what was asked for and nothing around it. A thing the user asks for (a dog, a logo, an icon, a button)
  is drawn as itself: no backdrop, board, card or coloured frame behind it, unless the user asks for one or
  the thing is one (a screen, a page, a poster and a card have a background of their own). To keep its parts
  together use a group, or a frame without a fill.
- Never give a layer the fill of what is behind it: a white card on a white board can't be seen. A board that
  only presents parts or components gets a background the parts don't use (a grey or a tint), and text never
  has the colour of its background. `edit_layers` answers with `advice` when it finds this; act on it.
- Nothing is written to disk until `save_document`.
"##;

/// The tools, as a tools list gives them: name, what it is for, and the JSON schema of what it takes.
#[must_use]
pub fn tools() -> Value {
    let document =
        json!({ "type": "string", "description": "The document to work on: its file path or its tab's name. Leave out for the document in front." });
    let ids = |what: &str| json!({ "type": "array", "items": { "type": "string" }, "description": what });
    json!([
        {
            "name": "get_state",
            "description": "What is open in Pencraft right now: the documents, and for the one in front its top-level layers with their boxes, the user's selection, its components and its variables. Call this first.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_guide",
            "description": "How a Pencraft document is built (the layer types and their properties, layouts, fills, effects, variables and components), how to design well in it, and which fonts are installed. Read it once before writing layers.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "list_skills",
            "description": "The skills Pencraft has: each one says how to design one kind of thing here (for example liquid glass and frosted interfaces), with when to use it and the files it has.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "read_skill",
            "description": "Read a skill before designing what it is for. Without `file` it gives the skill's main text; that text names further files (component layers, tokens, background) to read with `file` when you need them.",
            "inputSchema": { "type": "object", "required": ["name"], "properties": {
                "name": { "type": "string", "description": "The skill's name, as `list_skills` and the guide give it." },
                "file": { "type": "string", "description": "One of the skill's other files, such as `references/components.md`." }
            } }
        },
        {
            "name": "read_layers",
            "description": "Read layers as they are in the document, each with the box it has on the canvas (`layoutBox`). Without `ids` it reads the top-level layers. Children are given down to `depth`; deeper ones are only listed by id. With `find` it lists the layers whose name, text or type contains that text instead.",
            "inputSchema": { "type": "object", "properties": {
                "document": document,
                "ids": ids("The layers to read. An id like `instanceId/layerId` reads a layer inside a component instance, as that instance shows it."),
                "depth": { "type": "integer", "minimum": 0, "description": "How many levels of children to include (default 2)." },
                "find": { "type": "string", "description": "List the layers whose name, text or type contains this, instead of reading layers." }
            } }
        },
        {
            "name": "edit_layers",
            "description": "Change the document with a list of operations, applied in order as one undo step. If one fails, none is kept and the error says which. Returns each operation's result (the ids it made), the boxes of the layers it touched, and `advice` when Pencraft sees something wrong with the result (a layer in the colour of what is behind it, text that can't be read or is too faint, a layer that doesn't fit its frame, a font that isn't installed); put that right before going on. The user sees each touched layer flash on the canvas.",
            "inputSchema": { "type": "object", "required": ["operations"], "properties": {
                "document": document,
                "description": { "type": "string", "description": "A few words saying what this change does, shown to the user." },
                "operations": { "type": "array", "minItems": 1, "items": { "type": "object", "required": ["op"], "properties": {
                    "op": { "type": "string", "enum": ["insert", "update", "delete", "move", "copy", "replace", "set_variables"], "description": "insert: add `node` into `parent` (the top level without one) at `index` (on top without one). update: merge `properties` into the layer `id`; a property set to null is removed; for an instance or a layer inside one this writes an override. delete: remove the layer `id`. move: put the layer `id` into `parent` at `index`. copy: duplicate the layer `id` (into `parent` if given), then merge `properties` into the copy. replace: put `node` in the place of the layer `id`. set_variables: merge `variables` into the document's variables (null removes one), and `themes` into its theme axes." },
                    "id": { "type": "string", "description": "The layer the operation is about. `$0`, `$1`, ... stand for the id made by that earlier operation of this call." },
                    "parent": { "type": "string", "description": "The frame or group to put the layer in; `$0`-style references work here too." },
                    "index": { "type": "integer", "minimum": 0, "description": "The place among the parent's children, 0 at the back." },
                    "node": { "type": "object", "description": "A whole layer, with its children, as the guide describes. Ids are made for layers without one." },
                    "properties": { "type": "object", "description": "The properties to set." },
                    "variables": { "type": "object", "description": "Variables by name: {\"type\": \"color\" | \"number\" | \"string\" | \"boolean\", \"value\": ...}." },
                    "themes": { "type": "object", "description": "For set_variables: theme axes by name, each a list of its values with the default first, e.g. {\"Mode\": [\"Light\", \"Dark\"]}; null removes an axis." }
                } } }
            } }
        },
        {
            "name": "screenshot",
            "description": "A picture of layers as Pencraft draws them, to check your work. Without `ids` it shows every top-level layer. Several layers are shown together, as they lie on the canvas. With the picture comes a `review`: what Pencraft counted in those layers that is a mistake (unreadable text, layers that don't fit, a missing font) or marks the design as generated; put it right, judge the picture, and look again.",
            "inputSchema": { "type": "object", "properties": {
                "document": document,
                "ids": ids("The layers to show."),
                "scale": { "type": "number", "minimum": 0.1, "maximum": 3, "description": "Pixels per canvas unit (default 1). The picture is made smaller when it would be very large." }
            } }
        },
        {
            "name": "check_glass",
            "description": "Checks layers that use a `material` (frosted, liquid glass) by measuring a picture of them: which appearance each material came to (light or dark), the contrast of every text and icon on a material against the lightest and darkest of what is under it (at least 4.5:1), and how the layers are built (no glass on glass, nested corner radii, one glass variant per screen, something behind the glass to bend). Returns `problems`, each saying how to put it right; call it after `screenshot` on any design that has materials, and again after fixing.",
            "inputSchema": { "type": "object", "properties": { "document": document, "ids": ids("The layers to check, usually the screen's top-level frame; every top-level layer without it.") } }
        },
        {
            "name": "select_layers",
            "description": "Select layers for the user and bring them into view, to point at what you mean or what you made.",
            "inputSchema": { "type": "object", "required": ["ids"], "properties": { "document": document, "ids": ids("The layers to select; an empty list selects nothing.") } }
        },
        {
            "name": "open_document",
            "description": "Open a .pen file in a tab, or bring its tab to the front. Without `path` it starts a new, empty document.",
            "inputSchema": { "type": "object", "properties": { "path": { "type": "string", "description": "The file's full path." } } }
        },
        {
            "name": "save_document",
            "description": "Write the document to its file. A document that has no file yet needs `path`.",
            "inputSchema": { "type": "object", "properties": { "document": document, "path": { "type": "string", "description": "Save under this full path (ending in .pen) from now on." } } }
        },
        {
            "name": "export_layers",
            "description": "Export layers to a file: PNG, JPEG, WebP, PDF or SVG, picked by the path's ending. Several layers go into one file together; without `ids` every top-level layer does.",
            "inputSchema": { "type": "object", "required": ["path"], "properties": {
                "document": document,
                "ids": ids("The layers to export."),
                "path": { "type": "string", "description": "The file to write, with its ending (.png, .jpg, .webp, .pdf, .svg)." },
                "scale": { "type": "integer", "minimum": 1, "maximum": 3, "description": "Pixels per unit for PNG, JPEG and WebP (default 1)." }
            } }
        }
    ])
}

/// A layer's box on the canvas, in document coordinates.
fn box_of(scene: &SceneNode) -> Value {
    let b = layout::doc_bounds(scene);
    let round = |v: f32| (f64::from(v) * 100.0).round() / 100.0;
    json!({ "x": round(b.x), "y": round(b.y), "width": round(b.w), "height": round(b.h) })
}

/// The ids in `args[key]`, when it is a list of strings.
#[must_use]
pub fn ids(args: &Value, key: &str) -> Vec<String> {
    args.get(key).and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect()
}

/// What `get_state` says about one document.
#[must_use]
pub fn state(editor: &Editor) -> Value {
    fn components(nodes: &[Node], out: &mut Vec<Value>) {
        for node in nodes {
            if node.reusable == Some(true) {
                out.push(json!({ "id": node.id, "name": node.display_name() }));
            }
            components(node.kids(), out);
        }
    }
    let top: Vec<Value> = editor
        .scene
        .iter()
        .map(|scene| json!({ "id": scene.id, "type": kind_name(&scene.node.kind), "name": scene.node.display_name(), "layoutBox": box_of(scene), "children": scene.children.len() }))
        .collect();
    let mut reusable = Vec::new();
    components(&editor.doc.children, &mut reusable);
    json!({
        "file": editor.path.as_ref().map(|p| p.display().to_string()),
        "name": editor.title(),
        "unsavedChanges": editor.dirty,
        "selection": editor.selection,
        "topLevelLayers": top,
        "components": reusable,
        "variables": editor.doc.variables,
        "themes": editor.doc.themes,
    })
}

/// A layer kind's name in the file.
fn kind_name(kind: &Kind) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_owned)).unwrap_or_else(|| "layer".to_owned())
}

/// A scene node as the file would write it, with its box, and its children down to `depth` more levels.
fn describe(editor: &Editor, scene: &SceneNode, depth: u64) -> Value {
    // Human: a document layer is given as the document has it (an instance as its `ref`); a layer inside an
    // instance as the instance shows it, since it has no entry of its own in the file.
    let node = scene.doc_id.as_deref().and_then(|id| edit::find(&editor.doc, id)).unwrap_or(&scene.node);
    let mut value = serde_json::to_value(node).unwrap_or(Value::Null);
    let Some(object) = value.as_object_mut() else { return value };
    object.remove("children");
    object.insert("id".to_owned(), Value::String(scene.id.clone()));
    object.insert("layoutBox".to_owned(), box_of(scene));
    if !scene.children.is_empty() {
        if depth == 0 {
            object.insert("childrenNotShown".to_owned(), scene.children.iter().map(|c| Value::String(c.id.clone())).collect());
        } else {
            object.insert("children".to_owned(), scene.children.iter().map(|c| describe(editor, c, depth - 1)).collect());
        }
    }
    value
}

/// `read_layers`: the layers asked for (or the top level), or the layers a search finds.
///
/// # Errors
/// When a layer asked for isn't in the document.
pub fn read(editor: &Editor, args: &Value) -> Result<Value, String> {
    if let Some(wanted) = args.get("find").and_then(Value::as_str) {
        let wanted = wanted.to_lowercase();
        let mut found = Vec::new();
        for top in &editor.scene {
            top.walk(
                &mut |scene, _| {
                    let text = format!("{} {} {}", scene.node.display_name(), scene.node.content.as_deref().unwrap_or(""), kind_name(&scene.node.kind)).to_lowercase();
                    if text.contains(&wanted) && found.len() < MOST_FOUND {
                        found.push(json!({ "id": scene.id, "type": kind_name(&scene.node.kind), "name": scene.node.display_name(), "layoutBox": box_of(scene) }));
                    }
                },
                0,
            );
        }
        return Ok(json!({ "found": found }));
    }
    let depth = args.get("depth").and_then(Value::as_u64).unwrap_or(DEFAULT_DEPTH);
    let wanted = ids(args, "ids");
    let layers: Vec<Value> = if wanted.is_empty() {
        editor.scene.iter().map(|scene| describe(editor, scene, depth)).collect()
    } else {
        wanted
            .iter()
            .map(|id| {
                editor.scene_node(id).map(|scene| describe(editor, scene, depth)).ok_or_else(|| format!("There is no layer with the id \"{id}\"."))
            })
            .collect::<Result<_, _>>()?
    };
    Ok(json!({ "layers": layers }))
}

/// Rewrites a stroke written as one object, `{"thickness": 1, "fill": "#000", "align": "inside"}`, into the
/// `stroke`, `strokeWidth` and `strokeAlignment` the document has.
// Human: models often write a stroke this way (older design files did); taken as it stands it would be a paint
// of no known kind and draw nothing, without a word to the model.
fn plain_stroke(object: &mut Map<String, Value>) {
    let Some(Value::Object(stroke)) = object.get("stroke") else { return };
    if stroke.contains_key("type") || !(stroke.contains_key("thickness") || stroke.contains_key("align")) {
        return;
    }
    let stroke = stroke.clone();
    object.remove("stroke");
    if let Some(fill) = stroke.get("fill") {
        object.insert("stroke".to_owned(), fill.clone());
    }
    if let Some(width) = stroke.get("thickness") {
        object.insert("strokeWidth".to_owned(), width.clone());
    }
    let alignment = match stroke.get("align").and_then(Value::as_str) {
        Some("inside" | "inner") => Some("inner"),
        Some("outside" | "outer") => Some("outer"),
        Some("center") => Some("center"),
        _ => None,
    };
    if let Some(alignment) = alignment {
        object.insert("strokeAlignment".to_owned(), Value::String(alignment.to_owned()));
    }
}

/// Gives every layer of `node` that has no id (or an empty one) a fresh one.
fn give_ids(editor: &mut Editor, node: &mut Value, taken: &mut Vec<String>) {
    let Some(object) = node.as_object_mut() else { return };
    plain_stroke(object);
    let has = object.get("id").and_then(Value::as_str).is_some_and(|id| !id.is_empty());
    if !has {
        let mut id = editor.fresh_id();
        while taken.contains(&id) {
            id.push('x');
        }
        object.insert("id".to_owned(), Value::String(id));
    }
    if let Some(id) = object.get("id").and_then(Value::as_str) {
        taken.push(id.to_owned());
    }
    for child in object.get_mut("children").and_then(Value::as_array_mut).into_iter().flatten() {
        give_ids(editor, child, taken);
    }
    // Human: an instance's overrides can bring layers of their own (a filled slot, a replaced layer); those
    // need ids like any other layer, or the instance can't be built.
    for change in object.get_mut("descendants").and_then(Value::as_object_mut).into_iter().flat_map(|all| all.values_mut()) {
        if change.get("type").is_some() {
            give_ids(editor, change, taken);
        }
        for child in change.get_mut("children").and_then(Value::as_array_mut).into_iter().flatten() {
            give_ids(editor, child, taken);
        }
    }
}

/// A layer from the JSON a model wrote, with ids given where it left them out.
fn layer(editor: &mut Editor, node: Option<&Value>, taken: &mut Vec<String>) -> Result<Node, String> {
    let mut node = node.filter(|n| n.is_object()).cloned().ok_or("the operation needs a `node` (a layer object)")?;
    if node.get("type").and_then(Value::as_str).is_none() {
        return Err("the node needs a `type` (frame, text, rectangle, ...)".to_owned());
    }
    give_ids(editor, &mut node, taken);
    let layer: Node = serde_json::from_value(node).map_err(|e| format!("the node isn't a valid layer: {e}"))?;
    let mut clash = None;
    walk(&layer, &mut |n| {
        let problem =
            material_problem(Some(&n.kind), n.material.as_ref()).or_else(|| group_problem(Some(&n.kind), n.extra.get(crate::glass_check::GROUP)));
        if let Some(problem) = problem {
            clash = Some(format!("layer \"{}\": {problem}", n.id));
        } else if n.id.contains('/') {
            clash = Some(format!("the id \"{}\" contains a slash", n.id));
        } else if edit::find(&editor.doc, &n.id).is_some() {
            clash = Some(format!("the id \"{}\" is already used in the document", n.id));
        }
    });
    clash.map_or(Ok(layer), Err)
}

/// What is wrong with a `material` a model wrote for a layer of `kind` (`None` when the kind isn't known, as
/// for a layer inside an instance), said so that the model can put it right.
fn material_problem(kind: Option<&Kind>, material: Option<&Value>) -> Option<String> {
    let material = material.filter(|m| !m.is_null())?;
    if let Some(kind) = kind.filter(|kind| matches!(kind, Kind::Text | Kind::Icon | Kind::IconFont | Kind::Line | Kind::Group | Kind::Note)) {
        let name = kind_name(kind);
        return Some(format!("a {name} can't have a `material`; give the material to a frame, rectangle or ellipse and put the {name} in it"));
    }
    let problems = crate::material::problems(material);
    (!problems.is_empty()).then(|| problems.join("; "))
}

/// What is wrong with a `glassGroup` a model wrote, if anything.
fn group_problem(kind: Option<&Kind>, group: Option<&Value>) -> Option<String> {
    crate::glass_check::group_problem(kind, group?)
}

fn walk(node: &Node, visit: &mut impl FnMut(&Node)) {
    visit(node);
    for child in node.kids() {
        walk(child, visit);
    }
}

/// A place for a layer: the parent (`None` for the top level) and the index in it.
fn place(editor: &Editor, op: &Value, made: &[Option<String>]) -> Result<(Option<String>, usize), String> {
    let parent = text(op, "parent", made)?;
    if let Some(parent) = &parent {
        let node = edit::find(&editor.doc, parent).ok_or_else(|| format!("there is no layer with the id \"{parent}\" to put it in"))?;
        if !matches!(node.kind, Kind::Frame | Kind::Group) {
            return Err(format!("\"{parent}\" is a {}; layers go into a frame or a group", kind_name(&node.kind)));
        }
    }
    let index = op.get("index").and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).unwrap_or(usize::MAX);
    Ok((parent, index))
}

/// The text of `op[key]`, with a `$n` reference turned into the id operation `n` made.
fn text(op: &Value, key: &str, made: &[Option<String>]) -> Result<Option<String>, String> {
    let Some(value) = op.get(key).filter(|v| !v.is_null()) else { return Ok(None) };
    let text = value.as_str().ok_or_else(|| format!("`{key}` must be a text"))?;
    match text.strip_prefix('$').and_then(|n| n.parse::<usize>().ok()) {
        Some(n) => {
            made.get(n).cloned().flatten().map(Some).ok_or_else(|| format!("`{text}` names operation {n}, which made no layer before this one"))
        }
        None => Ok(Some(text.to_owned())),
    }
}

/// `properties` merged into the layer `id`: straight into a plain layer, as an override into an instance or a
/// layer inside one.
fn update(editor: &mut Editor, id: &str, properties: &Map<String, Value>) -> Result<(), String> {
    if properties.contains_key("id") || properties.contains_key("type") {
        return Err("an update can't change `id` or `type`; use `replace` for another kind of layer".to_owned());
    }
    let mut properties = properties.clone();
    plain_stroke(&mut properties);
    let properties = &properties;
    let kind = edit::find(&editor.doc, id).map(|node| node.kind.clone());
    let kind = kind.as_ref().filter(|kind| **kind != Kind::Ref);
    if let Some(problem) =
        material_problem(kind, properties.get("material")).or_else(|| group_problem(kind, properties.get(crate::glass_check::GROUP)))
    {
        return Err(problem);
    }
    match editor.role(id) {
        Role::Inside => {
            let (instance, key) = edit::owner(&editor.doc, id).ok_or_else(|| format!("there is no layer with the id \"{id}\""))?;
            edit::set_overrides(&mut editor.doc, &instance, Some(&key), properties);
        }
        Role::Instance => {
            edit::set_overrides(&mut editor.doc, id, None, properties);
        }
        Role::Plain | Role::Component => {
            let node = edit::find_mut(&mut editor.doc, id).ok_or_else(|| format!("there is no layer with the id \"{id}\""))?;
            let Ok(Value::Object(mut whole)) = serde_json::to_value(&*node) else { return Err("the layer couldn't be read".to_owned()) };
            for (key, value) in properties {
                if value.is_null() {
                    whole.remove(key);
                } else {
                    whole.insert(key.clone(), value.clone());
                }
            }
            *node = serde_json::from_value(Value::Object(whole)).map_err(|e| format!("the properties don't fit the layer: {e}"))?;
        }
    }
    Ok(())
}

/// One operation of `edit_layers`. Returns what it has to say, and the id it made or changed.
fn operate(editor: &mut Editor, op: &Value, made: &[Option<String>]) -> Result<(Value, Option<String>), String> {
    let kind = op.get("op").and_then(Value::as_str).ok_or("the operation needs an `op`")?;
    let id = || text(op, "id", made)?.ok_or_else(|| format!("`{kind}` needs an `id`"));
    let exists = |editor: &Editor, id: &str| edit::find(&editor.doc, id).map(|_| ()).ok_or_else(|| format!("there is no layer with the id \"{id}\""));
    let empty = Map::new();
    let properties = op.get("properties").and_then(Value::as_object).unwrap_or(&empty);
    match kind {
        "insert" => {
            let (parent, index) = place(editor, op, made)?;
            let node = layer(editor, op.get("node"), &mut Vec::new())?;
            let new = node.id.clone();
            edit::insert(&mut editor.doc, parent.as_deref(), index, node);
            Ok((json!({ "id": new }), Some(new)))
        }
        "update" => {
            let id = id()?;
            update(editor, &id, properties)?;
            Ok((json!({ "id": id }), Some(id)))
        }
        "delete" => {
            let id = id()?;
            exists(editor, &id)?;
            edit::remove(&mut editor.doc, &id);
            Ok((json!({ "deleted": id }), None))
        }
        "move" => {
            let id = id()?;
            exists(editor, &id)?;
            let (parent, index) = place(editor, op, made)?;
            if parent.as_deref().is_some_and(|p| edit::ancestry(&editor.doc, p).contains(&id)) {
                return Err("a layer can't be moved into itself".to_owned());
            }
            let (node, _) = edit::remove(&mut editor.doc, &id).ok_or("the layer couldn't be taken out")?;
            edit::insert(&mut editor.doc, parent.as_deref(), index, node);
            Ok((json!({ "id": id }), Some(id)))
        }
        "copy" => {
            let id = id()?;
            let slot = edit::slot(&editor.doc, &id).ok_or_else(|| format!("there is no layer with the id \"{id}\""))?;
            let mut copy = edit::find(&editor.doc, &id).cloned().ok_or("the layer couldn't be read")?;
            editor.renew_ids(&mut copy);
            // Human: a copy of a component is an instance elsewhere in the app; here it is asked for outright, so
            // it is a plain copy, no component itself.
            copy.reusable = None;
            let new = copy.id.clone();
            let (parent, index) =
                if op.get("parent").is_some_and(|p| !p.is_null()) { place(editor, op, made)? } else { (slot.parent, slot.index + 1) };
            edit::insert(&mut editor.doc, parent.as_deref(), index, copy);
            if !properties.is_empty() {
                update(editor, &new, properties)?;
            }
            Ok((json!({ "id": new }), Some(new)))
        }
        "replace" => {
            let id = id()?;
            let slot = edit::slot(&editor.doc, &id).ok_or_else(|| format!("there is no layer with the id \"{id}\""))?;
            let (old, _) = edit::remove(&mut editor.doc, &id).ok_or("the layer couldn't be taken out")?;
            let mut node = match layer(editor, op.get("node"), &mut Vec::new()) {
                Ok(node) => node,
                Err(error) => {
                    edit::insert(&mut editor.doc, slot.parent.as_deref(), slot.index, old);
                    return Err(error);
                }
            };
            // The new layer takes the old one's id unless it brought its own, so what points at it still does.
            if op.get("node").and_then(|n| n.get("id")).is_none() {
                node.id.clone_from(&id);
            }
            let new = node.id.clone();
            edit::insert(&mut editor.doc, slot.parent.as_deref(), slot.index, node);
            Ok((json!({ "id": new }), Some(new)))
        }
        "set_variables" => {
            // Human: theme axes ("Mode": ["Light", "Dark"]) are set with the variables that follow them.
            let themes = op.get("themes").and_then(Value::as_object);
            for (axis, values) in themes.into_iter().flatten() {
                let all = editor.doc.themes.get_or_insert_with(Map::new);
                if values.is_null() {
                    all.remove(axis);
                } else if values.as_array().is_some_and(|list| !list.is_empty() && list.iter().all(Value::is_string)) {
                    all.insert(axis.clone(), values.clone());
                } else {
                    return Err(format!("the theme axis \"{axis}\" needs a list of value names, the default first"));
                }
            }
            let nothing = Map::new();
            let given = match op.get("variables").and_then(Value::as_object) {
                Some(given) => given,
                None if themes.is_some() => &nothing,
                None => return Err("`set_variables` needs `variables` or `themes`".to_owned()),
            };
            let variables = editor.doc.variables.get_or_insert_with(Map::new);
            for (name, value) in given {
                if value.is_null() {
                    variables.remove(name);
                } else if value.get("type").is_some() && value.get("value").is_some() {
                    variables.insert(name.clone(), value.clone());
                } else {
                    return Err(format!("the variable \"{name}\" needs a `type` and a `value`"));
                }
            }
            Ok((json!({ "variables": given.len() }), None))
        }
        other => Err(format!("`{other}` is no operation; use insert, update, delete, move, copy, replace or set_variables")),
    }
}

/// `edit_layers`: the operations, in order, all or none. Returns each one's result and the ids of the layers
/// touched (for the app to lay out, report and flash).
///
/// # Errors
/// What went wrong and at which operation; the document is then as it was.
// Agent: WRITES editor.doc only; the caller makes the undo step before and lays out after.
pub fn edit(editor: &mut Editor, args: &Value) -> Result<(Vec<Value>, Vec<String>), String> {
    let operations =
        args.get("operations").and_then(Value::as_array).filter(|ops| !ops.is_empty()).ok_or("`edit_layers` needs a list of `operations`")?;
    let before = editor.doc.clone();
    let mut results = Vec::new();
    let mut made: Vec<Option<String>> = Vec::new();
    for (index, op) in operations.iter().enumerate() {
        match operate(editor, op, &made) {
            Ok((result, id)) => {
                results.push(result);
                made.push(id);
            }
            Err(error) => {
                editor.doc = before;
                return Err(format!("Operation {index} failed: {error}. Nothing of this call was kept."));
            }
        }
    }
    let mut touched: Vec<String> = made.into_iter().flatten().collect();
    touched.dedup();
    Ok((results, touched))
}

/// The boxes the layers `ids` have after the document was laid out again.
#[must_use]
pub fn boxes(editor: &Editor, ids: &[String]) -> Value {
    ids.iter().filter_map(|id| editor.scene_node(id).map(|scene| (id.clone(), box_of(scene)))).collect::<Map<String, Value>>().into()
}

/// Bytes as base64 text, the way a picture travels in a tool's answer.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let group =
            (u32::from(chunk[0]) << 16) | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8) | u32::from(chunk.get(2).copied().unwrap_or(0));
        let letter = |shift: u32| char::from(LETTERS[usize::try_from((group >> shift) & 63).unwrap_or(0)]);
        out.push(letter(18));
        out.push(letter(12));
        out.push(if chunk.len() > 1 { letter(6) } else { '=' });
        out.push(if chunk.len() > 2 { letter(0) } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        let doc = pen_doc::read(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"page","name":"Page","x":0,"y":0,"width":400,"height":300,"layout":"none","children":[
                {"type":"text","id":"title","x":20,"y":20,"content":"Hello","fill":"#000000"}]},
              {"type":"frame","id":"btn","reusable":true,"x":500,"y":0,"width":100,"height":40,"layout":"none","children":[
                {"type":"text","id":"label","content":"OK"}]},
              {"type":"ref","id":"inst","ref":"btn","x":500,"y":100}]}"##,
        )
        .unwrap();
        let mut editor = Editor::new(doc, None, 7);
        editor.refresh(&|_| (10.0, 10.0));
        editor
    }

    #[test]
    fn every_tool_has_a_name_a_description_and_a_schema() {
        let tools = tools();
        let names: Vec<&str> = tools.as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "get_state",
                "get_guide",
                "list_skills",
                "read_skill",
                "read_layers",
                "edit_layers",
                "screenshot",
                "check_glass",
                "select_layers",
                "open_document",
                "save_document",
                "export_layers"
            ]
        );
        for tool in tools.as_array().unwrap() {
            assert!(tool["description"].as_str().is_some_and(|d| d.len() > 20) && tool["inputSchema"]["type"] == "object", "{}", tool["name"]);
        }
    }

    #[test]
    fn reading_gives_layers_with_boxes_down_to_a_depth() {
        let editor = editor();
        let all = read(&editor, &json!({ "depth": 0 })).unwrap();
        assert_eq!(all["layers"].as_array().unwrap().len(), 3);
        assert_eq!(all["layers"][0]["childrenNotShown"], json!(["title"]));
        assert_eq!(all["layers"][0]["layoutBox"], json!({ "x": 0.0, "y": 0.0, "width": 400.0, "height": 300.0 }));
        // An instance is given as its `ref`, and a layer inside it as the instance shows it.
        let inside = read(&editor, &json!({ "ids": ["inst", "inst/label"] })).unwrap();
        assert_eq!((&inside["layers"][0]["type"], &inside["layers"][0]["ref"]), (&json!("ref"), &json!("btn")));
        assert_eq!((&inside["layers"][0]["children"][0]["id"], &inside["layers"][1]["content"]), (&json!("inst/label"), &json!("OK")));
        assert!(read(&editor, &json!({ "ids": ["nope"] })).is_err());
        let found = read(&editor, &json!({ "find": "hello" })).unwrap();
        assert_eq!(found["found"][0]["id"], json!("title"));
        assert_eq!(state(&editor)["components"], json!([{ "id": "btn", "name": "Frame" }]));
    }

    #[test]
    fn layers_an_instance_brings_in_its_overrides_get_ids() {
        let mut editor = editor();
        let operations = json!({ "operations": [
            { "op": "insert", "node": { "type": "ref", "id": "filled", "ref": "btn", "descendants": { "label": { "children": [{ "type": "text", "content": "a" }, { "type": "frame", "children": [{ "type": "text", "content": "b" }] }] } } } }
        ] });
        edit(&mut editor, &operations).unwrap();
        let written = serde_json::to_value(edit::find(&editor.doc, "filled").unwrap()).unwrap();
        let brought = written["descendants"]["label"]["children"].as_array().unwrap();
        let ids = [&brought[0]["id"], &brought[1]["id"], &brought[1]["children"][0]["id"]];
        assert!(ids.iter().all(|id| id.as_str().is_some_and(|id| !id.is_empty())) && ids[0] != ids[1] && ids[1] != ids[2], "{written}");
    }

    #[test]
    fn a_stroke_written_as_one_object_becomes_the_documents_three_properties() {
        let mut editor = editor();
        let operations = json!({ "operations": [
            { "op": "insert", "node": { "type": "rectangle", "id": "edge", "width": 10, "height": 10, "stroke": { "align": "inside", "thickness": 2, "fill": "#112233" } } },
            { "op": "insert", "node": { "type": "rectangle", "id": "plain", "width": 10, "height": 10, "stroke": { "type": "color", "color": "#445566" } } },
            { "op": "update", "id": "plain", "properties": { "stroke": { "thickness": 3, "fill": "#778899" } } }
        ] });
        edit(&mut editor, &operations).unwrap();
        let written = |id: &str| serde_json::to_value(edit::find(&editor.doc, id).unwrap()).unwrap();
        let edge = written("edge");
        assert_eq!((&edge["stroke"], &edge["strokeWidth"], &edge["strokeAlignment"]), (&json!("#112233"), &json!(2), &json!("inner")));
        let plain = written("plain");
        assert_eq!((&plain["stroke"], &plain["strokeWidth"]), (&json!("#778899"), &json!(3)));
    }

    #[test]
    fn edits_insert_update_move_copy_replace_and_delete() {
        let mut editor = editor();
        let (results, touched) = edit(
            &mut editor,
            &json!({ "operations": [
                { "op": "insert", "parent": "page", "node": { "type": "frame", "name": "Card", "layout": "vertical", "children": [{ "type": "text", "content": "In the card" }] } },
                { "op": "insert", "parent": "$0", "node": { "type": "rectangle", "id": "bar", "width": 40, "height": 4, "fill": "#ff0000" } },
                { "op": "update", "id": "title", "properties": { "content": "Changed", "fill": null, "fontSize": 24 } },
                { "op": "update", "id": "inst/label", "properties": { "content": "Cancel" } },
                { "op": "copy", "id": "bar", "properties": { "fill": "#00ff00" } },
                { "op": "move", "id": "title", "parent": "$0", "index": 0 },
                { "op": "replace", "id": "bar", "node": { "type": "ellipse", "width": 10, "height": 10 } },
                { "op": "set_variables", "themes": { "Mode": ["Light", "Dark"] }, "variables": { "accent": { "type": "color", "value": "#3b82f6" } } }
            ] }),
        )
        .unwrap();
        assert_eq!(editor.doc.themes.as_ref().unwrap()["Mode"], json!(["Light", "Dark"]));
        let card = results[0]["id"].as_str().unwrap().to_owned();
        let kids: Vec<&str> = edit::find(&editor.doc, &card).unwrap().kids().iter().map(|n| n.id.as_str()).collect();
        assert_eq!(kids.len(), 4);
        assert_eq!((kids[0], kids[2]), ("title", "bar"));
        let title = edit::find(&editor.doc, "title").unwrap();
        assert_eq!((title.content.as_deref(), title.fill.is_none()), (Some("Changed"), true));
        assert_eq!(edit::find(&editor.doc, "bar").unwrap().kind, Kind::Ellipse);
        let inst = edit::find(&editor.doc, "inst").unwrap();
        assert_eq!(serde_json::to_value(inst.descendants.as_ref().unwrap()).unwrap(), json!({ "label": { "content": "Cancel" } }));
        assert_eq!(edit::find(&editor.doc, "label").unwrap().content.as_deref(), Some("OK"));
        assert!(editor.doc.variables.as_ref().unwrap().contains_key("accent"));
        assert!(touched.contains(&card) && touched.contains(&"inst/label".to_owned()));
        // A delete, and the boxes of what was touched once the document is laid out again.
        edit(&mut editor, &json!({ "operations": [{ "op": "delete", "id": "bar" }] })).unwrap();
        assert!(edit::find(&editor.doc, "bar").is_none());
        editor.touched();
        editor.refresh(&|_| (10.0, 10.0));
        assert!(boxes(&editor, &touched)[&card]["width"].as_f64().is_some());
    }

    #[test]
    fn a_material_is_checked_and_what_is_wrong_is_said_so_that_it_can_be_put_right() {
        let mut editor = editor();
        let insert = |node: Value| json!({ "operations": [{ "op": "insert", "parent": "page", "node": node }] });
        let (results, _) = edit(
            &mut editor,
            &insert(json!({ "type": "frame", "id": "bar", "glassGroup": 8, "children": [
                { "type": "frame", "id": "pill", "width": 80, "height": 44, "cornerRadius": 22, "material": { "type": "liquid_glass", "tint": "#0a84ff" } }
            ] })),
        )
        .unwrap();
        assert_eq!(results[0]["id"], "bar");
        let before = editor.doc.clone();
        let failed = |editor: &mut Editor, args: Value| edit(editor, &args).unwrap_err();
        let error = failed(&mut editor, insert(json!({ "type": "frame", "material": { "type": "glass" } })));
        assert!(error.contains("`material.type` \"glass\" is not a material; use \"frosted\", \"liquid_glass\""), "{error}");
        let error = failed(&mut editor, insert(json!({ "type": "text", "content": "Hi", "material": "liquid_glass" })));
        assert!(error.contains("a text can't have a `material`") && error.contains("put the text in it"), "{error}");
        let error = failed(
            &mut editor,
            json!({ "operations": [{ "op": "update", "id": "pill", "properties": { "material": { "type": "liquid_glass", "advanced": { "blur": -4 } } } }] }),
        );
        assert!(error.contains("`material.advanced.blur` must be a number from 0 to 200"), "{error}");
        let error = failed(&mut editor, json!({ "operations": [{ "op": "update", "id": "pill", "properties": { "glassGroup": "tight" } }] }));
        assert!(error.contains("`glassGroup` must be a number"), "{error}");
        assert_eq!(editor.doc, before);
        // A sound change goes in, and null takes the material off again.
        edit(&mut editor, &json!({ "operations": [{ "op": "update", "id": "pill", "properties": { "material": "frosted" } }] })).unwrap();
        assert_eq!(edit::find(&editor.doc, "pill").unwrap().material, Some(json!("frosted")));
        edit(&mut editor, &json!({ "operations": [{ "op": "update", "id": "pill", "properties": { "material": null } }] })).unwrap();
        assert_eq!(edit::find(&editor.doc, "pill").unwrap().material, None);
    }

    #[test]
    fn a_failing_operation_leaves_the_document_as_it_was() {
        let mut editor = editor();
        let before = editor.doc.clone();
        let error = edit(
            &mut editor,
            &json!({ "operations": [
                { "op": "update", "id": "title", "properties": { "content": "Changed" } },
                { "op": "insert", "parent": "missing", "node": { "type": "text" } }
            ] }),
        )
        .unwrap_err();
        assert!(error.starts_with("Operation 1 failed") && error.contains("missing"), "{error}");
        assert_eq!(editor.doc, before);
        for bad in [
            json!({ "operations": [] }),
            json!({ "operations": [{ "op": "insert", "node": { "name": "no type" } }] }),
            json!({ "operations": [{ "op": "insert", "node": { "type": "text", "id": "title" } }] }),
            json!({ "operations": [{ "op": "insert", "parent": "title", "node": { "type": "text" } }] }),
            json!({ "operations": [{ "op": "move", "id": "page", "parent": "page" }] }),
            json!({ "operations": [{ "op": "update", "id": "title", "properties": { "type": "frame" } }] }),
            json!({ "operations": [{ "op": "insert", "parent": "$3", "node": { "type": "text" } }] }),
            json!({ "operations": [{ "op": "fly" }] }),
        ] {
            assert!(edit(&mut editor, &bad).is_err(), "{bad}");
            assert_eq!(editor.doc, before);
        }
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(
            (base64(b""), base64(b"f"), base64(b"fo"), base64(b"foo"), base64(b"foobar")),
            (String::new(), "Zg==".to_owned(), "Zm8=".to_owned(), "Zm9v".to_owned(), "Zm9vYmFy".to_owned())
        );
    }
}
