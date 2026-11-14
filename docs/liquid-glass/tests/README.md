# Fresh-agent tests of the liquid-glass skill

Each test gives an agent that has never seen this work only a user's request, Pencraft's public tools and
whatever those tools tell it (the guide, which lists the skill; `read_skill`). The agent builds the design,
checks it, and reports every place it had to guess or retry. Those reports are what the tests are for.

## Files

- `prompts/N.md`: the complete prompt of test N (the paths inside are absolute; adjust them when the
  checkout moves).
- `photos/`: five photo-like pictures made by a script from gradients, shapes and noise (no third-party
  material), used as content and backdrops.
- `pc.py`: calls one of Pencraft's tools from a shell, the way an MCP client would
  (`python pc.py tools` lists them).
- `start.sh N`, `stop.sh N`: start and stop a private Pencraft for one test. Its address file is written to
  `out/N/home`, so it neither sees nor disturbs a Pencraft you have open; `stop.sh` ends only that process.
- `out/N/design.pen`, `out/N/final.png`: what the agent of test N left. `renders/` holds the final pictures
  kept with the repository.

## Running one again

```bash
cargo build --manifest-path ../../../app/Cargo.toml
```

```bash
./start.sh 1
```

Then give `prompts/1.md` to a new agent (a fresh session of any coding agent that can run shell commands),
and afterwards:

```bash
./stop.sh 1
```

To look at a result yourself: `PENCRAFT_HOME=out/1/home python pc.py check_glass '{}'`.

## The tests

1. A photo-heavy feed with a floating glass tab bar (with a search tab) and a toolbar with grouped buttons
   and one tinted primary action.
2. The same over a light and over a dark text-heavy feed (the light or dark glass choice).
3. A media player with clear glass controls over bright artwork.
4. A dark desktop window with a glass sidebar, a popover menu and a frosted panel in the content.
5. Screen 1 in the Tinted, Reduced transparency and Increased contrast looks.

## What the runs exposed

Round 1 (tests 1-4), all ending with `check_glass` ok, but with friction that was then fixed:
regular glass let a sunset through so strongly that white labels on a selection failed (presets made more
opaque); the fix hints of `check_glass` did not fit several cases (now chosen by case); components were
drawn for light glass only and needed a dozen overrides on dark screens (labels now follow a `Mode`
token); the scroll-edge recipe left bars a flat backdrop; component spacing was off the review's scale;
text on the content was measured by nobody (now by `check_glass`); pictures had empty margins (fixed with
rounded clipping); the 35 % dimming value was not enough on its own (now a starting value).
Round 2 (`1b`, `3b`, `5`): test 1 passed on its first build with three edit calls; test 3 needed one
correction; test 5 showed that a layer's own `advanced` blocks the accessibility looks and that identical
glass panes were missing from the check's answer (both fixed).

## What is checked on every result

- `check_glass` answers `ok`: every label on glass has 4.5:1 against the worst of what is under it
  (measured on a picture), no glass lies on glass outside a group, nested corners are concentric, one glass
  variant per screen, and there is something behind the glass.
- The rim bends the backdrop: the unit test `the_rim_bends_what_is_behind_it_and_the_middle_does_not`
  compares a glass pane with a flat one along the rim.
- By eye: does it read as thick, curved glass or as a flat translucent box?
