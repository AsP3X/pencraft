//! The notes the chat shows for what the assistant does: one for each tool call.
//!
//! A note says what is happening while the tool runs ("Changing the design…"), then what was done, or that it
//! didn't work. Where the call itself says more, the note does too: the model's own few words on a change,
//! how many layers it read, the name of the file it wrote.

use serde_json::Value;

/// How far a tool call has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The tool is being called or is at work.
    Running,
    /// It did what was asked.
    Done,
    /// It answered with an error; the model usually tries again.
    Failed,
}

/// What a note knows about its tool call besides its words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The tool's name, without the server Claude Code puts before it.
    pub tool: String,
    /// The call the note is for now (the latest one, when several are told as one).
    pub id: String,
    /// More about the call in a few words ("3 layers", a file's name); empty when there is nothing to add.
    pub detail: String,
    /// How far the call has got.
    pub state: State,
    /// How many calls in a row this one note stands for.
    pub times: u32,
    /// The note's words are the model's own description of the change, not the tool's usual words.
    pub named: bool,
}

/// The words and the icon of a tool.
struct Words {
    /// The tool's name.
    tool: &'static str,
    /// Its icon in the Lucide set.
    icon: &'static str,
    /// While it runs.
    doing: &'static str,
    /// When it is done.
    did: &'static str,
    /// When it didn't work.
    failed: &'static str,
}

/// The tools the app gives a model, as the chat names them.
const TOOLS: [Words; 20] = [
    Words {
        tool: "get_state",
        icon: "eye",
        doing: "Looking at the document",
        did: "Looked at the document",
        failed: "Couldn't look at the document",
    },
    Words { tool: "get_guide", icon: "book-open", doing: "Reading the guide", did: "Read the guide", failed: "Couldn't read the guide" },
    Words {
        tool: "list_skills",
        icon: "book-open",
        doing: "Looking up the skills",
        did: "Looked up the skills",
        failed: "Couldn't look up the skills",
    },
    Words { tool: "read_skill", icon: "book-open", doing: "Reading a skill", did: "Read a skill", failed: "Couldn't read the skill" },
    Words { tool: "read_layers", icon: "book-open-text", doing: "Reading layers", did: "Read layers", failed: "Couldn't read the layers" },
    Words { tool: "edit_layers", icon: "pencil", doing: "Changing the design", did: "Changed the design", failed: "Couldn't change the design" },
    Words {
        tool: "screenshot",
        icon: "camera",
        doing: "Taking a picture to check",
        did: "Took a picture to check",
        failed: "Couldn't take a picture",
    },
    Words { tool: "check_glass", icon: "glasses", doing: "Checking the glass", did: "Checked the glass", failed: "Couldn't check the glass" },
    Words { tool: "select_layers", icon: "mouse-pointer-2", doing: "Selecting layers", did: "Selected layers", failed: "Couldn't select the layers" },
    Words { tool: "open_document", icon: "folder-open", doing: "Opening a document", did: "Opened a document", failed: "Couldn't open the document" },
    Words { tool: "save_document", icon: "save", doing: "Saving the document", did: "Saved the document", failed: "Couldn't save the document" },
    Words { tool: "export_layers", icon: "download", doing: "Exporting layers", did: "Exported layers", failed: "Couldn't export the layers" },
    Words {
        tool: "generate_image",
        icon: "image-plus",
        doing: "Generating a picture",
        did: "Generated a picture",
        failed: "Couldn't generate the picture",
    },
    Words { tool: "edit_image", icon: "wand-sparkles", doing: "Editing a picture", did: "Edited a picture", failed: "Couldn't edit the picture" },
    Words { tool: "search_web", icon: "search", doing: "Searching the web", did: "Searched the web", failed: "Couldn't search the web" },
    Words { tool: "read_web_page", icon: "globe", doing: "Reading a web page", did: "Read a web page", failed: "Couldn't read the web page" },
    Words { tool: "search_images", icon: "image", doing: "Looking for pictures", did: "Looked for pictures", failed: "Couldn't look for pictures" },
    Words { tool: "download_image", icon: "download", doing: "Fetching a picture", did: "Fetched a picture", failed: "Couldn't fetch the picture" },
    Words { tool: "ask_user", icon: "message-circle-question-mark", doing: "Asking you", did: "Asked you", failed: "Couldn't ask you" },
    // Human: not a tool the model calls: the app has a reviewer look at a finished design ([`super::critic`]).
    Words {
        tool: "review_design",
        icon: "eye",
        doing: "A reviewer is looking at the design",
        did: "A reviewer looked at the design",
        failed: "The design couldn't be reviewed",
    },
];

/// The icon of a tool this app doesn't know by name.
const OTHER_ICON: &str = "wrench";
/// The icon of a call that didn't work.
pub const FAILED_ICON: &str = "circle-x";
/// The icon that says the assistant's answer is there.
pub const ANSWERED_ICON: &str = "check";
/// The icon of next steps the assistant offers.
pub const SUGGESTED_ICON: &str = "lightbulb";
/// The icon of an answer the user gave to a question.
pub const PICKED_ICON: &str = "corner-down-right";
/// The icon that closes a question.
pub const CLOSE_ICON: &str = "x";
/// The icon of a note that isn't about a tool ("Stopped").
pub const PLAIN_ICON: &str = "sparkle";
/// The most letters of the model's own description a note shows.
const MOST_NAME: usize = 80;

/// A tool's name without the server before it: Claude Code calls an MCP tool `mcp__<server>__<tool>`.
#[must_use]
pub fn short(tool: &str) -> &str {
    tool.rsplit("__").next().unwrap_or(tool)
}

fn words(tool: &str) -> Option<&'static Words> {
    TOOLS.iter().find(|words| words.tool == tool)
}

/// The icon of the tool called `tool`.
#[must_use]
pub fn icon(tool: &str) -> &'static str {
    words(tool).map_or(OTHER_ICON, |words| words.icon)
}

/// What the tool called `tool` does, for the tools whose icon moves in a way of its own while they work:
/// reading turns the pages of a book, looking at the design moves an eye, changing it moves a pencil.
#[must_use]
pub fn act(tool: &str) -> Option<crate::tool_icons::Act> {
    use crate::tool_icons::Act;
    match tool {
        "get_guide" | "read_layers" => Some(Act::Reading),
        "get_state" => Some(Act::Looking),
        "edit_layers" => Some(Act::Changing),
        _ => None,
    }
}

/// What a note says while `tool` runs.
#[must_use]
pub fn doing(tool: &str) -> String {
    words(tool).map_or_else(|| format!("Using {tool}"), |words| words.doing.to_owned())
}

/// What a note says when `tool` is done.
#[must_use]
pub fn did(tool: &str) -> String {
    words(tool).map_or_else(|| format!("Used {tool}"), |words| words.did.to_owned())
}

/// What a note says when `tool` didn't work.
#[must_use]
pub fn failed(tool: &str) -> String {
    words(tool).map_or_else(|| format!("Couldn't use {tool}"), |words| words.failed.to_owned())
}

/// "1 layer", "3 layers".
fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// The last part of a path, whichever system wrote it.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The most letters of a picture's description a note shows.
const MOST_PROMPT: usize = 36;

/// What a call of `tool` with `input` is about, in a few words; empty when the tool's usual words say it all.
#[must_use]
pub fn detail(tool: &str, input: &Value) -> String {
    let ids = input.get("ids").and_then(Value::as_array).map_or(0, Vec::len);
    let layers = || if ids == 0 { String::new() } else { counted(ids, "layer", "layers") };
    let path = || input.get("path").and_then(Value::as_str).map(file_name).unwrap_or_default().to_owned();
    match tool {
        "read_layers" => match input.get("find").and_then(Value::as_str).map(str::trim) {
            Some(find) if !find.is_empty() => format!("“{find}”"),
            _ => layers(),
        },
        "screenshot" | "select_layers" | "check_glass" => layers(),
        "edit_layers" => match input.get("operations").and_then(Value::as_array).map_or(0, Vec::len) {
            0 | 1 => String::new(),
            changes => counted(changes, "change", "changes"),
        },
        "open_document" | "save_document" | "export_layers" => path(),
        "generate_image" => quoted(input.get("prompt")),
        "search_web" | "search_images" => quoted(input.get("query")),
        // Human: a page or a picture is told by the site it is on; its whole address is too long for a note.
        "read_web_page" | "download_image" => {
            let url = input.get("url").and_then(Value::as_str).unwrap_or_default();
            url.split_once("://").map_or(url, |(_, rest)| rest).split(['/', '?', '#']).next().unwrap_or_default().to_owned()
        }
        "edit_image" => quoted(input.get("instruction")),
        _ => String::new(),
    }
}

/// The start of a picture's description (or of what to change in one), in quotes; empty when there is none.
fn quoted(said: Option<&Value>) -> String {
    // Human: white space (new lines too) is one space, as in the model's description of a change.
    let prompt = said.and_then(Value::as_str).unwrap_or_default().split_whitespace().collect::<Vec<_>>().join(" ");
    if prompt.chars().count() > MOST_PROMPT {
        // Human: cut at the last whole word that fits; a word that ends just where the cut falls fits.
        let mut letters = prompt.chars();
        let cut: String = letters.by_ref().take(MOST_PROMPT).collect();
        let whole = if letters.next() == Some(' ') { cut.as_str() } else { cut.rsplit_once(' ').map_or(cut.as_str(), |(whole, _)| whole) };
        format!("“{}…”", whole.trim_end_matches(','))
    } else if prompt.is_empty() {
        String::new()
    } else {
        format!("“{prompt}”")
    }
}

/// The model's own few words on what a call of `tool` does, tidied for a note; `None` when it gave none.
// Human: only a change of the design comes with a description (the tool asks for one, "shown to the user").
#[must_use]
pub fn name(tool: &str, input: &Value) -> Option<String> {
    if tool != "edit_layers" {
        return None;
    }
    let said = input.get("description").and_then(Value::as_str)?.split_whitespace().collect::<Vec<_>>().join(" ");
    let said = said.trim_end_matches(['.', '…']);
    let mut letters = said.chars();
    let first = letters.next()?;
    let mut name: String = first.to_uppercase().chain(letters).collect();
    if name.chars().count() > MOST_NAME {
        name = name.chars().take(MOST_NAME - 1).chain(['…']).collect();
    }
    Some(name)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_web_tools_say_what_was_searched_for_and_which_site_was_read() {
        assert_eq!(detail("search_web", &json!({ "query": "Winterdom opening hours" })), "“Winterdom opening hours”");
        assert_eq!(detail("read_web_page", &json!({ "url": "https://www.hamburg.de/freizeit/dom?x=1" })), "www.hamburg.de");
        assert_eq!(detail("download_image", &json!({ "url": "https://live.staticflickr.com/1/2_b.jpg", "name": "hall" })), "live.staticflickr.com");
        assert_eq!((did("search_images"), failed("download_image")), ("Looked for pictures".to_owned(), "Couldn't fetch the picture".to_owned()));
    }

    #[test]
    fn every_tool_the_app_offers_has_words_and_a_bundled_icon() {
        let offered = crate::agent::tools();
        for tool in offered.as_array().unwrap() {
            let name = tool["name"].as_str().unwrap();
            assert!(words(name).is_some(), "{name}");
        }
        for icon in
            TOOLS.iter().map(|words| words.icon).chain([OTHER_ICON, FAILED_ICON, PLAIN_ICON, ANSWERED_ICON, SUGGESTED_ICON, PICKED_ICON, CLOSE_ICON])
        {
            assert!(crate::icons::outline("lucide", icon).is_some(), "{icon}");
        }
    }

    #[test]
    fn reading_looking_and_changing_move_in_their_own_way() {
        use crate::tool_icons::Act;
        assert_eq!((act("get_guide"), act("read_layers")), (Some(Act::Reading), Some(Act::Reading)));
        assert_eq!((act("get_state"), act("edit_layers")), (Some(Act::Looking), Some(Act::Changing)));
        assert_eq!((act("screenshot"), act("magic")), (None, None));
        // Each rests as the bundled icon of what it was doing.
        assert_eq!((icon("get_guide"), icon("get_state"), icon("edit_layers")), ("book-open", "eye", "pencil"));
    }

    #[test]
    fn a_tool_is_named_while_it_runs_when_done_and_when_it_failed() {
        assert_eq!(short("mcp__pencraft__edit_layers"), "edit_layers");
        assert_eq!(
            (doing("edit_layers"), did("edit_layers"), failed("edit_layers")),
            ("Changing the design".to_owned(), "Changed the design".to_owned(), "Couldn't change the design".to_owned())
        );
        assert_eq!(
            (doing("magic"), did("magic"), failed("magic"), icon("magic")),
            ("Using magic".to_owned(), "Used magic".to_owned(), "Couldn't use magic".to_owned(), OTHER_ICON)
        );
    }

    #[test]
    fn a_call_says_what_it_is_about() {
        assert_eq!(detail("read_layers", &json!({ "ids": ["a", "b", "c"] })), "3 layers");
        assert_eq!(detail("read_layers", &json!({ "ids": ["a"] })), "1 layer");
        assert_eq!(detail("read_layers", &json!({ "find": " Card " })), "“Card”");
        assert_eq!(detail("read_layers", &json!({})), "");
        assert_eq!(detail("edit_layers", &json!({ "operations": [{}, {}] })), "2 changes");
        assert_eq!(detail("edit_layers", &json!({ "operations": [{}] })), "");
        assert_eq!(detail("export_layers", &json!({ "path": "C:\\Users\\me\\Desktop\\dog.png" })), "dog.png");
        assert_eq!(detail("save_document", &json!({ "path": "/designs/site.pen" })), "site.pen");
        assert_eq!(detail("get_state", &json!({})), "");
    }

    #[test]
    fn generating_a_picture_is_named_with_the_start_of_its_description() {
        assert_eq!((doing("generate_image"), did("generate_image")), ("Generating a picture".to_owned(), "Generated a picture".to_owned()));
        assert_eq!(failed("generate_image"), "Couldn't generate the picture");
        assert_eq!(icon("generate_image"), "image-plus");
        assert_eq!(
            detail("generate_image", &json!({ "prompt": "A red fox in fresh snow at dawn, seen from the side" })),
            "“A red fox in fresh snow at dawn…”"
        );
        assert_eq!(detail("generate_image", &json!({ "prompt": "A fox" })), "“A fox”");
        // Human: a description written over several lines is shown on one.
        assert_eq!(detail("generate_image", &json!({ "prompt": " A red\n\n  fox\tin snow " })), "“A red fox in snow”");
        // Human: a cut that falls just after a word keeps that word.
        assert_eq!(
            detail("generate_image", &json!({ "prompt": "A red fox in fresh snow at dawn, the end of it" })),
            "“A red fox in fresh snow at dawn, the…”"
        );
    }

    #[test]
    fn editing_a_picture_is_named_with_the_start_of_its_instruction() {
        assert_eq!((doing("edit_image"), did("edit_image")), ("Editing a picture".to_owned(), "Edited a picture".to_owned()));
        assert_eq!(failed("edit_image"), "Couldn't edit the picture");
        assert_eq!(icon("edit_image"), "wand-sparkles");
        assert_eq!(
            detail("edit_image", &json!({ "instruction": "Make the sky a deep dusk blue and keep the people" })),
            "“Make the sky a deep dusk blue and…”"
        );
        assert_eq!(detail("edit_image", &json!({ "instruction": " Make it\n dusk " })), "“Make it dusk”");
        assert_eq!(detail("edit_image", &json!({})), "");
    }

    #[test]
    fn a_change_takes_the_models_own_words() {
        assert_eq!(name("edit_layers", &json!({ "description": "  draw the dog's\n head. " })).as_deref(), Some("Draw the dog's head"));
        assert_eq!(name("edit_layers", &json!({ "description": "   " })), None);
        assert_eq!(name("edit_layers", &json!({})), None);
        assert_eq!(name("read_layers", &json!({ "description": "x" })), None);
        let long = name("edit_layers", &json!({ "description": "a".repeat(200) })).unwrap();
        assert_eq!(long.chars().count(), MOST_NAME);
        assert!(long.ends_with('…'));
    }
}
