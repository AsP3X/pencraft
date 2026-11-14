//! Starter documents offered on the home screen. Their content is this app's own sample copy.

/// A small component kit and a card built from it: five reusable components, their instances with overrides,
/// flex layouts, and variables for colour, corners and the face that follow the Light and Dark theme.
// Human: the starter is the first design a model sees in a new document, so it has to keep the rules the app
// teaches: everything from variables, one neutral palette, a hairline with a soft shadow only on the card,
// and no notes from the app's own review (a test checks that).
// Source: sizes follow what the component libraries agree on (shadcn/ui, Radix Themes): controls 36 high with
// 14 px labels at weight 500, 16 px icons, corners from one scale, lines as the text colour made transparent.
pub const STARTER: &str = r##"{
  "version": "2.22",
  "children": [
    {
      "type": "frame",
      "id": "kitBd",
      "x": 0,
      "y": 0,
      "name": "Components",
      "width": 720,
      "layout": "vertical",
      "gap": 32,
      "padding": 48,
      "fill": "$--bg",
      "children": [
        {
          "type": "frame",
          "id": "kitHd",
          "name": "Heading",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 8,
          "children": [
            { "type": "text", "id": "kitTt", "name": "Title", "fill": "$--fg", "content": "Starter kit", "fontFamily": "$--font", "fontSize": 24, "fontWeight": "600" },
            { "type": "text", "id": "kitSb", "name": "About", "fill": "$--muted", "textGrowth": "fixed-width", "width": 520, "content": "Five components built from the document's variables. Change a variable and every instance follows; switch Mode to Dark in the panel on the right.", "fontFamily": "$--font", "fontSize": 14, "lineHeight": 1.5 }
          ]
        },
        {
          "type": "frame",
          "id": "secBt",
          "name": "Buttons",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 12,
          "children": [
            { "type": "text", "id": "capBt", "fill": "$--muted", "content": "Button: primary, secondary, outline", "fontFamily": "$--font", "fontSize": 13, "fontWeight": "500" },
            {
              "type": "frame",
              "id": "rowBt",
              "name": "Row",
              "gap": 12,
              "alignItems": "center",
              "children": [
                {
                  "type": "frame",
                  "id": "btnCm",
                  "name": "Button",
                  "reusable": true,
                  "height": 36,
                  "padding": [0, 16],
                  "gap": 8,
                  "justifyContent": "center",
                  "alignItems": "center",
                  "fill": "$--primary",
                  "cornerRadius": "$--radius",
                  "children": [
                    { "type": "text", "id": "btnLb", "name": "Label", "fill": "$--on-primary", "content": "Save changes", "fontFamily": "$--font", "fontSize": 14, "fontWeight": "500" }
                  ]
                },
                { "type": "ref", "id": "btnSc", "ref": "btnCm", "name": "Button, secondary", "fill": "$--secondary", "descendants": { "btnLb": { "content": "Duplicate", "fill": "$--fg" } } },
                { "type": "ref", "id": "btnOl", "ref": "btnCm", "name": "Button, outline", "fill": "$--card", "stroke": "$--line-strong", "strokeWidth": 1, "strokeAlignment": "inner", "descendants": { "btnLb": { "content": "Cancel", "fill": "$--fg" } } }
              ]
            }
          ]
        },
        {
          "type": "frame",
          "id": "secFd",
          "name": "Fields",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 12,
          "children": [
            { "type": "text", "id": "capFd", "fill": "$--muted", "content": "Input and select", "fontFamily": "$--font", "fontSize": 13, "fontWeight": "500" },
            {
              "type": "frame",
              "id": "rowFd",
              "name": "Row",
              "gap": 16,
              "children": [
                {
                  "type": "frame",
                  "id": "inpCm",
                  "name": "Input",
                  "reusable": true,
                  "width": 240,
                  "height": 36,
                  "padding": [0, 12],
                  "alignItems": "center",
                  "stroke": "$--line-strong",
                  "strokeWidth": 1,
                  "strokeAlignment": "inner",
                  "cornerRadius": "$--radius",
                  "children": [
                    { "type": "text", "id": "inpTx", "name": "Value", "fill": "$--muted", "content": "name@company.com", "fontFamily": "$--font", "fontSize": 14 }
                  ]
                },
                {
                  "type": "frame",
                  "id": "selCm",
                  "name": "Select",
                  "reusable": true,
                  "width": 240,
                  "height": 36,
                  "padding": [0, 12],
                  "gap": 8,
                  "justifyContent": "space_between",
                  "alignItems": "center",
                  "stroke": "$--line-strong",
                  "strokeWidth": 1,
                  "strokeAlignment": "inner",
                  "cornerRadius": "$--radius",
                  "children": [
                    { "type": "text", "id": "selTx", "name": "Value", "fill": "$--fg", "content": "Editor", "fontFamily": "$--font", "fontSize": 14 },
                    { "type": "icon", "id": "selIc", "name": "Chevron", "library": "lucide", "icon": "chevron-down", "width": 16, "height": 16, "fill": "$--muted" }
                  ]
                }
              ]
            }
          ]
        },
        {
          "type": "frame",
          "id": "secCh",
          "name": "Choices",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 12,
          "children": [
            { "type": "text", "id": "capCh", "fill": "$--muted", "content": "Checkbox and switch", "fontFamily": "$--font", "fontSize": 13, "fontWeight": "500" },
            {
              "type": "frame",
              "id": "rowCh",
              "name": "Row",
              "gap": 32,
              "alignItems": "center",
              "children": [
                {
                  "type": "frame",
                  "id": "chkCm",
                  "name": "Checkbox",
                  "reusable": true,
                  "gap": 8,
                  "alignItems": "center",
                  "children": [
                    {
                      "type": "frame",
                      "id": "chkBx",
                      "name": "Box",
                      "width": 16,
                      "height": 16,
                      "justifyContent": "center",
                      "alignItems": "center",
                      "fill": "$--primary",
                      "cornerRadius": "$--radius-xs",
                      "children": [
                        { "type": "icon", "id": "chkIc", "name": "Check", "library": "lucide", "icon": "check", "width": 12, "height": 12, "fill": "$--on-primary" }
                      ]
                    },
                    { "type": "text", "id": "chkLb", "name": "Label", "fill": "$--fg", "content": "Remember this device", "fontFamily": "$--font", "fontSize": 14 }
                  ]
                },
                {
                  "type": "frame",
                  "id": "swtCm",
                  "name": "Switch",
                  "reusable": true,
                  "gap": 8,
                  "alignItems": "center",
                  "children": [
                    {
                      "type": "frame",
                      "id": "swtTr",
                      "name": "Track",
                      "width": 32,
                      "height": 20,
                      "padding": 2,
                      "justifyContent": "end",
                      "alignItems": "center",
                      "fill": "$--primary",
                      "cornerRadius": 10,
                      "children": [
                        { "type": "ellipse", "id": "swtTh", "name": "Thumb", "width": 16, "height": 16, "fill": "$--on-primary" }
                      ]
                    },
                    { "type": "text", "id": "swtLb", "name": "Label", "fill": "$--fg", "content": "Email me a weekly summary", "fontFamily": "$--font", "fontSize": 14 }
                  ]
                }
              ]
            }
          ]
        }
      ]
    },
    {
      "type": "frame",
      "id": "invCd",
      "x": 800,
      "y": 0,
      "name": "Invite card",
      "width": 400,
      "layout": "vertical",
      "gap": 24,
      "padding": 24,
      "fill": "$--card",
      "cornerRadius": "$--radius-lg",
      "stroke": "$--line",
      "strokeWidth": 1,
      "strokeAlignment": "inner",
      "effect": { "type": "shadow", "offset": { "x": 0, "y": 4 }, "blur": 16, "color": "#0000000f" },
      "children": [
        {
          "type": "frame",
          "id": "invHd",
          "name": "Heading",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 4,
          "children": [
            { "type": "text", "id": "invTt", "name": "Title", "fill": "$--fg", "content": "Invite a teammate", "fontFamily": "$--font", "fontSize": 18, "fontWeight": "600" },
            { "type": "text", "id": "invSb", "name": "About", "fill": "$--muted", "textGrowth": "fixed-width", "width": "fill_container", "content": "They can open the Harbour Rota workspace as soon as they accept.", "fontFamily": "$--font", "fontSize": 14, "lineHeight": 1.5 }
          ]
        },
        {
          "type": "frame",
          "id": "invFs",
          "name": "Fields",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 16,
          "children": [
            {
              "type": "frame",
              "id": "invF1",
              "name": "Email",
              "width": "fill_container",
              "layout": "vertical",
              "gap": 8,
              "children": [
                { "type": "text", "id": "invL1", "name": "Label", "fill": "$--fg", "content": "Work email", "fontFamily": "$--font", "fontSize": 14, "fontWeight": "500" },
                { "type": "ref", "id": "invI1", "ref": "inpCm", "width": "fill_container", "descendants": { "inpTx": { "content": "maren.solheim@harbourrota.no", "fill": "$--fg" } } }
              ]
            },
            {
              "type": "frame",
              "id": "invF2",
              "name": "Role",
              "width": "fill_container",
              "layout": "vertical",
              "gap": 8,
              "children": [
                { "type": "text", "id": "invL2", "name": "Label", "fill": "$--fg", "content": "Role", "fontFamily": "$--font", "fontSize": 14, "fontWeight": "500" },
                { "type": "ref", "id": "invI2", "ref": "selCm", "width": "fill_container", "descendants": { "selTx": { "content": "Scheduler" } } }
              ]
            },
            { "type": "ref", "id": "invCk", "ref": "chkCm", "descendants": { "chkLb": { "content": "Send a welcome email" } } }
          ]
        },
        {
          "type": "frame",
          "id": "invFt",
          "name": "Actions",
          "width": "fill_container",
          "gap": 8,
          "justifyContent": "end",
          "children": [
            { "type": "ref", "id": "invB1", "ref": "btnCm", "fill": "$--secondary", "descendants": { "btnLb": { "content": "Cancel", "fill": "$--fg" } } },
            { "type": "ref", "id": "invB2", "ref": "btnCm", "descendants": { "btnLb": { "content": "Send invite" } } }
          ]
        }
      ]
    },
    {
      "type": "note",
      "id": "note1",
      "x": 1260,
      "y": 0,
      "width": 250,
      "content": "Try it: change --radius or --primary in the variables, switch the theme to Dark in the panel on the right, or select a button in the card and see that it is an instance of the one on the left."
    }
  ],
  "themes": {
    "Mode": ["Light", "Dark"]
  },
  "variables": {
    "--bg": { "type": "color", "value": [ { "value": "#f5f5f4" }, { "value": "#0c0c0d", "theme": { "Mode": "Dark" } } ] },
    "--card": { "type": "color", "value": [ { "value": "#ffffff" }, { "value": "#19191b", "theme": { "Mode": "Dark" } } ] },
    "--fg": { "type": "color", "value": [ { "value": "#18181b" }, { "value": "#fafafa", "theme": { "Mode": "Dark" } } ] },
    "--muted": { "type": "color", "value": [ { "value": "#5b5b63" }, { "value": "#a7a7b0", "theme": { "Mode": "Dark" } } ] },
    "--line": { "type": "color", "value": [ { "value": "#18181b1a" }, { "value": "#ffffff1a", "theme": { "Mode": "Dark" } } ] },
    "--line-strong": { "type": "color", "value": [ { "value": "#18181b33" }, { "value": "#ffffff29", "theme": { "Mode": "Dark" } } ] },
    "--primary": { "type": "color", "value": [ { "value": "#18181b" }, { "value": "#e7e7e9", "theme": { "Mode": "Dark" } } ] },
    "--on-primary": { "type": "color", "value": [ { "value": "#fafafa" }, { "value": "#18181b", "theme": { "Mode": "Dark" } } ] },
    "--secondary": { "type": "color", "value": [ { "value": "#e7e7e4" }, { "value": "#2a2a2e", "theme": { "Mode": "Dark" } } ] },
    "--radius-xs": { "type": "number", "value": 4 },
    "--radius": { "type": "number", "value": 8 },
    "--radius-lg": { "type": "number", "value": 12 },
    "--font": { "type": "string", "value": "Inter" }
  }
}"##;

/// The family the starter names, and what it is set in instead where that one isn't installed.
const STARTER_FACE: &str = "Inter";
/// Quiet interface faces, the first installed of which the starter is set in.
// Human: Segoe UI has no weight 500, which labels are set in; its variable version has every weight.
const STARTER_FACES: [&str; 8] =
    [STARTER_FACE, "Segoe UI Variable", "Segoe UI", "SF Pro Text", "Helvetica Neue", "Noto Sans", "DejaVu Sans", "Arial"];

/// The starter, set in the first of [`STARTER_FACES`] that `installed` knows.
// Human: no fonts ship with the app, so a starter that names one face is drawn in a stand-in on most computers,
// and the app's own review then says so about the app's own sample.
#[must_use]
pub fn starter(installed: &dyn Fn(&str) -> bool) -> String {
    let face = STARTER_FACES.into_iter().find(|face| installed(face)).unwrap_or(STARTER_FACE);
    STARTER.replace(&format!(r#""value": "{STARTER_FACE}""#), &format!(r#""value": "{face}""#))
}

#[cfg(test)]
mod tests {
    use pen_doc::scene::SceneNode;
    use pen_doc::vars::Vars;

    use crate::editor::Editor;

    #[test]
    fn the_starter_reads_and_its_instances_resolve() {
        let doc = pen_doc::read(super::STARTER).unwrap();
        let scene = pen_doc::scene::build(&doc, &pen_doc::vars::ThemeContext::defaults(&doc));
        let mut unresolved = 0;
        for node in &scene {
            node.walk(&mut |n, _| unresolved += usize::from(!n.resolved), 0);
        }
        assert_eq!(unresolved, 0);
        // Where Inter isn't installed the starter is set in a face that is.
        assert_eq!(super::starter(&|_| true), super::STARTER);
        assert!(super::starter(&|face| face == "Segoe UI").contains(r#""--font": { "type": "string", "value": "Segoe UI" }"#));
        assert_eq!(super::starter(&|_| false), super::STARTER);
    }

    #[test]
    fn the_starter_passes_the_apps_own_review_in_both_themes() {
        // The first value of a theme axis is the one in force: the starter as it is, then with Dark first.
        let dark = super::STARTER.replace(r#"["Light", "Dark"]"#, r#"["Dark", "Light"]"#);
        assert_ne!(dark, super::STARTER);
        for json in [super::STARTER, dark.as_str()] {
            let mut editor = Editor::new(pen_doc::read(json).unwrap(), None, 1);
            #[expect(clippy::cast_precision_loss, reason = "the letters of one line of sample copy")]
            editor.refresh(&|style| (style.size * 0.5 * style.content.chars().count() as f32, style.size * 1.2));
            let layers: Vec<&SceneNode> = editor.scene.iter().collect();
            let said = crate::review::review(&layers, Vars::new(&editor.doc), &|_| None, false);
            assert_eq!(said, Vec::<String>::new());
            let ids: Vec<String> = editor.scene.iter().map(|top| top.id.clone()).collect();
            assert_eq!(crate::advice::advice(&editor, &ids), Vec::<String>::new());
        }
    }
}
