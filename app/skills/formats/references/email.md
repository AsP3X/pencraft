# A picture of an email

Pencraft draws what an email should look like; someone still builds it for mail programs. So this is about
the layout that survives them. The width is the one firm figure (Mailchimp's templates and most guides);
button and type sizes are vendors' advice and differ, and are given here as this kit's choice.

## Measures

| Part | Value | Why |
|---|---|---|
| Body | One column, 600 wide, on a ground 60 or more wider at each side | Mail programs show little more; wider is cut or shrunk |
| On a phone | The same column at 390, 16 to 20 from the edges | Draw it as a second frame when the user asks |
| Side padding | 32 (24 on a phone) | |
| Headline | 28 to 36, one or two lines | |
| Text | 16 with a line height of 1.5; never under 13 | |
| Button | One, 44 to 52 high, 24 to 32 beside its label, drawn as a filled box with its label as text | It must be findable with a thumb and readable when pictures are off |
| Pictures | As wide as the column; text is never part of a picture | Many readers see the email first without pictures |
| Footer | Who sent it, the postal address, why the reader gets it, and an unsubscribe link that can be seen | Readers expect it, and the law in many places asks for it |
| Height | Three phone screens at most (about 2400) | Readers give an email some nine seconds; long ones are clipped |

- Order: the sender's mark, the one thing this email is about, what to do, then anything else, then the
  footer. One email, one purpose.
- Faces: mail programs fall back to the system's. Design in one that looks like them (Segoe UI, Helvetica,
  Arial, Georgia) so the built email matches the picture.
- More than a third of readers use a dark theme, and mail programs recolour light emails in it. Draw a dark
  copy beside the light one (the same frame with a dark ground and light text) when the email has a logo
  or coloured blocks, and give the logo a form that holds on both.
- The subject line and the preview text (50 to 100 characters) are part of the design: write them, over
  the email on its ground, as an inbox shows them.

## An email

```json
{"type":"frame","id":"format-email","name":"Email, 600 wide","width":720,"layout":"vertical","gap":24,"padding":[40,60],"alignItems":"center","fill":"$ground","children":[
  {"type":"frame","id":"format-email-subject","name":"Subject and preview","width":600,"layout":"vertical","gap":4,"children":[
    {"type":"text","id":"format-email-subject-line","name":"Subject","content":"Your place on Saturday: 11:40","fontFamily":"$font","fontSize":18,"fontWeight":"600","fill":"$ink","textGrowth":"fixed-width","width":"fill_container"},
    {"type":"text","id":"format-email-preview","name":"Preview","content":"Bring the toaster and its cable. Most repairs take about twenty minutes.","fontFamily":"$font","fontSize":14,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container"}]},
  {"type":"frame","id":"format-email-body","name":"Body","width":600,"layout":"vertical","cornerRadius":12,"fill":"$paper","clip":true,"children":[
    {"type":"frame","id":"format-email-header","name":"Header","width":"fill_container","padding":[24,32],"children":[
      {"type":"frame","id":"format-email-brand","name":"Brand","gap":13,"alignItems":"center","children":[
        {"type":"frame","id":"format-email-brand-tile","width":32,"height":32,"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$accent","children":[
          {"type":"icon","id":"format-email-brand-icon","library":"lucide","icon":"wrench","width":18,"height":18,"fill":"$on-accent"}]},
        {"type":"text","id":"format-email-brand-name","name":"Name","content":"Lantern Repair Café","fontFamily":"$font","fontSize":16,"fontWeight":"600","fill":"$ink"}]}]},
    {"type":"frame","id":"format-email-main","name":"Main","width":"fill_container","layout":"vertical","gap":20,"padding":[8,32,40,32],"children":[
      {"type":"text","id":"format-email-title","name":"Headline","content":"Your toaster is next in line","fontFamily":"$font","fontSize":32,"fontWeight":"700","fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.15,"letterSpacing":-0.5},
      {"type":"text","id":"format-email-text","name":"Text","content":"Hi Maren, we have a place for you at 11:40 on Saturday 28 November. Bring the toaster and its cable. Most repairs take about twenty minutes.","fontFamily":"$font","fontSize":16,"fill":"$ink","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5},
      {"type":"frame","id":"format-email-button","name":"Button","height":48,"padding":[0,28],"justifyContent":"center","alignItems":"center","cornerRadius":8,"fill":"$accent","children":[
        {"type":"text","id":"format-email-button-label","content":"Confirm my place","fontFamily":"$font","fontSize":16,"fontWeight":"600","fill":"$on-accent"}]},
      {"type":"text","id":"format-email-after","name":"Note","content":"Cannot make it? Tell us by Friday and we give the place to someone else.","fontFamily":"$font","fontSize":14,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5}]},
    {"type":"frame","id":"format-email-footer","name":"Footer","width":"fill_container","layout":"vertical","gap":4,"padding":[24,32],"stroke":"$line","strokeWidth":{"top":1},"children":[
      {"type":"text","id":"format-email-address","content":"Lantern Repair Café, Markveien 58, 0554 Oslo","fontFamily":"$font","fontSize":13,"fill":"$ink-soft","textGrowth":"fixed-width","width":"fill_container","lineHeight":1.5},
      {"type":"frame","id":"format-email-why","gap":6,"children":[
        {"type":"text","id":"format-email-why-text","content":"You get this because you booked a repair.","fontFamily":"$font","fontSize":13,"fill":"$ink-soft","lineHeight":1.5},
        {"type":"text","id":"format-email-unsubscribe","name":"Unsubscribe","content":"Unsubscribe","fontFamily":"$font","fontSize":13,"fill":"$ink-soft","lineHeight":1.5,"underline":true}]}]}]}]}
```
