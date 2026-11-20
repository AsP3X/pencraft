import json, os
D = "$font-display"; F = "$font"

def T(content, size, weight="400", fill="$ink", font=F, **kw):
    t = {"type": "text", "content": content, "fontFamily": font, "fontSize": size, "fontWeight": weight, "fill": fill}
    t.update(kw); return t

def W(content, size, weight="400", fill="$ink", font=F, lh=1.5, **kw):
    return T(content, size, weight, fill, font, textGrowth="fixed-width", width=kw.pop("width", "fill_container"), lineHeight=lh, **kw)

def Fr(name, children, **kw):
    f = {"type": "frame", "name": name, "children": children}; f.update(kw); return f

def button(label, quiet=False, height=48, pad=24, **kw):
    b = Fr("Button, quiet" if quiet else "Button", [T(label, 16, "700", "$ink", D)], height=height, padding=[0, pad],
           justifyContent="center", alignItems="center", cornerRadius=8, **kw)
    if quiet: b.update(stroke="$ink", strokeWidth=1.5, strokeAlignment="inner")
    else: b["fill"] = "$crust"
    return b

def brand(colour):
    return Fr("Brand", [
        Fr("Mark", [{"type": "ellipse", "width": 28, "height": 28, "fill": "$crust", "x": 0, "y": 0}], width=28, height=14, layout="none", clip=True),
        T("Morgenbrød", 20, "700", colour, D, letterSpacing=-0.4)], gap=12, alignItems="center")

ACTION = "Order your first loaf"

# ---------- 1 variables
vars_ = {"op": "set_variables", "variables": {
    "enamel": {"type": "color", "value": "#173f86"},
    "enamel-deep": {"type": "color", "value": "#0f2a5c"},
    "enamel-light": {"type": "color", "value": "#2a56a8"},
    "paper": {"type": "color", "value": "#f0f3f8"},
    "surface": {"type": "color", "value": "#ffffff"},
    "ink": {"type": "color", "value": "#0d1f45"},
    "ink-soft": {"type": "color", "value": "#48566f"},
    "line": {"type": "color", "value": "#0d1f4524"},
    "crust": {"type": "color", "value": "#eaa93a"},
    "crust-dark": {"type": "color", "value": "#b8741f"},
    "crumb": {"type": "color", "value": "#f6ead0"},
    "on-blue-soft": {"type": "color", "value": "#c3d0ea"},
    "blue-line": {"type": "color", "value": "#ffffff38"},
    "font-display": {"type": "string", "value": "Century Gothic"},
    "font": {"type": "string", "value": "Palatino Linotype"}}}

# ---------- 2 nav + hero
nav = Fr("Navigation", [
    brand("$paper"),
    Fr("Links", [T(x, 16, "400", "$paper") for x in ["How it works", "This week's loaves", "Prices", "Questions"]], gap=32, alignItems="center"),
    Fr("Actions", [T("Log in", 16, "400", "$paper"), button(ACTION, height=40, pad=16)], gap=24, alignItems="center")],
    id="nav", width=1440, height=72, padding=[0, 80], justifyContent="space_between", alignItems="center", fill="$enamel")

door = Fr("Picture: the loaf on the door handle", [
    {"type": "rectangle", "name": "Door", "x": 120, "y": 64, "width": 400, "height": 608, "cornerRadius": [8, 8, 0, 0], "fill": "$enamel-light"},
    {"type": "rectangle", "name": "Upper panel", "x": 168, "y": 112, "width": 304, "height": 232, "cornerRadius": 4, "stroke": "$blue-line", "strokeWidth": 2, "strokeAlignment": "inner"},
    {"type": "rectangle", "name": "Lower panel", "x": 168, "y": 400, "width": 304, "height": 272, "cornerRadius": [4, 4, 0, 0], "stroke": "$blue-line", "strokeWidth": 2, "strokeAlignment": "inner"},
    Fr("House number", [T("14", 28, "700", "$enamel", D)], x=32, y=96, width=64, height=64, justifyContent="center", alignItems="center", cornerRadius=6, fill="$paper"),
    {"type": "rectangle", "name": "Handle plate", "x": 140, "y": 348, "width": 24, "height": 88, "cornerRadius": 12, "fill": "$crust-dark"},
    {"type": "rectangle", "name": "Handle", "x": 144, "y": 384, "width": 112, "height": 16, "cornerRadius": 8, "fill": "$crust"},
    {"type": "ellipse", "name": "Loaf", "x": 150, "y": 426, "width": 164, "height": 96, "fill": "$crust-dark"},
    {"type": "path", "name": "Score", "x": 196, "y": 434, "width": 72, "height": 22, "viewBox": [0, 0, 72, 22], "geometry": "M2 20 C20 2 52 2 70 20", "stroke": "$crumb", "strokeWidth": 3},
    Fr("Bag", [T("Morgenbrød", 18, "700", "$enamel", D)],
       x=132, y=464, width=200, height=192, layout="vertical", gap=4, padding=[0, 0, 24, 24], justifyContent="end", cornerRadius=[2, 2, 10, 10], fill="$crumb"),
    {"type": "path", "name": "Strap", "x": 160, "y": 380, "width": 144, "height": 88, "viewBox": [0, 0, 144, 88], "geometry": "M6 86 L56 6 L88 6 L138 86", "stroke": "$crumb", "strokeWidth": 8, "strokeLinejoin": "round"},
    Fr("Delivery tag", [
        T("Tue 24 Nov, 06:41", 14, "700", "$ink-soft", D),
        T("Landbrød, 800 g", 18, "700", "$ink", D),
        W("Out of the oven at 03:10. Hung here by Jonas, bike 2.", 14, fill="$ink-soft", lh=1.4)],
        x=300, y=488, width=252, layout="vertical", gap=8, padding=16, cornerRadius=8, fill="$surface",
        effect={"type": "shadow", "shadowType": "outer", "color": "#07132e59", "offset": {"x": 0, "y": 12}, "blur": 28}),
], id="door", width=600, height="fill_container", layout="none", clip=True, cornerRadius=[20, 20, 0, 0], fill="$enamel-deep")

hero = Fr("Hero", [
    Fr("Words", [
        W("Sourdough on your door handle before seven.", 60, "700", "$paper", D, lh=1.06, letterSpacing=-1.5, name="Headline"),
        W("Morgenbrød is a bread subscription from a small bakery in Grünerløkka. We bake through the night and cycle a loaf to your door every Tuesday and Friday, anywhere inside Ring 3.", 20, fill="$on-blue-soft", width=552, name="What it is, and for whom"),
        Fr("Actions", [button(ACTION),
            Fr("Second action", [T("See this week's loaves", 16, "700", "$paper", D), {"type": "icon", "library": "lucide", "icon": "arrow-down", "width": 18, "height": 18, "fill": "$paper"}], gap=8, alignItems="center")],
            gap=32, alignItems="center"),
        T("Your first loaf is free. After that NOK 79 a loaf; skip or stop by text.", 16, fill="$on-blue-soft", name="Reassurance")],
        width=600, height="fill_container", layout="vertical", gap=32, justifyContent="center", padding=[0, 0, 32, 0]),
    door], id="hero", width=1440, height=704, padding=[32, 80, 0, 80], gap=80, fill="$enamel")

page = Fr("Morgenbrød · Landing page, desktop", [nav, hero], id="page", x=0, y=0, width=1440, layout="vertical", fill="$paper")

# ---------- 3 night + loaves
def stop(time, title, text, last=False):
    return Fr("Stop", [
        T(time, 28, "700", "$ink", D, letterSpacing=-0.5),
        Fr("Rail", [{"type": "ellipse", "width": 16, "height": 16, "fill": "$crust" if last else "$ink", **({"stroke": "$ink", "strokeWidth": 2, "strokeAlignment": "inner"} if last else {})},
                    {"type": "rectangle", "width": "fill_container", "height": 2, "fill": "$ink"}], width="fill_container", height=16, alignItems="center", gap=0),
        Fr("Words", [T(title, 18, "700", "$ink"), W(text, 16, fill="$ink-soft")], width="fill_container", layout="vertical", gap=8, padding=[0, 32, 0, 0])],
        width="fill_container", layout="vertical", gap=16)

night = Fr("How it is made: one loaf's night", [
    Fr("Head", [
        W("Tuesday's loaf starts on Monday morning.", 40, "700", "$ink", D, lh=1.1, letterSpacing=-1, width=520, name="Heading"),
        W("Nothing is hurried along with baker's yeast. Eighteen hours pass between the first fold and the oven, which is why the crumb is sour and open, and still good on day four.", 18, fill="$ink-soft", width=560)],
        width="fill_container", justifyContent="space_between", alignItems="end"),
    Fr("Timetable", [
        stop("Mon 09:00", "Mixed and folded by hand", "Wheat and wholegrain rye from Østfold, water, salt, and a starter we have fed every day since 2019."),
        stop("Mon 13:00", "Shaped, then left alone", "Fourteen hours in the cold room. This is where the sour comes from, and nobody can speed it up."),
        stop("Tue 03:00", "Into the stone oven", "Sigrid bakes 212 loaves a night, 48 at a time, and has them cooling on racks by half past four."),
        stop("Tue 06:45", "On your door handle", "Three cargo bikes leave Markveien at 05:15. The last loaf of the round hangs by 07:00.", last=True)],
        width="fill_container")],
    id="night", width=1440, layout="vertical", gap=64, padding=[80, 80, 96, 80])

SOFT = {"type": "color", "color": "$ink", "opacity": 0.78}
def loaf_row(day, name, text, weight, last=False):
    r = Fr("Loaf", [
        T(day, 14, "700", "$ink", D, textGrowth="fixed-width", width=104),
        Fr("What", [T(name, 20, "700", "$ink", D), W(text, 16, fill=SOFT, lh=1.4)], width="fill_container", layout="vertical", gap=4),
        T(weight, 14, "700", "$ink", D, textGrowth="fixed-width", width=56, textAlign="right")],
        width="fill_container", gap=16, padding=[16, 0])
    if not last: r.update(stroke={"type": "color", "color": "$ink", "opacity": 0.3}, strokeWidth={"bottom": 1})
    return r

def week(title, dates):
    return Fr("Week", [T(title, 16, "700", "$ink", D), T(dates, 14, "400", SOFT)], width="fill_container", justifyContent="space_between", alignItems="center",
              padding=[0, 0, 8, 0], stroke="$ink", strokeWidth={"bottom": 2})

slip = Fr("Picture: the bakery's slip for two weeks", [
    week("Week 48", "posted Sunday 22 November"),
    loaf_row("Tue 24 Nov", "Landbrød", "Wheat and wholegrain rye, baked dark", "800 g"),
    loaf_row("Fri 27 Nov", "Frøbrød", "Sunflower, linseed and toasted sesame", "750 g", last=True),
    Fr("Gap", [], width="fill_container", height=16),
    week("Week 49", "posted Sunday 29 November"),
    loaf_row("Tue 1 Dec", "Havrebrød", "Oat porridge folded into the dough", "800 g"),
    loaf_row("Fri 4 Dec", "Fjellrug", "Seven parts rye, caraway, keeps a week", "900 g", last=True)],
    id="slip", width=600, layout="vertical", padding=[32, 32, 16, 32], cornerRadius=4, fill="$crust")

loaves = Fr("Feature, large: this week's loaves", [
    slip,
    Fr("Words", [
        W("Friday's loaf is never Tuesday's loaf.", 40, "700", "$ink", D, lh=1.1, letterSpacing=-1, name="Heading"),
        W("Sigrid sets the two loaves for each week and posts them on Sunday. Friday's is the seeded or the darker one, because it has to last you a weekend.", 18, fill="$ink-soft"),
        Fr("Figure", [T("4 days", 28, "700", "$ink", D, letterSpacing=-0.5),
                      W("is how long a loaf keeps in its linen bag, cut side down on the board. No plastic, no fridge.", 16, fill="$ink-soft", width=360, lh=1.4)],
           gap=24, alignItems="center", padding=[8, 0, 0, 0])],
        width=560, layout="vertical", gap=24)],
    id="loaves", width=1440, padding=[0, 80, 96, 80], justifyContent="space_between", alignItems="center")

# ---------- 4 pair + prices
def chip(day, num, skipped=False):
    if skipped:
        return Fr("Delivery, skipped", [T(day, 14, fill="$ink-soft"), T(num, 20, "700", "$ink-soft", D, strikethrough=True), T("skipped", 14, fill="$ink-soft")],
                  width="fill_container", layout="vertical", gap=4, padding=[12, 0], alignItems="center", cornerRadius=8, stroke="$line", strokeWidth=1, strokeAlignment="inner")
    return Fr("Delivery", [T(day, 14, fill="$ink-soft"), T(num, 20, "700", "$ink", D), T("loaf", 14, fill="$ink-soft")],
              width="fill_container", layout="vertical", gap=4, padding=[12, 0], alignItems="center", cornerRadius=8, fill="$paper")

pair = Fr("Features, two of unequal weight", [
    Fr("Feature: skip a week", [
        Fr("Words", [
            W("Away for a week? Skip it by Sunday night.", 28, "700", "$ink", D, lh=1.15, letterSpacing=-0.5),
            W("Skip one loaf or a whole month from the link in any delivery text. You pay for the loaves that were baked for you and never for the gap.", 18, fill="$ink-soft", width=600)],
            width="fill_container", layout="vertical", gap=12),
        Fr("Four weeks of deliveries", [
            Fr("Days", [chip("Tue", "24"), chip("Fri", "27"), chip("Tue", "1"), chip("Fri", "4"), chip("Tue", "8", True), chip("Fri", "11", True), chip("Tue", "15"), chip("Fri", "18")], width="fill_container", gap=8),
            T("Eight deliveries, two skipped: six loaves, NOK 474.", 16, "700", "$ink")],
            width="fill_container", layout="vertical", gap=16)],
        width=736, height="fill_container", layout="vertical", gap=32, padding=32, justifyContent="space_between", cornerRadius=12, fill="$surface"),
    Fr("Feature: nobody has to be home", [
        Fr("Words", [
            W("Not up at seven? You do not have to be.", 28, "700", "$paper", D, lh=1.15, letterSpacing=-0.5),
            W("The loaf hangs on your handle in a linen bag. We take last time's bag back and text you when it is there.", 18, fill="$on-blue-soft")],
            width="fill_container", layout="vertical", gap=12),
        Fr("Delivery text", [
            T("Tue 24 Nov, 06:41", 14, "700", "$ink-soft", D),
            W("Your Landbrød is on the door handle at Toftes gate 14. It will be warm for another hour. Jonas, bike 2", 16, fill="$ink", lh=1.45)],
            width="fill_container", layout="vertical", gap=8, padding=24, cornerRadius=[12, 12, 12, 4], fill="$paper")],
        width=520, height="fill_container", layout="vertical", gap=24, padding=32, justifyContent="space_between", cornerRadius=12, fill="$enamel")],
    id="pair", width=1440, height=480, padding=[0, 80, 96, 80], gap=24)

def point(text):
    return Fr("Point", [{"type": "icon", "library": "lucide", "icon": "check", "width": 18, "height": 18, "fill": "$ink"}, W(text, 16, fill="$ink", lh=1.4)], width="fill_container", gap=12)

def plan(name, price, line, btn, points, rec=False, badge=None):
    head = [T(name, 20, "700", "$ink", D)]
    if badge:
        head.append(Fr("Badge", [T(badge, 14, "700", "$ink", D)], height=28, padding=[0, 12], alignItems="center", cornerRadius=14, fill="$crust"))
    p = Fr("Plan, recommended" if rec else "Plan", [
        Fr("Head", head, width="fill_container", height=28, justifyContent="space_between", alignItems="center"),
        Fr("Price", [Fr("Row", [T(price, 40, "700", "$ink", D, letterSpacing=-1), T("a week", 16, fill="$ink-soft")], gap=8, alignItems="end"),
                     W(line, 16, fill="$ink-soft", lh=1.4)], width="fill_container", layout="vertical", gap=8),
        btn,
        Fr("What is in it", [point(x) for x in points], width="fill_container", layout="vertical", gap=12)],
        width=416, height="fill_container", layout="vertical", gap=24, padding=32, cornerRadius=12, fill="$surface")
    if rec: p.update(stroke="$ink", strokeWidth=2, strokeAlignment="inner")
    return p

prices = Fr("Prices", [
    W("NOK 79 a loaf, cycling included.", 40, "700", "$ink", D, lh=1.1, letterSpacing=-1, width=880, name="Heading"),
    Fr("Plans", [
        plan("Ett brød", "NOK 158", "One loaf on Tuesday, one on Friday, at NOK 79 each. Right for one to three at the table.",
             button(ACTION, width="fill_container"),
             ["At your door before 07:00", "A clean linen bag each time", "Skip or stop by text, any week"], rec=True, badge="Most doors take this"),
        plan("To brød", "NOK 292", "Two loaves each time, at NOK 73 each. For a family, or one for the freezer.",
             button("Order your first two loaves", quiet=True, width="fill_container"),
             ["Everything in Ett brød", "The second loaf sliced, if you like", "NOK 6 less for every loaf"]),
        Fr("Where we cycle", [
            T("Where we cycle", 20, "700", "$ink", D),
            W("Every address inside Ring 3: Grünerløkka, Sagene, St. Hanshaugen, Tøyen, Gamle Oslo, Frogner and Majorstuen.", 16, fill="$ink-soft"),
            W("Outside the ring? Your loaf waits at the bakery in Markveien from 07:00, at the same price.", 16, fill="$ink-soft"),
            W("You are charged on the last day of the month, for the loaves you were brought.", 16, fill="$ink-soft")],
            width="fill_container", layout="vertical", gap=16, padding=[32, 0, 0, 24])],
        width="fill_container", height=420, gap=24)],
    id="prices", width=1440, layout="vertical", gap=48, padding=[0, 80, 96, 80])

# ---------- 5 questions + closing + footer
def q(question, answer=None, last=False):
    kids = [Fr("Trigger", [W(question, 18, "700", "$ink", lh=1.3), {"type": "icon", "library": "lucide", "icon": "minus" if answer else "plus", "width": 20, "height": 20, "fill": "$ink"}], width="fill_container", gap=24, alignItems="center")]
    if answer: kids.append(W(answer, 18, fill="$ink-soft", width=656))
    f = Fr("Question, open" if answer else "Question", kids, width="fill_container", layout="vertical", gap=12, padding=[24, 0])
    if not last: f.update(stroke="$line", strokeWidth={"bottom": 1})
    return f

faq = Fr("Questions", [
    Fr("Lead", [
        W("What people ask at the bakery door", 40, "700", "$ink", D, lh=1.1, letterSpacing=-1, name="Heading"),
        W("Something else? Write to hei@morgenbrod.example and Sigrid answers once the ovens are off, around ten.", 18, fill="$ink-soft")],
        width=416, layout="vertical", gap=24),
    Fr("List", [
        q("What if someone takes the bread from my door?", "It has happened four times in 3,112 deliveries. Tell us the same morning and your next loaf is free. If your stairwell is locked, give us the code and we hang it on your own door, not the street door."),
        q("Can I have it sliced?"),
        q("What is in the bread, and what is not?"),
        q("Do you bake in July and at Christmas?"),
        q("How do I stop?", last=True)],
        width=736, layout="vertical")],
    id="faq", width=1440, padding=[0, 80, 96, 80], justifyContent="space_between")

closing = Fr("Closing", [
    W("Friday's loaf is the seeded one. Order by Thursday at 18:00.", 40, "700", "$paper", D, lh=1.1, letterSpacing=-1, width=760, name="Headline"),
    Fr("Actions", [button(ACTION), T("The first one is free. No card until the second.", 16, fill="$on-blue-soft")], layout="vertical", gap=16, alignItems="end")],
    id="closing", width=1440, padding=[96, 80], justifyContent="space_between", alignItems="center", fill="$enamel")

def col(title, items):
    return Fr("Links", [T(title, 14, "700", "$paper", D)] + [T(i, 16, fill="$on-blue-soft") for i in items], layout="vertical", gap=12)

footer = Fr("Footer", [
    Fr("Top", [
        Fr("Lead", [brand("$paper"), W("Sourdough from a small bakery in Grünerløkka, cycled to doors inside Ring 3 on Tuesdays and Fridays.", 16, fill="$on-blue-soft")], width=360, layout="vertical", gap=16),
        Fr("Columns", [
            col("Subscription", ["How it works", "This week's loaves", "Prices"]),
            col("Help", ["Questions", "Skip a delivery", "Contact"]),
            col("Bakery", ["About Sigrid", "Open Tue to Sat, 07 to 15", "Work with us"])], gap=96)],
        width="fill_container", justifyContent="space_between"),
    Fr("Bottom", [
        T("© 2026 Morgenbrød AS, Markveien 58B, 0550 Oslo. Org.nr. 931 204 877", 14, fill="$on-blue-soft"),
        Fr("Policies", [T("Privacy", 14, fill="$on-blue-soft"), T("Terms", 14, fill="$on-blue-soft")], gap=24)],
        width="fill_container", padding=[24, 0, 0, 0], justifyContent="space_between", stroke="$blue-line", strokeWidth={"top": 1})],
    id="footer", width=1440, layout="vertical", gap=48, padding=[64, 80, 32, 80], fill="$enamel-deep")

def ins(n): return {"op": "insert", "parent": "page", "node": n}
calls = {
    "a1": {"description": "Palette and faces for Morgenbrød", "operations": [vars_]},
    "a2": {"description": "Page with navigation and hero", "operations": [{"op": "replace", "id": "page", "node": page}]},
    "all": {"description": "Whole page rebuilt on one spacing scale", "operations": [{"op": "replace", "id": "page", "node": Fr("Morgenbrød · Landing page, desktop", [nav, hero, night, loaves, pair, prices, faq, closing, footer], id="page", x=0, y=0, width=1440, layout="vertical", fill="$paper")}]},
    "a3": {"description": "The loaf's night and this week's loaves", "operations": [ins(night), ins(loaves)]},
    "a4": {"description": "Skipping, not being home, and prices", "operations": [ins(pair), ins(prices)]},
    "a5": {"description": "Questions, closing and footer", "operations": [ins(faq), ins(closing), ins(footer)]},
}
here = os.path.dirname(os.path.abspath(__file__))
for k, v in calls.items():
    json.dump(v, open(os.path.join(here, k + ".json"), "w", encoding="utf-8"), ensure_ascii=False)
