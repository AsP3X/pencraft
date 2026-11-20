# Whole screens

How the components stand together in a screen someone works in: a dashboard, a sign-in page and an inbox,
each as one frame to insert and change, and the measures of the screens that come up most after them.
They are worked examples of how the components stand together, not layouts to reproduce: a dashboard may
lead with its table, a sign-in page may be split or full-bleed, an inbox may have two panes. The
frames are built from instances: insert the components they name first (`ui-sidebar` and `ui-topbar` from
`references/navigation.md`, `ui-stat` from `references/tables.md`, `ui-avatar`, `ui-badge`, `ui-button`,
`ui-button-icon-leading`, `ui-input` and `ui-search-input` from `references/data.md` and
`references/controls.md`).

The three frames are measured at shadcn/ui's 2026 blocks (dashboard-01, login-01 to 04, sidebar-09). The
table of other screens at the end is convention, composed from this kit's measures; no library publishes
them.

Defaults for a screen (each one a place to start; the user's request and a product's own layout come
first):

- A desktop screen is 1440 by 900 (1280 by 800 for a small window), a phone 390 or 393 wide. The frame is the
  window's content; it has no browser bar around it unless the user asks for one.
- One sidebar or one row of top navigation, not both carrying the same links. One top bar height, one
  control height per row, one gutter (24; 16 under 1024 wide).
- Sections are 24 apart, cards in a row 16 apart. Content that is read (a letter, an article, a form) stays
  in a column of at most 768, however wide the screen.
- The data is one story: the same people, places and dates in the sidebar, the tiles, the table and the
  mail, and figures that add up across them.
- Draw the screen at a moment worth seeing: one row selected, one thing open or overdue, not the blank
  first run (that is the empty state of `references/data.md`).

## Dashboard

A sidebar of 288, a top bar of 48, then the content on `$ground` with 24 around and 24 between its parts: four stat
tiles 16 apart, one chart card across the width, and the table. The page's title is in the top bar; the
content needs no second heading. On `$ground` the table's head row has no fill of its own.

```json
{"type":"frame","id":"ui-screen-dashboard","name":"Dashboard","width":1440,"height":900,"fill":"$ground","clip":true,"children":[
  {"type":"ref","id":"ui-dashboard-sidebar","ref":"ui-sidebar","width":288,"height":"fill_container"},
  {"type":"frame","id":"ui-dashboard-main","name":"Main","width":"fill_container","height":"fill_container","layout":"vertical","children":[
    {"type":"ref","id":"ui-dashboard-topbar","ref":"ui-topbar","width":"fill_container"},
    {"type":"frame","id":"ui-dashboard-content","name":"Content","width":"fill_container","height":"fill_container","layout":"vertical","gap":24,"padding":24,"children":[
      {"type":"frame","id":"ui-dashboard-stats","name":"Stat tiles","width":"fill_container","gap":16,"children":[
        {"type":"ref","id":"ui-dashboard-stat-1","ref":"ui-stat","width":"fill_container","descendants":{"ui-stat-label":{"content":"Hours rostered"},"ui-stat-value":{"content":"1,284.5"},"ui-stat-delta-icon":{"icon":"trending-up"},"ui-stat-delta-label":{"content":"+12.5%"},"ui-stat-note":{"content":"More than last week"},"ui-stat-detail":{"content":"Week 47, all four desks"}}},
        {"type":"ref","id":"ui-dashboard-stat-2","ref":"ui-stat","width":"fill_container","descendants":{"ui-stat-label":{"content":"Shifts open"},"ui-stat-value":{"content":"2"},"ui-stat-delta-icon":{"icon":"trending-down"},"ui-stat-delta-label":{"content":"-3"},"ui-stat-note":{"content":"Three fewer than on Monday"},"ui-stat-detail":{"content":"Both on Sunday"}}},
        {"type":"ref","id":"ui-dashboard-stat-3","ref":"ui-stat","width":"fill_container","descendants":{"ui-stat-label":{"content":"Swaps asked"},"ui-stat-value":{"content":"5"},"ui-stat-delta-icon":{"icon":"trending-up"},"ui-stat-delta-label":{"content":"+2"},"ui-stat-note":{"content":"Four are already agreed"},"ui-stat-detail":{"content":"One waits for an answer"}}},
        {"type":"ref","id":"ui-dashboard-stat-4","ref":"ui-stat","width":"fill_container","descendants":{"ui-stat-label":{"content":"Overtime"},"ui-stat-value":{"content":"18.0 h"},"ui-stat-delta-icon":{"icon":"trending-down"},"ui-stat-delta-label":{"content":"-6.5 h"},"ui-stat-note":{"content":"Lowest since week 41"},"ui-stat-detail":{"content":"Spread over three people"}}}]},
      {"type":"frame","id":"ui-dashboard-chart","name":"Chart card","width":"fill_container","layout":"vertical","gap":24,"padding":24,"cornerRadius":"$radius-xl","fill":"$card","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000000d"},"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
        {"type":"frame","id":"ui-dashboard-chart-header","name":"Header","layout":"vertical","gap":4,"children":[
          {"type":"text","id":"ui-dashboard-chart-title","name":"Title","content":"Hours rostered per week","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground"},
          {"type":"text","id":"ui-dashboard-chart-description","name":"Description","content":"The last twelve weeks, all desks","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]},
        {"type":"frame","id":"ui-dashboard-chart-plot","name":"Plot","width":"fill_container","height":208,"gap":16,"children":[
          {"type":"frame","id":"ui-dashboard-bar-1","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-1-mark","width":"fill_container","height":140,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-1-label","content":"W36","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-2","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-2-mark","width":"fill_container","height":151,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-2-label","content":"W37","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-3","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-3-mark","width":"fill_container","height":158,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-3-label","content":"W38","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-4","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-4-mark","width":"fill_container","height":152,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-4-label","content":"W39","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-5","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-5-mark","width":"fill_container","height":161,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-5-label","content":"W40","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-6","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-6-mark","width":"fill_container","height":167,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-6-label","content":"W41","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-7","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-7-mark","width":"fill_container","height":163,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-7-label","content":"W42","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-8","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-8-mark","width":"fill_container","height":171,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-8-label","content":"W43","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-9","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-9-mark","width":"fill_container","height":165,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-9-label","content":"W44","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-10","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-10-mark","width":"fill_container","height":173,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-10-label","content":"W45","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-11","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-11-mark","width":"fill_container","height":158,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-11-label","content":"W46","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
          {"type":"frame","id":"ui-dashboard-bar-12","name":"Week","width":"fill_container","height":"fill_container","layout":"vertical","gap":8,"justifyContent":"end","alignItems":"center","children":[
            {"type":"rectangle","id":"ui-dashboard-bar-12-mark","width":"fill_container","height":178,"fill":"$primary","cornerRadius":[4,4,0,0]},
            {"type":"text","id":"ui-dashboard-bar-12-label","content":"W47","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]}]}]},
      {"type":"frame","id":"ui-dashboard-table-box","name":"Table","width":"fill_container","layout":"vertical","cornerRadius":"$radius-lg","fill":"$card","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","clip":true,"children":[
        {"type":"frame","id":"ui-dashboard-table-head","name":"Head row","width":"fill_container","height":40,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
          {"type":"frame","id":"ui-dashboard-table-head-shift","name":"Sorted column","width":"fill_container","gap":6,"alignItems":"center","children":[
            {"type":"text","id":"ui-dashboard-table-head-shift-label","content":"Shift","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
            {"type":"icon","id":"ui-dashboard-table-head-shift-icon","library":"lucide","icon":"arrow-up","width":14,"height":14,"fill":"$muted-foreground"}]},
          {"type":"text","id":"ui-dashboard-table-head-mate","content":"Teammate","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":176},
          {"type":"text","id":"ui-dashboard-table-head-status","content":"Status","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":104},
          {"type":"text","id":"ui-dashboard-table-head-hours","content":"Hours","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
          {"type":"frame","id":"ui-dashboard-table-head-action","width":32,"height":32,"children":[]}]},
        {"type":"frame","id":"ui-dashboard-table-row-1","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
          {"type":"text","id":"ui-dashboard-table-row-1-shift","content":"Ferry desk, Saturday 08:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"ui-dashboard-table-row-1-mate","content":"Aksel Bjørnstad-Lie","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
          {"type":"frame","id":"ui-dashboard-table-row-1-status","width":104,"alignItems":"center","children":[
            {"type":"ref","id":"ui-dashboard-table-row-1-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Covered"}},"fill":"#00000000","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"}]},
          {"type":"text","id":"ui-dashboard-table-row-1-hours","content":"6.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
          {"type":"frame","id":"ui-dashboard-table-row-1-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
            {"type":"icon","id":"ui-dashboard-table-row-1-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
        {"type":"frame","id":"ui-dashboard-table-row-2","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
          {"type":"text","id":"ui-dashboard-table-row-2-shift","content":"Harbour office, Saturday 14:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"ui-dashboard-table-row-2-mate","content":"Maren Solheim","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
          {"type":"frame","id":"ui-dashboard-table-row-2-status","width":104,"alignItems":"center","children":[
            {"type":"ref","id":"ui-dashboard-table-row-2-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Covered"}},"fill":"#00000000","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner"}]},
          {"type":"text","id":"ui-dashboard-table-row-2-hours","content":"6.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
          {"type":"frame","id":"ui-dashboard-table-row-2-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
            {"type":"icon","id":"ui-dashboard-table-row-2-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
        {"type":"frame","id":"ui-dashboard-table-row-3","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
          {"type":"text","id":"ui-dashboard-table-row-3-shift","content":"Ticket desk, Sunday 09:30","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"ui-dashboard-table-row-3-mate","content":"Nobody yet","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":176},
          {"type":"frame","id":"ui-dashboard-table-row-3-status","width":104,"alignItems":"center","children":[
            {"type":"ref","id":"ui-dashboard-table-row-3-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Open","fill":"$primary-foreground"}},"fill":"$primary"}]},
          {"type":"text","id":"ui-dashboard-table-row-3-hours","content":"7.5","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
          {"type":"frame","id":"ui-dashboard-table-row-3-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
            {"type":"icon","id":"ui-dashboard-table-row-3-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]},
        {"type":"frame","id":"ui-dashboard-table-row-4","name":"Row","width":"fill_container","height":44,"padding":[0,8,0,12],"gap":16,"alignItems":"center","children":[
          {"type":"text","id":"ui-dashboard-table-row-4-shift","content":"Night watch, Sunday 22:00","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"ui-dashboard-table-row-4-mate","content":"Ingrid Vang","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":176},
          {"type":"frame","id":"ui-dashboard-table-row-4-status","width":104,"alignItems":"center","children":[
            {"type":"ref","id":"ui-dashboard-table-row-4-badge","ref":"ui-badge","descendants":{"ui-badge-label":{"content":"Swap asked"}}}]},
          {"type":"text","id":"ui-dashboard-table-row-4-hours","content":"10.0","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":56,"textAlign":"right"},
          {"type":"frame","id":"ui-dashboard-table-row-4-action","name":"Row actions","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
            {"type":"icon","id":"ui-dashboard-table-row-4-action-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$muted-foreground"}]}]}]}]}]}]}
```

- The chart here is plain bars made of rectangles on one baseline, one per week, each as high as its value,
  with a label of 12 under it. For axes, gridlines, lines, areas, donuts and the rules that keep a chart
  true to its numbers, read the `charts` skill; a decorative curve is worse than no chart.
- Pencraft's review takes a row of equal tiles for the stock grid of feature cards. Four stat tiles over
  the chart and table they sum up are the form of a dashboard: that remark can stand here, and only here.
- On a screen 1024 wide the sidebar is the rail of 48 and the tiles stand two by two. On a phone the
  sidebar is a sheet, the top bar holds a menu button, and tiles, chart and table stack in one column with
  16 around.

## Sign in

A column of 384 centred in the screen, 24 from its edges at least (40 from 768 wide): the brand (a 24 tile
and the name), then a card with 24 around and 24 between its parts: a title of 16 at weight 500 with one
line under it, the fields, the buttons one above the other at full width, the main one first. "Forgot your
password?" stands on the password label's row at the trailing edge. The page's ground is `$ground`, a shade off the
card.

```json
{"type":"frame","id":"ui-screen-sign-in","name":"Sign in","width":1280,"height":800,"layout":"vertical","gap":24,"justifyContent":"center","alignItems":"center","padding":24,"fill":"$ground","children":[
  {"type":"frame","id":"ui-sign-in-brand","name":"Brand","gap":8,"alignItems":"center","children":[
    {"type":"frame","id":"ui-sign-in-logo","name":"Logo","width":24,"height":24,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$primary","children":[
      {"type":"icon","id":"ui-sign-in-logo-icon","library":"lucide","icon":"ship","width":16,"height":16,"fill":"$primary-foreground"}]},
    {"type":"text","id":"ui-sign-in-name","content":"Harbour Rota","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]},
  {"type":"frame","id":"ui-sign-in-card","name":"Card","width":384,"layout":"vertical","gap":24,"padding":24,"cornerRadius":"$radius-xl","fill":"$card","effect":{"type":"shadow","offset":{"x":0,"y":1},"blur":2,"color":"#0000000d"},"stroke":"$hairline","strokeWidth":1,"strokeAlignment":"inner","children":[
    {"type":"frame","id":"ui-sign-in-header","name":"Header","width":"fill_container","layout":"vertical","gap":4,"children":[
      {"type":"text","id":"ui-sign-in-title","name":"Title","content":"Sign in","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-sign-in-description","name":"Description","content":"Use the address your rota is sent to.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
    {"type":"frame","id":"ui-sign-in-fields","name":"Fields","width":"fill_container","layout":"vertical","gap":24,"children":[
      {"type":"frame","id":"ui-sign-in-email","name":"Field","width":"fill_container","layout":"vertical","gap":8,"children":[
        {"type":"text","id":"ui-sign-in-email-label","name":"Label","content":"Work email","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
        {"type":"ref","id":"ui-sign-in-email-input","ref":"ui-input","width":"fill_container","descendants":{"ui-input-value":{"content":"maren@harbour.example","fill":"$foreground"}}}]},
      {"type":"frame","id":"ui-sign-in-password","name":"Field","width":"fill_container","layout":"vertical","gap":8,"children":[
        {"type":"frame","id":"ui-sign-in-password-head","width":"fill_container","justifyContent":"space_between","alignItems":"center","children":[
          {"type":"text","id":"ui-sign-in-password-label","name":"Label","content":"Password","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"},
          {"type":"text","id":"ui-sign-in-password-link","content":"Forgot your password?","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","underline":true}]},
        {"type":"ref","id":"ui-sign-in-password-input","ref":"ui-input","width":"fill_container","descendants":{"ui-input-value":{"content":"••••••••••••","fill":"$foreground"}}}]}]},
    {"type":"frame","id":"ui-sign-in-actions","name":"Actions","width":"fill_container","layout":"vertical","gap":8,"children":[
      {"type":"ref","id":"ui-sign-in-submit","ref":"ui-button","width":"fill_container","descendants":{"ui-button-label":{"content":"Sign in"}}},
      {"type":"frame","id":"ui-sign-in-sso","height":36,"padding":[0,10],"gap":6,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$control","stroke":"$border","strokeWidth":1,"strokeAlignment":"inner","width":"fill_container","children":[
        {"type":"text","id":"ui-sign-in-sso-label","content":"Sign in with single sign-on","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground"}]}]},
    {"type":"frame","id":"ui-sign-in-other","width":"fill_container","justifyContent":"center","children":[
      {"type":"text","id":"ui-sign-in-other-text","content":"New here? Ask your manager for an invite.","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground"}]}]}]}
```

- Sign up is the same frame with its own fields; say once, under the button, what happens next.
- Split screen: two halves. The form's half has 40 around, the brand at its top left and a form of 320
  centred in what is left, without a card; the other half is `$muted` with one picture or one sentence set
  large. Under 1024 wide only the form's half is shown.
- Other ways to sign in are outline buttons under the main one, after a line of 14 in `$muted-foreground`
  between two `$border` lines ("or").
- An error stands in its field (`"state":"invalid"` with a sentence under it); a wrong password is said
  once, over the fields, as an alert.

## Inbox, and any list with a detail beside it

Three panes: a rail of icons 48 wide, a list 300 wide, and the detail taking the rest. The list's header has
16 around: the title at 16 and weight 500 with a filter at the trailing edge, and a search input under it.
Each row has 16 around and 8 between its lines: who and when (the time in 12 at the trailing edge), the
subject at weight 500, and two lines of 12 in `$muted-foreground`. The open row has `$accent`. The detail
has a bar 56 high with its actions, then what is read in a column of 640 to 768, 40 from the bar.

```json
{"type":"frame","id":"ui-screen-inbox","name":"Inbox","width":1280,"height":800,"fill":"$background","clip":true,"children":[
  {"type":"frame","id":"ui-inbox-rail","name":"Rail","width":49,"height":"fill_container","layout":"vertical","gap":16,"padding":[8,8,8,8],"alignItems":"center","fill":"$sidebar","stroke":"$border","strokeWidth":{"right":1},"children":[
    {"type":"frame","id":"ui-inbox-logo","name":"Logo","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-lg","fill":"$primary","children":[
      {"type":"icon","id":"ui-inbox-logo-icon","library":"lucide","icon":"ship","width":16,"height":16,"fill":"$primary-foreground"}]},
    {"type":"frame","id":"ui-inbox-rail-menu","name":"Menu","layout":"vertical","gap":4,"children":[
      {"type":"frame","id":"ui-inbox-rail-inbox","name":"Inbox","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","fill":"$accent","children":[
        {"type":"icon","id":"ui-inbox-rail-inbox-icon","library":"lucide","icon":"inbox","width":16,"height":16,"fill":"$foreground"}]},
      {"type":"frame","id":"ui-inbox-rail-sent","name":"Sent","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-inbox-rail-sent-icon","library":"lucide","icon":"send","width":16,"height":16,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-inbox-rail-archive","name":"Archive","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-inbox-rail-archive-icon","library":"lucide","icon":"archive","width":16,"height":16,"fill":"$muted-foreground"}]},
      {"type":"frame","id":"ui-inbox-rail-bin","name":"Bin","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
        {"type":"icon","id":"ui-inbox-rail-bin-icon","library":"lucide","icon":"trash","width":16,"height":16,"fill":"$muted-foreground"}]}]}]},
  {"type":"frame","id":"ui-inbox-list","name":"List","width":301,"height":"fill_container","layout":"vertical","fill":"$sidebar","stroke":"$border","strokeWidth":{"right":1},"children":[
    {"type":"frame","id":"ui-inbox-list-header","name":"Header","width":"fill_container","layout":"vertical","gap":14,"padding":16,"stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"frame","id":"ui-inbox-list-title-row","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-inbox-list-title","name":"Title","content":"Inbox","fontFamily":"$font","fontSize":16,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"frame","id":"ui-inbox-unread","name":"Unread only","gap":8,"alignItems":"center","children":[
          {"type":"text","id":"ui-inbox-unread-label","content":"Unread","fontFamily":"$font","fontSize":14,"fill":"$foreground"},
          {"type":"frame","id":"ui-inbox-unread-track","name":"Track","width":32,"height":18,"padding":2,"alignItems":"center","cornerRadius":9,"fill":"$input","children":[
            {"type":"ellipse","id":"ui-inbox-unread-thumb","name":"Thumb","width":14,"height":14,"fill":"$background"}]}]}]},
      {"type":"ref","id":"ui-inbox-search","ref":"ui-search-input","width":"fill_container","descendants":{"ui-search-input-value":{"content":"Search mail"}}}]},
    {"type":"frame","id":"ui-inbox-mail-1","name":"Mail, open","width":"fill_container","layout":"vertical","gap":8,"padding":16,"stroke":"$border","strokeWidth":{"bottom":1},"fill":"$accent","children":[
      {"type":"frame","id":"ui-inbox-mail-1-head","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-inbox-mail-1-sender","content":"Aksel Bjørnstad-Lie","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-inbox-mail-1-time","content":"09:12","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"text","id":"ui-inbox-mail-1-subject","content":"Saturday swap with Ingrid","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-inbox-mail-1-preview","content":"Ingrid said yes this morning. Can you move me to the harbour office from 14:00?","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":260,"lineHeight":1.4}]},
    {"type":"frame","id":"ui-inbox-mail-2","name":"Mail","width":"fill_container","layout":"vertical","gap":8,"padding":16,"stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"frame","id":"ui-inbox-mail-2-head","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-inbox-mail-2-sender","content":"Ingrid Vang","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-inbox-mail-2-time","content":"08:47","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"text","id":"ui-inbox-mail-2-subject","content":"Re: Saturday swap","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-inbox-mail-2-preview","content":"Fine by me, as long as I am done by two. I have the ferry desk keys already.","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":260,"lineHeight":1.4}]},
    {"type":"frame","id":"ui-inbox-mail-3","name":"Mail","width":"fill_container","layout":"vertical","gap":8,"padding":16,"stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"frame","id":"ui-inbox-mail-3-head","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-inbox-mail-3-sender","content":"Ticket desk","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-inbox-mail-3-time","content":"Yesterday","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"text","id":"ui-inbox-mail-3-subject","content":"Sunday 09:30 is still open","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-inbox-mail-3-preview","content":"Nobody has taken the ticket desk on Sunday morning. It closes for booking on Friday.","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":260,"lineHeight":1.4}]},
    {"type":"frame","id":"ui-inbox-mail-4","name":"Mail","width":"fill_container","layout":"vertical","gap":8,"padding":16,"children":[
      {"type":"frame","id":"ui-inbox-mail-4-head","width":"fill_container","gap":8,"alignItems":"center","children":[
        {"type":"text","id":"ui-inbox-mail-4-sender","content":"Solveig Aune","fontFamily":"$font","fontSize":14,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
        {"type":"text","id":"ui-inbox-mail-4-time","content":"Monday","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground"}]},
      {"type":"text","id":"ui-inbox-mail-4-subject","content":"Time off in week 49","fontFamily":"$font","fontSize":14,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"text","id":"ui-inbox-mail-4-preview","content":"I would like Thursday and Friday off in week 49 if the rota allows it.","fontFamily":"$font","fontSize":12,"fill":"$muted-foreground","textGrowth":"fixed-width","width":260,"lineHeight":1.4}]}]},
  {"type":"frame","id":"ui-inbox-detail","name":"Detail","width":"fill_container","height":"fill_container","layout":"vertical","children":[
    {"type":"frame","id":"ui-inbox-detail-bar","name":"Bar","width":"fill_container","height":56,"padding":[0,16,0,24],"gap":8,"alignItems":"center","stroke":"$border","strokeWidth":{"bottom":1},"children":[
      {"type":"text","id":"ui-inbox-detail-place","content":"Inbox","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"},
      {"type":"frame","id":"ui-inbox-detail-actions","name":"Actions","gap":4,"alignItems":"center","children":[
        {"type":"frame","id":"ui-inbox-reply","name":"Reply","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
          {"type":"icon","id":"ui-inbox-reply-icon","library":"lucide","icon":"reply","width":16,"height":16,"fill":"$foreground"}]},
        {"type":"frame","id":"ui-inbox-archive","name":"Archive","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
          {"type":"icon","id":"ui-inbox-archive-icon","library":"lucide","icon":"archive","width":16,"height":16,"fill":"$foreground"}]},
        {"type":"frame","id":"ui-inbox-more","name":"More","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":"$radius-md","children":[
          {"type":"icon","id":"ui-inbox-more-icon","library":"lucide","icon":"ellipsis","width":16,"height":16,"fill":"$foreground"}]}]}]},
    {"type":"frame","id":"ui-inbox-detail-body","name":"Body","width":"fill_container","height":"fill_container","layout":"vertical","padding":[40,24],"alignItems":"center","children":[
      {"type":"frame","id":"ui-inbox-letter","name":"Letter","width":640,"layout":"vertical","gap":16,"children":[
        {"type":"frame","id":"ui-inbox-letter-header","name":"Header","width":"fill_container","layout":"vertical","gap":8,"children":[
          {"type":"text","id":"ui-inbox-letter-subject","name":"Subject","content":"Saturday swap with Ingrid","fontFamily":"$font","fontSize":24,"fontWeight":"500","fill":"$foreground","textGrowth":"fixed-width","width":"fill_container"},
          {"type":"text","id":"ui-inbox-letter-meta","name":"From","content":"Aksel Bjørnstad-Lie to you, today at 09:12","fontFamily":"$font","fontSize":14,"fill":"$muted-foreground","textGrowth":"fixed-width","width":"fill_container"}]},
        {"type":"text","id":"ui-inbox-letter-1","content":"Hi Maren,","fontFamily":"$font","fontSize":16,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
        {"type":"text","id":"ui-inbox-letter-2","content":"Ingrid said yes this morning, so we would like to swap on Saturday. She takes the ferry desk from 08:00 and I take the harbour office from 14:00.","fontFamily":"$font","fontSize":16,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
        {"type":"text","id":"ui-inbox-letter-3","content":"That keeps us both under our hours: I am at 32 of 37.5 this week and she is at 30.","fontFamily":"$font","fontSize":16,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6},
        {"type":"text","id":"ui-inbox-letter-4","content":"Aksel","fontFamily":"$font","fontSize":16,"fill":"$foreground","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.6}]}]}]}]}
```

The open row and the detail show the same thing, word for word. On a phone the list is the screen and the
detail a screen of its own with a back button.

## Other screens

Convention, composed from this kit's measures. Build them from the components and say in one line that the
layout is the usual one, not a library's.

| Screen | Build |
|---|---|
| Settings | In the app's frame: a column of at most 768. Either tabs over section cards, or a second list of links 200 to 240 wide (rows 32) beside them. A section card has a title of 16, a line under it, fields 24 apart, and a footer band with its own Save at the trailing edge. What destroys (delete the account) is the last card, its button a destructive tint. Switches save at once and get no Save. |
| Pricing | Three plans (two or four at most) as cards 320 to 360 wide, 24 apart, all one height: the plan's name at 16, the price at 36 with "/month" at 14 in `$muted-foreground`, one line, a full-width button, then what is in it as rows of 14 with a 16 `check`, 12 apart. The plan you recommend has the one primary button and a badge; the others outline buttons. Never a larger middle card. A monthly or yearly switch stands centred, 32 above the cards. |
| Checkout | Two columns on at most 1120: the form 600 to 660 wide (contact, delivery, payment as sections 32 apart, fields 16 apart, two short fields side by side), and a summary 320 to 400 wide on `$muted-half`: the items with thumbnails of 48, the sums as rows of 14 ending on one edge, the total at 16 and weight 500 over a `$border` line. The pay button is as wide as the form and names the amount. |
| Onboarding | A column of 480 to 560 centred: "Step 2 of 4" over a progress bar, a title of 24, one line of 14, one question per step (the choices of `references/chat.md` fit), then Back (outline) at the leading and Continue (primary) at the trailing edge. |
| Search results | The search input on top (at most 640 wide), a line of 14 in `$muted-foreground` with the count and the query, then results as rows: a title of 16 at weight 500, two lines of 14, one line of 12 for where it is from; 24 apart or with a `$border` line between. Filters, if any, in a column of 240 at the leading edge. Pagination centred under them. |
| Detail page | A breadcrumb in the top bar. A page header with the title, a badge for its state and the actions. Under it two columns, 24 apart: the main cards (at most 768) and a narrower one of 280 to 320 with properties as label and value rows and the latest activity. |
| Not found, error | Centred in the screen in a column of at most 448, 24 between parts: a 40 tile with an icon (not a giant "404"), a title of 24, one sentence saying what happened in the user's terms, then the way on as a primary button and the way back as an outline one. |
