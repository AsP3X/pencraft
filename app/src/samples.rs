//! Starter documents offered on the home screen. Their content is this app's own sample copy.

/// A small landing-page board: flex layouts, a reusable button with two instances, themed variables and a note.
pub const STARTER: &str = r##"{
  "version": "2.22",
  "children": [
    {
      "type": "frame",
      "id": "btnCm",
      "x": 0,
      "y": -140,
      "name": "Button",
      "reusable": true,
      "padding": [12, 22],
      "justifyContent": "center",
      "alignItems": "center",
      "fill": "$--accent",
      "cornerRadius": 10,
      "children": [
        { "type": "text", "id": "btnLb", "name": "Label", "fill": "#ffffff", "content": "Get started", "fontFamily": "Inter", "fontSize": 15, "fontWeight": "600" }
      ]
    },
    {
      "type": "frame",
      "id": "lndPg",
      "x": 0,
      "y": 0,
      "name": "Landing page",
      "width": 1200,
      "clip": true,
      "layout": "vertical",
      "gap": 48,
      "padding": [72, 96],
      "fill": "$--bg",
      "children": [
        {
          "type": "frame",
          "id": "navBr",
          "name": "Navigation",
          "width": "fill_container",
          "justifyContent": "space_between",
          "alignItems": "center",
          "children": [
            { "type": "text", "id": "brand", "name": "Brand", "fill": "$--fg", "content": "Northwind Studio", "fontFamily": "Inter", "fontSize": 20, "fontWeight": "700" },
            {
              "type": "frame",
              "id": "navLk",
              "name": "Links",
              "gap": 28,
              "children": [
                { "type": "text", "id": "lnk01", "fill": "$--muted", "content": "Work", "fontFamily": "Inter", "fontSize": 15 },
                { "type": "text", "id": "lnk02", "fill": "$--muted", "content": "Services", "fontFamily": "Inter", "fontSize": 15 },
                { "type": "text", "id": "lnk03", "fill": "$--muted", "content": "Contact", "fontFamily": "Inter", "fontSize": 15 }
              ]
            }
          ]
        },
        {
          "type": "frame",
          "id": "hero1",
          "name": "Hero",
          "width": "fill_container",
          "layout": "vertical",
          "gap": 20,
          "children": [
            { "type": "text", "id": "hTitl", "name": "Headline", "fill": "$--fg", "textGrowth": "fixed-width", "width": "fill_container", "content": "Design calm, useful products.", "fontFamily": "Inter", "fontSize": 64, "fontWeight": "700", "letterSpacing": -1.5, "lineHeight": 1.1 },
            { "type": "text", "id": "hSub1", "name": "Subtitle", "fill": "$--muted", "textGrowth": "fixed-width", "width": 640, "content": "A small sample board: rows and columns lay themselves out, the button is a reusable component, and the colours follow the Light and Dark theme.", "fontFamily": "Inter", "fontSize": 19, "lineHeight": 1.5 },
            {
              "type": "frame",
              "id": "hActs",
              "name": "Actions",
              "gap": 12,
              "children": [
                { "type": "ref", "id": "cta01", "ref": "btnCm" },
                { "type": "ref", "id": "cta02", "ref": "btnCm", "fill": "$--card", "stroke": "$--border", "strokeWidth": 1, "descendants": { "btnLb": { "content": "See our work", "fill": "$--fg" } } }
              ]
            }
          ]
        },
        {
          "type": "frame",
          "id": "cards",
          "name": "Features",
          "width": "fill_container",
          "gap": 24,
          "children": [
            {
              "type": "frame", "id": "card1", "name": "Card", "width": "fill_container", "layout": "vertical", "gap": 12, "padding": 24, "fill": "$--card", "cornerRadius": 16, "stroke": "$--border", "strokeWidth": 1,
              "effect": { "type": "shadow", "offset": { "x": 0, "y": 4 }, "blur": 16, "color": "#0000000f" },
              "children": [
                { "type": "ellipse", "id": "ico01", "width": 40, "height": 40, "fill": { "type": "gradient", "gradientType": "linear", "rotation": 135, "colors": [ { "color": "#6366f1", "position": 0 }, { "color": "#22d3ee", "position": 1 } ] } },
                { "type": "text", "id": "ct01", "fill": "$--fg", "content": "Research", "fontFamily": "Inter", "fontSize": 18, "fontWeight": "600" },
                { "type": "text", "id": "cb01", "fill": "$--muted", "textGrowth": "fixed-width", "width": "fill_container", "content": "Interviews and quick tests before the first pixel.", "fontFamily": "Inter", "fontSize": 15, "lineHeight": 1.5 }
              ]
            },
            {
              "type": "frame", "id": "card2", "name": "Card", "width": "fill_container", "layout": "vertical", "gap": 12, "padding": 24, "fill": "$--card", "cornerRadius": 16, "stroke": "$--border", "strokeWidth": 1,
              "effect": { "type": "shadow", "offset": { "x": 0, "y": 4 }, "blur": 16, "color": "#0000000f" },
              "children": [
                { "type": "rectangle", "id": "ico02", "width": 40, "height": 40, "cornerRadius": 10, "fill": { "type": "gradient", "gradientType": "linear", "rotation": 135, "colors": [ { "color": "#f97316", "position": 0 }, { "color": "#facc15", "position": 1 } ] } },
                { "type": "text", "id": "ct02", "fill": "$--fg", "content": "Interface", "fontFamily": "Inter", "fontSize": 18, "fontWeight": "600" },
                { "type": "text", "id": "cb02", "fill": "$--muted", "textGrowth": "fixed-width", "width": "fill_container", "content": "Layouts that hold up on every screen size.", "fontFamily": "Inter", "fontSize": 15, "lineHeight": 1.5 }
              ]
            },
            {
              "type": "frame", "id": "card3", "name": "Card", "width": "fill_container", "layout": "vertical", "gap": 12, "padding": 24, "fill": "$--card", "cornerRadius": 16, "stroke": "$--border", "strokeWidth": 1,
              "effect": { "type": "shadow", "offset": { "x": 0, "y": 4 }, "blur": 16, "color": "#0000000f" },
              "children": [
                { "type": "polygon", "id": "ico03", "width": 40, "height": 40, "polygonCount": 6, "fill": { "type": "gradient", "gradientType": "radial", "colors": [ { "color": "#34d399", "position": 0 }, { "color": "#059669", "position": 1 } ] } },
                { "type": "text", "id": "ct03", "fill": "$--fg", "content": "Systems", "fontFamily": "Inter", "fontSize": 18, "fontWeight": "600" },
                { "type": "text", "id": "cb03", "fill": "$--muted", "textGrowth": "fixed-width", "width": "fill_container", "content": "Components and tokens your team can reuse.", "fontFamily": "Inter", "fontSize": 15, "lineHeight": 1.5 }
              ]
            }
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
      "content": "Try it: press R and drag to draw, select a card and change its fill, or switch the theme to Dark in the panel on the right."
    },
    {
      "type": "path",
      "id": "arrw1",
      "x": 1300,
      "y": 260,
      "name": "Arrow",
      "rotation": -45,
      "geometry": "M7 0l0 14m7-7l-7 7-7-7",
      "width": 80,
      "height": 80,
      "stroke": "#ff8302",
      "strokeWidth": 10,
      "strokeLinejoin": "round",
      "strokeLinecap": "round"
    }
  ],
  "themes": {
    "Mode": ["Light", "Dark"]
  },
  "variables": {
    "--bg": { "type": "color", "value": [ { "value": "#f7f7f5" }, { "value": "#111113", "theme": { "Mode": "Dark" } } ] },
    "--card": { "type": "color", "value": [ { "value": "#ffffff" }, { "value": "#1c1c1f", "theme": { "Mode": "Dark" } } ] },
    "--fg": { "type": "color", "value": [ { "value": "#18181b" }, { "value": "#fafafa", "theme": { "Mode": "Dark" } } ] },
    "--muted": { "type": "color", "value": [ { "value": "#6b6b73" }, { "value": "#a1a1aa", "theme": { "Mode": "Dark" } } ] },
    "--border": { "type": "color", "value": [ { "value": "#e4e4e7" }, { "value": "#2a2a30", "theme": { "Mode": "Dark" } } ] },
    "--accent": { "type": "color", "value": "#4f46e5" }
  }
}"##;

#[cfg(test)]
mod tests {
    #[test]
    fn the_starter_reads_and_its_instances_resolve() {
        let doc = pen_doc::read(super::STARTER).unwrap();
        let scene = pen_doc::scene::build(&doc, &pen_doc::vars::ThemeContext::defaults(&doc));
        let mut unresolved = 0;
        for node in &scene {
            node.walk(&mut |n, _| unresolved += usize::from(!n.resolved), 0);
        }
        assert_eq!(unresolved, 0);
    }
}
