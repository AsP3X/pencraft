---
name: slides
description: Design presentation slides and decks in Quill that carry an argument - a title, statement, big number, one chart, comparison, quote, plan and closing ask, on 16:9 frames with type large enough for a room, or denser slides for a deck that is read. Use when the user asks for slides, a deck, a presentation, a pitch deck, a keynote, a talk, a board or investor presentation, a one-pager in slide form, or names PowerPoint, Keynote or Google Slides. Covers the order of a pitch deck, titles as conclusions, type sizes for presenting and for reading, slide kinds, rhythm and export. Do not use for social carousels (the formats skill), a landing page, a printed poster, or a chart by itself (the charts skill).
license: Apache-2.0 with Commons Clause (see LICENSE in the Quill repository)
metadata:
  product: pencraft
  version: "1"
---

# Slides in Quill

A deck is an argument cut into steps. Each slide makes one point, says it in its title, and shows the
evidence for it; the titles read in order are the whole case. A generated deck is recognised by the
opposite: topic titles ("Overview", "Key insights"), three bullets with an icon each, the same layout
twelve times, text too small for anyone past the second row. This skill is how to build the case. The look
is designed by the guide's "Designing well".

Work with Quill's tools (`get_state`, `read_layers`, `edit_layers`, `screenshot`, `export_layers`). The
reference file named below is read with `read_skill` (name `slides`, `file` as given).

Part of this skill is firm and part is a starting point. It says which, and the difference matters:

- Firm: the list "Every time" below, and the type floors of section 1: a slide nobody can read has failed.
  Do these without being asked, on every design this skill is used for. Only the user's own words overrule
  them; then do what was asked and say in one line what it costs.
- A starting point: the margins, where the title stands, which kinds of slide a deck uses and in what
  rhythm, and the look of the example deck. Begin there, and change it when the design has a reason. They
  are still the expected way: leave one only with a reason, which can be short and needs nobody's leave.
- A design that already has a look, a brand or a system keeps it. Take from this skill what the design
  lacks, and nothing that would replace what it has.
- The examples in the reference files show how a thing is built in Quill and at what size. Their colours,
  faces, words and arrangement are one answer among many, not the look to reproduce.
- The checklist and "When it looks wrong" at the end are there to be gone through every time, as a check
  of your own work. A line in them that speaks of a starting point is a starting point still.

Every time:

1. Decide first whether the deck is presented (seen from a room, with someone talking) or read (sent as
   a file, with nobody there). The two have different type sizes and different amounts of text
   (section 1). When the user does not say, ask, or take presented and say so.
2. One idea per slide. A second idea is a second slide. Slides cost nothing.
3. The title is the conclusion, as a sentence, at the top: "Open shifts are found too late", not "Shift
   coverage". Someone who reads only the titles gets the argument.
4. Nothing that must be read is smaller than the floor of its deck: 48 presented, 24 read.
5. Write all the titles before drawing any slide (section 2).
6. One frame per slide, 1920 by 1080, all with the same margins, side by side on the canvas.

## 1. Which deck

| | Presented | Read |
|---|---|---|
| Who explains it | The speaker | The slide itself |
| Words on a slide | 5 to 25 | 40 to 120 |
| Title | 56 to 80, two lines at most | 44 to 56, two lines, a full sentence with its "because" |
| What is read | 48 at least; 60 to 72 is better | 28 to 36 |
| Labels in a chart, a source | 32 to 40 | 20 to 24 |
| A figure that is the point | 240 to 420 | 120 to 200 |
| A slide holds | The point and one piece of evidence | The point, the evidence, and what follows from it |
| Screenshots of small text | Never: nobody can read them | Allowed, cropped to what matters and with the point marked |

The sizes are pixels on a 1920 wide frame. They are worked out from the two floors the sources give (no
type under 30 points when presenting, 24 points as the least for accessibility) on the classic slide,
where one point is two of these pixels; the ranges above the floors are this kit's.

## 2. The argument first

Write the deck as a list of titles before the first layer, and read it aloud: does it persuade without the
slides? Then give each title the kind of slide its evidence needs (section 4).

- Five to seven ideas for a whole deck; ten slides for a pitch, rarely more than fifteen for a talk of
  twenty minutes. If the list is longer, some of it is appendix.
- Start with the conclusion or the question, not with an agenda or "About us". End with what the audience
  should do, rather than with "Thank you" or "Questions?".
- A common order for a proposal: what is wrong, what it costs, what we propose, why it will work, what it
  takes, the decision we ask for.

| A pitch deck, by Y Combinator's template | By Sequoia's outline |
|---|---|
| Title: the company and one line saying what it does | Company purpose, in one sentence |
| Problem | Problem |
| Solution | Solution |
| Traction, and a second slide of it if there is more | Why now |
| What makes the company special | Market size |
| Business model | Competition |
| Market and how it grows | Product |
| Team | Business model |
| The ask: how much, and what it pays for | Team, then financials |

Take the first when there is traction to show early, the second when the market needs explaining. Either
way each slide's title is a claim, not the name of its row in this table.

## 3. The frame

- 1920 by 1080 (16:9). A 4:3 deck is 1440 by 1080; make it only when the user asks.
- Margins, the same on every slide: 96 at the sides, 80 on top, 56 at the bottom. Nothing but a ground or a
  picture goes outside them. No source gives margins for slides; these are this kit's.
- The title starts at the top left of that area on every slide that has one, so it does not jump when
  the slide changes. Words are at the leading edge, not centred; centre only a lone figure or a few words.
- A footer, if any, is one line of 24 at the bottom: the deck's short name left, the page number right.
  No logo on every slide, no date, no "Confidential" unless the user asks.
- Two grounds: the deck's own and its opposite (dark where the deck is light) for the slides that should
  stop the room: a section's start, a big number, a quote. One accent with one job.

## 4. Kinds of slide

| Kind | For | Rules |
|---|---|---|
| Title | What this is, for whom, when, by whom | The headline is the deck's claim or question, 110 to 140; one line of 40 under it |
| Statement | A point that needs no picture | One sentence, 96 to 120, and at most one line saying where it comes from |
| Big number | One figure that makes the point | The figure 240 to 420, beside it in words what it counts; the title says what it means |
| Evidence | A chart, a table of a few rows, a picture | One per slide; the title is what the chart shows; the source in one line under it. Build the chart with the `charts` skill, with labels at 32 to 40 |
| Comparison | Before and after, us and them, two options | Two columns, the same rows in both, the side you argue for marked by weight or the accent |
| Quote | A person's words as proof | The words 88 to 110, then who said it with role and place. Real people only |
| Plan | Steps in time | Three or four steps side by side, each with its date; the last one is the decision or result |
| Picture | The thing itself | The picture fills the slide; the title on it or on a band, reading 4.5:1 |
| List | Things that really are a list | Three to five lines of 48 to 60, no icons, no sub-points; if they are steps, use Plan |
| Section | A new part of a long deck | The part's claim on the opposite ground. Skip it under twelve slides |
| The ask | What the audience should do now | One sentence; what it costs or needs under it |

`references/slides.md` has one of each of the main kinds, as one deck.

## 5. Rhythm

- The same kind three times in a row tires a room: vary it. A deck of eight usually draws on five kinds
  or more.
- After two slides of evidence, one that needs no reading: a statement, a number, a quote.
- The opposite ground two or three times in a deck, where the argument turns.
- The things that mark a generated deck: an accent line under every title, body text centred, three
  boxes with an icon each, a title slide with a gradient, every slide a title and bullets, blue because
  nothing was chosen.

## 6. On the canvas, and out of it

- Slides stand side by side in reading order, 96 apart, tops level, five to a row; each frame is named
  with its number and kind ("04 Evidence").
- What the speaker says is not on the slide. If the user wants notes, put them as a text 28 under each
  slide, outside its frame.
- `export_layers` writes one file per slide: `.pdf` (text stays text) or `.png` at `scale` 1 for 1920 by
  1080, 2 for a sharper projector. Quill makes no PowerPoint, Keynote or Google Slides file and no PDF
  of several pages: say so, and that the slides can be placed into any of them as pictures or joined as
  pages.

## 7. Workflow

1. `get_state`, `read_layers`: a brand, variables, earlier slides.
2. Presented or read (section 1). The titles, as a list (section 2). The kind for each (section 4).
3. The direction, by "Designing well": two grounds, one accent, one display face and one text face, as
   variables. `references/slides.md` begins with a set to start from.
4. The slides, one frame each, from the nearest example or built new by the measures here.
5. `screenshot` of all slides together at a small scale: do the titles line up, does the rhythm change,
   can each be read at that size? Then each slide by itself; put right what the review counted.
6. Tell the user the argument in three sentences, how many slides there are, and what you made up.

## 8. Checklist

- The titles alone make the case, and each is a sentence with a verb.
- One idea on every slide; no slide needs "and" in its title.
- Nothing under the deck's floor except the footer.
- Titles start at the same place on every slide; margins are the same.
- At most two slides of one kind in a row; the opposite ground is used on purpose.
- Figures, dates and names agree across slides; dates are real (the weekday fits the date).
- The last slide asks for something.

## 9. What Quill cannot do, and what to do instead

| Missing | Instead |
|---|---|
| A PowerPoint, Keynote or Google Slides file | One PDF or PNG per slide |
| A PDF of several pages | One file per slide; say that they can be joined |
| Animation, builds, transitions | A build is two or three slides that differ by one thing |
| Speaker notes | Texts under the slides, outside the frames |
| Master slides | One slide made well and copied; the margins and the title's place kept by hand |

## 10. When it looks wrong

| It looks like | Cause | Fix |
|---|---|---|
| A document on a screen | A read deck's density shown in a room | Split each slide; 48 at least; say the rest aloud |
| A template | Title and three bullets on every slide | A kind for each point (section 4) |
| Empty | Topic titles and stock pictures | Titles that claim something, evidence that proves it |
| Unreadable from the back | Thin grey text, a screenshot, a chart with twelve labels | Weight 500 or more, strong contrast, one series, labels at 36 |
| Restless | A new layout, colour and size on every slide | One grid, two grounds, one accent; change the kind, not the system |
