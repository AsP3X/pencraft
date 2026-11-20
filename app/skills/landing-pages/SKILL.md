---
name: landing-pages
description: Design a landing page, home page or marketing site in Pencraft that makes an argument for one product to one buyer - the hero, proof, feature sections, customer quote, pricing table, questions, closing call to action and footer, in an order that follows the argument, at web widths with a phone version. Use when the user asks for a landing page, a home page, a marketing or product website, a hero section, a pricing page or pricing table, a waitlist or launch page, a features, testimonial, FAQ or footer section, or "a website for" something. Do not use for the screens of an app or dashboard (the ui-components skill), social posts, ads or emails (the formats skill), a logo, or slides.
license: Apache-2.0 with Commons Clause (see LICENSE in the Pencraft repository)
metadata:
  product: pencraft
  version: "1"
---

# Landing pages in Pencraft

A landing page is an argument: who this is for, what it does for them, why they should believe it, what it
costs, what to do now. A generated page is recognised by its skeleton before its colours: a centred
headline about nothing in particular, two buttons, three equal cards, a row of grey logos, a pricing table,
all in the order every other page has. This skill is how to build the argument instead. The look itself is
designed by the guide's "Designing well"; read this for what goes on the page and where.

Work with Pencraft's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`). The reference file
named below is read with `read_skill` (name `landing-pages`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and real content: no placeholder pictures, no invented customers of
  real companies, figures that agree. Do these without being asked, on every design this skill is used for.
  Only the user's own words overrule them; then do what was asked and say in one line what it costs.
- A starting point: the measures of section 3, the order and number of sections, the form of the hero, and
  the look of the example sections. Begin there, and change it when the design has a reason. They are still
  the expected way: leave one only with a reason, which can be short (the product has two plans, not three;
  the face has no such weight) and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Pencraft and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. Know the argument before the first layer: the buyer, the offer, the strongest proof, the objection,
   the action (section 1). A page without them is decoration.
2. The headline says what the visitor gets or what the thing is, in words only this product could use.
   "The modern platform for teams" and three verbs in a row ("Plan. Track. Ship.") are the lines to avoid.
3. Show the product: a real screen, drawn, or the real thing photographed. Never a grey box, never an
   abstract shape standing in for it. With no way to get a photograph of a thing, draw the thing itself
   plainly and say that a photograph should take its place.
4. One main action, said the same way wherever it appears. A button's label says what happens ("Start a
   free month"), rather than "Learn more", "Get started" or "Explore". A button that leads somewhere else (a
   free plan, a call with sales) says where it leads.

Starting points (not part of the list above: begin here, and change one when the user or the design
gives a reason):

- Sections differ in form because they do different jobs. No row of three equal cards with an icon, a
  title and a line. No section that could be moved to another product's page unchanged.
- The logo is at the top left, the navigation is visible, and the first screen shows that the page goes
  on.

## 1. The argument

Write five lines before you draw. Take them from the user's words and files; what is missing, ask for in
one question, or make up something specific and say that you did. For a real business, make up only what
the page needs to look finished (a price, a time, an address) and list every made-up fact in your answer.
Do not make up customers, reviews or figures of success for a real business: leave that section out.

| Line | Answers | Becomes |
|---|---|---|
| The buyer | Who decides, in their own job's words | The subhead's "for ..." and the vocabulary of the whole page |
| The offer | What they get, as an outcome they would say aloud | The headline |
| The proof | The one strongest reason to believe: a figure, a named customer, the product itself | The section right after the hero |
| The objection | What makes them hesitate (price, effort to switch, trust) | A feature section, the questions, the note under the button |
| The action | The one next step, and what it costs them | Every button |

The order of the sections is the order of that argument, not a habit. If the proof is stronger than the
features, it comes first. If the price is the advantage, it is not at the bottom. If the product is new and
has no customers, there is no logo row: show more of the product instead.

## 2. The sections and their jobs

| Section | Its job | Leave it out when |
|---|---|---|
| Navigation | Says where else one can go; carries the action | Rarely: a launch page still has the logo and the one button |
| Hero | The offer, the buyer, the action, the product | Rarely: something has to say the offer first, in whatever form |
| Customers (names or logos) | "Others like me use it" | There are none yet, or they are not like the buyer |
| Feature, large | One thing the product does, for one task of the buyer, with its picture and a figure | |
| Features, two or three of unequal weight | The second-order reasons, each in the form its content needs | They would only repeat the hero |
| Customer quote | A named person saying a specific thing, with what changed | There is no real quote: never invent a name for a real company |
| Prices | What it costs, which plan most should take | The price is "talk to us": then a line and the action, not a table |
| Questions | The objections, answered plainly | They would be filler ("Is it secure? Yes.") |
| Closing | The action once more, after the argument | The page is one screen long |
| Footer | The company, the links, the legal line | Rarely: a page of one screen may fold it into one line |

Four to seven sections between navigation and footer. More is a site map, not a page.
`references/sections.md` has each as a ready frame.

## 3. Measures

| Part | Value | From |
|---|---|---|
| Frame | 1440 wide; as tall as the page | Common practice; no standard |
| Content | 1280 wide, 80 at each side (1152 for a page that is mostly text) | Tailwind's container scale |
| Section | 96 above and below (64 to 128); bands of colour run to the frame's edges | Common practice |
| Navigation | 72 high, the logo left, links in the middle or after it, the action right | Common practice |
| Headline | 56 to 72 with a line height of 1.0 to 1.1, three lines at most, 12 to 20 words wide at most 600 | Common practice |
| Subhead | 18 to 22 with a line height of 1.5, at most 560 wide | |
| Section heading | 36 to 48, line height 1.1 | |
| Text | 17 to 18, line height 1.5, lines of 50 to 75 characters (500 to 700 wide) | Typographic convention |
| Smallest text | 14 (legal lines, labels in a product picture 12) | |
| Button | 48 high (40 in the navigation), 24 beside the label, 16 at weight 600 | This kit's |
| Between a heading and its text | 16 to 24; between text and buttons 24 to 32 | This kit's |
| Spacing and type | Gaps and paddings in steps of 4 (4, 8, 12, 16, 24, 32, 48, 64, 80, 96); seven type sizes at most on a page | Pencraft's review counts both |
| First screen | The top 800 to 900: offer, action, the product, and the start of what follows | NN/g: no false floor |

Only the container widths are a published scale. The rest is what well-made pages have in common; no
source this skill could read states them, so hold them as defaults, not as rules.

## 4. The hero

| Form | Use it when |
|---|---|
| Words left, product right | The default: there is a screen worth showing and a sentence worth reading |
| Words over a wide product screen | The product is wide (a dashboard, an editor); the screen runs off the bottom of the first screen |
| Words beside a phone | The product is an app: the `devices` skill gives the phone |
| Words on a photograph | The product is a place, a thing or a service: a real picture, darkened enough that white text reads 4.5:1 |
| Words alone, very large | The claim is the whole point and the product comes right under it |
| Words beside a tiled set of pictures | There are many things to show (a shop, a portfolio) |

- The headline is the offer; the subhead says what the thing is and who it is for, in one or two
  sentences; then the action, with at most one quieter second action as a text link with an arrow; then
  one line that removes a worry ("No card needed") or gives a number.
- Centred words with two equal buttons and nothing else is the generated hero. Words alone is a real form
  only when the type is the design.
- The product picture is a drawn screen at a moment that shows the offer: the open shift, the saved hour.
  Build it from real content with the ui-components skill, then show a part of it large. It may be cut
  off by the section's edge; it may not be blank.

## 5. Making sections that are not interchangeable

- A feature section starts from a task of the buyer ("fill an open shift"), not from a feature's name
  ("Smart scheduling"). Its heading is a sentence; its picture shows that task; its figure is a before and
  after, a count or a time.
- Two or three smaller reasons stand side by side at unequal widths (7 to 5, or 2 to 1 to 1), each in its
  own form: one a small chart, one a quote, one a list. If they come out as the same card three times,
  they are one list: write them as three lines of text.
- Proof is specific: a person with a name, a role and a company; a figure with its unit and what it was
  before; customer names set as words in one quiet colour when there are no logos to use. Never grey
  rectangles for logos, never five stars with no source.
- Prices: three plans (two or four at most) of equal size, 24 apart. The plan you recommend has the one
  filled button, a 2 wide outline and a small badge; the larger or raised middle card is the habit to
  avoid, unless the design's own system calls for it. Each has its name, its
  price with the unit, one line saying who it is for, the button, then what is in it as rows with a
  `check`. With monthly and yearly prices, a switch above the plans says the saving in words.
- Questions are the buyer's own, in their words, the first one open. Four to six.
- The closing section is one sentence and the action, on the page's strongest colour. It does not repeat
  the hero's headline.
- A band of colour (the quote, the closing, the footer) separates better than a line. Two bands of one
  colour work best apart.

## 6. Words

- Use the buyer's words, not the product's feature names. "The rota" where they say rota.
- Leave out: unlock, elevate, seamless, effortless, supercharge, revolutionary, all-in-one, next-generation,
  and any headline that would fit a competitor.
- Numbers are exact and odd (9 minutes, 240 crews, NOK 408), with their unit, and the same wherever they
  appear on the page.
- No eyebrow in spaced capitals over a heading, no section numbers, no emoji.

## 7. The phone version

A second frame 390 wide, one column, 20 at the sides, sections 64 apart. The navigation is the logo and a
menu button; the headline is 36 to 44; the product picture comes under the buttons; columns stack in the
order of their importance; the plans stack with the recommended one first; buttons are as wide as the
column and 48 high. Make it when the user asks for a responsive page or a mobile page; say when you did
not.

## 8. Workflow

1. `get_state`, `read_layers`: is there a brand, variables, a product screen already.
2. The five lines (section 1). Then the direction, by "Designing well": a palette, one display face and
   one text face, set as variables. `references/sections.md` begins with a set to start from.
3. The order of sections, written as a list.
4. The page as one vertical frame 1440 wide; insert each section into it from `references/sections.md`
   with your own content, or build it new by the measures above. The product pictures first: they decide
   the rest.
5. `screenshot` of the whole page (a section by itself is drawn without the page's ground). Check the
   first screen by the numbers: `read_layers` gives each section's place, and the section after the hero
   should begin above 900. Can someone say what this is, for whom and what to do? Put right what the
   review counted.
6. Tell the user in three sentences what the page argues, and what you made up.

## 9. Checklist

- The five lines can be read off the page.
- The headline would not fit another product. The button says what happens. Both are the same wherever
  they recur.
- The product is on the first screen. Nothing is a placeholder.
- No two sections have the same build. No three equal cards. No centred hero with two buttons.
- Names, figures and prices agree across the page and add up (a yearly price is twelve of its monthly
  one, less the saving it claims).
- Text lines stay under 75 characters; nothing under 14 outside a product picture.
- The footer names a real-looking company, an address and the legal links.

## 10. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| Any product's page | The stock order and a headline that fits anything | The five lines; reorder; rewrite the headline from the offer |
| A template | Every section centred, the same card everywhere | Words at the leading edge; a different form for each job |
| Empty | Huge type, no product | The product picture, large, on the first screen |
| A brochure | Paragraphs, no action after the hero | One action in the navigation, the hero, the prices and the closing |
| Busy | Seven colours, three button styles, an icon on every heading | One accent with a job, one filled button, icons only inside the product |
| Unbelievable | Round figures, stock names, five grey logos | Odd exact figures, named people, customer names as words |
