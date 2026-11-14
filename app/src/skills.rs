//! Skills: what Pencraft teaches an AI model about one kind of design work, beyond the general guide.
//!
//! A skill is a folder in the open Agent Skills layout (`app/skills/<name>/SKILL.md` with `name` and
//! `description` up front, and reference files beside it). The folders are built into the program, so every
//! way a model reaches a skill gives it the same text: the `list_skills` and `read_skill` tools, the index in
//! the guide, and the copies `pencraft skills install` writes where AI clients look for skills.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

/// A skill's main file.
const MAIN: &str = "SKILL.md";
/// The folders under a home or project folder where AI clients look for skills, each with who reads it.
// Human: from each client's own documentation, read on 14 November 2026: `.agents/skills` is read by Codex,
// Cursor, Gemini CLI, OpenCode and GitHub Copilot; `.claude/skills` by Claude Code (and Cursor, OpenCode and
// VS Code too). Two copies reach all of them.
const PLACES: [(&str, &str); 2] = [(".agents/skills", "Codex, Cursor, Gemini CLI, OpenCode, GitHub Copilot"), (".claude/skills", "Claude Code")];
const USAGE: &str = "usage: pencraft skills                      list the skills\n       pencraft skills show NAME [FILE]     print a skill's main file, or one of its other files\n       pencraft skills install [--project DIR | --dir DIR]\n                                            write the skills where AI clients look for them: in your home\n                                            folder, in a project's folder, or into one folder you name";

/// One skill: its name and its files.
pub struct Skill {
    /// The name, which is also its folder's.
    pub name: &'static str,
    /// Its files: path inside the folder, and text. The first is `SKILL.md`.
    files: &'static [(&'static str, &'static str)],
}

/// Every skill Pencraft has.
pub const ALL: &[Skill] = &[Skill {
    name: "liquid-glass",
    files: &[
        (MAIN, include_str!("../skills/liquid-glass/SKILL.md")),
        ("references/components.md", include_str!("../skills/liquid-glass/references/components.md")),
        ("references/tokens.md", include_str!("../skills/liquid-glass/references/tokens.md")),
        ("references/design-language.md", include_str!("../skills/liquid-glass/references/design-language.md")),
    ],
}];

impl Skill {
    /// The value of `key` in the main file's front matter.
    fn front(&self, key: &str) -> Option<&'static str> {
        let (_, main) = self.files.first()?;
        let matter = main.strip_prefix("---")?.split("\n---").next()?;
        matter.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix(':')).map(str::trim)
    }

    /// When to use it, in the skill's own words.
    #[must_use]
    pub fn description(&self) -> &'static str {
        self.front("description").unwrap_or("")
    }

    /// The text of one of its files; the main file without a path.
    #[must_use]
    pub fn file(&self, path: Option<&str>) -> Option<&'static str> {
        let path = path.map_or(MAIN, |path| path.trim_start_matches("./"));
        self.files.iter().find(|(name, _)| *name == path).map(|(_, text)| *text)
    }

    /// The paths of its files.
    pub fn paths(&self) -> impl Iterator<Item = &'static str> {
        self.files.iter().map(|(path, _)| *path)
    }
}

/// The skill called `name`.
#[must_use]
pub fn find(name: &str) -> Option<&'static Skill> {
    ALL.iter().find(|skill| skill.name == name)
}

/// The part of the guide that says which skills there are and when to read one.
#[must_use]
pub fn index() -> String {
    let mut out = String::from(
        "## Skills\nFor some kinds of design Pencraft has a skill: what to decide, how to build it here, and how to check it. When a request fits one, read it with `read_skill` before you design, and follow it.\n",
    );
    for skill in ALL {
        out.push_str("- `");
        out.push_str(skill.name);
        out.push_str("`: ");
        out.push_str(skill.description());
        out.push('\n');
    }
    out
}

/// `list_skills`: each skill with when to use it and the files it has.
#[must_use]
pub fn list() -> Value {
    json!({ "skills": ALL.iter().map(|skill| json!({ "name": skill.name, "description": skill.description(), "files": skill.paths().collect::<Vec<_>>() })).collect::<Vec<_>>() })
}

/// `read_skill`: the text of a skill's main file, or of the file `file` names.
///
/// # Errors
/// A sentence naming the skills or the files there are, when the one asked for isn't among them.
pub fn read(args: &Value) -> Result<&'static str, String> {
    let names = || ALL.iter().map(|skill| format!("\"{}\"", skill.name)).collect::<Vec<_>>().join(", ");
    let name = args.get("name").and_then(Value::as_str).ok_or_else(|| format!("`read_skill` needs the skill's `name`: {}", names()))?;
    let skill = find(name).ok_or_else(|| format!("There is no skill \"{name}\". The skills are: {}.", names()))?;
    let file = args.get("file").and_then(Value::as_str);
    skill.file(file).ok_or_else(|| {
        let files = skill.paths().map(|path| format!("\"{path}\"")).collect::<Vec<_>>().join(", ");
        format!("The skill \"{name}\" has no file \"{}\". Its files are: {files}.", file.unwrap_or(MAIN))
    })
}

/// Writes every skill into `root` (a folder of skills), one folder each. Returns the main files written.
///
/// # Errors
/// When a folder or file can't be written.
// Agent: WRITES `<root>/<skill>/...`, replacing those files; touches nothing else in `root`.
pub fn install(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut written = Vec::new();
    for skill in ALL {
        let folder = root.join(skill.name);
        for (path, text) in skill.files {
            let file = folder.join(path);
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file, text)?;
        }
        written.push(folder.join(MAIN));
    }
    Ok(written)
}

#[expect(clippy::print_stdout, reason = "the answer of a command typed in a terminal")]
fn say(text: &str) {
    println!("{text}");
}

#[expect(clippy::print_stderr, reason = "the error of a command typed in a terminal")]
fn complain(text: &str) {
    eprintln!("{text}");
}

/// The user's home folder.
fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// `pencraft skills ...`: lists the skills, prints one, or writes them where AI clients look for skills.
// Agent: WRITES only for `install`, and only into the skills folders it then names.
#[must_use]
pub fn command(args: &[String]) -> ExitCode {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        [] | ["list"] => {
            for skill in ALL {
                say(&format!("{}\n    {}\n    files: {}", skill.name, skill.description(), skill.paths().collect::<Vec<_>>().join(", ")));
            }
            ExitCode::SUCCESS
        }
        ["show", name] | ["show", name, _] => match read(&json!({ "name": name, "file": words.get(2) })) {
            Ok(text) => {
                say(text);
                ExitCode::SUCCESS
            }
            Err(error) => {
                complain(&error);
                ExitCode::from(2)
            }
        },
        ["install"] | ["install", "--project" | "--dir", _] => {
            let roots: Vec<(PathBuf, &str)> = match (words.get(1).copied(), words.get(2)) {
                (Some("--dir"), Some(folder)) => vec![(PathBuf::from(folder), "the folder you named")],
                (Some("--project"), Some(folder)) => PLACES.iter().map(|(place, who)| (Path::new(folder).join(place), *who)).collect(),
                _ => {
                    let Some(home) = home() else {
                        complain("Couldn't find your home folder. Name a folder: pencraft skills install --dir DIR");
                        return ExitCode::from(2);
                    };
                    PLACES.iter().map(|(place, who)| (home.join(place), *who)).collect()
                }
            };
            for (root, who) in roots {
                match install(&root) {
                    Ok(written) => {
                        for file in written {
                            say(&format!("{}    (read by {who})", file.display()));
                        }
                    }
                    Err(error) => {
                        complain(&format!("Couldn't write into {}: {error}", root.display()));
                        return ExitCode::FAILURE;
                    }
                }
            }
            ExitCode::SUCCESS
        }
        _ => {
            complain(USAGE);
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pen_doc::model::Node;

    /// The fenced `json` blocks of a markdown text.
    fn blocks(text: &str) -> Vec<&str> {
        text.split("```json\n").skip(1).filter_map(|rest| rest.split("\n```").next()).collect()
    }

    /// Gives every layer of `value` that has none an id, as `edit_layers` does.
    fn give_ids(value: &mut Value, next: &mut usize) {
        if let Some(layer) = value.as_object_mut() {
            *next += 1;
            layer.entry("id").or_insert_with(|| json!(format!("made-{next}")));
            for child in layer.get_mut("children").and_then(Value::as_array_mut).into_iter().flatten() {
                give_ids(child, next);
            }
        }
    }

    fn walk(node: &Node, visit: &mut impl FnMut(&Node)) {
        visit(node);
        for child in node.kids() {
            walk(child, visit);
        }
    }

    #[test]
    fn every_skill_keeps_to_the_agent_skills_format_all_clients_read() {
        for skill in ALL {
            let main = skill.file(None).unwrap();
            // The spec's rules, which the strictest clients enforce: the front matter opens the file, the name
            // is the folder's (lowercase letters, digits and single hyphens, at most 64), the description at
            // most 1024 characters, the main file under 500 lines.
            assert!(main.starts_with("---"), "{}", skill.name);
            assert_eq!(skill.front("name"), Some(skill.name));
            let sound = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-';
            assert!(skill.name.len() <= 64 && skill.name.chars().all(sound) && !skill.name.contains("--"));
            assert!(!skill.name.starts_with('-') && !skill.name.ends_with('-'));
            let description = skill.description();
            assert!((40..=1024).contains(&description.chars().count()), "{}", description.len());
            assert!(!description.contains('<') && !description.contains('>'));
            assert!(main.lines().count() < 500);
            // Every file the texts name is there, and every file there is named by the main file.
            for (path, text) in skill.files {
                for named in
                    text.split('`').filter(|word| word.starts_with("references/") && Path::new(word).extension().is_some_and(|ending| ending == "md"))
                {
                    assert!(skill.file(Some(named)).is_some(), "{path} names {named}, which the skill doesn't have");
                }
                assert!(*path == MAIN || main.contains(path), "{path} is never named in {MAIN}");
            }
        }
    }

    #[test]
    fn the_layers_a_skill_shows_are_layers_pencraft_takes() {
        let mut layers = 0;
        for skill in ALL {
            for (path, text) in skill.files {
                for block in blocks(text) {
                    // An example with a gap to fill in, or several layers in a row, is not one layer.
                    if block.contains("...") || block.contains('<') || block.lines().filter(|line| line.starts_with('{')).count() > 1 {
                        continue;
                    }
                    let value: Value = serde_json::from_str(block).unwrap_or_else(|error| {
                        panic!(
                            "{path}: {error}
{block}"
                        )
                    });
                    if value.get("type").is_none() {
                        continue;
                    }
                    let mut value = value;
                    give_ids(&mut value, &mut 0);
                    let node: Node = serde_json::from_value(value).unwrap_or_else(|error| panic!("{path}: {error}\n{block}"));
                    walk(&node, &mut |layer| {
                        layers += 1;
                        let wrong = layer.material.as_ref().map(crate::material::problems).unwrap_or_default();
                        assert_eq!(wrong, Vec::<String>::new(), "{path}: layer {}", layer.id);
                        let group =
                            layer.extra.get(crate::glass_check::GROUP).and_then(|group| crate::glass_check::group_problem(Some(&layer.kind), group));
                        assert_eq!(group, None, "{path}: layer {}", layer.id);
                    });
                }
            }
        }
        assert!(layers > 50, "only {layers} layers were found in the skills' examples");
    }

    #[test]
    fn a_skill_is_listed_read_and_written_out_whole() {
        assert_eq!(list()["skills"][0]["name"], "liquid-glass");
        assert!(index().contains("- `liquid-glass`: Design Liquid Glass interfaces"));
        assert!(read(&json!({ "name": "liquid-glass" })).unwrap().contains("# Liquid glass in Pencraft"));
        assert!(read(&json!({ "name": "liquid-glass", "file": "references/tokens.md" })).unwrap().starts_with("# Tokens"));
        assert!(read(&json!({ "name": "glass" })).unwrap_err().contains("The skills are: \"liquid-glass\""));
        assert!(read(&json!({ "name": "liquid-glass", "file": "x.md" })).unwrap_err().contains("\"references/components.md\""));
        assert!(read(&json!({})).unwrap_err().contains("needs the skill's `name`"));

        let root = std::env::temp_dir().join(format!("pencraft-skills-test-{}", std::process::id()));
        let written = install(&root).unwrap();
        assert_eq!(written, [root.join("liquid-glass").join(MAIN)]);
        for (path, text) in ALL[0].files {
            assert_eq!(std::fs::read_to_string(root.join("liquid-glass").join(path)).unwrap(), *text);
        }
        std::fs::remove_dir_all(&root).unwrap();
    }
}
