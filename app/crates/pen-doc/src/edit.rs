//! Finding, inserting, removing and moving nodes in a document by id.

use std::collections::HashSet;

use crate::model::{Document, Node};

/// Where a node sits: its parent (`None` for the document root) and its index among the parent's children.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Slot {
    /// The parent's id, `None` at the top level.
    pub parent: Option<String>,
    /// Position among the parent's children, 0 = back.
    pub index: usize,
}

/// The node with `id`.
#[must_use]
pub fn find<'a>(doc: &'a Document, id: &str) -> Option<&'a Node> {
    fn search<'a>(nodes: &'a [Node], id: &str) -> Option<&'a Node> {
        nodes.iter().find_map(|n| if n.id == id { Some(n) } else { search(n.kids(), id) })
    }
    search(&doc.children, id)
}

/// The node with `id`, to change.
#[must_use]
pub fn find_mut<'a>(doc: &'a mut Document, id: &str) -> Option<&'a mut Node> {
    fn search<'a>(nodes: &'a mut [Node], id: &str) -> Option<&'a mut Node> {
        for node in nodes {
            if node.id == id {
                return Some(node);
            }
            if let Some(found) = node.children.as_deref_mut().and_then(|kids| search(kids, id)) {
                return Some(found);
            }
        }
        None
    }
    search(&mut doc.children, id)
}

/// Where the node with `id` sits.
#[must_use]
pub fn slot(doc: &Document, id: &str) -> Option<Slot> {
    fn search(nodes: &[Node], parent: Option<&str>, id: &str) -> Option<Slot> {
        for (index, node) in nodes.iter().enumerate() {
            if node.id == id {
                return Some(Slot { parent: parent.map(str::to_owned), index });
            }
            if let Some(found) = search(node.kids(), Some(&node.id), id) {
                return Some(found);
            }
        }
        None
    }
    search(&doc.children, None, id)
}

/// For a scene id of a node inside an instance (`instance/inner`, see [`crate::scene::SceneNode::id`]): the
/// document instance it belongs to and the key of its overrides in that instance's `descendants`.
#[must_use]
pub fn owner(doc: &Document, scene_id: &str) -> Option<(String, String)> {
    let (instance, key) = scene_id.split_once('/')?;
    find(doc, instance).filter(|node| node.kind == crate::model::Kind::Ref)?;
    Some((instance.to_owned(), key.to_owned()))
}

/// Writes property overrides into the instance with id `instance`: onto the instance itself, or with `key` onto
/// the node inside it that the key names (see [`owner`]). Returns whether the instance was found.
// Human: this is how an instance is edited: the component stays as it is, and the instance records what differs
// (written to the file as the `ref`'s own keys and its `descendants`).
// Agent: a `null` among `changes` clears a property inside the instance; on the instance itself it only
// removes the instance's own override, since a `ref` has no way to say "nothing here".
pub fn set_overrides(doc: &mut Document, instance: &str, key: Option<&str>, changes: &serde_json::Map<String, serde_json::Value>) -> bool {
    use serde_json::Value;
    let Some(node) = find_mut(doc, instance) else { return false };
    if let Some(key) = key {
        let entry = node.descendants.get_or_insert_with(serde_json::Map::new).entry(key).or_insert_with(|| Value::Object(serde_json::Map::new()));
        match entry.as_object_mut() {
            Some(entry) => entry.extend(changes.clone()),
            None => *entry = Value::Object(changes.clone()),
        }
        return true;
    }
    let Ok(Value::Object(mut whole)) = serde_json::to_value(&*node) else { return false };
    whole.extend(changes.clone());
    let Ok(overridden) = serde_json::from_value(Value::Object(whole)) else { return false };
    *node = overridden;
    true
}

/// The children an instance has put in place of those of the node `key` names inside it, if it has.
#[must_use]
pub fn override_children(doc: &Document, instance: &str, key: &str) -> Option<Vec<serde_json::Value>> {
    find(doc, instance)?.descendants.as_ref()?.get(key)?.get("children")?.as_array().cloned()
}

/// Takes the override of `property` off the node `key` names inside `instance`, so the component shows through
/// again. Returns whether there was one.
pub fn clear_override(doc: &mut Document, instance: &str, key: &str, property: &str) -> bool {
    let Some(descendants) = find_mut(doc, instance).and_then(|node| node.descendants.as_mut()) else { return false };
    let Some(entry) = descendants.get_mut(key).and_then(serde_json::Value::as_object_mut) else { return false };
    let had = entry.remove(property).is_some();
    if entry.is_empty() {
        descendants.remove(key);
    }
    had
}

/// Takes the node with id `child` out of the children `instance` has put into one of its nodes (a filled slot).
/// Returns whether it was there; a node that comes from the component is not touched.
pub fn remove_override_child(doc: &mut Document, instance: &str, child: &str) -> bool {
    fn prune(children: &mut Vec<serde_json::Value>, child: &str) -> bool {
        let before = children.len();
        children.retain(|node| node.get("id").and_then(serde_json::Value::as_str) != Some(child));
        before != children.len()
            || children
                .iter_mut()
                .any(|node| node.get_mut("children").and_then(serde_json::Value::as_array_mut).is_some_and(|inner| prune(inner, child)))
    }
    let Some(descendants) = find_mut(doc, instance).and_then(|node| node.descendants.as_mut()) else { return false };
    let removed = descendants
        .values_mut()
        .any(|entry| entry.get_mut("children").and_then(serde_json::Value::as_array_mut).is_some_and(|children| prune(children, child)));
    if removed {
        // What was overridden on the removed node goes with it.
        descendants.retain(|key, _| key != child && !key.starts_with(&format!("{child}/")));
    }
    removed
}

/// The ids from the top level down to the node with `id` (inclusive).
#[must_use]
pub fn ancestry(doc: &Document, id: &str) -> Vec<String> {
    let mut chain = Vec::new();
    let mut current = Some(id.to_owned());
    while let Some(id) = current {
        current = slot(doc, &id).and_then(|s| s.parent);
        chain.push(id);
    }
    chain.reverse();
    chain
}

/// The children list a node with id `parent` holds (the top level for `None`), created when missing.
pub fn children_mut<'a>(doc: &'a mut Document, parent: Option<&str>) -> Option<&'a mut Vec<Node>> {
    match parent {
        None => Some(&mut doc.children),
        Some(id) => Some(find_mut(doc, id)?.children.get_or_insert_with(Vec::new)),
    }
}

/// Takes the node with `id` out of the document.
pub fn remove(doc: &mut Document, id: &str) -> Option<(Node, Slot)> {
    let slot = slot(doc, id)?;
    let list = children_mut(doc, slot.parent.as_deref())?;
    (slot.index < list.len()).then(|| (list.remove(slot.index), slot))
}

/// Puts `node` into `parent` at `index` (clamped to the end).
///
/// Returns `false` when the parent doesn't exist; the node is then dropped.
pub fn insert(doc: &mut Document, parent: Option<&str>, index: usize, node: Node) -> bool {
    match children_mut(doc, parent) {
        Some(list) => {
            let index = index.min(list.len());
            list.insert(index, node);
            true
        }
        None => false,
    }
}

/// Every id in the document.
#[must_use]
pub fn ids(doc: &Document) -> HashSet<String> {
    fn collect(nodes: &[Node], out: &mut HashSet<String>) {
        for node in nodes {
            out.insert(node.id.clone());
            collect(node.kids(), out);
        }
    }
    let mut out = HashSet::new();
    collect(&doc.children, &mut out);
    out
}

/// Makes fresh ids: five letters and digits (`YKsGt`).
#[derive(Clone, Debug)]
pub struct IdMaker {
    state: u64,
}

impl IdMaker {
    /// A maker seeded with `seed` (any value; the clock is a good one).
    #[must_use]
    pub fn new(seed: u64) -> IdMaker {
        IdMaker { state: seed | 1 }
    }

    /// An id not in `taken`; it is added to `taken`.
    pub fn fresh(&mut self, taken: &mut HashSet<String>) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
        loop {
            let id: String = (0..5)
                .map(|_| {
                    // Human: xorshift64 — not secure, only needs to spread ids out.
                    self.state ^= self.state << 13;
                    self.state ^= self.state >> 7;
                    self.state ^= self.state << 17;
                    char::from(ALPHABET[index_of(self.state, ALPHABET.len())])
                })
                .collect();
            if taken.insert(id.clone()) {
                return id;
            }
        }
    }

    /// Gives `node` and all its descendants fresh ids (for pasted and duplicated copies).
    pub fn renew(&mut self, node: &mut Node, taken: &mut HashSet<String>) {
        node.id = self.fresh(taken);
        for child in node.children.iter_mut().flatten() {
            self.renew(child, taken);
        }
    }
}

/// `value mod len` as an index.
#[expect(clippy::cast_possible_truncation, reason = "the remainder is below len, which is a usize")]
fn index_of(value: u64, len: usize) -> usize {
    (value % len as u64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Kind;

    fn doc() -> Document {
        serde_json::from_str(
            r#"{"version":"2.22","children":[{"type":"frame","id":"f","children":[{"type":"text","id":"t"}]},{"type":"rectangle","id":"r"}]}"#,
        )
        .unwrap()
    }

    #[test]
    fn overrides_are_written_onto_the_instance_and_into_its_descendants() {
        let mut d: Document = serde_json::from_str(
            r#"{"version":"2.22","children":[
              {"type":"frame","id":"btn","reusable":true,"children":[{"type":"text","id":"label","content":"OK"}]},
              {"type":"ref","id":"a","ref":"btn","descendants":{"label":{"content":"Old"}}}]}"#,
        )
        .unwrap();
        assert_eq!(owner(&d, "a/label"), Some(("a".to_owned(), "label".to_owned())));
        assert_eq!(owner(&d, "a/ic/dot"), Some(("a".to_owned(), "ic/dot".to_owned())));
        assert_eq!((owner(&d, "a"), owner(&d, "btn/label")), (None, None));
        let changes = |json: &str| serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(json).unwrap();
        assert!(set_overrides(&mut d, "a", Some("label"), &changes(r##"{"fill":"#ff0000"}"##)));
        assert!(set_overrides(&mut d, "a", None, &changes(r#"{"x":12,"name":"Primary"}"#)));
        assert!(!set_overrides(&mut d, "missing", None, &changes("{}")));
        let a = find(&d, "a").unwrap();
        assert_eq!(serde_json::to_value(a.descendants.as_ref().unwrap()).unwrap(), serde_json::json!({"label":{"content":"Old","fill":"#ff0000"}}));
        assert_eq!((a.name.as_deref(), a.kind.clone(), a.reference.as_deref()), (Some("Primary"), Kind::Ref, Some("btn")));
    }

    #[test]
    fn a_slot_is_filled_emptied_and_given_back() {
        let mut d: Document = serde_json::from_str(
            r#"{"version":"2.22","children":[
              {"type":"frame","id":"card","reusable":true,"children":[{"type":"frame","id":"body","slot":[]}]},
              {"type":"ref","id":"a","ref":"card","descendants":{
                "body":{"gap":4,"children":[{"type":"text","id":"t1"},{"type":"frame","id":"f1","children":[{"type":"text","id":"t2"}]}]},
                "t2":{"content":"Deep"}}}]}"#,
        )
        .unwrap();
        assert_eq!(override_children(&d, "a", "body").map(|c| c.len()), Some(2));
        assert_eq!(override_children(&d, "a", "missing"), None);
        // A nested filled layer goes with what was overridden on it; a layer of the component is not removable.
        assert!(remove_override_child(&mut d, "a", "t2"));
        assert!(!remove_override_child(&mut d, "a", "body"));
        let descendants = |d: &Document| serde_json::to_value(find(d, "a").unwrap().descendants.as_ref().unwrap()).unwrap();
        assert_eq!(
            descendants(&d),
            serde_json::json!({"body":{"gap":4,"children":[{"type":"text","id":"t1"},{"type":"frame","id":"f1","children":[]}]}})
        );
        assert!(clear_override(&mut d, "a", "body", "children"));
        assert_eq!(descendants(&d), serde_json::json!({"body":{"gap":4}}));
        assert!(clear_override(&mut d, "a", "body", "gap") && !clear_override(&mut d, "a", "body", "gap"));
        assert_eq!(descendants(&d), serde_json::json!({}));
    }

    #[test]
    fn find_slot_remove_insert() {
        let mut d = doc();
        assert_eq!(find(&d, "t").map(|n| &n.kind), Some(&Kind::Text));
        assert_eq!(slot(&d, "t"), Some(Slot { parent: Some("f".into()), index: 0 }));
        assert_eq!(ancestry(&d, "t"), vec!["f".to_owned(), "t".to_owned()]);
        let (node, _) = remove(&mut d, "r").unwrap();
        assert!(insert(&mut d, Some("f"), 99, node));
        assert_eq!(slot(&d, "r"), Some(Slot { parent: Some("f".into()), index: 1 }));
        assert!(!insert(&mut d, Some("missing"), 0, Node::new(Kind::Rectangle, "x".into())));
    }

    #[test]
    fn fresh_ids_are_unique() {
        let d = doc();
        let mut taken = ids(&d);
        let mut maker = IdMaker::new(42);
        let a = maker.fresh(&mut taken);
        let b = maker.fresh(&mut taken);
        assert_ne!(a, b);
        assert_eq!(a.len(), 5);
        assert!(!a.contains('/'));
    }
}
