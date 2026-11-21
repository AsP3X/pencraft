//! The `.pen` design document without a window.
//!
//! - [`model`]: the document tree as the file stores it, read and written without losing keys.
//! - [`binary`]: the same tree as binary values, each key written once: what a `.quill` file holds.
//! - [`vars`]: variables and themes.
//! - [`scene`]: the document with component instances expanded.
//! - [`layout`]: flex layout, sizing and transforms.
//! - [`path`]: SVG path data.
//! - [`connector`]: lines between layers that follow them.
//! - [`edit`]: finding and moving nodes by id.
//! - [`stamp`]: Pencraft's own mark in a document, and the features only Pencraft draws.

pub mod binary;
pub mod color;
pub mod connector;
pub mod edit;
pub mod geom;
pub mod layout;
pub mod model;
pub mod path;
pub mod scene;
pub mod stamp;
pub mod value;
pub mod vars;

pub use model::{Document, Kind, Node};

/// Why a `.pen` file could not be read.
#[derive(Debug)]
pub struct ReadError {
    /// What the JSON reader said, with its line and column.
    pub source: serde_json::Error,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "not a readable .pen document: {}", self.source)
    }
}

impl std::error::Error for ReadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Reads a `.pen` document from its JSON text.
///
/// # Errors
/// [`ReadError`] when the text isn't JSON or lacks `version`; unknown properties are kept, not refused.
pub fn read(text: &str) -> Result<Document, ReadError> {
    serde_json::from_str(text).map_err(|source| ReadError { source })
}

/// Writes a document as JSON with two-space indents and no trailing newline.
///
/// # Errors
/// Only when a value can't be represented in JSON (never for documents built by [`read`]).
pub fn write(doc: &Document) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
  "version": "2.13",
  "children": [
    {
      "type": "frame",
      "id": "YKsGt",
      "x": 0,
      "y": 0,
      "name": "Welcome",
      "width": 1000,
      "height": 1458,
      "clip": true,
      "layout": "none",
      "children": [
        {
          "type": "path",
          "id": "AmD9T",
          "x": 267.8010819142764,
          "rotation": -45,
          "stroke": "#ff8302ff",
          "strokeWidth": 50,
          "geometry": "M7 0l0 14m7-7l-7 7-7-7",
          "futureProperty": {
            "kept": true
          }
        }
      ]
    }
  ],
  "themes": {
    "Mode": [
      "Light",
      "Dark"
    ]
  }
}"##;

    #[test]
    fn our_files_round_trip_byte_for_byte() {
        let doc = read(SAMPLE).unwrap();
        let written = write(&doc).unwrap();
        assert_eq!(written, SAMPLE);
        assert_eq!(write(&read(&written).unwrap()).unwrap(), written);
    }

    #[test]
    fn unknown_types_and_keys_survive() {
        let doc = read(r#"{"version":"9","children":[{"type":"hologram","id":"h","beam":3}],"plugins":[1]}"#).unwrap();
        assert_eq!(doc.children[0].kind, Kind::Other("hologram".into()));
        let text = write(&doc).unwrap();
        assert!(text.contains("\"hologram\"") && text.contains("\"beam\": 3") && text.contains("\"plugins\""));
        assert!(read("{}").is_err());
    }

    #[test]
    fn a_material_and_a_glass_group_round_trip_with_keys_this_version_does_not_know() {
        let text = r#"{
  "version": "2.22",
  "children": [
    {
      "type": "frame",
      "id": "bar",
      "children": [
        {
          "type": "frame",
          "id": "pill",
          "material": {
            "type": "liquid_glass",
            "variant": "clear",
            "tint": "$accent",
            "advanced": {
              "blur": 3
            },
            "future": true
          }
        },
        {
          "type": "rectangle",
          "id": "card",
          "material": "frosted"
        }
      ],
      "glassGroup": 8
    }
  ]
}"#;
        let doc = read(text).unwrap();
        let pill = &doc.children[0].kids()[0];
        assert_eq!(pill.material.as_ref().and_then(|m| m.get("variant")).and_then(|v| v.as_str()), Some("clear"));
        assert_eq!(write(&doc).unwrap(), text);
        // A file from before materials has none, and writes none.
        assert!(read(SAMPLE).unwrap().children[0].material.is_none());
        assert!(!write(&read(SAMPLE).unwrap()).unwrap().contains("material"));
    }
}
