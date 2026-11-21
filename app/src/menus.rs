//! The menu bar at the top of the screen (macOS): File, Edit, View, Window and Help.
//!
//! craft-menu lays the menus out the Mac way (the app menu with Settings and Quit, Minimize and Zoom in
//! Window, the open tabs listed there) and reports what was chosen as a command id; this module says what is in
//! the menus and which [`Command`] each id means.

use craft_menu::{Chord, CommandRef, Document, Item, ItemRole, Menu, MenuBar, MenuRole, Node};
use serde_json::json;

use crate::export::Format;
use crate::palette::Command;

/// What the menus show of the app's state.
pub struct State<'a> {
    /// Whether a document is open in front.
    pub document: bool,
    /// Whether anything is selected in it.
    pub selection: bool,
    /// The recent files, most recent first.
    pub recent: &'a [String],
    /// The open tabs' names, and which is in front.
    pub tabs: Vec<(String, bool)>,
}

/// The id of the item that opens the recent file at `index`.
const OPEN_RECENT: &str = "file.open_recent";
/// The id of the item that brings the tab at `index` to the front.
const SHOW_TAB: &str = "window.tab";
/// The start of the ids of the items that export the selection; the format's file ending follows.
const EXPORT: &str = "file.export";
/// The id of items for things this version can't do yet; they are shown greyed, in their place.
const LATER: &str = "later";

/// The ids of the plain items, with the command each runs.
const COMMANDS: [(&str, Command); 23] = [
    ("file.new", Command::NewFile),
    ("file.open", Command::OpenFile),
    ("file.home", Command::Dashboard),
    ("file.import", Command::Import),
    ("file.generate", Command::GenerateImage),
    ("file.save", Command::Save),
    ("file.save_as", Command::SaveAs),
    ("file.export_pen", Command::ExportPen),
    ("file.close_tab", Command::CloseTab),
    ("edit.undo", Command::Undo),
    ("edit.redo", Command::Redo),
    ("edit.cut", Command::Cut),
    ("edit.copy", Command::Copy),
    ("edit.paste", Command::Paste),
    ("edit.delete", Command::Delete),
    ("edit.select_all", Command::SelectAll),
    ("app.settings", Command::Settings),
    ("view.interface", Command::ToggleInterface),
    ("view.full_screen", Command::FullScreen),
    ("window.theme", Command::ToggleTheme),
    ("window.previous_tab", Command::PreviousTab),
    ("window.next_tab", Command::NextTab),
    ("window.actions", Command::Actions),
];

/// The command a chosen menu item runs, or `None` for an item that does nothing yet.
#[must_use]
pub fn command(chosen: &CommandRef) -> Option<Command> {
    let index = || chosen.params.get("index").and_then(serde_json::Value::as_u64).and_then(|i| usize::try_from(i).ok());
    match chosen.id.as_str() {
        OPEN_RECENT => index().map(Command::OpenRecent),
        SHOW_TAB => index().map(Command::ShowTab),
        "window.move_tab_left" => Some(Command::MoveTab(true)),
        "window.move_tab_right" => Some(Command::MoveTab(false)),
        // Human: `file.export.png` and its like; `file.export_pen` is a plain item.
        id if id.strip_prefix(EXPORT).is_some_and(|format| format.starts_with('.')) => {
            Format::ALL.into_iter().find(|format| id.strip_prefix(EXPORT) == Some(&format!(".{}", format.extension()))).map(Command::Export)
        }
        id => COMMANDS.iter().find(|(known, _)| *known == id).map(|(_, command)| *command),
    }
}

/// An item with a shortcut written the Mac way (`⇧⌘N`); a shortcut that doesn't parse is left off.
fn item(id: &str, label: &str, keys: &str, enabled: bool) -> Node {
    let item = Item::new(id, label).enabled(enabled);
    Node::Item(match Chord::parse(keys) {
        Ok(chord) if !keys.is_empty() => item.shortcut(chord),
        _ => item,
    })
}

/// An item this version can't do yet: shown, greyed.
fn later(label: &str) -> Node {
    Node::Item(Item::new(LATER, label).enabled(false))
}

/// The Help menu: links, none of which this version follows.
fn help_menu() -> Menu {
    Menu::new(
        "Help",
        vec![
            later("Documentation"),
            later("Prompt Gallery & Tips"),
            later("Website"),
            Node::Separator,
            later("Cursor Extension"),
            later("VSCode Extension"),
            Node::Separator,
            later("Join Our Discord"),
        ],
    )
    .role(MenuRole::Help)
}

/// The menus for `state`.
// Human: what this version can't do yet stays in its place, greyed, so the menus keep their order. Settings is tagged so the Mac
// layout moves it into the app menu as "Settings… ⌘,"; Quit, Hide, Minimize, Zoom and Bring All to Front are
// the layout's own.
#[must_use]
pub fn bar(state: &State<'_>) -> MenuBar {
    let (doc, picked) = (state.document, state.document && state.selection);
    let recent: Vec<Node> = if state.recent.is_empty() {
        vec![later("No Recent Files")]
    } else {
        state
            .recent
            .iter()
            .enumerate()
            .map(|(index, path)| {
                let name = std::path::Path::new(path).file_name().map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
                Node::Item(Item::new(CommandRef::with_params(OPEN_RECENT, json!({ "index": index })), name))
            })
            .collect()
    };
    let file = Menu::new(
        "File",
        vec![
            item("file.new", "New File", "⇧⌘N", true),
            item("file.open", "Open…", "⌘O", true),
            Node::submenu("Open Recent", recent),
            item("file.home", "Home", "⌘N", true),
            Node::Separator,
            item("file.import", "Import Image/SVG/Figma…", "", doc),
            item("file.generate", "Generate Image…", "", doc),
            // Human: with nothing selected the export takes every top-level layer, so it is there whenever a
            // document is open.
            Node::submenu(
                "Export Selection to…",
                Format::ALL.map(|format| item(&format!("{EXPORT}.{}", format.extension()), format.label(), "", doc)).to_vec(),
            ),
            later("Export Snapshot…"),
            Node::Separator,
            later("Guide: How to Import from Figma…"),
            Node::Separator,
            later("Install command-line tool…"),
            Node::Separator,
            item("file.save", "Save", "⌘S", doc),
            item("file.save_as", "Save As…", "⇧⌘S", doc),
            // Human: a document is saved as a `.quill` file; a `.pen` file, which other programs open, is exported.
            item("file.export_pen", "Export .pen File…", "", doc),
            Node::Separator,
            item("file.close_tab", "Close Tab", "⌘W", doc),
        ],
    )
    .role(MenuRole::File);
    let edit = Menu::new(
        "Edit",
        vec![
            item("edit.undo", "Undo", "⌘Z", doc),
            item("edit.redo", "Redo", "⇧⌘Z", doc),
            Node::Separator,
            item("edit.cut", "Cut", "⌘X", picked),
            item("edit.copy", "Copy", "⌘C", picked),
            item("edit.paste", "Paste", "⌘V", doc),
            item("edit.delete", "Delete", "", picked),
            Node::Separator,
            item("edit.select_all", "Select All", "⌘A", doc),
            Node::Item(Item::new("app.settings", "Settings…").role(ItemRole::Settings)),
        ],
    )
    .role(MenuRole::Edit);
    let view = Menu::new(
        "View",
        vec![
            item("view.interface", "Show/Hide UI", "⌘\\", true),
            Node::submenu("Chat Text Size", ["Smaller", "Default", "Larger"].map(later).to_vec()),
            item("window.actions", "Search Actions…", "⌘P", true),
            Node::Separator,
            item("view.full_screen", "Toggle Full Screen", "⌃⌘F", true),
        ],
    )
    .role(MenuRole::View);
    let several = state.tabs.len() > 1;
    let window = Menu::new(
        "Window",
        vec![
            item("window.theme", "Toggle Light/Dark Mode", "", true),
            Node::Separator,
            item("window.previous_tab", "Show Previous Tab", "⇧⌘[", several),
            item("window.next_tab", "Show Next Tab", "⇧⌘]", several),
            item("window.move_tab_left", "Move Tab Left", "⌃⇧PageUp", several),
            item("window.move_tab_right", "Move Tab Right", "⌃⇧PageDown", several),
        ],
    )
    .role(MenuRole::Window);
    let help = help_menu();
    let documents = state
        .tabs
        .iter()
        .enumerate()
        .map(|(index, (name, front))| Document::new(CommandRef::with_params(SHOW_TAB, json!({ "index": index })), name.clone()).active(*front))
        .collect();
    MenuBar { menus: vec![file, edit, view, window, help], documents }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(menu: &Menu) -> Vec<String> {
        menu.children
            .iter()
            .map(|node| match node {
                Node::Item(item) => item.label.clone(),
                Node::Separator => "—".to_owned(),
                Node::Submenu { label, .. } => format!("{label} ▸"),
                Node::Standard { label, .. } => label.clone(),
            })
            .collect()
    }

    #[test]
    fn the_menus_keep_their_items_and_order() {
        let recent = ["/designs/site.pen".to_owned()];
        let bar = bar(&State { document: true, selection: false, recent: &recent, tabs: vec![("site".to_owned(), true), ("app".to_owned(), false)] });
        assert_eq!(bar.menus.iter().map(|m| m.title.as_str()).collect::<Vec<_>>(), ["File", "Edit", "View", "Window", "Help"]);
        assert_eq!(
            labels(&bar.menus[0]),
            [
                "New File",
                "Open…",
                "Open Recent ▸",
                "Home",
                "—",
                "Import Image/SVG/Figma…",
                "Generate Image…",
                "Export Selection to… ▸",
                "Export Snapshot…",
                "—",
                "Guide: How to Import from Figma…",
                "—",
                "Install command-line tool…",
                "—",
                "Save",
                "Save As…",
                "Export .pen File…",
                "—",
                "Close Tab"
            ]
        );
        assert_eq!(labels(&bar.menus[1])[..9], ["Undo", "Redo", "—", "Cut", "Copy", "Paste", "Delete", "—", "Select All"]);
        assert_eq!(labels(&bar.menus[3]), ["Toggle Light/Dark Mode", "—", "Show Previous Tab", "Show Next Tab", "Move Tab Left", "Move Tab Right"]);
        // The open tabs are listed in the Window menu, the one in front marked.
        assert_eq!(bar.documents.iter().map(|d| (d.label.as_str(), d.active)).collect::<Vec<_>>(), [("site", true), ("app", false)]);
        // Every shortcut written here parses, and every item that isn't greyed runs something.
        for item in bar.items() {
            assert!(!item.enabled || command(&item.command).is_some(), "{}", item.label);
        }
        let shortcut = |id: &str| bar.find(&CommandRef::new(id)).and_then(|item| item.shortcut);
        assert!(shortcut("file.new").is_some() && shortcut("window.move_tab_left").is_some() && shortcut("view.full_screen").is_some());
    }

    #[test]
    fn chosen_items_become_commands() {
        assert_eq!(command(&CommandRef::new("file.save")), Some(Command::Save));
        assert_eq!(command(&CommandRef::with_params(OPEN_RECENT, json!({ "index": 2 }))), Some(Command::OpenRecent(2)));
        assert_eq!(command(&CommandRef::with_params(SHOW_TAB, json!({ "index": 0 }))), Some(Command::ShowTab(0)));
        assert_eq!(command(&CommandRef::new("window.move_tab_left")), Some(Command::MoveTab(true)));
        assert_eq!(command(&CommandRef::new(LATER)), None);
        assert_eq!(command(&CommandRef::new("file.export.jpg")), Some(Command::Export(Format::Jpeg)));
        assert_eq!(command(&CommandRef::new("file.export.gif")), None);
        // Without anything selected, Cut, Copy and Delete are greyed; with no document so is Save.
        let none = bar(&State { document: false, selection: false, recent: &[], tabs: Vec::new() });
        let enabled = |id: &str| none.find(&CommandRef::new(id)).is_some_and(|item| item.enabled);
        assert!(!enabled("file.save") && !enabled("edit.copy") && enabled("file.open") && enabled("file.new"));
        // A `.pen` file is exported, from the document in front.
        assert_eq!(command(&CommandRef::new("file.export_pen")), Some(Command::ExportPen));
        assert!(!enabled("file.export_pen") && !enabled("file.save_as"));
        let open = super::bar(&State { document: true, selection: false, recent: &[], tabs: Vec::new() });
        assert!(open.find(&CommandRef::new("file.export_pen")).is_some_and(|item| item.enabled && item.label == "Export .pen File…"));
    }
}
