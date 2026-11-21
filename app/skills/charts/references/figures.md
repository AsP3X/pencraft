# How figures are written

Conventions for figures in charts, tiles and tables, gathered from common practice and the samples of the
libraries this skill follows. No design system read for this skill publishes them as rules, so they are
this kit's own; where the product or its country writes figures another way, that way wins.

## One quantity, one way

| Kind | Write | Not |
|---|---|---|
| A count | 3,140 with the separator of the reader's country (3.140 in German, 3 140 in French) | 3140 beside 23,960 |
| A large count in a tile or on an axis | 23.9k, 1.2M: one decimal, the same for every figure beside it | 23.9k beside 1,204 |
| Money | The symbol where that currency puts it (£1,284.50, 1.284,50 €), two decimals or none, the same in a whole column | $1,284.5 |
| A share | 78%, or 78.3% where tenths matter; shares of one whole to the same decimals | 78.33333% |
| A change | With its sign and its unit: +12.5%, -3, +2.4 pts. Say what it is compared with | "12.5%" alone |
| A change between two shares | In points ("+2 pts"), not per cent of a per cent | "+2%" for 30% to 32% |
| A measure | The unit after a space (18.0 h, 4.2 km), once per column if the head names it | 18.0h |
| A range | An en dash without spaces for numbers (8–12), "to" between words and dates | 8 - 12 |
| Nothing | A word that says why ("Nobody yet", "No data"); 0 only for a real zero | A lone dash where a zero is meant |

- Decimals: one count of them per column and per axis. 6.0, 7.5, 10.0, not 6, 7.5, 10.
- Rounding in a tile; the exact figure in the table or the tooltip under it. Both come from one value.
- Negative numbers take a minus sign, not brackets and not red alone.
- Figures in a column end on one edge. Quill has no figures of one width: give the column one fixed
  width with `"textAlign":"right"`, and avoid faces whose figures dip below the line (Georgia, Constantia).
- Never shorten a figure or a date with an ellipsis. Shorten the label beside it.

## Dates and time

| Where | Write |
|---|---|
| A chart's axis | "Jun", "9 Nov", "W47", "08:00"; the year once, in the title or where it changes |
| A table | One form for the column: "21 Nov 2026" or "21/11/2026" as the country writes it |
| A time span | "June to November 2026"; "23 to 29 November" |
| Something recent | "5 min ago", "Yesterday", then the date after a week |
| A duration | "2 h 15 min", or "2:15" where the column head says hours |

Dates are real: check the weekday, the number of days in the month and the week number before writing
them. A chart's periods are evenly spaced, and a period without data is left empty, not joined across.

## A change, and its colour

A change is green or red only where up or down is truly good or bad for the reader (late departures up is
bad; tickets sold up is good). Where it is neither, it is `$foreground` with its sign and an arrow.
Whatever its colour, the sign and the word carry it too.
