//! What a model is told about designing well, beyond how a document is built ([`crate::agent::GUIDE`]).
//!
//! A model left to itself designs the average of everything it has seen: the same face, the same violet
//! gradient, the same three cards. This is the app's answer to that: a way of working (decide, set the system,
//! build, look, improve) and the list of what gives a generated design away.

use crate::faces;

/// How many installed font families the model is told about.
const MOST_FAMILIES: usize = 160;

/// How to design, for whichever model works on a document.
// Source: the method follows what Anthropic published on design by models ("Improving frontend design through
// skills"; the frontend-design skill; "Harness design for long-running application development": decide a
// direction before building, name what to avoid, judge the picture against set criteria), and the lists of
// tells follow the Impeccable catalogue of anti-patterns and the Vercel web interface guidelines. From the
// Impeccable skill (v4.6) also: the kinds of surface and how far a look may reach on each (its modes), choosing
// how much colour before which, the direction written as idea, own world and first view, the faces and looks
// models land on next, two looks as the ceiling, and what "bolder", "quieter", "simpler" and "polish" mean.
pub const CRAFT: &str = r#"# Designing well

Left to your defaults you produce the average of every design you have seen, and people see at a glance that a
model made it. Whenever you design something with a look of its own (a screen, a page, a poster, a card, a
component set), work the way a designer does. For a small change to an existing design (a colour, a label, one
more row) skip this and match what is there. What the user asks for outright always comes first: if they want a
violet gradient, they get one.

## 1. Decide before you draw
First say what kind of thing this is, because it decides how far a look may reach:
- Something that has to win a person over (a landing page, a poster, a cover, an invitation): the design is
  what they judge, so the look may own and order the whole. Its first view is a thesis, not a header: it shows
  the thing doing its job, at full scale. Ask what someone would describe an hour after one glance; if the
  honest answer is a mood, the idea has not committed yet.
- A screen someone works in (an app, a dashboard, settings, a table, a form): they came to get a job done and
  should know where everything is before they notice the look. The look lends four things only: type,
  palette, density and one signature move you can name ("the schedule set in a timetable's column rhythm").
  It never supplies the layout, the navigation or the controls: those are the standard ones people already
  know, and a screen dressed up as a terminal, a ledger or a newspaper is a costume. One quiet family can carry
  everything, in steps of about 1.2. Colour still does real jobs (a tinted ground or a coloured rail, selection,
  states, data); a grey screen with one accent is the stock tool without its polish, not restraint. An app
  someone opens by choice (their library, their bank, a hobby) keeps the familiar structure but may commit to
  colour and voice more than a tool at work: a ground with a temperature, and a first line that says what
  matters today in a sentence.
- A page someone reads (an article, documentation, a guide): the look owns the frame (masthead, navigation,
  ground, title and section openers, how tables and notes are set) and the reading column inside stays calm.

Then find the direction in the subject's own world. Note what this kind of design always looks like, and its
exact opposite: both are the rut. List five to seven real things the audience knows by heart (a publication, a
notation, a place, a ritual, a printed thing: a ferry timetable, a record sleeve, a lab notebook, a field guide),
not only the subject's most obvious object, and take the one that carries the subject best. A direction you
could give to any other subject unchanged is no direction. Two designs for two subjects should not share a
face or a palette.

Write the direction down for yourself in three lines before the first layer: the one idea this design owns and
the stock arrangement it refuses; the palette and shapes, plain enough to recognise with every word removed;
the first view, as what sits where and at what size. A line that reads like a mood is not decided yet. Dark or
light is never a default: say in one sentence who looks at this, where and in what light, and let that choose.

## 2. Set the system first, as variables
If the document already has variables or components, they are the system: use them and add nothing beside
them. Otherwise, before the first layer, make variables with `set_variables` and then use only those:
- Colour: choose how much colour before which colours. Restrained (greys tinted from the look plus one accent:
  right for most working screens and reading pages), committed (one saturated colour carries a third to half of
  the surface), a full palette (three or four colours, each with a named role), or drenched (the surface is the
  colour). Things that have to win someone over may take the bolder ones; when unsure between tidy and
  committed, commit. Colour works at the scale of the page: fields that own whole regions, not accents
  sprinkled over a neutral ground.
- Then the roles: a background, a surface, text, quieter text, a line, and the accent. One family of greys,
  tinted slightly toward the dominant colour, never pure grey beside a warm colour. On a coloured surface the
  quieter text is a shade of that colour or the text colour made transparent, never grey. Red, amber and green
  mean states and never decorate.
- Type: at most two families, from the installed ones listed below: one with character for display, one quiet
  one for reading. Choose a face like an object from the subject's world. A scale of five to seven sizes and
  nothing between them.
- Space: steps of 4 (4, 8, 12, 16, 24, 32, 48, 64, 96). Corners: one radius for controls, one for cards.

## 3. Build with a hierarchy
- One thing is read first, one second, one third. Make the first unmistakable: size jumps of two to three
  times, not 1.2; weights far apart (300 against 700 or 800, not 400 against 500); a quieter colour for what
  matters less. (On a working screen the steps are smaller and the order comes from position, weight and colour.)
- Large headlines: line height 1.0 to 1.15 and letter spacing slightly negative, never tighter than 4% of the
  size. Running text: 15 to 18 px, line height 1.4 to 1.6, lines of 45 to 75 characters
  (`textGrowth: "fixed-width"`).
- Align to a few shared edges so the eye has lines to follow. Centre only short, lone things. Let one part be
  clearly larger than the rest instead of giving every part the same weight; leave generous room around what
  matters and keep related things close: space is what groups. More space above a heading than below it.
- One main action per view, made unmistakable; at most four choices side by side where someone has to decide,
  the rest grouped or left out.
- A control has room: 8 to 12 above and below a button's label (4 in a small chip), 8 or more between a
  button and the text around it, and on a phone whatever is tapped is about 44 high. What someone is meant to
  press looks like a button, not like a line of text. Initials, figures or capitals alone in a circle or a
  badge can sit high or low: check them at `scale: 2` and nudge with 1 or 2 of padding until they sit centred.
- A heading beside a table or a list starts level with its first row, never centred beside it with a void
  above and below. A drawn object has to be believable as the thing (a string goes through its eyelet).
- Set a surface apart in one way only: a line, or a shadow, or a tint. Not all three, and no card inside a
  card. A shadow has an offset and a soft blur, in a dark shade of the ground.
- Write real copy: what this product would truly say, specific names, uneven numbers ("1,284", not "1,000"),
  rows of different lengths, buttons that name their action ("Send invoice", not "Submit"). A working screen
  shows real data with one state worth seeing (an overdue row, a warning, a very long name), never rows of the
  same stand-in.
- Icons: `icon` layers from one set at one size per role. Never emoji.
- Where the subject is something one wants to see (food, a place, a product, someone's work) and the design
  has to win a person over, text alone feels unfinished. Draw one confident picture for it from shapes and
  `path` layers in the palette (a flat illustration, a pattern, a diagram that belongs to the subject), large
  and in one place. Never a grey box that stands for a photograph, and no small decorations scattered about.
- Finish like a print proof; most of what looks careless is small. In lists and tables every row is built the
  same way and each part starts or ends on the same edge in every row: figures get one fixed width and
  `textAlign: "right"` (or the part before them fills the row), so "9 days" and "112 days" end on one line.
  The same kind of thing has one size, weight and colour everywhere, and a colour means one thing: if red is
  "late", everything late is red and nothing else is. Panels side by side share top and bottom edges
  (`height: "fill_container"`). A label never touches what it labels. A list above a tab bar or a footer
  shows whole rows only and ends with room (16 or more below its last row, and no line under that row); a
  row cut off by the bar, or a rule sitting on it, looks like a mistake in a picture. Show fewer rows, or
  end with a "See all" line. Amounts of one kind share one weight and
  colour; a state (pending, overdue) is shown by a word or a mark beside the amount, not by a third style. No panel is left half empty by accident:
  make it as tall as its content or give it content.

## 4. What gives a generated design away: leave these out
The user's own words can earn any of these; reaching for one when nobody asked means you were not deciding.
Then rebuild that part, don't soften it.
- Violet or indigo gradients, gradient text, glowing blobs, a coloured glow around a box, glass blur as
  decoration (a `material` the user asked for, used as the `liquid-glass` skill says, is not decoration), a hard block shadow without blur.
- The row of three equal cards with icon, title and one line; the hero with a headline, a subtitle and two
  buttons, all centred; a big number over a small label, three times in a row; little charts and progress
  rings that show nothing real.
- A small label in spaced capitals over a headline: the headline carries itself, so delete the label. Sections
  numbered 01, 02, 03 when the order tells nothing. One coloured word in the headline; an icon in a rounded
  square over every heading; a coloured stripe down the side of a card.
- Inter, Roboto or Arial because nothing was chosen; one size and weight for everything; a monospaced face to
  look technical where there is no code, data or measurement.
- The next defaults are no better. Faces: Fraunces, Playfair Display, Cormorant, Lora, Space Grotesk, Syne,
  DM Sans, Outfit, Plus Jakarta Sans, IBM Plex. Looks: cream with a serif headline and terracotta; near black
  with one neon accent and glowing edges; newspaper hairlines with an italic serif and small spaced mono
  labels; flat loud colour with black boxes, block shadows and heavy condensed capitals. A warm, bookish or
  family subject does not have to be cream and serif: its world has cloth, thread, jackets and endpapers in
  every strong colour. If someone could guess your look from the kind of product alone, choose again.
- Copy that sells instead of saying: unlock, elevate, seamless, effortless, supercharge. Lorem ipsum, John Doe,
  Acme.

## 5. Look, and judge it as someone else's work
Build the whole first, then take one `screenshot` of all of it. The app adds what it counted in the layers
(`review`); put those right first. Then judge the picture as a strict art director would judge a stranger's
work, on four things, the first two weighing most:
1. Is it one whole with a mood of its own, or parts next to each other?
2. Was each choice made for this subject, or could another product use this design unchanged?
3. Craft: hierarchy, alignment, spacing rhythm, contrast; nothing cut off, cramped or floating.
4. Does it do its job: is what the user needs findable at once? (For a working screen this weighs most: could
   someone do the task from it, and does it read as a tool of its kind?)
Name everything weak plainly, fix all of it in one go, and look once more to confirm. Two looks are the
ceiling: more rounds make a design tamer, not better. If the direction itself is weak, change the direction
instead of polishing it. Look closely at dense parts with `scale: 2`. Before you finish, take one thing away.

## 6. Changing a design that is already there
A change keeps what is there: its look, its copy, its variables, and everything the user did not name. Touch
only what was named; if the design's own system cannot do what is asked, ask before adding a colour, a face or
a radius. What the user's words mean:
- "Bolder": the part opts out of the design's own strongest moves. Bring it up to them with what the design
  already owns (its display type at full size, its colour as a field, its motif), make one decisive move
  completely and quiet what is around it. If everything got louder, it got flatter. Not more effects.
- "Quieter" or "calmer": fewer colours and less saturation, lighter weights, thinner or no lines, no glows or
  blurs, more room. Keep a few anchors and the character: quiet is not grey and not everything small.
- "Simpler" or "cleaner": take away. One main action, no repeated information, no box that only groups (use
  space), fewer sizes and colours. Never what someone needs in order to decide.
- "Polish" or "tidy up": no new ideas. Shared edges, the spacing and type scales, the same thing looking the
  same everywhere, contrast, real copy. If the idea itself is what's wrong, say so instead of redesigning
  quietly.
- "What do you think?" or "review this": change nothing. Say whether it could be any product's design, what
  works, and the three things that matter most, each with what to do instead.
"#;

/// Worlds a design can borrow its look from: each a way of setting type, using colour and arranging a page
/// that exists outside of software templates.
const WORLDS: [&str; 24] = [
    "a railway timetable: tabular figures, hairlines, one signal colour",
    "a Swiss exhibition poster: one grotesque face, a strict grid, big flush-left type",
    "a field guide to birds: a bookish serif, small captions, specimen plates on off-white",
    "a nautical chart: fine lines, depth tints of one hue, small precise labels",
    "a record sleeve of the 1960s: two inks, bold geometric shapes, type as picture",
    "a broadsheet front page: columns, rules, a heavy headline serif, dense but ordered",
    "an engineering drawing: thin lines, monospaced notes, title blocks, one accent for dimensions",
    "a museum wall label: generous white, a quiet serif, small grey captions",
    "an airport wayfinding sign: very large condensed sans, strong colour fields, pictograms",
    "a paperback cover of the 1970s: a warm two-colour palette, one large illustration field, tight type",
    "a laboratory notebook: a ruled grid, handwritten-sized notes, stamped dates, one highlighter colour",
    "a cookbook page: a wide margin for notes, a friendly serif, numbered steps set large",
    "a concert handbill: stacked condensed capitals of different sizes filling the width",
    "a bank note: fine guilloche-like lines, engraved serif capitals, two deep inks",
    "a seed packet: a framed picture area, small-caps labels, botanical greens and one flower colour",
    "a ski resort trail map: three coded colours with meaning, rounded sans, legend first",
    "a letterpress business card: much empty paper, one small block of perfectly set type",
    "a control panel: grouped controls, engraved labels, status lights as the only colour",
    "a film title card: one line of large type on a dark field, credits small below",
    "an atlas index: fine multi-column lists, bold entry words, page-edge tabs",
    "a market stall price board: chalk-white on dark, numbers larger than words",
    "a library catalogue card: typewriter face, red ruled line, punched-hole order",
    "a mid-century annual report: generous margins, one accent colour, charts as decoration-free facts",
    "a weather map: isolines, a tinted scale with meaning, small sans labels",
];

/// The ground a design can stand on, and the kind of face that can lead it: dealt with the worlds, so two
/// designs for the same request don't come out in the same colours and letters.
const GROUNDS: [&str; 5] = [
    "a light, warm paper ground",
    "a dark ground with light type",
    "one strong colour as the ground of the main part",
    "a cool, pale ground with one deep ink",
    "white, with colour only in large blocks",
];
/// See [`GROUNDS`].
const LEADS: [&str; 6] = [
    "a serif with character leads",
    "a condensed sans leads",
    "a plain grotesque in two far-apart weights leads",
    "a slab or typewriter face leads",
    "a wide or geometric sans leads",
    "a humanist sans leads and a serif reads",
];

/// A note for the first message of a conversation: three worlds, picked by `seed`, to take a design's direction
/// from when the user gave none.
// Human: told only what to avoid, a model lands on its next favourite, the same every time. Three different
// starting points each time make two designs for two requests differ, and the model still chooses.
// Source: Anthropic, "Improving frontend design through skills" (models converge on a new default once the
// old one is ruled out); dealing a direction per page is how the taste-skill project breaks that.
#[must_use]
pub fn directions(seed: u64) -> String {
    let count = WORLDS.len() as u64;
    let at = |step: u64| WORLDS[usize::try_from((seed.wrapping_add(step * 7)) % count).unwrap_or(0)];
    format!(
        "<pencraft_directions>\nIf this is a design with a look of its own and neither the user nor the document sets its look: before you decide, picture it in each of these three worlds, dealt at random for this conversation, and weigh them against the worlds you found in the subject itself, and take the one that the audience would recognise as theirs and that makes the subject clearest; a dealt world that loses still shows what yours lacks (a palette's full commitment, a grid's courage), so raise yours to match. Borrow a world's way of setting type, using colour and ordering a page, not its literal props; on a screen someone works in or a page someone reads it lends only type, palette, density and one signature move.\n1. {}\n2. {}\n3. {}\nName the first picture anyone would draw for this subject (for a coffee roaster by the sea: waves, a tide line): every other design for it has that picture, so it may be a detail of yours but not its main idea; find the second idea, in how the thing is made, used or kept. Two things are not yours to choose this time, because left to choose you would land on the same ground and the same face for every request of this kind: the design stands on {}, and in its type {}. Build your direction on these two (pick the installed family of that kind that suits the subject, and the colours that suit this ground); everything else is your decision. Only the user's own words or the document's existing variables overrule them.
</pencraft_directions>",
        at(0),
        at(1),
        at(2),
        GROUNDS[usize::try_from(seed / 3 % GROUNDS.len() as u64).unwrap_or(0)],
        LEADS[usize::try_from(seed / 5 % LEADS.len() as u64).unwrap_or(0)]
    )
}

/// The installed font families, as the model is told about them; empty when none were found.
#[must_use]
pub fn fonts() -> String {
    let families = faces::listed();
    if families.is_empty() {
        return String::new();
    }
    let more = if families.len() > MOST_FAMILIES { format!(", and {} more", families.len() - MOST_FAMILIES) } else { String::new() };
    format!(
        "## Fonts installed on this computer\nText is drawn in the `fontFamily` and `fontWeight` it names when that family is installed; any other family is drawn in a stand-in. Each family with the weights it comes in (a range means any weight in it):\n{}{more}.\nFigures: prices, amounts and anything in a column need lining figures of one width. Georgia, Constantia, Corbel and Candara have old-style figures that dip below the line (\"$19.50\" beside \"$84\" looks uneven): keep them for running text and set the figures in the other family. A fixed-width face (Courier New, Consolas, Cascadia, Lucida Console) leaves wide gaps around commas and points: use one only for small figures, codes and times, never for a headline amount.\n",
        families.iter().take(MOST_FAMILIES).map(String::as_str).collect::<Vec<_>>().join("; ")
    )
}

/// Everything a model is told about a document and about designing: the format, the craft, the fonts.
#[must_use]
pub fn guide() -> String {
    format!("{}\n{}\n{CRAFT}\n{}", crate::agent::GUIDE, crate::skills::index(), fonts())
}

/// What goes with a picture of `layers` many layers: what the review counted, and how to judge the picture.
#[must_use]
pub fn with_picture(notes: &[String], layers: usize) -> Option<String> {
    // Human: a single shape or a small drawing has no system to review; the questions are for designs.
    if notes.is_empty() && layers < 12 {
        return None;
    }
    let mut said = String::new();
    if !notes.is_empty() {
        said.push_str("review (counted in these layers; put each right unless the user asked for exactly that):\n");
        for note in notes {
            said.push_str("- ");
            said.push_str(note);
            said.push('\n');
        }
    }
    said.push_str(
        "Before you call this done, judge the picture as a strict art director would judge someone else's work: is it one whole with a mood of its own; was each choice made for this subject, or could another product use it unchanged; is anything misaligned, cramped, cut off or without hierarchy; is what the user needs findable at once, with one main action? Name what is weak, fix all of it in one go, and look once more to confirm; a second look that finds nothing new is the end.",
    );
    Some(said)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_drawing_gets_no_questions_and_a_design_gets_its_review() {
        assert_eq!(with_picture(&[], 3), None);
        assert!(with_picture(&[], 40).is_some_and(|said| said.starts_with("Before you call this done")));
        let said = with_picture(&["Text too faint.".to_owned()], 3).unwrap();
        assert!(said.starts_with("review") && said.contains("- Text too faint.\n") && said.contains("art director"));
        assert!(guide().starts_with("# Working on a Pencraft document") && guide().contains("# Designing well"));
        // Three different worlds each time, and other ones for another conversation.
        let dealt = directions(5);
        assert!(dealt.contains(WORLDS[5]) && dealt.contains(WORLDS[12]) && dealt.contains(WORLDS[19]));
        assert_ne!(dealt, directions(6));
    }
}
