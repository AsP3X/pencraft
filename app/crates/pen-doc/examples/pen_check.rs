//! Reads `.pen` files, writes them back in memory, lays them out, and reports what it found.
//!
//! `cargo run -p pen-doc --example pen_check -- FILE...`: one line per file with node counts and whether a
//! second write equals the first (our own output must round-trip exactly), and whether the first write holds the
//! same JSON as the file that was read (nothing lost or altered when another program's file passes through).

use std::process::ExitCode;

use pen_doc::layout::{TextStyle, layout};
use pen_doc::vars::Vars;

fn main() -> ExitCode {
    let files: Vec<String> = std::env::args().skip(1).collect();
    if files.is_empty() {
        report(&["usage: pen_check FILE...".to_owned()]);
        return ExitCode::FAILURE;
    }
    let mut lines = Vec::new();
    let mut failed = false;
    for file in &files {
        match check(file) {
            Ok(line) => lines.push(line),
            Err(error) => {
                failed = true;
                lines.push(format!("{file}: {error}"));
            }
        }
    }
    report(&lines);
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

/// Every character half the size wide: enough to exercise the layout without fonts.
fn rough_measure(style: &TextStyle) -> (f32, f32) {
    let chars = style.content.chars().count().min(100_000);
    let width = f32::from(u16::try_from(chars).unwrap_or(u16::MAX)) * style.size * 0.5;
    match style.wrap {
        Some(wrap) if wrap > 0.0 => (wrap, (width / wrap).ceil().max(1.0) * style.size * 1.2),
        _ => (width, style.size * 1.2),
    }
}

fn check(file: &str) -> Result<String, Box<dyn std::error::Error>> {
    let text = std::fs::read_to_string(file)?;
    let doc = pen_doc::read(&text)?;
    let first = pen_doc::write(&doc)?;
    let second = pen_doc::write(&pen_doc::read(&first)?)?;
    let before: serde_json::Value = serde_json::from_str(&text)?;
    let after: serde_json::Value = serde_json::from_str(&first)?;
    let kept = match first_difference(&before, &after, "$") {
        None if first == text => "identical".to_owned(),
        None => "same data".to_owned(),
        Some(place) => format!("LOSSY at {place}"),
    };
    let mut scene = pen_doc::scene::build(&doc, &pen_doc::vars::ThemeContext::defaults(&doc));
    layout(&mut scene, Vars::new(&doc), &rough_measure);
    let mut nodes = 0usize;
    let mut unresolved = 0usize;
    let mut finite = true;
    for root in &scene {
        root.walk(
            &mut |n, _| {
                nodes += 1;
                unresolved += usize::from(!n.resolved);
                finite &= n.to_doc.m.iter().all(|v| v.is_finite()) && n.local.w.is_finite() && n.local.h.is_finite();
            },
            0,
        );
    }
    Ok(format!(
        "{file}: version {}, {} top-level, {nodes} scene nodes, {unresolved} missing components, round-trip {}, input {kept}, layout {}",
        doc.version,
        doc.children.len(),
        if first == second { "stable" } else { "UNSTABLE" },
        if finite { "finite" } else { "NOT FINITE" }
    ))
}

/// The path of the first place where two JSON values differ, or `None` when they hold the same data.
fn first_difference(a: &serde_json::Value, b: &serde_json::Value, at: &str) -> Option<String> {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (key, value) in x {
                match y.get(key) {
                    Some(other) => {
                        if let Some(place) = first_difference(value, other, &format!("{at}.{key}")) {
                            return Some(place);
                        }
                    }
                    None => return Some(format!("{at}.{key} (dropped)")),
                }
            }
            y.keys().find(|key| !x.contains_key(*key)).map(|key| format!("{at}.{key} (added)"))
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Some(format!("{at} (length {} became {})", x.len(), y.len()));
            }
            x.iter().zip(y).enumerate().find_map(|(i, (v, w))| first_difference(v, w, &format!("{at}[{i}]")))
        }
        (Value::Number(x), Value::Number(y)) => {
            let same = x == y || x.as_f64().zip(y.as_f64()).is_some_and(|(v, w)| v.to_bits() == w.to_bits());
            (!same).then(|| format!("{at} ({x} became {y})"))
        }
        _ => (a != b).then(|| format!("{at} ({a} became {b})")),
    }
}

#[expect(clippy::print_stdout, reason = "the tool's report is its output")]
fn report(lines: &[String]) {
    for line in lines {
        println!("{line}");
    }
}
