# Fresh-agent tests of the ui-components skill

The same method as `docs/liquid-glass/tests` and with its scripts: an agent that has never seen this work
gets a user's request, Pencraft's public tools and whatever those tell it, builds the design, and reports
every place it had to guess or retry.

## Running one

From `docs/liquid-glass/tests` (the prompts name `out/uiN` there as the test's folder):

```bash
./start.sh ui1
```

Then give `prompts/1.md` from this folder to a new agent, and afterwards:

```bash
./stop.sh ui1
```

The paths inside the prompts are absolute; adjust them when the checkout moves.

## The tests

1. A settings dialog "in shadcn style" over a dimmed page (a named library; a dialog with several fields).
2. A state sheet for buttons: five variants, five states, light and dark ("standard look").
3. A command palette "like Linear or Raycast but plain", in three moments.
4. A dark phone screen: a list, a bottom drawer with a select and a note, and a toast.

`2b` and `4b` are the second round of 2 and 4. `renders/` holds the final pictures of the first round.

## What the first round exposed

All four built what was asked without a tool error. What the reports named was then fixed:

- In Pencraft's checks: the advice on a layer with the fill of its frame did not see a scrim lying between
  them (a dialog over a dimmed page); the review measured text against only the last fill of a frame and took
  a half see-through fill for a solid one; a picture with nothing counted came without a line saying so; the
  questions that come with a picture asked for "a mood of its own" of standard components.
- In the skill: the board of reusable components contradicted "nothing else" and ended up in the pictures;
  states lacked a hover for secondary and destructive buttons, a way to show a button without an icon
  loading, one meaning of disabled, the ring wrapper for buttons, and a build for the state board; the scrim
  was blurred (which read as a fault) and invisible in the dark theme; a drawer had no body for fields; there
  was no list row, no sizes for a phone, no place for a toast when a drawer is open; nested overrides, the
  caret of a typed field, key caps for shortcuts and the open or closed select were not said; field spacing
  was given three ways; Segoe UI, which the skill named, has no weight 500.

## What the second round exposed

`2b` built the sheet with row names, the components board out of the picture and no button changing width,
in the same six edit calls; `4b` needed three edit calls where `4` needed five, with no correction to the
picture. What they named was fixed in the skill: the destructive hover tint failed the contrast check; the
state board's headings stood 3 beside the buttons; the example had no loading cell; a toast put at the top
of a phone hid the page's title (it now goes above the drawer); the drawer's own buttons did not follow the
phone sizes; select and textarea padding on a phone were not given. These last fixes have not been tried by
a third round.

Not fixed, and worth knowing: the box `edit_layers` reports for a layer just inserted can be a few units off
what `read_layers` gives afterwards (text set in a font variable seems to be measured again); `screenshot` makes a large picture smaller than the `scale` asked for without
saying where the limit is; text made faint by `opacity` is not measured; there is no tool that lists the icon
names; `copy` gives the layers of a copy new ids, so a copied board is addressed by reading it first.
