# Talking to an assistant

The parts of a chat with an AI assistant: the bubble, an attachment, the trace of what the assistant did, a
code block, the bar that is typed into, a question the assistant asks, and a thread made of them. They use
the variables of `references/tokens.md` (the code block also `font-mono`: set it to an installed fixed-width
family) and the button of `references/controls.md`: insert that first.

Measured at the assistant components shadcn/ui added to its registry in 2026 (message, bubble, attachment,
marker, trace, code block, questionnaire). That is one well-known way to draw a chat; a product with its
own (bubbles on both sides, avatars, a coloured assistant) keeps its own, and takes the measures.

## Bubble and message

What the person wrote is a bubble: `$primary` with `$primary-foreground`, radius `$radius-xl`, 8 above and
below and 12 beside the text, 14 with a line height of 1.6, at most 80% of the thread's width, at the
trailing edge. What the assistant answers has no bubble: plain text across the thread's width, in
`$foreground`. That difference is the whole look; do not put both in bubbles, and never give the assistant a
coloured one.

```json
{"type":"frame","id":"ui-bubble","name":"Bubble","reusable":true,"width":400,"padding":[8,12],"cornerRadius":"$radius-xl","fill":"$primary","children":[
  {"type":"text","id":"ui-bubble-text","name":"Text","content":"Can Aksel and Ingrid swap on Saturday without either going over their hours?","fontFamily":"$font","fontSize":14,"fill":"$primary-foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6}]}
```

- Messages are 32 apart (24 in Compact); parts of one message 12 apart; two messages in a row from the same
  side 8 apart.
- A name or a time over a message is 12 at weight 500 in `$muted-foreground`, starting level with the text.
  Avatars are optional: where they are used they are 32, 8 beside the message, on both sides or neither.
- Under an answer, its actions (copy, good, again) are icons of 16 in `$muted-foreground`, 12 apart.
- Other bubbles, for a product that wants them: `$muted` (a neutral speaker), an outline, and
  `$destructive-tint` with `$destructive` text for a message that failed to send.

## Attachment

A file in a message or waiting in the prompt bar: a row with radius `$radius-xl` and a `$border` outline and no fill, 8
above and below and 10 beside, at least 160 wide; a 40 tile in `$muted` with a 16 icon, the name at weight
500 and one line of 12 under it. Several are 12 apart. A picture is a square thumbnail of 96 with the same
radius instead.

```json
{"type":"frame","id":"ui-attachment","name":"Attachment","reusable":true,"width":240,"padding":[8,10],"gap":8,"alignItems":"center","cornerRadius":"$radius-xl","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-attachment-media","name":"Media","width":40,"height":40,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-lg","fill":"$muted","children":[
    {"type":"icon","id":"ui-attachment-icon","name":"Icon","library":"lucide","icon":"file-text","width":16,"height":16,"fill":"$foreground"}]},
  {"type":"frame","id":"ui-attachment-text","width":"fill_container","layout":"vertical","gap":2,"children":[
    {"type":"text","id":"ui-attachment-title","name":"Title","content":"rota-week-48.pdf","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"ui-attachment-description","name":"Description","content":"PDF, 184 KB","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}
```

## Trace

What the assistant did before it answered: one line per step, 14 in `$muted-foreground` with a 16 icon, 8
apart. A step that has details opens: its chevron points down, and the details hang on a 1 wide `$border`
line under the icon's middle, 16 from it. A step still running shows `loader-circle` and is the last line.

```json
{"type":"frame","id":"ui-trace","name":"Trace","width":480,"layout":"vertical","gap":8,"children":[
  {"type":"frame","id":"ui-trace-1","name":"Step","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"icon","id":"ui-trace-1-icon","library":"lucide","icon":"check","width":16,"height":16,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-trace-1-label","content":"Read the rota for week 48","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"ui-trace-2","name":"Step, open","width":"fill_container","layout":"vertical","children":[
    {"type":"frame","id":"ui-trace-2-trigger","name":"Trigger","width":"fill_container","gap":8,"alignItems":"center","children":[
      {"type":"icon","id":"ui-trace-2-icon","library":"lucide","icon":"chevron-down","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"text","id":"ui-trace-2-label","content":"Checked both contracts for weekend hours","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
    {"type":"frame","id":"ui-trace-2-indent","width":"fill_container","padding":[0,0,0,7],"children":[
      {"type":"frame","id":"ui-trace-2-content","name":"Content","width":"fill_container","padding":[8,0,4,16],"stroke":"$border","strokeWidth":{"left":1},"children":[
        {"type":"text","id":"ui-trace-2-text","content":"Aksel: 32 of 37.5 hours used. Ingrid: 30 of 37.5. A six hour shift fits both.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6}]}]}]}]}
```

## Code block

Radius `$radius-lg`, a 1 wide `$input` border, `$card`. A header with a 16 icon and the file's name at weight
500, and `copy` at the trailing edge; the code 16 from the sides in the fixed-width family, one text layer
per line so indents hold. A changed line has `$muted` behind it across the block; a removed one
`$destructive-tint`. Write code that would run and that belongs to the conversation.

```json
{"type":"frame","id":"ui-code-block","name":"Code block","width":480,"layout":"vertical","cornerRadius":"$radius-lg","fill":"$card","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-code-block-header","name":"Header","width":"fill_container","padding":[12,16],"justifyContent":"space_between","alignItems":"center","children":[
    {"type":"frame","id":"ui-code-block-name","gap":8,"alignItems":"center","children":[
      {"type":"icon","id":"ui-code-block-icon","library":"lucide","icon":"file-code","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"text","id":"ui-code-block-title","name":"Title","content":"swap.sql","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"icon","id":"ui-code-block-copy","name":"Copy","library":"lucide","icon":"copy","width":16,"height":16,"fill":"$muted-foreground"}]},
  {"type":"frame","id":"ui-code-block-content","name":"Content","width":"fill_container","layout":"vertical","padding":[0,16,16,16],"children":[
    {"type":"text","id":"ui-code-block-line-1","content":"update shifts","fontFamily":"$font-mono","fontSize":13,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
    {"type":"text","id":"ui-code-block-line-2","content":"   set teammate = 'ingrid.vang'","fontFamily":"$font-mono","fontSize":13,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
    {"type":"text","id":"ui-code-block-line-3","content":" where desk = 'ferry'","fontFamily":"$font-mono","fontSize":13,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
    {"type":"text","id":"ui-code-block-line-4","content":"   and starts = '2026-11-28 08:00';","fontFamily":"$font-mono","fontSize":13,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6}]}]}
```

## Prompt bar

Where the person types: a field with radius `$radius-lg` in two rows. On top the text (a placeholder in
`$muted-foreground`), 12 from the edges; below it a row of tools 8 from the edges: at the leading edge what
is added to the message (`paperclip`, the scope or model as a quiet button with a `chevron-down`), at the
trailing edge the one filled control, a 32 square with `arrow-up`. While an answer is being written that
square holds `square` (stop). The bar is as wide as the thread and stands at its bottom.

```json
{"type":"frame","id":"ui-prompt-bar","name":"Prompt bar","reusable":true,"width":592,"layout":"vertical","cornerRadius":"$radius-lg","fill":["$background","$control"],"stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[
  {"type":"frame","id":"ui-prompt-bar-input","name":"Input","width":"fill_container","padding":[12,12,8,12],"children":[
    {"type":"text","id":"ui-prompt-bar-value","name":"Value","content":"Ask about this week's rota","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"ui-prompt-bar-tools","name":"Tools","width":"fill_container","padding":[0,8,8,8],"justifyContent":"space_between","alignItems":"center","children":[
    {"type":"frame","id":"ui-prompt-bar-leading","gap":4,"alignItems":"center","children":[
      {"type":"frame","id":"ui-prompt-bar-attach","name":"Attach","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-prompt-bar-attach-icon","library":"lucide","icon":"paperclip","width":16,"height":16,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-prompt-bar-scope","name":"Scope","height":32,"padding":[0,8],"gap":4,"alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"text","id":"ui-prompt-bar-scope-label","content":"Week 48","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$muted-foreground"},
        {"type":"icon","id":"ui-prompt-bar-scope-icon","library":"lucide","icon":"chevron-down","width":12,"height":12,"fill":"$muted-foreground"}]}]},
    {"type":"frame","id":"ui-prompt-bar-send","name":"Send","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
      {"type":"icon","id":"ui-prompt-bar-send-icon","library":"lucide","icon":"arrow-up","width":16,"height":16,"fill":"$primary-foreground"}]}]}]}
```

## Question

When the assistant needs a choice before it goes on. A line of 12 saying which question this is, the
question at 16 and weight 500, then the choices 12 apart: each a row with a `$border` outline and radius
`$radius-md`, 14 above and below and 16 beside, an indicator of 16 (a circle for one answer, a rounded square
for several), a label at weight 500 and one line in `$muted-foreground`. The chosen row has `$muted` and its
outline in `$ring`. The actions end at the trailing edge.

```json
{"type":"frame","id":"ui-question","name":"Question","width":480,"layout":"vertical","gap":24,"children":[
  {"type":"frame","id":"ui-question-header","name":"Header","width":"fill_container","layout":"vertical","gap":8,"children":[
    {"type":"text","id":"ui-question-progress","name":"Progress","content":"Question 1 of 2","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"},
    {"type":"text","id":"ui-question-title","name":"Title","content":"Who should take the ferry desk on Saturday morning?","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"ui-question-choices","name":"Choices","width":"fill_container","layout":"vertical","gap":12,"children":[
    {"type":"frame","id":"ui-question-1","name":"Choice, chosen","width":"fill_container","padding":[14,16],"gap":12,"cornerRadius":"$radius-md","strokeWidth":1,"strokeAlignment":"inner","fill":"$muted","stroke":"$ring","children":[
      {"type":"frame","id":"ui-question-1-mark","height":20,"alignItems":"center","children":[
        {"type":"frame","id":"ui-question-1-indicator","name":"Indicator","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$primary","children":[
          {"type":"ellipse","id":"ui-question-1-dot","width":6,"height":6,"fill":"$primary-foreground"}]}]},
      {"type":"frame","id":"ui-question-1-text","width":"fill_container","layout":"vertical","gap":2,"children":[
        {"type":"text","id":"ui-question-1-label","content":"Ingrid Vang","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-question-1-detail","content":"30 of 37.5 hours used this week","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]},
    {"type":"frame","id":"ui-question-2","name":"Choice","width":"fill_container","padding":[14,16],"gap":12,"cornerRadius":"$radius-md","strokeWidth":1,"strokeAlignment":"inner","stroke":"$border","children":[
      {"type":"frame","id":"ui-question-2-mark","height":20,"alignItems":"center","children":[
        {"type":"frame","id":"ui-question-2-indicator","name":"Indicator","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[]}]},
      {"type":"frame","id":"ui-question-2-text","width":"fill_container","layout":"vertical","gap":2,"children":[
        {"type":"text","id":"ui-question-2-label","content":"Solveig Aune","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-question-2-detail","content":"On call until noon","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]},
    {"type":"frame","id":"ui-question-3","name":"Choice","width":"fill_container","padding":[14,16],"gap":12,"cornerRadius":"$radius-md","strokeWidth":1,"strokeAlignment":"inner","stroke":"$border","children":[
      {"type":"frame","id":"ui-question-3-mark","height":20,"alignItems":"center","children":[
        {"type":"frame","id":"ui-question-3-indicator","name":"Indicator","width":16,"height":16,"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$control","stroke":"$input","strokeWidth":1,"strokeAlignment":"inner","children":[]}]},
      {"type":"frame","id":"ui-question-3-text","width":"fill_container","layout":"vertical","gap":2,"children":[
        {"type":"text","id":"ui-question-3-label","content":"Leave it open","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-question-3-detail","content":"It stays on the board for anyone to take","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]}]}]},
  {"type":"frame","id":"ui-question-actions","name":"Actions","width":"fill_container","gap":8,"justifyContent":"end","alignItems":"center","children":[
    {"type":"frame","id":"ui-question-skip","name":"Skip","height":36,"padding":[0,10],"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
      {"type":"text","id":"ui-question-skip-label","content":"Skip","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
    {"type":"ref","id":"ui-question-next","ref":"ui-button","descendants":{"ui-button-label":{"content":"Continue"}}}]}]}
```

## Thread

A column of at most 768 for reading, 24 from the screen's edges. A marker (a date, "New messages") is 14 in
`$muted-foreground` between two `$border` lines. The newest message is at the bottom, 32 above the prompt
bar. Draw a thread at a moment worth seeing: a question, the steps the assistant took, an answer that says
something exact, and what it produced.

```json
{"type":"frame","id":"ui-chat-thread","name":"Thread","width":640,"layout":"vertical","gap":32,"padding":24,"children":[
  {"type":"frame","id":"ui-chat-marker","name":"Marker","width":"fill_container","gap":8,"alignItems":"center","children":[
    {"type":"rectangle","id":"ui-chat-marker-line-1","width":"fill_container","height":1,"fill":"$border"},
    {"type":"text","id":"ui-chat-marker-text","content":"Today","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"},
    {"type":"rectangle","id":"ui-chat-marker-line-2","width":"fill_container","height":1,"fill":"$border"}]},
  {"type":"frame","id":"ui-chat-user","name":"Message, user","width":"fill_container","justifyContent":"end","children":[
    {"type":"ref","id":"ui-chat-user-bubble","ref":"ui-bubble"}]},
  {"type":"frame","id":"ui-chat-assistant","name":"Message, assistant","width":"fill_container","layout":"vertical","gap":12,"children":[
    {"type":"frame","id":"ui-chat-trace","name":"Trace","width":"fill_container","layout":"vertical","gap":8,"children":[
      {"type":"frame","id":"ui-chat-trace-1","name":"Step","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"icon","id":"ui-chat-trace-1-icon","library":"lucide","icon":"check","width":16,"height":16,"fill":"$muted-foreground"},
        {"type":"text","id":"ui-chat-trace-1-label","content":"Read the rota for week 48","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
      {"type":"frame","id":"ui-chat-trace-2","name":"Step, open","width":"fill_container","layout":"vertical","children":[
        {"type":"frame","id":"ui-chat-trace-2-trigger","name":"Trigger","width":"fill_container","gap":8,"alignItems":"center","children":[
          {"type":"icon","id":"ui-chat-trace-2-icon","library":"lucide","icon":"chevron-down","width":16,"height":16,"fill":"$muted-foreground"},
          {"type":"text","id":"ui-chat-trace-2-label","content":"Checked both contracts for weekend hours","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"frame","id":"ui-chat-trace-2-indent","width":"fill_container","padding":[0,0,0,7],"children":[
          {"type":"frame","id":"ui-chat-trace-2-content","name":"Content","width":"fill_container","padding":[8,0,4,16],"stroke":"$border","strokeWidth":{"left":1},"children":[
            {"type":"text","id":"ui-chat-trace-2-text","content":"Aksel: 32 of 37.5 hours used. Ingrid: 30 of 37.5. A six hour shift fits both.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6}]}]}]}]},
    {"type":"text","id":"ui-chat-answer","name":"Answer","content":"Yes. Aksel takes the harbour office from 14:00 and Ingrid the ferry desk from 08:00. Both stay under 37.5 hours, and neither works two weekends in a row.","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
    {"type":"ref","id":"ui-chat-file","ref":"ui-attachment"},
    {"type":"frame","id":"ui-chat-actions","name":"Actions","gap":12,"alignItems":"center","children":[
      {"type":"icon","id":"ui-chat-copy","library":"lucide","icon":"copy","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"icon","id":"ui-chat-good","library":"lucide","icon":"thumbs-up","width":16,"height":16,"fill":"$muted-foreground"},
      {"type":"icon","id":"ui-chat-again","library":"lucide","icon":"refresh-cw","width":16,"height":16,"fill":"$muted-foreground"}]}]},
  {"type":"ref","id":"ui-chat-prompt","ref":"ui-prompt-bar","width":"fill_container"}]}
```
