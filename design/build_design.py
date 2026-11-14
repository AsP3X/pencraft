"""Builds Pencraft's own design file through `pencraft mcp`, the way an AI client would."""
import base64
import json
import os
import subprocess
import sys

S = sys.argv[1]
TARGET = sys.argv[2]
# The Pencraft program: the PENCRAFT environment variable, else `pencraft` on PATH.
PENCRAFT = os.environ.get('PENCRAFT', 'pencraft')
proc = subprocess.Popen([PENCRAFT, 'mcp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
_n = 0


def rpc(method, params):
    global _n
    _n += 1
    proc.stdin.write(json.dumps({"jsonrpc": "2.0", "id": _n, "method": method, "params": params}) + "\n")
    proc.stdin.flush()
    return json.loads(proc.stdout.readline())


def call(name, args=None, shot=None):
    r = rpc("tools/call", {"name": name, "arguments": args or {}})["result"]
    texts = []
    for c in r["content"]:
        if c["type"] == "text":
            texts.append(c["text"])
        elif shot:
            open(f"{S}/{shot}.png", "wb").write(base64.b64decode(c["data"]))
    if r["isError"]:
        print("ERROR in", name, "\n".join(texts)[:1500])
        sys.exit(1)
    return "\n".join(texts)


# ---------------------------------------------------------------- builders
def fr(name, c=None, **k):
    d = {"type": "frame", "name": name}
    d.update(k)
    if c is not None:
        d["children"] = c
    return d


def row(name, c=None, **k):
    k.setdefault("alignItems", "center")
    return fr(name, c, layout="horizontal", **k)


def col(name, c=None, **k):
    return fr(name, c, layout="vertical", **k)


def tx(s, size=13, fill="$fg", weight="400", **k):
    return {"type": "text", "content": s, "fontFamily": "Inter", "fontSize": size, "fontWeight": weight, "fill": fill, **k}


def ic(name, size=16, fill="$fg", **k):
    return {"type": "icon", "library": "lucide", "icon": name, "width": size, "height": size, "fill": fill, **k}


def ref(cid, **k):
    return {"type": "ref", "ref": cid, **k}


def comp(cid, node):
    node["id"] = cid
    node["reusable"] = True
    return node


def spacer():
    return fr("Spacer", [], width="fill_container", height=1)


def line(**k):
    return {"type": "rectangle", "name": "Divider", "width": "fill_container", "height": 1, "fill": "$border", **k}


def vline(h=16):
    return {"type": "rectangle", "name": "Divider", "width": 1, "height": h, "fill": "$border"}


BORDER = {"stroke": "$outline", "strokeWidth": 1, "strokeAlignment": "inner"}
SHADOW = {"effect": {"type": "shadow", "shadowType": "outer", "color": "#00000026", "offset": {"x": 0, "y": 8}, "blur": 24}}


def shown(label, node):
    """A part with its name above it, as it stands on a board."""
    return col(label, [tx(label, 11, "$muted", "500"), node], gap=10)


def staged(label, node):
    """A small piece on the surface it is used on, so outlines and light fills show as they do in the app."""
    return col(label, [tx(label, 11, "$muted", "500"), row("Surface", [node], padding=16, cornerRadius=12, fill="$bg")], gap=10)


def section(title, children, **k):
    k.setdefault("gap", 32)
    return col(title, [tx(title, 18, "$fg", "600"), row(title + " items", children, alignItems="start", **k)], gap=20, width="fill_container")


def board(name, children, x, y, **k):
    k.setdefault("width", 1600)
    return col(name, [tx(name, 32, "$fg", "700")] + children, x=x, y=y, padding=56, gap=48, fill="$board", cornerRadius=24, **BORDER, **k)


# ---------------------------------------------------------------- variables
def themed(light, dark):
    return {"type": "color", "value": [{"value": light}, {"value": dark, "theme": {"Mode": "Dark"}}]}


VARIABLES = {
    "bg": themed("#fafafa", "#18181b"), "fg": themed("#020202", "#c8c8c8"), "muted": themed("#71717a", "#a1a1aa"),
    "border": themed("#e4e4e7", "#202020"), "canvas": themed("#f6f6f6", "#18181b"), "island": themed("#fafafa", "#27272a"),
    "hover": themed("#f4f4f5", "#3f3f46"), "input": themed("#e4e4e7", "#3f3f46"), "primary": themed("#18181b", "#fafafa"),
    "primary-fg": themed("#fafafa", "#18181b"), "outline": themed("#e4e4e7", "#3a3a3e"), "board": themed("#e8e8ec", "#0e0e10"),
    "accent": {"type": "color", "value": "#3b82f6"}, "component": {"type": "color", "value": "#d946ef"},
    "instance": {"type": "color", "value": "#8b5cf6"}, "agent": {"type": "color", "value": "#607dff"},
    "connected": {"type": "color", "value": "#22c55e"}, "checking": {"type": "color", "value": "#f59e0b"},
    "radius-control": {"type": "number", "value": 6}, "radius-chip": {"type": "number", "value": 8},
    "radius-island": {"type": "number", "value": 14}, "radius-settings": {"type": "number", "value": 16},
}
PALETTE = ["bg", "fg", "muted", "border", "canvas", "island", "hover", "input", "primary", "primary-fg", "outline"]
MARKS = [("accent", "Selection and focus"), ("component", "Components"), ("instance", "Instances"), ("agent", "AI at work"), ("connected", "Connected"), ("checking", "Checking")]


# ---------------------------------------------------------------- foundations
def swatch(var, note=None):
    return col(var, [fr("Colour", [], width=104, height=64, cornerRadius=10, fill="$" + var, **BORDER), tx(var, 12, "$fg", "500"), tx(note or "", 11, "$muted")][: 3 if note else 2], gap=6)


def palette_card(mode):
    return col(mode + " palette", [tx(mode, 14, "$fg", "600"), row("Swatches", [swatch(v) for v in PALETTE], gap=14, alignItems="start")],
               gap=16, padding=24, cornerRadius=16, fill="$board", theme={"Mode": mode}, **BORDER)


TYPE = [("Window title", 17, "500", "Pencraft"), ("Settings page title", 20, "500", "Connect your AI provider"), ("Home title", 30, "700", "Pencraft"),
        ("Settings section", 14, "500", "Setup providers"), ("Message", 13.5, "400", "Create, plan or research..."), ("Sidebar text", 13, "400", "Landing page"),
        ("Size label", 12.5, "400", "320 × 179"), ("Properties text", 11.5, "400", "Fill Width"), ("Properties small", 10.5, "400", "Dimensions")]
SIZES = [("Tab bar height", 53), ("Title island height", 41), ("Tab height", 27), ("Sidebar width", 377), ("Sidebar row", 28), ("Tool rail width", 36), ("Properties width", 228),
         ("Properties row", 24), ("Island gap", 8), ("Composer height", 91), ("Message bar", "620 × 41"), ("Zoom island", "116 × 30"), ("Action search", "742 × 400"), ("Icon", 16)]
ICONS = ["mouse-pointer-2", "scaling", "square", "circle", "triangle", "pen-tool", "type", "frame", "sticky-note", "globe", "hand", "message-circle", "component", "search",
         "ellipsis", "panel-left", "panel-right", "house", "folder", "plus", "minus", "menu", "settings", "share", "play", "messages-square", "layers", "presentation", "library",
         "maximize-2", "rocket", "arrow-up", "chevron-down", "chevron-right", "rotate-cw", "eye", "x", "check", "link", "tag", "sparkle", "terminal", "save", "undo-2", "redo-2",
         "layout-grid", "image", "download"]


def foundations(x, y):
    radii = [("Control", 6), ("Chip", 8), ("Island", 14), ("Settings window", 16), ("Bar ends", 20.5)]
    return board("Foundations", [
        section("Colours by theme", [palette_card("Light"), ], gap=24),
        section("Dark theme", [palette_card("Dark")]),
        section("Marks", [swatch(v, note) for v, note in MARKS], gap=20),
        section("Type (Inter)", [col("Type list", [row(n, [tx(n + f" · {s}", 11, "$muted", width=200, textGrowth="fixed-width"), tx(sample, s, "$fg", w)], gap=16) for n, s, w, sample in TYPE], gap=14)]),
        section("Corner radii", [col(n, [fr("Corner", [], width=72, height=72, cornerRadius=r, fill="$hover", **BORDER), tx(f"{n} · {r}", 11, "$muted")], gap=8) for n, r in radii], gap=24),
        section("Sizes", [col("Size list", [row(n, [tx(n, 12, "$muted", width=160, textGrowth="fixed-width"), tx(str(v), 12, "$fg", "500")]) for n, v in SIZES[i::2]], gap=8) for i in (0, 1)], gap=64),
        section("Icons (Lucide, 16, line 1.3)", [col("Icon rows", [row("Icons", [col(n, [ic(n, 18)], padding=9, cornerRadius=8, fill="$hover") for n in ICONS[i:i + 24]], gap=10) for i in (0, 24)], gap=10)]),
    ], x, y)


# ---------------------------------------------------------------- components
def button(cid, label, fill, text, border=False):
    extra = BORDER if border else {}
    return comp(cid, row(label, [tx(label, 13, text, "500", id=cid + "-label")], padding=[8, 14], cornerRadius=8, fill=fill, justifyContent="center", **extra))


def icon_button(cid, icon, active=False):
    return comp(cid, row("Icon button", [ic(icon, 16, id=cid + "-icon")], width=26, height=26, cornerRadius=6, justifyContent="center", fill="$hover" if active else "#00000000"))


def switch(cid, on):
    return comp(cid, fr("Switch " + ("on" if on else "off"), [{"type": "ellipse", "x": 13 if on else 2, "y": 2, "width": 10, "height": 10, "fill": "#ffffff"}],
                        layout="none", width=25, height=14, cornerRadius=7, fill="$accent" if on else "$input"))


def checkbox(cid, on):
    kids = [ic("check", 10, "#ffffff")] if on else []
    extra = {} if on else BORDER
    return comp(cid, row("Checkbox " + ("on" if on else "off"), kids, width=14, height=14, cornerRadius=3, justifyContent="center", fill="$accent" if on else "#00000000", **extra))


def segmented(cid, items, chosen, width=196):
    return comp(cid, row("Segmented", [row(i, [tx(i, 11.5, "$fg" if i == chosen else "$muted")], width="fill_container", height=20, cornerRadius=4, justifyContent="center",
                                           fill="$bg" if i == chosen else "#00000000") for i in items], width=width, height=24, padding=2, cornerRadius=5, fill="$hover"))


def num_field(cid, label, value, icon=None, width=92):
    lead = ic(icon, 13, "$muted") if icon else tx(label, 11.5, "$muted")
    return comp(cid, row("Field " + label, [lead, tx(value, 11.5, "$fg", id=cid + "-value")], width=width, height=24, padding=[0, 8], gap=10, cornerRadius=5, fill="$hover"))


def layer_row(cid, kind, colour="$fg", fill="#00000000"):
    return comp(cid, row("Layer row " + kind, [ic("chevron-right", 10, "$muted"), ic("frame", 13, "$muted", id=cid + "-icon"), tx("Card", 13, colour, id=cid + "-name"), spacer(),
                                               {"type": "ellipse", "width": 4, "height": 4, "fill": "$muted"}], width=345, height=28, padding=[0, 8], gap=8, cornerRadius=6, fill=fill))


def keycap(s):
    return row("Key", [tx(s, 10, "$muted")], width=18, height=18, cornerRadius=4, fill="$hover", justifyContent="center")


def action_row(cid, chosen):
    text = "#ffffff" if chosen else "$fg"
    return comp(cid, row("Action row" + (" chosen" if chosen else ""), [ic("save", 16, text, id=cid + "-icon"), tx("Save", 14, text, id=cid + "-name"), spacer(),
                                                                         row("Keys", [keycap("⌘"), keycap("S")], gap=4, id=cid + "-keys")],
                         width=718, height=36, padding=[0, 12], gap=12, cornerRadius=8, fill="$accent" if chosen else "#00000000"))


def menu_row(cid):
    return comp(cid, row("Menu row", [ic("mouse-pointer-2", 14, "$fg", id=cid + "-icon"), tx("Move", 13, "$fg", id=cid + "-name"), spacer(), tx("V", 12, "$muted", id=cid + "-key")],
                         width=192, height=28, padding=[0, 8], gap=8, cornerRadius=6))


def pill(cid, text, on):
    colour = "$connected" if on else "$muted"
    return comp(cid, row("State " + text, [{"type": "ellipse", "width": 5, "height": 5, "fill": colour}, tx(text, 12, colour)], height=24, padding=[0, 10], gap=7, cornerRadius=12,
                         fill="#22c55e26" if on else "#00000000", stroke="$connected" if on else "$outline", strokeWidth=1, strokeAlignment="inner"))


def provider_card():
    cid = "provider-card"
    return comp(cid, row("Provider card", [
        row("Tile", [tx("A", 14, "$fg", "500", id=cid + "-letter")], width=32, height=32, cornerRadius=8, fill="$hover", justifyContent="center"),
        col("Names", [tx("Anthropic Claude Code", 14, "$fg", id=cid + "-name"), tx("Fable 5.2 · Opus 5.5 · Sonnet 5.5 · Haiku 5.5", 12, "$muted", id=cid + "-models")], gap=5),
        spacer(), fr("State", [ref("pill-off")], id=cid + "-state"), ic("chevron-right", 12, "$muted")],
        width=760, height=66, padding=[0, 16], gap=14, cornerRadius=12, **BORDER))


def nav_row(cid, chosen):
    return comp(cid, row("Settings page" + (" chosen" if chosen else ""), [ic("settings", 14, "$fg" if chosen else "$muted", id=cid + "-icon"), tx("General", 13, "$fg" if chosen else "$muted", id=cid + "-name")],
                         width=176, height=32, padding=[0, 10], gap=10, cornerRadius=8, fill="$hover" if chosen else "#00000000"))


def settings_row(cid, control):
    return comp(cid, row("Settings row", [col("Texts", [tx("Let AI models work on documents", 13, "$fg", id=cid + "-title"),
                                                         tx("AI models on this computer can read and change open documents.", 12, "$muted", id=cid + "-hint")], gap=4),
                                          spacer(), fr("Control", [control], id=cid + "-control")], width=720, height=56, gap=16))


def ai_cursor():
    return comp("ai-cursor", fr("AI cursor", [
        {"type": "path", "geometry": "M0 0 L0 16 L4.5 12 L11.5 11.5 Z", "viewBox": [0, 0, 12, 16], "x": 0, "y": 0, "width": 12, "height": 16, "fill": "$agent", "stroke": "#ffffff", "strokeWidth": 1},
        row("Name", [tx("Claude", 11, "#ffffff", id="ai-cursor-name")], x=12, y=16, padding=[3, 6], cornerRadius=10, fill="$agent")], layout="none", width=64, height=38))


def components(x, y):
    return board("Components", [
        section("Buttons", [staged("Primary", button("btn-primary", "New design", "$primary", "$primary-fg")), staged("Outline", button("btn-outline", "Open file", "#00000000", "$fg", True)),
                            staged("Wide", comp("btn-wide", row("Wide button", [ic("component", 14), tx("Create Component", 12.5, "$fg", id="btn-wide-label")], width=196, height=26, gap=8, cornerRadius=6, fill="$hover", justifyContent="center"))),
                            staged("Icon button", icon_button("icon-btn", "settings")), staged("Icon button, active", icon_button("icon-btn-active", "menu", True)),
                            staged("Send", comp("btn-send", row("Send", [ic("arrow-up", 12, "$primary-fg")], width=20, height=20, cornerRadius=10, fill="$primary", justifyContent="center")))]),
        section("Controls", [staged("Switch on", switch("switch-on", True)), staged("Switch off", switch("switch-off", False)), staged("Checkbox on", checkbox("check-on", True)),
                             staged("Checkbox off", checkbox("check-off", False)),
                             staged("Choice", comp("choice", row("Choice", [tx("Small (Default)", 12, "$fg", id="choice-value"), ic("chevron-down", 12, "$muted")], height=25, padding=[0, 8], gap=8, cornerRadius=6, fill="$hover"))),
                             staged("Text field", comp("field", row("Text field", [tx("https://api.example.com/v1", 12.5, "$muted", id="field-value")], width=260, height=30, padding=[0, 10], cornerRadius=6, fill="$hover"))),
                             staged("Number field", num_field("num-field", "W", "320")), staged("Icon field", num_field("icon-field", "rotation", "0", "rotate-cw")),
                             staged("Segmented", segmented("segmented", ["Freeform", "Vertical", "Horizontal"], "Vertical"))]),
        section("Tabs and chips", [staged("Tab", comp("tab", row("Tab", [ic("folder", 14), tx("Untitled", 13, "$fg", "500", id="tab-name")], height=27, padding=[0, 10], gap=7, cornerRadius=8, fill="$hover"))),
                                   staged("Sidebar chip", comp("chip", row("Chip", [ic("message-circle", 16, id="chip-icon"), tx("Chat", 13, "$fg", id="chip-name")], height=29, padding=[0, 10], gap=8, cornerRadius=8, fill="$hover"))),
                                   staged("Idea", comp("idea", row("Idea", [tx("Design a playful ordering app for matcha cakes", 13, "$fg", id="idea-text")], height=30, padding=[0, 12], cornerRadius=8, **BORDER))),
                                   staged("Selection chip", comp("selection-chip", row("Selection chip", [ic("frame", 12, "$muted"), tx("Card", 12, "$muted", id="selection-chip-name")], height=21, padding=[0, 6], gap=5, cornerRadius=5, **BORDER))),
                                   staged("Connected", pill("pill-on", "Connected automatically", True)), staged("Not connected", pill("pill-off", "Not connected", False)),
                                   staged("Message", comp("toast", row("Message", [tx("Exported Card.png", 13, "$primary-fg", id="toast-text")], padding=10, cornerRadius=8, fill="$primary")))]),
        section("Rows", [col("Layer rows", [staged("Layer row", layer_row("layer-row", "plain")), staged("Layer row, selected", layer_row("layer-row-selected", "selected", "$fg", "#3b82f633")),
                                            staged("Layer row, component", layer_row("layer-row-component", "component", "$component"))], gap=20),
                         col("Menu rows", [staged("Tool menu row", menu_row("menu-row")), staged("Settings page", nav_row("nav-row", False)), staged("Settings page, chosen", nav_row("nav-row-chosen", True))], gap=20),
                         col("Action rows", [staged("Action row", action_row("action-row", False)), staged("Action row, chosen", action_row("action-row-chosen", True))], gap=20)]),
        section("Settings", [col("Settings pieces", [staged("Provider card", provider_card()), staged("Settings row", settings_row("settings-row", ref("switch-on")))], gap=24)]),
        section("Canvas marks", [staged("Resize handle", comp("handle", {"type": "rectangle", "name": "Handle", "width": 8, "height": 8, "fill": "#ffffff", "stroke": "$accent", "strokeWidth": 1})),
                                 staged("Rotation dot", comp("rotate-dot", {"type": "ellipse", "name": "Rotation dot", "width": 11, "height": 11, "fill": "#ffffff", "stroke": "$accent", "strokeWidth": 1.5})),
                                 staged("Corner radius handle", comp("radius-handle", {"type": "ellipse", "name": "Radius handle", "width": 9, "height": 9, "fill": "#ffffff", "stroke": "$accent", "strokeWidth": 1})),
                                 staged("Size label", comp("size-label", row("Size label", [tx("320 × 179", 12.5, "#ffffff", id="size-label-text")], padding=[2, 6], cornerRadius=4, fill="$accent"))),
                                 staged("AI cursor", ai_cursor())], gap=40),
    ], x, y)


# ---------------------------------------------------------------- application parts
def island(name, children, **k):
    k.setdefault("fill", "$bg")
    return fr(name, children, **{**BORDER, **k})


def title_island():
    lights = row("Window buttons", [{"type": "ellipse", "width": 12, "height": 12, "fill": c} for c in ("#ff5f57", "#febc2e", "#28c840")], gap=8)
    return comp("part-title", island("Title island", [lights, fr("Gap", [], width=44, height=1), tx("Pencraft", 17, "$fg", "500"), fr("Gap", [], width=14, height=1), ic("house", 16),
                                                      fr("Gap", [], width=14, height=1), vline(), fr("Gap", [], width=8, height=1), ref("tab"), fr("Gap", [], width=10, height=1), ic("plus", 16, "$muted")],
                                     layout="horizontal", alignItems="center", height=41, padding=[0, 14, 0, 13], cornerRadius=20.5))


def window_actions():
    return comp("part-actions", island("Window actions", [ref("icon-btn-active"), ref("icon-btn"), vline(), row("Share", [ic("share", 16, "$muted"), tx("Share", 13, "$muted")], gap=7),
                                                          ic("play", 16, "$muted"), ic("globe", 16, "$muted")], layout="horizontal", alignItems="center", height=41, padding=[0, 14, 0, 10], gap=10, cornerRadius=20.5))


def rail():
    tools = [("square", True), ("pen-tool", False), ("type", False), ("frame", False), ("sticky-note", False), ("globe", False), ("hand", False), ("message-circle", False), ("component", False), ("search", False)]
    kids = [row("Move tool", [ic("mouse-pointer-2", 16)], width=28, height=28, cornerRadius=14, fill="$hover", justifyContent="center"), ic("chevron-down", 9, "$muted")]
    for name, arrow in tools:
        kids.append(row(name, [ic(name, 16, "$muted")], width=28, height=32, justifyContent="center"))
        if arrow:
            kids.append(ic("chevron-down", 9, "$muted"))
    circle = lambda n, i: island(n, [ic(i, 16, "$muted")], layout="horizontal", alignItems="center", justifyContent="center", width=36, height=36, cornerRadius=18)
    return comp("part-rail", col("Tool rail", [circle("Sidebar button", "panel-left"), island("Tools", kids, layout="vertical", alignItems="center", width=36, padding=[4, 0], gap=1, cornerRadius=18),
                                               circle("More", "ellipsis")], gap=8, alignItems="center"))


def tool_menu():
    rows = [("mouse-pointer-2", "Move", "V"), ("scaling", "Scale", "K")]
    shapes = [("square", "Rectangle", "R"), ("circle", "Ellipse", "O"), ("triangle", "Polygon", ""), ("image", "Image or SVG…", "")]
    menu = lambda name, items: island(name, [ref("menu-row", descendants={"menu-row-icon": {"icon": i}, "menu-row-name": {"content": n}, "menu-row-key": {"content": k}}) for i, n, k in items],
                                      layout="vertical", padding=4, cornerRadius=10, fill="$island", **SHADOW)
    return comp("part-tool-menus", row("Tool menus", [menu("Pointer menu", rows), menu("Shape menu", shapes)], gap=24, alignItems="start"))


def sidebar_header(active):
    def item(icon, on):
        return row(icon, [ic(icon, 16, "$fg" if on else "$muted")], width=30, height=29, cornerRadius=8, justifyContent="center", fill="$hover" if on else "#00000000")
    chat = ref("chip") if active == "chat" else row("Chat", [ic("message-circle", 16, "$muted"), tx("Chat", 13, "$muted")], height=29, padding=[0, 10], gap=8)
    return row("Sidebar header", [item("messages-square", False), chat, spacer(), item("layers", active == "layers"), item("presentation", False), item("component", False), item("library", False)],
               width="fill_container", gap=2)


def layer(cid, name, icon, depth, open_=True):
    return ref(cid, padding=[0, 8, 0, 8 + depth * 14], descendants={cid + "-name": {"content": name}, cid + "-icon": {"icon": icon}})


def sidebar_layers():
    rows = [layer("layer-row", "Arrow", "pen-tool", 0), layer("layer-row", "Try it: press R and drag to draw", "sticky-note", 0), layer("layer-row", "Landing page", "frame", 0),
            layer("layer-row", "Features", "frame", 1), layer("layer-row", "Card", "frame", 2), layer("layer-row-selected", "Card", "frame", 2), layer("layer-row", "Card", "frame", 2),
            layer("layer-row", "Hero", "frame", 1), layer("layer-row", "Navigation", "frame", 1), layer("layer-row-component", "Button", "frame", 0), layer("layer-row", "Label", "type", 1)]
    return comp("part-sidebar-layers", island("Sidebar, layers", [col("Head", [sidebar_header("layers"), row("Title", [tx("Layers", 13, "$fg", "500")], height=32, padding=[0, 4])], width="fill_container", padding=[12, 12, 8, 12], gap=10),
                                                                  line(), col("Layer list", rows, width="fill_container", padding=[12, 16], gap=0)],
                                              layout="vertical", width=377, height=690, cornerRadius=14, clip=True))


def sidebar_chat():
    ideas = ["Plan a car-free weekend in Kyoto for two", "Design a retro-futuristic landing page", "Design a vintage ticketing site for an arthouse", "Design a playful ordering app for matcha cakes"]
    body = col("Chat start", [tx("Ask me to design anything", 13.5, "$fg", "500")] + [ref("idea", descendants={"idea-text": {"content": i}}) for i in ideas] + [
        fr("Gap", [], width=1, height=8), tx("Generate code", 13.5, "$fg", "500"),
        tx("Ask the agent to build your design with React, Tailwind, Next.js, or your preferred stack.", 13, "$muted", textGrowth="fixed-width", width=337, lineHeight=1.3),
        fr("Gap", [], width=1, height=8), tx("Attach context files", 13.5, "$fg", "500"),
        tx("Drop a PNG/JPG, DESIGN.md or any text file here or attach via +", 13, "$muted", textGrowth="fixed-width", width=337, lineHeight=1.3)], width="fill_container", padding=[72, 20, 20, 20], gap=10)
    head = col("Head", [sidebar_header("chat"), row("Title", [tx("New chat", 13, "$fg", "500"), spacer(), row("Wide", [ic("maximize-2", 13, "$muted")], width=26, height=26, cornerRadius=6, justifyContent="center", **BORDER)],
                                                    width="fill_container", height=32, padding=[0, 4])], width="fill_container", padding=[12, 12, 8, 12], gap=10)
    return comp("part-sidebar-chat", island("Sidebar, chat", [head, line(), body], layout="vertical", width=377, height=690, cornerRadius=14, clip=True))


def model_picker():
    return row("Model", [ic("rocket", 14, "$muted"), tx("Claude Opus 5.5", 13, "$muted"), ic("chevron-down", 10, "$muted"), ref("btn-send")], gap=8)


def composer():
    return comp("part-composer", island("Message box", [tx("Create, plan or research... @ to attach, / for skills", 13.5, "$muted"), spacer(), row("Bottom", [ic("plus", 16, "$muted"), spacer(), model_picker()], width="fill_container")],
                                        layout="vertical", width=377, height=91, padding=14, cornerRadius=14, justifyContent="space_between"))


def message_bar():
    return comp("part-message-bar", island("Message bar", [ic("plus", 16, "$muted"), ref("selection-chip"), spacer(), model_picker()], layout="horizontal", alignItems="center", width=408, height=41, padding=[0, 10, 0, 16], gap=12, cornerRadius=20.5))


def zoom():
    return comp("part-zoom", island("Zoom", [ic("minus", 14, "$muted"), tx("59%", 12.5, "$muted"), ic("plus", 14, "$muted")], layout="horizontal", alignItems="center", justifyContent="space_between", width=116, height=30, padding=[0, 12], cornerRadius=15))


def prop_section(title, children, icon=None):
    head = row("Title", [tx(title, 12.5, "$fg", "500"), spacer()] + ([ic(icon, 13, "$muted")] if icon else []), width="fill_container", height=24)
    return col(title, [head] + children, width="fill_container", padding=16, gap=7)


def check_line(label, on):
    return row(label, [ref("check-on" if on else "check-off"), tx(label, 11.5, "$muted")], gap=8, width="fill_container", height=24)


def field(label, value, icon=None, width=92):
    cid = "icon-field" if icon else "num-field"
    over = {cid + "-value": {"content": value}}
    node = ref(cid, width=width, descendants=over)
    if icon and icon != "rotate-cw":
        node = row("Field " + label, [ic(icon, 13, "$muted"), tx(value, 11.5, "$fg")], width=width, height=24, padding=[0, 8], gap=10, cornerRadius=5, fill="$hover")
    elif not icon and label != "W":
        node = row("Field " + label, [tx(label, 11.5, "$muted"), tx(value, 11.5, "$fg")], width=width, height=24, padding=[0, 8], gap=10, cornerRadius=5, fill="$hover")
    return node


def icon_group(icons):
    return row("Group", [row(i, [ic(i, 13, "$muted")], width="fill_container", height=24, justifyContent="center") for i in icons], width="fill_container", cornerRadius=5, fill="$hover")


def properties():
    pair = lambda a, b: row("Pair", [a, b], width="fill_container", gap=12)
    seg = lambda items, chosen: row("Segmented", [row(i, [tx(i, 11 if len(items) < 5 else 9.5, "$fg" if i == chosen else "$muted")], width="fill_container", height=20, cornerRadius=4, justifyContent="center", fill="$bg" if i == chosen else "#00000000") for i in items],
                                    width="fill_container", height=24, padding=2, cornerRadius=5, fill="$hover")
    plus = lambda t: row(t, [tx(t, 12.5, "$fg", "500"), spacer(), ic("plus", 13, "$muted")], width="fill_container", height=24, padding=[0, 16])
    return comp("part-properties", island("Properties", [
        col("Header", [row("Name", [ic("frame", 14), tx("Card", 12.5, "$fg", "500"), spacer(), ic("panel-right", 13, "$muted")], width="fill_container", height=24, gap=8), ref("btn-wide")], width="fill_container", padding=16, gap=10),
        line(), row("Context", [tx("Context", 12.5, "$fg", "500"), spacer(), ic("chevron-down", 12, "$muted")], width="fill_container", height=44, padding=[0, 16]),
        line(), prop_section("Alignment", [pair(icon_group(["align-start-vertical", "align-center-vertical", "align-end-vertical"]), icon_group(["align-start-horizontal", "align-center-horizontal", "align-end-horizontal"]))]),
        line(), prop_section("Position", [tx("Placed by the parent's layout", 10.5, "$muted"), pair(field("rotation", "0", "rotate-cw"), icon_group(["copy", "flip-horizontal-2", "flip-vertical-2"]))]),
        line(), prop_section("Layout", [tx("Dimensions", 10.5, "$muted"), pair(field("W", "320"), field("H", "179")), pair(check_line("Fill Width", True), check_line("Fill Height", False)),
                                        pair(check_line("Fit Width", False), check_line("Fit Height", True)), seg(["Freeform", "Vertical", "Horizontal"], "Vertical"),
                                        pair(field("gap", "12", "between-horizontal-start"), field("padding", "24", "square-dashed")), tx("Justify", 10.5, "$muted"), seg(["Start", "Center", "End", "Between", "Around"], "Start"),
                                        tx("Align", 10.5, "$muted"), seg(["Start", "Center", "End"], "Start")], "layout-grid"),
        line(), prop_section("Appearance", [pair(field("opacity", "100", "blend"), field("radius", "16", "scan")), check_line("Clip Content", False)], "eye"),
        line(), prop_section("Fill", [row("Paint", [fr("Swatch", [], width=14, height=14, cornerRadius=3, fill="#ffffff", **BORDER), tx("$--card", 11.5, "$accent"), spacer(), ic("minus", 13, "$muted")],
                                          width="fill_container", height=24, padding=[0, 8], gap=8, cornerRadius=5, fill="$hover")], "plus"),
        line(), fr("Gap", [], width=1, height=10), plus("Stroke"), plus("Effects"), plus("Export"), fr("Gap", [], width=1, height=10),
    ], layout="vertical", width=228, cornerRadius=14, clip=True))


def action_search():
    def act(cid, icon, name, keys):
        return ref(cid, descendants={cid + "-icon": {"icon": icon}, cid + "-name": {"content": name}, cid + "-keys": {"children": [keycap(k) for k in keys]}})
    group = lambda t: row(t, [tx(t, 12, "$muted")], height=28, padding=[0, 14])
    return comp("part-action-search", island("Action search", [
        row("Search", [ic("search", 16, "$muted"), tx("Search actions…", 14, "$muted")], width="fill_container", height=52, padding=[0, 20], gap=12), line(),
        col("Actions", [group("File"), act("action-row-chosen", "layout-grid", "Open dashboard", []), act("action-row", "save", "Save", ["⌘", "S"]), act("action-row", "save-all", "Save as…", ["⌘", "⇧", "S"]),
                        act("action-row", "file-plus", "New file", ["⌘", "⇧", "N"]), act("action-row", "folder-open", "Open file…", ["⌘", "O"]), group("Edit"), act("action-row", "undo-2", "Undo", ["⌘", "Z"]),
                        act("action-row", "redo-2", "Redo", ["⌘", "⇧", "Z"])], width="fill_container", padding=[8, 12], gap=0)],
        layout="vertical", width=742, height=400, cornerRadius=14, fill="$island", clip=True, **SHADOW))


def settings_window():
    pages = [("settings", "General", False), ("sparkle", "Agents", True), ("tag", "Agent names", False), ("message-square", "Chat", False), ("terminal", "MCP", False)]
    nav = col("Pages", [row("Title", [tx("Settings", 15, "$fg", "500")], height=48, padding=[0, 10])] + [
        ref("nav-row-chosen" if on else "nav-row", descendants={("nav-row-chosen" if on else "nav-row") + "-icon": {"icon": i}, ("nav-row-chosen" if on else "nav-row") + "-name": {"content": n}}) for i, n, on in pages],
        width=208, height="fill_container", padding=[20, 16], gap=2)
    providers = [("O", "OpenAI ChatGPT", "GPT 6.1 Sol · GPT 6.1 Terra · GPT 6 Astra", False), ("A", "Anthropic Claude Code", "Fable 5.2 · Opus 5.5 · Sonnet 5.5 · Haiku 5.5", True), ("C", "Cursor", "Gemini 3.8 Flash · Grok 4.6 · Composer 2.5", False)]
    cards = [ref("provider-card", width="fill_container", descendants={"provider-card-letter": {"content": l}, "provider-card-name": {"content": n}, "provider-card-models": {"content": m},
                                                                       "provider-card-state": {"children": [ref("pill-on" if on else "pill-off")]}}) for l, n, m, on in providers]
    rows = [ref("settings-row", width="fill_container"), line(),
            ref("settings-row", width="fill_container", descendants={"settings-row-title": {"content": "Agent cursor size"}, "settings-row-hint": {"content": "How large an AI model's cursor is drawn on the canvas."},
                                                                     "settings-row-control": {"children": [ref("choice")]}})]
    content = col("Page", [row("Top", [tx("Connect your AI provider", 20, "$fg", "500"), spacer(), ic("x", 14, "$muted")], width="fill_container"),
                           tx("Use tokens from your existing subscriptions. Pencraft automatically connects to Claude Code and Codex on your system.", 13.5, "$muted", textGrowth="fixed-width", width=420, lineHeight=1.3),
                           row("Sign in", [ref("btn-primary", descendants={"btn-primary-label": {"content": "Sign in with ChatGPT"}}), ref("btn-outline", descendants={"btn-outline-label": {"content": "Sign in with GitHub Copilot"}})], gap=12),
                           fr("Gap", [], width=1, height=8), tx("Setup providers", 14, "$fg", "500")] + cards + [fr("Gap", [], width=1, height=8), tx("Agents", 14, "$fg", "500")] + rows,
                  width="fill_container", height="fill_container", padding=[36, 40], gap=14)
    return comp("part-settings", island("Settings window", [nav, {"type": "rectangle", "name": "Divider", "width": 1, "height": "fill_container", "fill": "$border"}, content],
                                        layout="horizontal", alignItems="start", width=1100, height=760, cornerRadius=16, clip=True))


def home():
    card = lambda title, hint, colour: col(title, [fr("Preview", [], width="fill_container", height=84, cornerRadius=8, fill=colour), tx(title, 14, "$fg"), tx(hint, 12, "$muted")], width=200, padding=14, gap=8, cornerRadius=12, **BORDER)
    recent = lambda n, p: row(n, [ic("file", 14, "$muted"), tx(n, 13, "$fg"), tx(p, 12, "$muted")], gap=10, height=28)
    return comp("part-home", col("Home", [tx("Pencraft", 30, "$fg", "700"), tx("Design on an infinite canvas. Files stay .pen documents on your disk.", 13.5, "$muted"),
                                          row("Buttons", [ref("btn-primary"), ref("btn-outline")], gap=10), fr("Gap", [], width=1, height=12), tx("Start from", 13.5, "$fg", "600"),
                                          row("Starters", [card("Blank canvas", "An empty document", "$hover"), card("Starter board", "Layouts, a component, themes", "#3b82f633")], gap=14),
                                          fr("Gap", [], width=1, height=12), tx("Recent", 13.5, "$fg", "600"), recent("pencraft", "~/Documents/Pencraft/design"), recent("landing", "~/Documents/site")],
                                 width=620, padding=40, gap=12))


def close_prompt():
    return comp("part-close-prompt", island("Close prompt", [tx("Save changes to “Untitled”?", 15, "$fg", "600"), tx("Your changes will be lost if you don't save them.", 13, "$muted"), fr("Gap", [], width=1, height=6),
                                                             row("Buttons", [ref("btn-outline", descendants={"btn-outline-label": {"content": "Don't Save"}}), spacer(),
                                                                             ref("btn-outline", descendants={"btn-outline-label": {"content": "Cancel"}}), ref("btn-primary", descendants={"btn-primary-label": {"content": "Save"}})], width="fill_container", gap=8)],
                                            layout="vertical", width=400, padding=24, gap=8, cornerRadius=14, fill="$island", **SHADOW))


def design_card(x, y, title, colour):
    return col(title, [fr("Mark", [], width=34, height=34, cornerRadius=10, fill=colour), tx(title, 15, "#18181b", "600"), tx("Layouts that hold up on every screen size.", 12, "#6b6b73", textGrowth="fixed-width", width=200, lineHeight=1.4)],
               x=x, y=y, width=240, height=150, padding=20, gap=10, cornerRadius=16, fill="#ffffff", stroke="#e4e4e7", strokeWidth=1, strokeAlignment="inner")


def canvas_marks():
    x, y, w, h = 60, 80, 240, 150
    kids = [design_card(x, y, "Interface", "#f59e0b"),
            {"type": "rectangle", "name": "Selection box", "x": x, "y": y, "width": w, "height": h, "stroke": "$accent", "strokeWidth": 1.5},
            {"type": "rectangle", "name": "Rotation stem", "x": x + w / 2 - 0.5, "y": y - 26, "width": 1, "height": 26, "fill": "$accent"}, ref("rotate-dot", x=x + w / 2 - 5.5, y=y - 37)]
    kids += [ref("handle", x=cx - 4, y=cy - 4) for cx in (x, x + w) for cy in (y, y + h)]
    kids += [ref("radius-handle", x=cx - 4.5, y=cy - 4.5) for cx in (x + 14, x + w - 14) for cy in (y + 14, y + h - 14)]
    kids.append(ref("size-label", x=x + w / 2 - 36, y=y + h + 8, descendants={"size-label-text": {"content": "240 × 150"}}))
    ax = 380
    kids += [design_card(ax, y, "Systems", "#10b981"),
             {"type": "rectangle", "name": "AI wash", "x": ax, "y": y, "width": w, "height": h, "cornerRadius": 16, "fill": "#607dff1a"},
             {"type": "rectangle", "name": "AI light band", "x": ax, "y": y + 20, "width": w, "height": 60, "fill": {"type": "gradient", "gradientType": "linear", "rotation": 180, "colors": [{"color": "#607dff00", "position": 0}, {"color": "#607dff73", "position": 1}]}},
             {"type": "rectangle", "name": "AI flash outline", "x": ax - 6, "y": y - 6, "width": w + 12, "height": h + 12, "stroke": "$agent", "strokeWidth": 2},
             ref("ai-cursor", x=ax + w / 2, y=y + h / 2),
             tx("Selected layer", 11, "$muted", x=x, y=30), tx("Layer an AI model just changed", 11, "$muted", x=ax, y=30)]
    return comp("part-canvas-marks", fr("Canvas marks", kids, layout="none", width=680, height=290, cornerRadius=14, fill="$canvas", clip=True, **BORDER))


def parts(x, y):
    return board("Application parts", [
        section("Window top", [shown("Title island", title_island()), shown("Window actions", window_actions()), shown("Zoom", zoom())]),
        section("Tools", [shown("Tool rail", rail()), shown("Tool menus", tool_menu()), shown("Canvas marks", canvas_marks())], gap=48),
        section("Sidebar", [shown("Sidebar, layers", sidebar_layers()), shown("Sidebar, chat", sidebar_chat()), col("Message boxes", [shown("Message box", composer()), shown("Message bar", message_bar())], gap=28), shown("Properties", properties())]),
        section("Over the window", [shown("Action search", action_search()), col("Small windows", [shown("Close prompt", close_prompt()), shown("Message", ref("toast"))], gap=28)]),
        section("Settings and home", [shown("Settings window", settings_window())]),
        section("Home", [staged("Home screen", home())]),
    ], x, y, width=1900)


# ---------------------------------------------------------------- screens
def design_mock():
    return fr("Design", [tx("Northwind Studio", 14, "#18181b", "700", x=60, y=60), tx("Design calm, useful products.", 34, "#18181b", "700", x=60, y=100),
                         tx("A small sample board: rows and columns lay themselves out.", 13, "#6b6b73", x=60, y=152),
                         design_card(60, 200, "Research", "#3b82f6"), design_card(316, 200, "Interface", "#f59e0b"),
                         {"type": "rectangle", "name": "Selection box", "x": 316, "y": 200, "width": 240, "height": 150, "stroke": "$accent", "strokeWidth": 1.5},
                         ref("size-label", x=316 + 84, y=358, descendants={"size-label-text": {"content": "240 × 150"}}), ref("ai-cursor", x=520, y=150)],
              layout="none", width=640, height=420, fill="#f7f7f5")


def editor_screen(name, x, y, mode):
    top = row("Top", [ref("part-title"), spacer(), ref("part-actions")], width="fill_container", height=53, padding=[11, 8, 1, 8])
    left = col("Left", [ref("part-sidebar-layers", height="fill_container"), ref("part-composer")], height="fill_container", gap=8)
    centre = col("Canvas", [row("Design row", [design_mock()], width="fill_container", height="fill_container", justifyContent="center"), row("Bottom", [spacer(), ref("part-zoom")], width="fill_container")],
                 width="fill_container", height="fill_container", padding=[0, 6, 0, 0])
    body = row("Body", [left, ref("part-rail"), centre, ref("part-properties", height="fill_container")], width="fill_container", height="fill_container", padding=[8, 8, 8, 8], gap=8, alignItems="start")
    return col(name, [top, body], x=x, y=y, width=1352, height=878, fill="$canvas", clip=True, cornerRadius=12, theme={"Mode": mode}, **BORDER)


def simple_screen(name, x, y, child, mode="Light"):
    top = row("Top", [ref("part-title"), spacer(), ref("part-actions")], width="fill_container", height=53, padding=[11, 8, 1, 8])
    return col(name, [top, row("Middle", [child], width="fill_container", height="fill_container", justifyContent="center")], x=x, y=y, width=1352, height=878, fill="$canvas", clip=True, cornerRadius=12, theme={"Mode": mode}, **BORDER)


# ---------------------------------------------------------------- run
def box(answer, key="$0"):
    data = json.loads(answer)
    first = data["results"][0]["id"]
    return data["layoutBoxes"][first]


print(rpc("initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "Claude", "version": "1"}})["result"]["serverInfo"])
import os
if os.path.exists(TARGET):
    # Work in the tab the file is already open in, and replace what is there.
    old = [l["id"] for l in json.loads(call("open_document", {"path": TARGET}))["front"]["topLevelLayers"]]
    if old:
        call("edit_layers", {"description": "Clearing the old boards", "operations": [{"op": "delete", "id": i} for i in old]})
else:
    call("open_document")
call("edit_layers", {"description": "Setting up Pencraft's colours and themes", "operations": [{"op": "set_variables", "themes": {"Mode": ["Light", "Dark"]}, "variables": VARIABLES}]})
y = 0
for title, make in (("Foundations", foundations), ("Components", components), ("Application parts", parts)):
    answer = call("edit_layers", {"description": "Building " + title, "operations": [{"op": "insert", "node": make(0, y)}]})
    print(title, "advice:", json.loads(answer).get("advice"))
    b = box(answer)
    print(title, b)
    y += b["height"] + 160
screens = [editor_screen("Screen: editor, light", 0, y, "Light"), editor_screen("Screen: editor, dark", 1512, y, "Dark"),
           simple_screen("Screen: home", 0, y + 1038, ref("part-home")), simple_screen("Screen: settings, dark", 1512, y + 1038, ref("part-settings"), "Dark")]
out = json.loads(call("edit_layers", {"description": "Assembling the screens", "operations": [{"op": "insert", "node": s} for s in screens]}))
print("Screens advice:", out.get("advice"))
ids = [r["id"] for r in out["results"]]
state = json.loads(call("get_state"))
tops = [(l["id"], l["name"]) for l in state["front"]["topLevelLayers"]]
print(tops)
for i, (lid, name) in enumerate(tops):
    print(name, call("screenshot", {"ids": [lid], "scale": 1}, shot=f"design-{i}")[:80])
print(call("save_document", {"path": TARGET}))
