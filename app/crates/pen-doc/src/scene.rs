//! The scene: the document with every component instance (`ref`) expanded, ready to lay out and draw.
//!
//! The document stays as the user wrote it; the scene is rebuilt from it after every change.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::geom::{Affine, Rect};
use crate::model::{Document, Kind, Node};
use crate::vars::ThemeContext;

/// How deeply instances may nest (an instance inside a component inside an instance...) before expansion stops.
/// Also guards against a component that instances itself.
const MAX_INSTANCE_DEPTH: usize = 16;

/// One node of the scene.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneNode {
    /// Unique within the scene: the document id, or `instance/inner` for nodes inside an instance.
    pub id: String,
    /// The document node this scene node is, when it is one (instance roots are; their insides are not).
    pub doc_id: Option<String>,
    /// The node's properties, after component overrides; `children` is always `None` here.
    pub node: Node,
    /// Child scene nodes, back to front.
    pub children: Vec<SceneNode>,
    /// Theme axis values in force for this node.
    pub theme: ThemeContext,
    /// `false` for a `ref` whose component is missing: drawn as an empty placeholder.
    pub resolved: bool,
    /// Box in the parent's coordinates (unrotated), set by [`crate::layout`].
    pub local: Rect,
    /// Local → document transform (position, rotation, flips), set by [`crate::layout`].
    pub to_doc: Affine,
}

impl SceneNode {
    /// Visits this node and every descendant, parents first.
    pub fn walk<'a>(&'a self, visit: &mut impl FnMut(&'a SceneNode, usize), depth: usize) {
        visit(self, depth);
        for child in &self.children {
            child.walk(visit, depth + 1);
        }
    }
}

/// Builds the scene for `doc`, with `theme` in force at the top level (usually [`ThemeContext::defaults`]).
#[must_use]
pub fn build(doc: &Document, theme: &ThemeContext) -> Vec<SceneNode> {
    let mut index: HashMap<&str, &Node> = HashMap::new();
    for child in &doc.children {
        index_nodes(child, &mut index);
    }
    let builder = Builder { index };
    doc.children.iter().map(|node| builder.convert(node, theme, None, true, 0)).collect()
}

/// The scene nodes of the top-level nodes `which` of `doc` alone (by their place among its children), in that
/// order: what [`build`] gives for them, without building the rest of the document.
// Human: an edit changes one screen of a design with dozens. Its instances may still show components that
// live anywhere in the document, so every node is looked at for the index; none but the asked ones is copied.
#[must_use]
pub fn build_some(doc: &Document, theme: &ThemeContext, which: &[usize]) -> Vec<SceneNode> {
    let mut index: HashMap<&str, &Node> = HashMap::new();
    for child in &doc.children {
        index_nodes(child, &mut index);
    }
    let builder = Builder { index };
    which.iter().filter_map(|at| doc.children.get(*at)).map(|node| builder.convert(node, theme, None, true, 0)).collect()
}

/// The scene node of `node` by itself: what [`build`] gives for it where it lies in `doc`. `node` is a node of
/// `doc` that is in no instance, and `theme` is the theme in force for the node it lies in.
// Human: a drag changes one layer of thousands. Only that layer and what is in it are built again; the
// document is looked through for components only when an instance is among them.
#[must_use]
pub fn build_node(doc: &Document, node: &Node, theme: &ThemeContext) -> SceneNode {
    fn holds_instance(node: &Node) -> bool {
        node.kind == Kind::Ref || node.kids().iter().any(holds_instance)
    }
    let mut index: HashMap<&str, &Node> = HashMap::new();
    if holds_instance(node) {
        for child in &doc.children {
            index_nodes(child, &mut index);
        }
    }
    Builder { index }.convert(node, theme, None, true, 0)
}

fn index_nodes<'a>(node: &'a Node, index: &mut HashMap<&'a str, &'a Node>) {
    index.insert(node.id.as_str(), node);
    for child in node.kids() {
        index_nodes(child, index);
    }
}

struct Builder<'a> {
    index: HashMap<&'a str, &'a Node>,
}

impl Builder<'_> {
    /// One document (or expanded component) node → scene node.
    // Human: `prefix` is set inside an instance: there ids become `instanceId/childId`, so two instances of
    // one component never share scene ids, and those nodes aren't document nodes (`doc_id` None).
    fn convert(&self, node: &Node, theme: &ThemeContext, prefix: Option<&str>, in_doc: bool, depth: usize) -> SceneNode {
        let id = match prefix {
            Some(prefix) => format!("{prefix}/{}", node.id),
            None => node.id.clone(),
        };
        let doc_id = in_doc.then(|| node.id.clone());
        if node.kind == Kind::Ref {
            if let Some(expanded) = self.expand(node).filter(|_| depth < MAX_INSTANCE_DEPTH) {
                let theme = theme.with(expanded.theme.as_ref());
                let children = expanded.kids().iter().map(|child| self.convert(child, &theme, Some(&id), false, depth + 1)).collect();
                return scene_node(id, doc_id, strip(expanded), children, theme, true);
            }
            return scene_node(id, doc_id, strip(node.clone()), Vec::new(), theme.clone(), false);
        }
        let theme = theme.with(node.theme.as_ref());
        let children = node.kids().iter().map(|child| self.convert(child, &theme, prefix, in_doc, depth)).collect();
        scene_node(id, doc_id, strip(node.clone()), children, theme, true)
    }

    /// A `ref` node as the node it instances, with the instance's own properties and `descendants` applied.
    // Agent: RETURNS None when the referenced id is missing or the merge doesn't parse; works on JSON values so
    // any property (known or not) can be overridden.
    fn expand(&self, instance: &Node) -> Option<Node> {
        let target = self.index.get(instance.reference.as_deref()?)?;
        let mut value = serde_json::to_value(target).ok()?;
        let object = value.as_object_mut()?;
        object.remove("reusable");
        // Human: a component can name `states` (selected, disabled, ...), each the changes that state makes, written
        // like an instance's own: properties of the root, and `descendants`. An instance picks one with `state`
        // (or several, applied in order); what the instance itself overrides still wins over the state.
        let states = object.remove("states");
        let picked: Vec<&str> = match instance.extra.get("state") {
            Some(Value::String(name)) => vec![name.as_str()],
            Some(Value::Array(names)) => names.iter().filter_map(Value::as_str).collect(),
            _ => Vec::new(),
        };
        for name in picked {
            let Some(state) = states.as_ref().and_then(|states| states.get(name)).and_then(Value::as_object) else { continue };
            for (key, v) in state {
                if !SKIPPED.contains(&key.as_str()) {
                    object.insert(key.clone(), v.clone());
                }
            }
            for (path, change) in state.get("descendants").and_then(Value::as_object).into_iter().flatten() {
                apply_descendant(object, path, change);
            }
        }
        let overrides = serde_json::to_value(instance).ok()?;
        for (key, v) in overrides.as_object()? {
            if !SKIPPED.contains(&key.as_str()) {
                object.insert(key.clone(), v.clone());
            }
        }
        // Human: an instance that lists `children` of its own shows those in place of the component's;
        // this is how a slot is filled.
        if let Some(children) = &instance.children {
            object.insert("children".to_owned(), serde_json::to_value(children).ok()?);
        }
        for (path, change) in instance.descendants.iter().flatten() {
            apply_descendant(object, path, change);
        }
        serde_json::from_value(value).ok()
    }
}

/// The keys of an instance or a state that are not properties to lay over the component's root.
const SKIPPED: [&str; 8] = ["type", "id", "ref", "descendants", "reusable", "children", "state", "states"];

fn strip(mut node: Node) -> Node {
    node.children = None;
    node
}

fn scene_node(id: String, doc_id: Option<String>, node: Node, children: Vec<SceneNode>, theme: ThemeContext, resolved: bool) -> SceneNode {
    SceneNode { id, doc_id, node, children, theme, resolved, local: Rect::ZERO, to_doc: Affine::IDENTITY }
}

/// Where the node a `descendants` key segment names sits under `node`: the child indexes leading to it.
// Human: a key is a node's id wherever it sits in the component (ids are unique there), or its name when only
// one node has it. The search doesn't go into a nested
// instance, whose insides belong to another component.
fn locate(node: &Map<String, Value>, segment: &str) -> Option<Vec<usize>> {
    fn search(node: &Map<String, Value>, key: &str, segment: &str, trail: &mut Vec<usize>, found: &mut Vec<Vec<usize>>) {
        let Some(children) = node.get("children").and_then(Value::as_array) else { return };
        for (index, child) in children.iter().enumerate() {
            let Some(child) = child.as_object() else { continue };
            trail.push(index);
            if child.get(key).and_then(Value::as_str) == Some(segment) {
                found.push(trail.clone());
            }
            if child.get("type").and_then(Value::as_str) != Some("ref") {
                search(child, key, segment, trail, found);
            }
            trail.pop();
        }
    }
    let mut found = Vec::new();
    search(node, "id", segment, &mut Vec::new(), &mut found);
    if found.is_empty() {
        search(node, "name", segment, &mut Vec::new(), &mut found);
        if found.len() != 1 {
            return None;
        }
    }
    found.into_iter().next()
}

/// The node at `trail` (child indexes) under `node`.
fn descend<'a>(mut node: &'a mut Map<String, Value>, trail: &[usize]) -> Option<&'a mut Map<String, Value>> {
    for index in trail {
        node = node.get_mut("children")?.as_array_mut()?.get_mut(*index)?.as_object_mut()?;
    }
    Some(node)
}

/// Applies one `descendants` entry: `type` present replaces the node, otherwise its keys are merged in.
// Human: a key names a node of the component by id;
// a key with slashes goes through a nested instance, `nestedInstanceId/idInsideIt`. The part after a nested
// instance is handed to that instance as its own override, and applied when it is expanded in turn.
fn apply_descendant(root: &mut Map<String, Value>, path: &str, change: &Value) {
    let Some(change) = change.as_object() else { return };
    let mut current = root;
    let mut segments = path.split('/');
    while let Some(segment) = segments.next() {
        let Some(trail) = locate(current, segment) else { return };
        let Some(next) = descend(current, &trail) else { return };
        current = next;
        if current.get("type").and_then(Value::as_str) == Some("ref") {
            let rest: Vec<&str> = segments.collect();
            if rest.is_empty() {
                break;
            }
            let inner = current.entry("descendants").or_insert_with(|| Value::Object(Map::new()));
            let Some(inner) = inner.as_object_mut() else { return };
            match inner.get_mut(&rest.join("/")).and_then(Value::as_object_mut) {
                Some(existing) if !change.contains_key("type") => existing.extend(change.clone()),
                _ => {
                    inner.insert(rest.join("/"), Value::Object(change.clone()));
                }
            }
            return;
        }
    }
    if change.contains_key("type") {
        let id = current.get("id").cloned();
        current.clone_from(change);
        if let Some(id) = id.filter(|_| !change.contains_key("id")) {
            current.insert("id".into(), id);
        }
    } else {
        for (key, value) in change {
            current.insert(key.clone(), value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instances_take_overrides_and_get_unique_ids() {
        let doc: Document = serde_json::from_str(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"btn","reusable":true,"fill":"#000000","children":[{"type":"text","id":"label","content":"OK"}]},
              {"type":"ref","id":"a","ref":"btn","x":10,"fill":"#ff0000","descendants":{"label":{"content":"Cancel"}}},
              {"type":"ref","id":"b","ref":"missing"}]}"##,
        )
        .unwrap();
        let scene = build(&doc, &ThemeContext::defaults(&doc));
        let a = &scene[1];
        assert_eq!(a.node.kind, Kind::Frame);
        assert_eq!(a.doc_id.as_deref(), Some("a"));
        assert!(matches!(&a.node.fill, Some(crate::model::Paints::One(crate::model::Paint::Solid(c))) if c == "#ff0000"));
        assert_eq!(a.children[0].id, "a/label");
        assert_eq!(a.children[0].doc_id, None);
        assert_eq!(a.children[0].node.content.as_deref(), Some("Cancel"));
        assert!(!scene[2].resolved);
    }

    #[test]
    fn overrides_reach_deep_nodes_named_nodes_and_nested_instances() {
        let doc: Document = serde_json::from_str(
            r##"{"version":"2.22","children":[
              {"type":"frame","id":"icon","reusable":true,"children":[{"type":"rectangle","id":"dot","fill":"#000000"}]},
              {"type":"frame","id":"btn","reusable":true,"children":[
                {"type":"frame","id":"row","children":[{"type":"text","id":"label","name":"Label","content":"OK"},{"type":"ref","id":"ic","ref":"icon"}]},
                {"type":"frame","id":"slot","slot":[]}]},
              {"type":"ref","id":"a","ref":"btn","descendants":{
                "label":{"content":"Deep"},
                "ic/dot":{"fill":"#ff0000"},
                "slot":{"children":[{"type":"text","id":"filled","content":"In the slot"}]}}},
              {"type":"ref","id":"b","ref":"btn","descendants":{"Label":{"content":"By name"}}},
              {"type":"ref","id":"c","ref":"btn","children":[{"type":"text","id":"only","content":"Replaced"}]}]}"##,
        )
        .unwrap();
        let scene = build(&doc, &ThemeContext::defaults(&doc));
        let a = &scene[2];
        let row = &a.children[0];
        assert_eq!((row.id.as_str(), row.children[0].id.as_str()), ("a/row", "a/label"));
        assert_eq!(row.children[0].node.content.as_deref(), Some("Deep"));
        // The nested instance's inside carries the instance's id in its path, and took the override.
        let dot = &row.children[1].children[0];
        assert_eq!(dot.id, "a/ic/dot");
        assert!(matches!(&dot.node.fill, Some(crate::model::Paints::One(crate::model::Paint::Solid(c))) if c == "#ff0000"));
        assert_eq!(a.children[1].children[0].node.content.as_deref(), Some("In the slot"));
        assert_eq!(scene[3].children[0].children[0].node.content.as_deref(), Some("By name"));
        // The other instance of the icon is untouched, and an instance with its own children shows those.
        assert!(
            matches!(&scene[3].children[0].children[1].children[0].node.fill, Some(crate::model::Paints::One(crate::model::Paint::Solid(c))) if c == "#000000")
        );
        assert_eq!(scene[4].children.len(), 1);
        assert_eq!(scene[4].children[0].node.content.as_deref(), Some("Replaced"));
    }

    #[test]
    fn an_instance_takes_a_state_of_its_component_and_its_own_overrides_win() {
        let doc: Document = serde_json::from_str(
            r##"{"version":"2.22","children":[
            {"type":"frame","id":"row","reusable":true,"fill":"#000000","opacity":1,
             "states":{"selected":{"fill":"#222222","opacity":0.5,"descendants":{"label":{"fill":"#ffffff"}}},
                       "disabled":{"opacity":0.3}},
             "children":[{"type":"text","id":"label","content":"Board","fill":"#999999"}]},
            {"type":"ref","id":"plain","ref":"row"},
            {"type":"ref","id":"on","ref":"row","state":"selected","opacity":0.8,"descendants":{"label":{"content":"Inbox"}}},
            {"type":"ref","id":"both","ref":"row","state":["selected","disabled","unknown"]}]}"##,
        )
        .unwrap();
        let scene = build(&doc, &ThemeContext::defaults(&doc));
        let fill = |node: &SceneNode| serde_json::to_value(&node.node.fill).unwrap();
        let opacity = |node: &SceneNode| serde_json::to_value(&node.node.opacity).unwrap();
        // The component and a plain instance show the default.
        assert_eq!((fill(&scene[0]), fill(&scene[1]), fill(&scene[1].children[0])), ("#000000".into(), "#000000".into(), "#999999".into()));
        assert!(scene[1].node.extra.is_empty());
        // The state changes the root and a layer inside; the instance's own opacity and text still apply.
        let on = &scene[2];
        assert_eq!((fill(on), opacity(on), fill(&on.children[0])), ("#222222".into(), 0.8.into(), "#ffffff".into()));
        assert_eq!((on.id.as_str(), on.children[0].node.content.as_deref()), ("on", Some("Inbox")));
        // Several states apply in order; one the component doesn't have changes nothing.
        assert_eq!((fill(&scene[3]), opacity(&scene[3])), ("#222222".into(), 0.3.into()));
    }

    #[test]
    fn a_component_that_instances_itself_stops() {
        let doc: Document = serde_json::from_str(
            r#"{"version":"2.22","children":[{"type":"frame","id":"c","reusable":true,"children":[{"type":"ref","id":"r","ref":"c"}]}]}"#,
        )
        .unwrap();
        let scene = build(&doc, &ThemeContext::defaults(&doc));
        let mut depth = 0;
        scene[0].walk(&mut |_, d| depth = depth.max(d), 0);
        assert!(depth <= MAX_INSTANCE_DEPTH + 1);
    }
}
