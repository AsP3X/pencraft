# Base UI and Radix (Primitives, Themes, Colors, Icons): anatomy, state model and design tokens, as of November 2026

Method note: everything below was read on 2026-11-14 from primary sources: the Base UI docs' machine-readable `.md` pages (`base-ui.com/react/components/<name>.md`), the `radix-ui/themes` token and component CSS on `main`, the `radix-ui/colors` source, and the `radix-ui/website` MDX that backs radix-ui.com. Pixel values in tables are computed from the token CSS at `scaling = 100%` and `radius = medium` unless stated. Items I could not verify are in each section's Gaps.

Source shorthands used in citations:
- BUI-INDEX = https://base-ui.com/llms.txt
- BUI-C/<name> = https://base-ui.com/react/components/<name>
- RT-TOKENS = https://github.com/radix-ui/themes/tree/main/packages/radix-ui-themes/src/styles/tokens
- RT-COMP = https://github.com/radix-ui/themes/tree/main/packages/radix-ui-themes/src/components
- RP-C/<name> = https://www.radix-ui.com/primitives/docs/components/<name>

## 1. Component lists, named parts and state model (Base UI vs Radix Primitives)

### Takeaway
Base UI (v1.9.0, Oct 2026) ships 37 components and Radix Primitives ships 30 plus 5 utilities; both use the same compound-part idea, but Base UI splits every floating element into Portal > (Backdrop) > Positioner > Popup > Arrow and exposes state as boolean presence attributes (`data-open`, `data-checked`, `data-highlighted`, `data-starting-style`), while Radix uses one `Content` part and enumerated `data-state="open|closed|checked|..."`. Base UI additionally exposes the full form-field state set (`valid/invalid/dirty/touched/filled/focused/required/readonly`) on every form control, which is the most complete "which states must I draw" list either library offers.

### Cited Findings

#### Component inventories
- Base UI components (37): Accordion, Alert Dialog, Autocomplete, Avatar, Button, Checkbox, Checkbox Group, Collapsible, Combobox, Context Menu, Dialog, Drawer, Field, Fieldset, Form, Input, Menu, Menubar, Meter, Navigation Menu, Number Field, OTP Field, Popover, Preview Card, Progress, Radio Group (Radio), Scroll Area, Select, Separator, Slider, Switch, Tabs, Toast, Toggle, Toggle Group, Toolbar, Tooltip. Utilities: CSP Provider, Direction Provider, mergeProps, useRender. — [Base UI docs index](https://base-ui.com/llms.txt)
- Radix Primitives components (30): Accordion, Alert Dialog, Aspect Ratio, Avatar, Checkbox, Collapsible, Context Menu, Dialog, Dropdown Menu, Form, Hover Card, Label, Menubar, Navigation Menu, One-Time Password Field, Password Toggle Field, Popover, Progress, Radio Group, Scroll Area, Select, Separator, Slider, Switch, Tabs, Toast, Toggle, Toggle Group, Toolbar, Tooltip. Utilities: Accessible Icon, Direction Provider, Portal, Slot, Visually Hidden. — [radix-ui/website docs tree](https://github.com/radix-ui/website/tree/main/data/primitives/docs/components)
- Only in Base UI: Autocomplete, Combobox, Button, Checkbox Group, Drawer (swipe gestures, snap points, indent effect), Field, Fieldset, Input, Meter, Number Field (with scrub area), plus anchored toasts and a filterable Menu (`Menu.FilterProvider`/`Menu.Input`, preview in v1.9.0). — [Base UI docs index](https://base-ui.com/llms.txt); [v1.9.0 notes](https://base-ui.com/react/overview/releases/v1-9-0)
- Only in Radix Primitives: Aspect Ratio, Label, Password Toggle Field, and the utilities Accessible Icon / Visually Hidden / Slot / Portal. Naming differences for the same thing: Radix "Dropdown Menu" = Base UI "Menu"; Radix "Hover Card" = Base UI "Preview Card". — [radix-ui/website docs tree](https://github.com/radix-ui/website/tree/main/data/primitives/docs/components)
- Correction to the brief: Toast, Menubar, Context Menu and Form exist in BOTH libraries (Radix has `Toast`, `Menubar`, `ContextMenu`, `Form`); they are not Base UI exclusives. — [Radix Toast](https://www.radix-ui.com/primitives/docs/components/toast); [Radix Menubar](https://www.radix-ui.com/primitives/docs/components/menubar)

#### The two state vocabularies (what a drawing tool must enumerate)
- Base UI uses presence attributes, one per state. Overlay parts: `data-open`, `data-closed`, `data-starting-style` (first frame of enter), `data-ending-style` (exit), `data-instant` (skip animation: values `click|dismiss|focus|delay|trigger-change|group`), `data-anchor-hidden`, `data-side`, `data-align`. Triggers: `data-popup-open`, `data-pressed`, `data-disabled`. — [Base UI Popover](https://base-ui.com/react/components/popover)
- Base UI form-control state set, present on Checkbox, Radio, Switch, Input, Number Field, OTP Field, Select trigger, Combobox input, Slider and on every Field part (Root, Label, Control, Description, Error): `data-disabled`, `data-readonly`, `data-required`, `data-valid`, `data-invalid`, `data-dirty`, `data-touched`, `data-filled`, `data-focused`; plus `data-checked`/`data-unchecked`/`data-indeterminate` on checkable controls and `data-placeholder` on Select/Combobox. — [Base UI Switch](https://base-ui.com/react/components/switch); [Base UI Field](https://base-ui.com/react/components/field)
- Base UI list items: `data-highlighted` (keyboard/hover cursor), `data-selected` (Select/Combobox item), `data-checked`/`data-unchecked` (Menu radio/checkbox items), `data-disabled`. Highlight and selection are separate states and can co-occur. — [Base UI Select](https://base-ui.com/react/components/select)
- Radix uses enumerated `data-state`: `open|closed` (Dialog, Popover, Menu, Collapsible, Accordion, Select trigger), `checked|unchecked|indeterminate` (Checkbox), `checked|unchecked` (Switch, Radio, Select item), `on|off` (Toggle), `active|inactive` (Tabs), `closed|delayed-open|instant-open` (Tooltip), `visible|hidden` (ScrollArea scrollbar, NavigationMenu indicator), `complete|indeterminate|loading` (Progress); plus `data-disabled`, `data-highlighted`, `data-placeholder`, `data-orientation`, `data-side`, `data-align`, and Form's `data-valid`/`data-invalid`. — [Radix Checkbox](https://www.radix-ui.com/primitives/docs/components/checkbox); [Radix Tooltip](https://www.radix-ui.com/primitives/docs/components/tooltip)
- Base UI's Field validates "onSubmit" by default (`validationMode='onSubmit'`), so the invalid state normally appears only after a submit attempt. — [Base UI Field](https://base-ui.com/react/components/field)

#### Anatomy differences worth encoding
- Base UI Dialog/AlertDialog: Root > Trigger, Portal > Backdrop, Viewport > Popup > Title, Description, Close. Radix: Root > Trigger, Portal > Overlay, Content > Title, Description, Close; Radix AlertDialog replaces Close with `Cancel` and `Action`. — [Base UI Dialog](https://base-ui.com/react/components/dialog); [Radix Alert Dialog](https://www.radix-ui.com/primitives/docs/components/alert-dialog)
- Base UI Select: Root > Label, Trigger(Value, Icon), Portal > Backdrop, Positioner > Popup > ScrollUpArrow, Arrow, List > Item(ItemText, ItemIndicator), Separator, Group(GroupLabel), ScrollDownArrow. Radix Select: Trigger(Value, Icon), Portal > Content > ScrollUpButton, Viewport > Item(ItemText, ItemIndicator), Group(Label), Separator, ScrollDownButton, Arrow. — [Base UI Select](https://base-ui.com/react/components/select); [Radix Select](https://www.radix-ui.com/primitives/docs/components/select)
- Base UI Slider has more named parts than Radix: Root > Label, Value, Control > Track > Indicator, Thumb (Radix: Root > Track > Range, Thumb). Base UI Progress and Meter: Root > Label, Track > Indicator, Value (Radix Progress: Root > Indicator only). — [Base UI Slider](https://base-ui.com/react/components/slider); [Radix Slider](https://www.radix-ui.com/primitives/docs/components/slider)
- Base UI Tabs has an explicit moving `Tabs.Indicator` part (Root > List > Tab, Indicator; Panel); Radix Tabs has no indicator part (Root > List > Trigger; Content). Tab active state: Base UI `data-active`, Radix `data-state="active"`. — [Base UI Tabs](https://base-ui.com/react/components/tabs); [Radix Tabs](https://www.radix-ui.com/primitives/docs/components/tabs)
- Base UI Combobox adds parts no Radix primitive has: InputGroup(Input, Trigger, Icon, Clear, Value, Chips > Chip > ChipRemove), and in the popup Status, Empty, List > Row > Item(ItemIndicator), Group(GroupLabel), Separator. `data-list-empty` and the `Empty` part make the "no results" state a first-class design state. — [Base UI Combobox](https://base-ui.com/react/components/combobox)
- Base UI Number Field: Root > ScrubArea(ScrubAreaCursor), Group > Decrement, Input, Increment; extra state `data-scrubbing`. Defaults step 1, smallStep 0.1, largeStep 10. — [Base UI Number Field](https://base-ui.com/react/components/number-field)
- Base UI Toast: Provider > Portal > Viewport > Root > Content > Title, Description, Action, Close; anchored variant adds Positioner and Arrow. States: `data-expanded`, `data-limited`, `data-swiping`, `data-swipe-direction`, `data-type`, `data-behind` on Content. Provider defaults `limit=3`, `timeout=5000`. Radix Toast: Provider > Root(Title, Description, Action, Close), Viewport; duration 5000, swipeDirection right, swipeThreshold 50. — [Base UI Toast](https://base-ui.com/react/components/toast); [Radix Toast](https://www.radix-ui.com/primitives/docs/components/toast)
- Base UI ScrollArea exposes overflow state for edge fades: `data-has-overflow-x/y`, `data-overflow-x-start/end`, `data-overflow-y-start/end`, `data-scrolling`, and `data-hovering` on Scrollbar. Radix ScrollArea: `type="hover"` default, `scrollHideDelay=600`. — [Base UI Scroll Area](https://base-ui.com/react/components/scroll-area); [Radix Scroll Area](https://www.radix-ui.com/primitives/docs/components/scroll-area)
- Base UI usage guidance with design consequences: prefer Combobox over Select when the list is large enough to need filtering; Autocomplete is for free-form text with optional suggestions; a context menu must never be the only way to perform an action; tooltips are supplementary visual labels only (not available to touch/screen-reader users); a preview card's link must remain the only accessible interface. — [Base UI Combobox](https://base-ui.com/react/components/combobox); [Base UI Context Menu](https://base-ui.com/react/components/context-menu); [Base UI Tooltip](https://base-ui.com/react/components/tooltip); [Base UI Preview Card](https://base-ui.com/react/components/preview-card)

#### Full per-part reference (machine-extracted)
The two blocks below are extracted mechanically from the docs sources. Format for Base UI: `- Part: description` / `data:` attributes (prefix `data-` removed, value enums in brackets) / `props:` positioning- and behaviour-relevant defaults. Format for Radix: `- Part: data[attr=values] props[defaults] vars[CSS variables]`. Sources: each `https://base-ui.com/react/components/<name>.md` page and each `https://github.com/radix-ui/website/blob/main/data/primitives/docs/components/<name>.mdx` file. The Radix `props[...]` defaults were regex-parsed from MDX and a few may be attributed to the wrong part; treat them as indicative and the Base UI block as authoritative.

##### Base UI v1.9 parts, data attributes and defaults

```text

######## accordion
ANATOMY:
<Accordion.Root>
  <Accordion.Item>
    <Accordion.Header>
      <Accordion.Trigger />
    </Accordion.Header>
    <Accordion.Panel />
  </Accordion.Item>
</Accordion.Root>;
- Root: Groups all parts of the accordion. Renders a `<div>` element.
    data: orientation disabled
    props: hiddenUntilFound=`false` multiple=`false` disabled=`false` orientation=`'vertical'` keepMounted=`false`
- Trigger: A button that opens and closes the corresponding panel. Renders a `<button>` element.
    data: panel-open disabled index(number)
- Item: Groups an accordion header with the corresponding panel. Renders a `<div>` element.
    data: open disabled index(number)
    props: disabled=`false`
- Header: A heading that labels the corresponding panel. Renders an `<h3>` element.
    data: open disabled index(number)
- Panel: A collapsible panel with the accordion item contents. Renders a `<div>` element.
    data: open orientation disabled index(number) starting-style ending-style
    props: hiddenUntilFound=`false` keepMounted=`false`

######## alert-dialog
ANATOMY:
<AlertDialog.Root>
  <AlertDialog.Trigger />
  <AlertDialog.Portal>
    <AlertDialog.Backdrop />
    <AlertDialog.Viewport>
      <AlertDialog.Popup>
        <AlertDialog.Title />
        <AlertDialog.Description />
        <AlertDialog.Close />
      </AlertDialog.Popup>
    </AlertDialog.Viewport>
  </AlertDialog.Portal>
</AlertDialog.Root>;
- Root: Groups all parts of the alert dialog. Doesn't render its own HTML element.
- Trigger: A button that opens the alert dialog. Renders a `<button>` element.
    data: popup-open disabled
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Popup: A container for the dialog contents. Renders a `<div>` element.
    data: open closed nested nested-dialog-open starting-style ending-style
- Title: A heading that labels the dialog. Renders an `<h2>` element.
- Description: A paragraph with additional information about the dialog. Renders a `<p>` element.
- Close: A button that closes the dialog. Renders a `<button>` element.
    data: disabled
- Viewport: A positioning container for the dialog popup that can be made scrollable. Renders a `<div>` element.
    data: open closed nested nested-dialog-open starting-style ending-style

######## autocomplete
USAGE: - **Avoid when selection state is needed**: Use [Combobox](/react/components/combobox.md) instead of Autocomplete if the selection should be remembered and the input value cannot be custom. Unlike Combobox, Autocomplete's input can contain free-form text, as its suggestions only _optionally_ autocomplete the text. - **Form controls must have an accessible name**: It can be created using a `<label>
ANATOMY:
<Autocomplete.Root>
  <Autocomplete.InputGroup>
    <Autocomplete.Input />
    <Autocomplete.Trigger />
    <Autocomplete.Icon />
    <Autocomplete.Clear />
    <Autocomplete.Value />
  </Autocomplete.InputGroup>
  <Autocomplete.Portal>
    <Autocomplete.Backdrop />
    <Autocomplete.Positioner>
      <Autocomplete.Popup>
        <Autocomplete.Arrow />
        <Autocomplete.Status />
        <Autocomplete.Empty />
        <Autocomplete.List>
          <Autocomplete.Row>
            <Autocomplete.Item />
          </Autocomplete.Row>
          <Autocomplete.Separator />
          <Autocomplete.Group>
            <Autocomplete.GroupLabel />
          </Autocomplete.Group>
          <Autocomplete.Collection />
        </Autocomplete.List>
      </Autocomplete.Popup>
    </Autocomplete.Positioner>
  </Autocomplete.Portal>
</Autocomplete.Root>;
- Root: Groups all parts of the autocomplete. Doesn't render its own HTML element.
    props: autoHighlight=`false` highlightItemOnHover=`true` grid=`false` limit=`-1` loopFocus=`true` modal=`false` mode=`'list'` virtualized=`false` disabled=`false`
- Trigger: A button that opens the popup. Renders a `<button>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly required valid invalid dirty touched filled focused
    props: disabled=`false`
- Value: The current value of the autocomplete. Doesn't render its own HTML element.
- Input: A text input to search for items in the list. Renders an `<input>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly required valid invalid dirty touched filled focused
    props: disabled=`false`
- Icon: An icon that indicates that the trigger button opens the popup. Renders a `<span>` element.
- Clear: Clears the value when clicked. Renders a `<button>` element.
    data: popup-open disabled visible starting-style ending-style
    props: disabled=`false` keepMounted=`false`
- List: A list container for the items. Renders a `<div>` element.
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the popup against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') empty side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the list. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') empty side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual item in the list. Renders a `<div>` element.
    data: highlighted disabled
    props: disabled=`false`
- Group: Groups related items with the corresponding label. Renders a `<div>` element.
- GroupLabel: An accessible label that is automatically associated with its parent group. Renders a `<div>` element.
- Separator: A visual separator between items or groups. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`
- Status: Displays a status message whose content changes are announced politely to screen readers. Useful for conveying the status of an asynchronously loaded list. This component's root element must remain mounted in the DOM to 
- Empty: Renders its children only when the list is empty. Requires the `items` prop on the root component. Announces changes politely to screen readers. This component's root element must remain mounted in the DOM to announce ch
- Collection: Renders filtered list items. Doesn't render its own HTML element. If rendering a flat list, pass a function child to the `List` component instead, which implicitly wraps it.
- Row: Displays a single row of items in a grid list. Enable `grid` on the root component to turn the listbox into a grid. Renders a `<div>` element.
- InputGroup: A wrapper for the input and its associated controls. Renders a `<div>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly valid invalid dirty touched filled focused

######## avatar
ANATOMY:
<Avatar.Root>
  <Avatar.Image src="" />
  <Avatar.Fallback>LT</Avatar.Fallback>
</Avatar.Root>;
- Root: Displays a user's profile picture, initials, or fallback icon. Renders a `<span>` element.
- Image: The image to be displayed in the avatar. Renders an `<img>` element.
    data: error loading starting-style ending-style
    props: keepMounted=`false`
- Fallback: Rendered when the image fails to load or when no image is provided. Renders a `<span>` element.
    props: delay=`0`

######## button
USAGE: - **Submit buttons**: Unlike the native button element, `type="submit"` must be specified on Button for it to act as a submit button. - **Links**: The Button component enforces button semantics (`role="button"`, keyboard interaction, disabled state). It should not be used for links. See [Rendering links as buttons](/react/components/button.md) below.
ANATOMY:
<Button />;
- Button: A button component that can be used to trigger actions. Renders a `<button>` element.
    data: disabled

######## checkbox-group
USAGE: - **Form controls must have an accessible name**: It can be created using `<label>` elements, or the `Field` and `Fieldset` components. See [Labeling a checkbox group](/react/components/checkbox-group.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<CheckboxGroup>
  <Checkbox.Root />
</CheckboxGroup>;
- CheckboxGroup: Provides a shared state to a series of checkboxes.
    data: disabled
    props: disabled=`false`

######## checkbox
USAGE: - **Form controls must have an accessible name**: It can be created using a `<label>` element or the `Field` component. See [Labeling a checkbox](/react/components/checkbox.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<Checkbox.Root>
  <Checkbox.Indicator />
</Checkbox.Root>;
- Root: Represents the checkbox itself. Renders a `<span>` element and a hidden `<input>` beside.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused indeterminate
    props: disabled=`false`
- Indicator: Indicates whether the checkbox is ticked. Renders a `<span>` element.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused indeterminate starting-style ending-style
    props: keepMounted=`false`

######## collapsible
ANATOMY:
<Collapsible.Root>
  <Collapsible.Trigger />
  <Collapsible.Panel />
</Collapsible.Root>;
- Root: Groups all parts of the collapsible. Renders a `<div>` element.
    data: open closed starting-style ending-style
    props: disabled=`false`
- Trigger: A button that opens and closes the collapsible panel. Renders a `<button>` element.
    data: panel-open
- Panel: A panel with the collapsible contents. Renders a `<div>` element.
    data: open closed starting-style ending-style
    props: hiddenUntilFound=`false` keepMounted=`false`

######## combobox
USAGE: - **Combobox is a filterable Select**: Use Combobox when the input is restricted to a set of predefined selectable items, similar to [Select](/react/components/select.md) but whose items are filterable using an input. Prefer using Combobox over Select when the number of items is sufficiently large to warrant filtering. - **Avoid for simple search widgets**: Combobox does not allow free-form text i
ANATOMY:
<Combobox.Root>
  <Combobox.Label />
  <Combobox.InputGroup>
    <Combobox.Input />
    <Combobox.Trigger />
    <Combobox.Icon />
    <Combobox.Clear />
    <Combobox.Value />
    <Combobox.Chips>
      <Combobox.Chip>
        <Combobox.ChipRemove />
      </Combobox.Chip>
    </Combobox.Chips>
  </Combobox.InputGroup>
  <Combobox.Portal>
    <Combobox.Backdrop />
    <Combobox.Positioner>
      <Combobox.Popup>
        <Combobox.Arrow />
        <Combobox.Status />
        <Combobox.Empty />
        <Combobox.List>
          <Combobox.Row>
            <Combobox.Item>
              <Combobox.ItemIndicator />
            </Combobox.Item>
          </Combobox.Row>
          <Combobox.Separator />
          <Combobox.Group>
            <Combobox.GroupLabel />
          </Combobox.Group>
          <Combobox.Collection />
        </Combobox.List>
      </Combobox.Popup>
    </Combobox.Positioner>
  </Combobox.Portal>
</Combobox.Root>;
- Root: Groups all parts of the combobox. Doesn't render its own HTML element.
    props: autoHighlight=`false` highlightItemOnHover=`true` grid=`false` limit=`-1` loopFocus=`true` modal=`false` multiple=`false` virtualized=`false` disabled=`false`
- Trigger: A button that opens the popup. Renders a `<button>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly required valid invalid dirty touched filled focused placeholder
    props: disabled=`false`
- Value: The current value of the combobox. Doesn't render its own HTML element.
- Input: A text input to search for items in the list. Renders an `<input>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly required valid invalid dirty touched filled focused
    props: disabled=`false`
- Icon: An icon that indicates that the trigger button opens the popup. Renders a `<span>` element.
- Clear: Clears the value when clicked. Renders a `<button>` element.
    data: popup-open disabled visible starting-style ending-style
    props: disabled=`false` keepMounted=`false`
- List: A list container for the items. Renders a `<div>` element.
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the popup against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') empty side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the list. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') empty side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual item in the list. Renders a `<div>` element.
    data: selected highlighted disabled
    props: disabled=`false`
- Group: Groups related items with the corresponding label. Renders a `<div>` element.
- GroupLabel: An accessible label that is automatically associated with its parent group. Renders a `<div>` element.
- Separator: A visual separator between items or groups. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`
- Label: An accessible label that is automatically associated with the combobox trigger. Renders a `<div>` element.
- Status: Displays a status message whose content changes are announced politely to screen readers. Useful for conveying the status of an asynchronously loaded list. This component's root element must remain mounted in the DOM to 
- Empty: Renders its children only when the list is empty. Requires the `items` prop on the root component. Announces changes politely to screen readers. This component's root element must remain mounted in the DOM to announce ch
- Collection: Renders filtered list items. Doesn't render its own HTML element. If rendering a flat list, pass a function child to the `List` component instead, which implicitly wraps it.
- Row: Displays a single row of items in a grid list. Enable `grid` on the root component to turn the listbox into a grid. Renders a `<div>` element.
- Chips: A container for the chips in a multiselectable input. Renders a `<div>` element.
- Chip: An individual chip that represents a value in a multiselectable input. Renders a `<div>` element.
- ChipRemove: A button to remove a chip. Renders a `<button>` element.
- ItemIndicator: Indicates whether the item is selected. Renders a `<span>` element.
    data: starting-style ending-style
    props: keepMounted=`false`
- InputGroup: A wrapper for the input and its associated controls. Renders a `<div>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) list-empty pressed disabled readonly valid invalid dirty touched filled focused placeholder

######## context-menu
USAGE: - **Use context menus as an enhancement**: Don't make a context menu the only way to perform actions. Users may not discover or be able to open a context menu, especially on touch devices or with assistive technology. Always provide visible controls for the actions that are available in the context menu.
ANATOMY:
<ContextMenu.Root>
  <ContextMenu.Trigger />
  <ContextMenu.Portal>
    <ContextMenu.Backdrop />
    <ContextMenu.Positioner>
      <ContextMenu.Popup>
        <ContextMenu.Arrow />
        <ContextMenu.Item />
        <ContextMenu.LinkItem />
        <ContextMenu.Separator />
        <ContextMenu.SubmenuRoot>
          <ContextMenu.SubmenuTrigger />
        </ContextMenu.SubmenuRoot>
        <ContextMenu.Group>
          <ContextMenu.GroupLabel />
        </ContextMenu.Group>
        <ContextMenu.RadioGroup>
          <ContextMenu.RadioItem>
            <ContextMenu.RadioItemIndicator />
          </ContextMenu.RadioItem>
        </ContextMenu.RadioGroup>
        <ContextMenu.CheckboxItem>
          <ContextMenu.CheckboxItemIndicator />
        </ContextMenu.CheckboxItem>
      </ContextMenu.Popup>
    </ContextMenu.Positioner>
  </ContextMenu.Portal>
</ContextMenu.Root>;
- Root: A component that creates a context menu activated by right clicking or long pressing. Doesn't render its own HTML element.
    props: highlightItemOnHover=`true` loopFocus=`true` disabled=`false` orientation=`'vertical'`
- Trigger: An area that opens the menu on right click or long press. Renders a `<div>` element.
    data: popup-open pressed
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the menu popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the context menu popup against the pointer or a custom anchor. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'start'` side=`'bottom'` collisionPadding=`5` sticky=`false`
- Popup: A container for the menu items. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') instant('click' | 'dismiss' | 'group' | 'trigger-change') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the menu anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual interactive item in the menu. Renders a `<div>` element.
    data: highlighted disabled
    props: closeOnClick=`true` disabled=`false`
- Group: Groups related menu items with the corresponding label. Renders a `<div>` element.
- GroupLabel: An accessible label that is automatically associated with its parent group. Renders a `<div>` element.
- Separator: A separator element accessible to screen readers. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`
- SubmenuRoot: Groups all parts of a submenu. Doesn't render its own HTML element.
    props: highlightItemOnHover=`true` loopFocus=`true` disabled=`false` orientation=`'vertical'`
- SubmenuTrigger: A menu item that opens a submenu. Renders a `<div>` element.
    data: popup-open highlighted disabled
    props: disabled=`false` openOnHover=`true` delay=`100` closeDelay=`0`
- RadioGroup: Groups related radio items. Renders a `<div>` element.
    props: disabled=`false`
- RadioItem: A menu item that works like a radio button in a given group. Renders a `<div>` element.
    data: checked unchecked highlighted disabled
    props: closeOnClick=`false` disabled=`false`
- RadioItemIndicator: Indicates whether the radio item is selected. Renders a `<span>` element.
    data: checked unchecked disabled starting-style ending-style
    props: keepMounted=`false`
- CheckboxItem: A menu item that toggles a setting on or off. Renders a `<div>` element.
    data: checked unchecked highlighted disabled
    props: closeOnClick=`false` disabled=`false`
- CheckboxItemIndicator: Indicates whether the checkbox item is ticked. Renders a `<span>` element.
    data: checked unchecked disabled starting-style ending-style
    props: keepMounted=`false`
- LinkItem: A link in the menu that can be used to navigate to a different page or section. Renders an `<a>` element.
    data: highlighted
    props: closeOnClick=`false`

######## dialog
USAGE: - **Dialog doesn't support gestures:** Use [Drawer](/react/components/drawer.md) when you need gesture support or snap points. A panel that slides in from the edge of the screen and doesn't need gesture support is a positioned Dialog.
ANATOMY:
<Dialog.Root>
  <Dialog.Trigger />
  <Dialog.Portal>
    <Dialog.Backdrop />
    <Dialog.Viewport>
      <Dialog.Popup>
        <Dialog.Title />
        <Dialog.Description />
        <Dialog.Close />
      </Dialog.Popup>
    </Dialog.Viewport>
  </Dialog.Portal>
</Dialog.Root>;
- Root: Groups all parts of the dialog. Doesn't render its own HTML element.
    props: modal=`true`
- Trigger: A button that opens the dialog. Renders a `<button>` element.
    data: popup-open disabled
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Popup: A container for the dialog contents. Renders a `<div>` element.
    data: open closed nested nested-dialog-open starting-style ending-style
- Title: A heading that labels the dialog. Renders an `<h2>` element.
- Description: A paragraph with additional information about the dialog. Renders a `<p>` element.
- Close: A button that closes the dialog. Renders a `<button>` element.
    data: disabled
- Viewport: A positioning container for the dialog popup that can be made scrollable. Renders a `<div>` element.
    data: open closed nested nested-dialog-open starting-style ending-style

######## drawer
USAGE: - **Drawer extends [Dialog](/react/components/dialog.md):** It adds gesture support, snap points, and indent effects. If you don't need these, use Dialog instead. A panel that slides in from the edge of the screen and doesn't need gesture support is a positioned Dialog.
ANATOMY:
<Drawer.Provider>
  <Drawer.IndentBackground />
  <Drawer.Indent>
    <Drawer.Root>
      <Drawer.Trigger />
      <Drawer.SwipeArea />
      <Drawer.Portal>
        <Drawer.Backdrop />
        <Drawer.Viewport>
          <Drawer.Popup>
            <Drawer.Content>
              <Drawer.Title />
              <Drawer.Description />
              <Drawer.Close />
            </Drawer.Content>
          </Drawer.Popup>
        </Drawer.Viewport>
      </Drawer.Portal>
    </Drawer.Root>
  </Drawer.Indent>
</Drawer.Provider>;
- Root: Groups all parts of the drawer. Doesn't render its own HTML element.
    props: modal=`true` swipeDirection=`'down'`
- Provider: Provides a shared context for coordinating global Drawer UI, such as indent/background effects based on whether any Drawer is open. Doesn't render its own HTML element.
- Trigger: A button that opens the drawer. Renders a `<button>` element.
    data: popup-open disabled
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Popup: A container for the drawer contents. Renders a `<div>` element.
    data: open closed expanded nested-drawer-open nested-drawer-swiping swipe-direction('up' | 'down' | 'left' | 'right') swipe-dismiss swiping starting-style ending-style
- Content: A container for the drawer contents. Renders a `<div>` element.
- Title: A heading that labels the drawer. Renders an `<h2>` element.
- Description: A paragraph with additional information about the drawer. Renders a `<p>` element.
- Close: A button that closes the drawer. Renders a `<button>` element.
    data: disabled
- Viewport: A positioning container for the drawer popup that can be made scrollable. Renders a `<div>` element.
    data: open closed nested starting-style ending-style
- Indent: A wrapper element intended to contain your app's main UI. Applies `data-active` when any drawer within the nearest `<Drawer.Provider>` is open. Renders a `<div>` element.
- IndentBackground: An element placed before `<Drawer.Indent>` to render a background layer that can be styled based on whether any drawer is open. Renders a `<div>` element.
- SwipeArea: An invisible area that listens for swipe gestures to open the drawer. Renders a `<div>` element.
    data: open closed disabled swipe-direction('up' | 'down' | 'left' | 'right') swiping
    props: disabled=`false`
- VirtualKeyboardProvider: Provides keyboard-aware focus and scroll handling for bottom-sheet drawers with form fields.

######## field
ANATOMY:
<Field.Root>
  <Field.Label />
  <Field.Control />
  <Field.Description />
  <Field.Item />
  <Field.Error />
  <Field.Validity />
</Field.Root>;
- Root: Groups all parts of the field. Renders a `<div>` element.
    data: disabled valid invalid dirty touched filled focused
    props: disabled=`false` validationMode=`'onSubmit'`
- Item: Groups individual items in a checkbox group or radio group with a label and description. Renders a `<div>` element.
    data: disabled valid invalid dirty touched filled focused
    props: disabled=`false`
- Description: A paragraph with additional information about the field. Renders a `<p>` element.
    data: disabled valid invalid dirty touched filled focused
- Control: The form control to label and validate. Renders an `<input>` element. You can omit this part and use any Base UI input component instead. For example, [Input](https://base-ui.com/react/components/input), [Checkbox](https
    data: disabled valid invalid dirty touched filled focused
- Label: An accessible label that is automatically associated with the field control. Renders a `<label>` element.
    data: disabled valid invalid dirty touched filled focused
- Error: An error message displayed if the field control fails validation. Renders a `<div>` element.
    data: disabled valid invalid dirty touched filled focused starting-style ending-style
- Validity: Used to display a custom message based on the field's validity. Requires `children` to be a function that accepts field validity state as an argument.

######## fieldset
ANATOMY:
<Fieldset.Root>
  <Fieldset.Legend />
</Fieldset.Root>;
- Root: Groups a shared legend with related controls. Renders a `<fieldset>` element.
- Legend: An accessible label that is automatically associated with the fieldset. Renders a `<div>` element.
    data: disabled

######## form
ANATOMY:
<Form>
  <Field.Root>
    <Field.Label />
    <Field.Control />
    <Field.Error />
  </Field.Root>
</Form>;
- Form: A native form element with consolidated error handling. Renders a `<form>` element.
    props: validationMode=`'onSubmit'`

######## input
USAGE: - **Form controls must have an accessible name**: It can be created using a `<label>` element or the `Field` component. See the [forms guide](/react/handbook/forms.md).
ANATOMY:
<Input />;
- Input: A native input element that automatically works with [Field](https://base-ui.com/react/components/field). Renders an `<input>` element.
    data: disabled valid invalid dirty touched filled focused

######## menu
ANATOMY:
<Menu.FilterProvider>
  <Menu.Root>
    <Menu.Trigger />
    <Menu.Portal>
      <Menu.Backdrop />
      <Menu.Positioner>
        <Menu.Popup>
          <Menu.Arrow />
          <Menu.Input />
          <Menu.Clear />
          <Menu.Empty />
          <Menu.List>
            <Menu.Item />
            <Menu.LinkItem />
            <Menu.Separator />
            <Menu.SubmenuRoot>
              <Menu.SubmenuTrigger />
            </Menu.SubmenuRoot>
            <Menu.Group>
              <Menu.GroupLabel />
            </Menu.Group>
            <Menu.RadioGroup>
              <Menu.GroupLabel />
              <Menu.RadioItem>
                <Menu.RadioItemIndicator />
              </Menu.RadioItem>
            </Menu.RadioGroup>
            <Menu.CheckboxItem>
              <Menu.CheckboxItemIndicator />
            </Menu.CheckboxItem>
          </Menu.List>
          <Menu.Viewport />
        </Menu.Popup>
      </Menu.Positioner>
    </Menu.Portal>
  </Menu.Root>
</Menu.FilterProvider>;
- Root: Groups all parts of the menu. Doesn't render its own HTML element.
    props: highlightItemOnHover=`true` loopFocus=`true` modal=`true` disabled=`false` orientation=`'vertical'`
- Trigger: A button that opens the menu. Renders a `<button>` element.
    data: popup-open pressed disabled
    props: disabled=`false` delay=`100` closeDelay=`0`
- Input: A search field that filters the menu items. Requires the menu to be wrapped in `Menu.FilterProvider`. Renders an `<input>` element.
    data: highlighted
- Clear: A button that clears the input text. Requires the menu to be wrapped in `Menu.FilterProvider`. Renders a `<button>` element when the input has text.
    data: disabled
    props: disabled=`false`
- List: Groups menu items so other content, such as a filter input, can share the popup. Renders a `<div>` element.
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the menu popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the menu popup against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the menu items. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') instant('click' | 'dismiss' | 'group' | 'trigger-change') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the menu anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual interactive item in the menu. Renders a `<div>` element.
    data: highlighted disabled
    props: closeOnClick=`true` disabled=`false`
- Viewport: A viewport for displaying content transitions. This component is only required if one popup can be opened by multiple triggers, its content changes based on the trigger, and switching between them is animated. Renders a 
    data: activation-direction( ${'left' | 'right' | ''} ${'down' | 'up' | ''} ) current instant('click' | 'dismiss' | 'group' | 'trigger-change') previous transitioning
- Group: Groups related menu items with the corresponding label. Renders a `<div>` element.
- GroupLabel: An accessible label that is automatically associated with its parent group. Renders a `<div>` element.
- Separator: A separator element accessible to screen readers. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`
- Empty: A message shown when the menu has no matching items. Requires the menu to be wrapped in `Menu.FilterProvider`. Renders a `<div>` element.
- SubmenuRoot: Groups all parts of a submenu. Doesn't render its own HTML element.
    props: highlightItemOnHover=`true` loopFocus=`true` disabled=`false` orientation=`'vertical'`
- SubmenuTrigger: A menu item that opens a submenu. Renders a `<div>` element.
    data: popup-open highlighted disabled
    props: disabled=`false` openOnHover=`true` delay=`100` closeDelay=`0`
- RadioGroup: Groups related radio items. Renders a `<div>` element.
    props: disabled=`false`
- RadioItem: A menu item that works like a radio button in a given group. Renders a `<div>` element.
    data: checked unchecked highlighted disabled
    props: closeOnClick=`false` disabled=`false`
- RadioItemIndicator: Indicates whether the radio item is selected. Renders a `<span>` element.
    data: checked unchecked disabled starting-style ending-style
    props: keepMounted=`false`
- CheckboxItem: A menu item that toggles a setting on or off. Renders a `<div>` element.
    data: checked unchecked highlighted disabled
    props: closeOnClick=`false` disabled=`false`
- CheckboxItemIndicator: Indicates whether the checkbox item is ticked. Renders a `<span>` element.
    data: checked unchecked disabled starting-style ending-style
    props: keepMounted=`false`
- FilterProvider: Enables filtering for the menu or submenu it wraps. Add `Menu.Input` to the popup and place its items in `Menu.List`. Wrap each searchable submenu in its own provider. Doesn't render its own HTML element.
    props: autoHighlight=`false`
- LinkItem: A link in the menu that can be used to navigate to a different page or section. Renders an `<a>` element.
    data: highlighted
    props: closeOnClick=`false`

######## menubar
ANATOMY:
<Menubar>
  <Menu.Root>
    <Menu.Trigger />
    <Menu.Portal>
      <Menu.Backdrop />
      <Menu.Positioner>
        <Menu.Popup>
          <Menu.Arrow />
          <Menu.Item />
          <Menu.LinkItem />
          <Menu.Separator />
          <Menu.SubmenuRoot>
            <Menu.SubmenuTrigger />
          </Menu.SubmenuRoot>
          <Menu.Group>
            <Menu.GroupLabel />
          </Menu.Group>
          <Menu.RadioGroup>
            <Menu.RadioItem>
              <Menu.RadioItemIndicator />
            </Menu.RadioItem>
          </Menu.RadioGroup>
          <Menu.CheckboxItem>
            <Menu.CheckboxItemIndicator />
          </Menu.CheckboxItem>
          <Menu.Viewport />
        </Menu.Popup>
      </Menu.Positioner>
    </Menu.Portal>
  </Menu.Root>
</Menubar>;
- Menubar: The container for menus.
    data: orientation('horizontal' | 'vertical') has-submenu-open modal
    props: loopFocus=`true` modal=`true` disabled=`false` orientation=`'horizontal'`

######## meter
ANATOMY:
<Meter.Root>
  <Meter.Label />
  <Meter.Track>
    <Meter.Indicator />
  </Meter.Track>
  <Meter.Value />
</Meter.Root>;
- Root: Groups all parts of the meter and provides the value for screen readers. Renders a `<div>` element.
    props: min=`0` max=`100`
- Value: A text element displaying the current value. Renders a `<span>` element.
- Indicator: Visualizes the position of the value along the range. Renders a `<div>` element.
- Track: Contains the meter indicator and represents the entire range of the meter. Renders a `<div>` element.
- Label: An accessible label for the meter. Renders a `<span>` element.

######## navigation-menu
ANATOMY:
<NavigationMenu.Root>
  <NavigationMenu.List>
    <NavigationMenu.Item>
      <NavigationMenu.Trigger>
        <NavigationMenu.Icon />
      </NavigationMenu.Trigger>
      <NavigationMenu.Content>
        <NavigationMenu.Link />
      </NavigationMenu.Content>
    </NavigationMenu.Item>
  </NavigationMenu.List>
  <NavigationMenu.Portal>
    <NavigationMenu.Backdrop />
    <NavigationMenu.Positioner>
      <NavigationMenu.Popup>
        <NavigationMenu.Arrow />
        <NavigationMenu.Viewport />
      </NavigationMenu.Popup>
    </NavigationMenu.Positioner>
  </NavigationMenu.Portal>
</NavigationMenu.Root>;
- Root: Groups all parts of the navigation menu. Renders a `<nav>` element at the root, or `<div>` element when nested.
    props: delay=`50` closeDelay=`50` orientation=`'horizontal'`
- Trigger: Opens the navigation menu popup when hovered or clicked, revealing the associated content. Renders a `<button>` element.
    data: popup-open pressed disabled
    props: disabled=`false`
- Icon: An icon that indicates that the trigger button opens a menu.
    data: popup-open
- List: Contains a list of navigation menu items. Renders a `<ul>` element.
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: A backdrop for the navigation menu popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the navigation menu against the currently active trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') instant side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the navigation menu contents. Renders a `<nav>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element pointing toward the navigation menu's current anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual navigation menu item. Renders a `<li>` element.
- Content: A container for the content of the navigation menu item that is moved into the popup when the item is active. Renders a `<div>` element.
    data: open closed activation-direction('left' | 'right' | 'up' | 'down') starting-style ending-style
    props: keepMounted=`false`
- Viewport: The clipping viewport of the navigation menu's current content. Renders a `<div>` element.
- Link: A link in the navigation menu that can be used to navigate to a different page or section. Renders an `<a>` element.
    data: active
    props: closeOnClick=`false`

######## number-field
USAGE: - **Form controls must have an accessible name**: It can be created using a `<label>` element or the `Field` component. See the [forms guide](/react/handbook/forms.md).
ANATOMY:
<NumberField.Root>
  <NumberField.ScrubArea>
    <NumberField.ScrubAreaCursor />
  </NumberField.ScrubArea>
  <NumberField.Group>
    <NumberField.Decrement />
    <NumberField.Input />
    <NumberField.Increment />
  </NumberField.Group>
</NumberField.Root>;
- Root: Groups all parts of the number field and manages its state. Renders a `<div>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
    props: step=`1` smallStep=`0.1` largeStep=`10` disabled=`false`
- Input: The native input control in the number field. Renders an `<input>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
- Group: Groups the input with the increment and decrement buttons. Renders a `<div>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
- ScrubArea: An interactive area where the user can click and drag to change the field value. Renders a `<span>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
- ScrubAreaCursor: A custom element to display instead of the native cursor while using the scrub area. Renders a `<span>` element. This component uses the [Pointer Lock API](https://developer.mozilla.org/en-US/docs/Web/API/Pointer_Lock_AP
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
- Decrement: A stepper button that decreases the field value when clicked. Renders a `<button>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing
- Increment: A stepper button that increases the field value when clicked. Renders a `<button>` element.
    data: disabled readonly required valid invalid dirty touched filled focused scrubbing

######## otp-field
USAGE: - **Form controls must have an accessible name**: It can be created using a `<label>` element or the `Field` component. See [Labeling an OTP field](/react/components/otp-field.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<OTPField.Root>
  <OTPField.Input />
  <OTPField.Separator />
</OTPField.Root>;
- Root: Groups all OTP field parts and manages their state. Renders a `<div>` element.
    data: disabled readonly required valid invalid dirty touched complete filled focused
    props: disabled=`false`
- Input: An individual OTP character input. Renders an `<input>` element.
    data: disabled readonly required valid invalid dirty touched complete filled focused
- Separator: A separator element accessible to screen readers. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`

######## popover
ANATOMY:
<Popover.Root>
  <Popover.Trigger />
  <Popover.Portal>
    <Popover.Backdrop />
    <Popover.Positioner>
      <Popover.Popup>
        <Popover.Arrow />
        <Popover.Viewport>
          <Popover.Title />
          <Popover.Description />
          <Popover.Close />
        </Popover.Viewport>
      </Popover.Popup>
    </Popover.Positioner>
  </Popover.Portal>
</Popover.Root>;
- Root: Groups all parts of the popover. Doesn't render its own HTML element.
    props: modal=`false`
- Trigger: A button that opens the popover. Renders a `<button>` element.
    data: popup-open pressed disabled
    props: openOnHover=`false` delay=`300` closeDelay=`0`
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: An overlay displayed beneath the popover. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the popover against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the popover contents. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') instant('click' | 'dismiss' | 'focus' | 'trigger-change') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the popover anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Title: A heading that labels the popover. Renders an `<h2>` element.
- Description: A paragraph with additional information about the popover. Renders a `<p>` element.
- Close: A button that closes the popover. Renders a `<button>` element.
- Viewport: A viewport for displaying content transitions. This component is only required if one popup can be opened by multiple triggers, its content changes based on the trigger, and switching between them is animated. Renders a 
    data: activation-direction( ${'left' | 'right' | ''} ${'down' | 'up' | ''} ) current instant('click' | 'dismiss' | 'focus' | 'trigger-change') previous transitioning

######## preview-card
USAGE: - **Protect screen reader users' current context**: Exposing each preview to screen readers would force users through its contents before they could continue through the page, repeatedly disrupting their current context. Keep the link as the only accessible interface and include all previewed information at its destination. - **Keep popup content supplementary**: Avoid placing unique or essential 
ANATOMY:
<PreviewCard.Root>
  <PreviewCard.Trigger />
  <PreviewCard.Portal>
    <PreviewCard.Backdrop />
    <PreviewCard.Positioner>
      <PreviewCard.Popup>
        <PreviewCard.Arrow />
        <PreviewCard.Viewport />
      </PreviewCard.Popup>
    </PreviewCard.Positioner>
  </PreviewCard.Portal>
</PreviewCard.Root>;
- Root: Groups all parts of the preview card. Doesn't render its own HTML element.
- Trigger: A link that opens the preview card. Renders an `<a>` element.
    data: popup-open
    props: delay=`600` closeDelay=`300`
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Backdrop: A presentational overlay displayed beneath the popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the popup against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the preview card contents. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the preview card anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Viewport: A viewport for displaying content transitions. This component is only required if one popup can be opened by multiple triggers, its content changes based on the trigger, and switching between them is animated. Renders a 
    data: activation-direction( ${'left' | 'right' | ''} ${'down' | 'up' | ''} ) current instant('dismiss' | 'focus') previous transitioning

######## progress
ANATOMY:
<Progress.Root>
  <Progress.Label />
  <Progress.Track>
    <Progress.Indicator />
  </Progress.Track>
  <Progress.Value />
</Progress.Root>;
- Root: Groups all parts of the progress bar and provides the task completion status to screen readers. Renders a `<div>` element.
    data: complete indeterminate progressing
    props: min=`0` max=`100`
- Value: A text element displaying the current value. Renders a `<span>` element.
    data: complete indeterminate progressing
- Indicator: Visualizes the completion status of the task. Renders a `<div>` element.
    data: complete indeterminate progressing
- Track: Contains the progress bar indicator. Renders a `<div>` element.
    data: complete indeterminate progressing
- Label: An accessible label for the progress bar. Renders a `<span>` element.
    data: complete indeterminate progressing

######## radio-group
USAGE: - **Form controls must have an accessible name**: It can be created using `<label>` elements, or the `Field` and `Fieldset` components. See [Labeling a radio group](/react/components/radio-group.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<RadioGroup>
  <Radio.Root>
    <Radio.Indicator />
  </Radio.Root>
</RadioGroup>;
- RadioGroup: 
- Root: Represents the radio button itself. Renders a `<span>` element and a hidden `<input>` beside.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused
- Indicator: Indicates whether the radio button is selected. Renders a `<span>` element.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused starting-style ending-style
    props: keepMounted=`false`
- RadioGroup: Provides shared state to a series of radio buttons. Renders a `<div>` element.
    data: disabled
    props: disabled=`false`

######## scroll-area
ANATOMY:
<ScrollArea.Root>
  <ScrollArea.Viewport>
    <ScrollArea.Content />
  </ScrollArea.Viewport>
  <ScrollArea.Scrollbar>
    <ScrollArea.Thumb />
  </ScrollArea.Scrollbar>
  <ScrollArea.Corner />
</ScrollArea.Root>;
- Root: Groups all parts of the scroll area. Renders a `<div>` element.
    data: has-overflow-x has-overflow-y overflow-x-end overflow-x-start overflow-y-end overflow-y-start scrolling
- Content: A container for the content of the scroll area. Renders a `<div>` element.
    data: has-overflow-x has-overflow-y overflow-x-end overflow-x-start overflow-y-end overflow-y-start scrolling
- Viewport: The actual scrollable container of the scroll area. Renders a `<div>` element.
    data: has-overflow-x has-overflow-y overflow-x-end overflow-x-start overflow-y-end overflow-y-start scrolling
- Thumb: The draggable part of the scrollbar that indicates the current scroll position. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') scrolling
- Scrollbar: A vertical or horizontal scrollbar for the scroll area. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') has-overflow-x has-overflow-y hovering overflow-x-end overflow-x-start overflow-y-end overflow-y-start scrolling
    props: orientation=`'vertical'` keepMounted=`false`
- Corner: A small rectangular area that appears at the intersection of horizontal and vertical scrollbars. Renders a `<div>` element.

######## select
USAGE: - **Prefer Combobox for large lists**: Select is not filterable, aside from basic keyboard typeahead functionality to find items by focusing and highlighting them. Prefer [Combobox](/react/components/combobox.md) instead of Select when the number of items is sufficiently large to warrant filtering. - **Special positioning behavior**: The select popup by default overlaps its trigger so the selected
ANATOMY:
<Select.Root>
  <Select.Label />
  <Select.Trigger>
    <Select.Value />
    <Select.Icon />
  </Select.Trigger>
  <Select.Portal>
    <Select.Backdrop />
    <Select.Positioner>
      <Select.Popup>
        <Select.ScrollUpArrow />
        <Select.Arrow />
        <Select.List>
          <Select.Item>
            <Select.ItemText />
            <Select.ItemIndicator />
          </Select.Item>
          <Select.Separator />
          <Select.Group>
            <Select.GroupLabel />
          </Select.Group>
        </Select.List>
        <Select.ScrollDownArrow />
      </Select.Popup>
    </Select.Positioner>
  </Select.Portal>
</Select.Root>;
- Root: Groups all parts of the select. Doesn't render its own HTML element.
    props: highlightItemOnHover=`true` modal=`true` multiple=`false` disabled=`false`
- Trigger: A button that opens the select popup. Renders a `<button>` element.
    data: popup-open popup-side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start' | null) pressed disabled readonly required valid invalid dirty touched filled focused placeholder
- Value: A text label of the currently selected item. Renders a `<span>` element.
    data: placeholder
- Icon: An icon that indicates that the trigger button opens a select popup. Renders a `<span>` element.
    data: popup-open
- List: A container for the select items. Renders a `<div>` element.
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
- Backdrop: An overlay displayed beneath the select popup. Renders a `<div>` element.
    data: open closed starting-style ending-style
- Positioner: Positions the select popup. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('none' | 'top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: alignItemWithTrigger=`true` disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'bottom'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the select list. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') side('none' | 'top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the select popup anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') side('none' | 'top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Item: An individual option in the select popup. Renders a `<div>` element.
    data: selected highlighted disabled
    props: disabled=`false`
- Group: Groups related select items with the corresponding label. Renders a `<div>` element.
- GroupLabel: An accessible label that is automatically associated with its parent group. Renders a `<div>` element.
- Separator: A visual separator between items or groups. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`
- Label: An accessible label that is automatically associated with the select trigger. Renders a `<div>` element.
- ItemText: A text label of the select item. Renders a `<div>` element.
- ItemIndicator: Indicates whether the select item is selected. Renders a `<span>` element.
    data: starting-style ending-style
- ScrollUpArrow: An element that scrolls the select popup up when hovered. Does not render when using touch input. Renders a `<div>` element.
    data: direction('up') side('none' | 'top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') visible starting-style ending-style
    props: keepMounted=`false`
- ScrollDownArrow: An element that scrolls the select popup down when hovered. Does not render when using touch input. Renders a `<div>` element.
    data: direction('down') side('none' | 'top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') visible starting-style ending-style
    props: keepMounted=`false`

######## separator
ANATOMY:
<Separator />;
- Separator: A separator element accessible to screen readers. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
    props: orientation=`'horizontal'`

######## slider
USAGE: - **Form controls must have an accessible name**: Prefer `<Slider.Label>`, or provide an `aria-label` on each `<Slider.Thumb>` when no visible label is rendered. See [Labeling a slider](/react/components/slider.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<Slider.Root>
  <Slider.Label />
  <Slider.Value />
  <Slider.Control>
    <Slider.Track>
      <Slider.Indicator />
      <Slider.Thumb />
    </Slider.Track>
  </Slider.Control>
</Slider.Root>;
- Root: Groups all parts of the slider. Renders a `<div>` element.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused
    props: thumbAlignment=`'center'` thumbCollisionBehavior=`'push'` step=`1` largeStep=`10` minStepsBetweenValues=`0` min=`0` max=`100` disabled=`false` orientation=`'horizontal'`
- Value: Displays the current value of the slider as text. Renders an `<output>` element.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused
- Indicator: Visualizes the current value of the slider. Renders a `<div>` element.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused
- Track: Contains the slider indicator and represents the entire range of the slider. Renders a `<div>` element.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused
- Thumb: The draggable part of the slider at the tip of the indicator. Renders a `<div>` element and a nested `<input type="range">`.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused index
    props: disabled=`false`
- Control: The clickable, interactive part of the slider. Renders a `<div>` element.
    data: dragging orientation('horizontal' | 'vertical') disabled valid invalid dirty touched focused
- Label: An accessible label that is automatically associated with the slider thumbs. Renders a `<div>` element.

######## switch
USAGE: - **Form controls must have an accessible name**: It can be created using a `<label>` element or the `Field` component. See [Labeling a switch](/react/components/switch.md) and the [forms guide](/react/handbook/forms.md).
ANATOMY:
<Switch.Root>
  <Switch.Thumb />
</Switch.Root>;
- Root: Represents the switch itself. Renders a `<span>` element and a hidden `<input>` beside.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused
    props: disabled=`false`
- Thumb: The movable part of the switch that indicates whether the switch is on or off. Renders a `<span>`.
    data: checked unchecked disabled readonly required valid invalid dirty touched filled focused

######## tabs
ANATOMY:
<Tabs.Root>
  <Tabs.List>
    <Tabs.Tab />
    <Tabs.Indicator />
  </Tabs.List>
  <Tabs.Panel />
</Tabs.Root>;
- Root: Groups the tabs and the corresponding panels. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') activation-direction('left' | 'right' | 'up' | 'down' | 'none')
    props: orientation=`'horizontal'`
- List: Groups the individual tab buttons. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') activation-direction('left' | 'right' | 'up' | 'down' | 'none')
    props: activateOnFocus=`false` loopFocus=`true`
- Panel: A panel displayed when the corresponding tab is active. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') activation-direction('left' | 'right' | 'up' | 'down' | 'none') hidden index starting-style ending-style
    props: keepMounted=`false`
- Indicator: A visual indicator that can be styled to match the position of the currently active tab. Renders a `<span>` element.
    data: orientation('horizontal' | 'vertical') activation-direction('left' | 'right' | 'up' | 'down' | 'none')
- Tab: An individual interactive tab button that toggles the corresponding panel. Renders a `<button>` element.
    data: orientation('horizontal' | 'vertical') disabled activation-direction('left' | 'right' | 'up' | 'down' | 'none') active

######## toast
ANATOMY:
<Toast.Provider>
  <Toast.Portal>
    <Toast.Viewport>
      {/* Stacked toasts */}
      <Toast.Root>
        <Toast.Content>
          <Toast.Title />
          <Toast.Description />
          <Toast.Action />
          <Toast.Close />
        </Toast.Content>
      </Toast.Root>
      {/* Anchored toasts */}
      <Toast.Positioner>
        <Toast.Root>
          <Toast.Arrow />
          <Toast.Content>
            <Toast.Title />
            <Toast.Description />
            <Toast.Action />
            <Toast.Close />
          </Toast.Content>
        </Toast.Root>
      </Toast.Positioner>
    </Toast.Viewport>
  </Toast.Portal>
</Toast.Provider>;
- Root: Groups all parts of an individual toast. Renders a `<div>` element.
    data: expanded(boolean) limited(boolean) swipe-direction('up' | 'down' | 'left' | 'right') swiping(boolean) type(string) starting-style ending-style
    props: swipeDirection=`['down', 'right']`
- Provider: Provides a context for creating and managing toasts.
    props: limit=`3` timeout=`5000`
- Portal: A portal element that moves the viewport to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
- Positioner: Positions the toast against the anchor. Renders a `<div>` element.
    data: anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'top'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Arrow: Displays an element positioned against the toast anchor. Renders a `<div>` element.
    data: uncentered align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Content: A container for the contents of a toast. Renders a `<div>` element.
    data: behind(boolean) expanded(boolean)
- Title: A title that labels the toast. Renders an `<h2>` element.
    data: type(string)
- Description: A description that describes the toast. Can be used as the default message for the toast when no title is provided. Renders a `<p>` element.
    data: type(string)
- Close: Closes the toast when clicked. Renders a `<button>` element.
    data: type(string)
- Action: Performs an action when clicked. Renders a `<button>` element.
    data: type(string)
- Viewport: A container viewport for toasts. Renders a `<div>` element.
    data: expanded(boolean)
- createToastManager: Creates a new toast manager.
- useToastManager: Returns the array of toasts and methods to manage them.

######## toggle-group
ANATOMY:
<ToggleGroup />;
- ToggleGroup: Provides a shared state to a series of toggle buttons.
    data: orientation('horizontal' | 'vertical') disabled multiple
    props: loopFocus=`true` multiple=`false` disabled=`false` orientation=`'horizontal'`

######## toggle
ANATOMY:
<Toggle />;
- Toggle: A two-state button that can be on or off. Renders a `<button>` element.
    data: pressed disabled
    props: disabled=`false`

######## toolbar
USAGE: To ensure that toolbars are accessible and helpful, follow these guidelines: - **Use inputs sparingly**: Left and right arrow keys are used to both move the text insertion cursor in an input, and to navigate among controls in horizontal toolbars. When using an input in a horizontal toolbar, use only one and place it as the last element of the toolbar.
ANATOMY:
<Toolbar.Root>
  <Toolbar.Button />
  <Toolbar.Link />
  <Toolbar.Separator />
  <Toolbar.Group>
    <Toolbar.Button />
    <Toolbar.Button />
  </Toolbar.Group>
  <Toolbar.Input />
</Toolbar.Root>;
- Root: A container for grouping a set of controls, such as buttons, toggle groups, or menus. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') disabled
    props: loopFocus=`true` orientation=`'horizontal'`
- Input: A native input element that integrates with Toolbar keyboard navigation. Renders an `<input>` element.
    data: orientation('horizontal' | 'vertical') disabled focusable
    props: disabled=`false`
- Group: Groups several toolbar items or toggles. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical') disabled
    props: disabled=`false`
- Separator: A separator element accessible to screen readers. Renders a `<div>` element.
    data: orientation('horizontal' | 'vertical')
- Button: A button that can be used as-is or as a trigger for other components. Renders a `<button>` element.
    data: orientation('horizontal' | 'vertical') disabled focusable
    props: disabled=`false`
- Link: A link component. Renders an `<a>` element.
    data: orientation('horizontal' | 'vertical')

######## tooltip
USAGE: - **Prefer using tooltips as visual labels only**: Tooltips should act as supplementary visual labels for sighted mouse and keyboard users. Tooltips alone are not accessible to touch or screen reader users. See [Alternatives to tooltips](/react/components/tooltip.md) for more details. - **Provide an accessible name for the trigger**: Tooltips are visual-only elements and are not a replacement for 
ANATOMY:
<Tooltip.Provider>
  <Tooltip.Root>
    <Tooltip.Trigger />
    <Tooltip.Portal>
      <Tooltip.Positioner>
        <Tooltip.Popup>
          <Tooltip.Arrow />
          <Tooltip.Viewport />
        </Tooltip.Popup>
      </Tooltip.Positioner>
    </Tooltip.Portal>
  </Tooltip.Root>
</Tooltip.Provider>;
- Root: Groups all parts of the tooltip. Doesn't render its own HTML element.
    props: disabled=`false`
- Provider: Provides a shared delay for multiple tooltips. The grouping logic ensures that once a tooltip becomes visible, the adjacent tooltips will be shown instantly.
    props: timeout=`400`
- Trigger: An element to attach the tooltip to. Renders a `<button>` element.
    data: popup-open trigger-disabled
    props: closeOnClick=`true` disabled=`false` delay=`600` closeDelay=`0`
- Portal: A portal element that moves the popup to a different part of the DOM. By default, the portal element is appended to `<body>`. Renders a `<div>` element.
    props: keepMounted=`false`
- Positioner: Positions the tooltip against the trigger. Renders a `<div>` element.
    data: open closed anchor-hidden align('start' | 'center' | 'end') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
    props: disableAnchorTracking=`false` align=`'center'` alignOffset=`0` side=`'top'` sideOffset=`0` arrowPadding=`5` collisionPadding=`5` sticky=`false` positionMethod=`'absolute'`
- Popup: A container for the tooltip contents. Renders a `<div>` element.
    data: open closed align('start' | 'center' | 'end') instant('delay' | 'dismiss' | 'focus') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start') starting-style ending-style
- Arrow: Displays an element positioned against the tooltip anchor. Renders a `<div>` element.
    data: open closed uncentered align('start' | 'center' | 'end') instant('delay' | 'dismiss' | 'focus') side('top' | 'bottom' | 'left' | 'right' | 'inline-end' | 'inline-start')
- Viewport: A viewport for displaying content transitions. This component is only required if one popup can be opened by multiple triggers, its content changes based on the trigger, and switching between them is animated. Renders a 
    data: activation-direction( ${'left' | 'right' | ''} ${'down' | 'up' | ''} ) current instant('delay' | 'dismiss' | 'focus') previous transitioning
```

##### Radix Primitives parts, data attributes, defaults and CSS variables

```text

######## accordion
ANATOMY:
	<Accordion.Root>
		<Accordion.Item>
			<Accordion.Header>
				<Accordion.Trigger />
			</Accordion.Header>
			<Accordion.Content />
		</Accordion.Item>
	</Accordion.Root>
- Root: data[orientation=[vertical,horizontal]] props[collapsible=false orientation="vertical"]
- Item: data[state=[open,closed] disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Header: data[state=[open,closed] disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Trigger: data[state=[open,closed] disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Content: data[state=[open,closed] disabled=Presentwhendisabled orientation=[vertical,horizontal]] vars[--radix-accordion-content-height --radix-accordion-content-width]

######## alert-dialog
ANATOMY:
	<AlertDialog.Root>
		<AlertDialog.Trigger />
		<AlertDialog.Portal>
			<AlertDialog.Overlay />
			<AlertDialog.Content>
				<AlertDialog.Title />
				<AlertDialog.Description />
				<AlertDialog.Cancel />
				<AlertDialog.Action />
			</AlertDialog.Content>
		</AlertDialog.Portal>
	</AlertDialog.Root>
- Root: data[]
- Trigger: data[state=[open,closed]]
- Portal: data[]
- Overlay: data[state=[open,closed]]
- Content: data[state=[open,closed]]
- Cancel: data[]
- Action: data[]
- Title: data[]
- Description: data[]

######## aspect-ratio
ANATOMY:
export default () => <AspectRatio.Root />;
- Root: data[] props[ratio=1]

######## avatar
ANATOMY:
	<Avatar.Root>
		<Avatar.Image />
		<Avatar.Fallback />
	</Avatar.Root>
- Root: data[]
- Image: data[]
- Fallback: data[]

######## checkbox
ANATOMY:
	<Checkbox.Root>
		<Checkbox.Indicator />
	</Checkbox.Root>
- Root: data[state=[checked,unchecked,indeterminate] disabled=Presentwhendisabled]
- Indicator: data[state=[checked,unchecked,indeterminate] disabled=Presentwhendisabled]

######## collapsible
ANATOMY:
	<Collapsible.Root>
		<Collapsible.Trigger />
		<Collapsible.Content />
	</Collapsible.Root>
- Root: data[state=[open,closed] disabled=Presentwhendisabled]
- Trigger: data[state=[open,closed] disabled=Presentwhendisabled]
- Content: data[state=[open,closed] disabled=Presentwhendisabled] vars[--radix-collapsible-content-height --radix-collapsible-content-width]

######## context-menu
ANATOMY:
	<ContextMenu.Root>
		<ContextMenu.Trigger />
		<ContextMenu.Portal>
			<ContextMenu.Content>
				<ContextMenu.Label />
				<ContextMenu.Item />
				<ContextMenu.Group>
					<ContextMenu.Item />
				</ContextMenu.Group>
				<ContextMenu.CheckboxItem>
					<ContextMenu.ItemIndicator />
				</ContextMenu.CheckboxItem>
				<ContextMenu.RadioGroup>
					<ContextMenu.RadioItem>
						<ContextMenu.ItemIndicator />
					</ContextMenu.RadioItem>
				</ContextMenu.RadioGroup>
				<ContextMenu.Sub>
					<ContextMenu.SubTrigger />
					<ContextMenu.Portal>
						<ContextMenu.SubContent />
					</ContextMenu.Portal>
				</ContextMenu.Sub>
				<ContextMenu.Separator />
			</ContextMenu.Content>
		</ContextMenu.Portal>
	</ContextMenu.Root>
- Root: data[] props[modal=true]
- Trigger: data[state=[open,closed]]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[loop=false alignOffset=0 avoidCollisions=true collisionPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-context-menu-content-available-height --radix-context-menu-content-available-width --radix-context-menu-content-transform-origin --radix-context-menu-trigger-height --radix-context-menu-trigger-width]
- Arrow: data[] props[width=10 height=5]
- Item: data[highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- Group: data[]
- Label: data[]
- CheckboxItem: data[state=[checked,unchecked,indeterminate] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- RadioGroup: data[]
- RadioItem: data[state=[checked,unchecked,indeterminate] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- ItemIndicator: data[state=[checked,unchecked,indeterminate]]
- Separator: data[]
- Sub: data[]
- SubTrigger: data[state=[open,closed] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- SubContent: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[loop=false sideOffset=0 align="start" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-context-menu-content-available-height --radix-context-menu-content-available-width --radix-context-menu-content-transform-origin --radix-context-menu-trigger-height --radix-context-menu-trigger-width]

######## dialog
ANATOMY:
	<Dialog.Root>
		<Dialog.Trigger />
		<Dialog.Portal>
			<Dialog.Overlay />
			<Dialog.Content>
				<Dialog.Title />
				<Dialog.Description />
				<Dialog.Close />
			</Dialog.Content>
		</Dialog.Portal>
	</Dialog.Root>
- Root: data[] props[modal=true]
- Trigger: data[state=[open,closed]]
- Portal: data[]
- Overlay: data[state=[open,closed]]
- Content: data[state=[open,closed]]
- Close: data[]
- Title: data[]
- Description: data[]

######## dropdown-menu
ANATOMY:
	<DropdownMenu.Root>
		<DropdownMenu.Trigger />
		<DropdownMenu.Portal>
			<DropdownMenu.Content>
				<DropdownMenu.Label />
				<DropdownMenu.Item />
				<DropdownMenu.Group>
					<DropdownMenu.Item />
				</DropdownMenu.Group>
				<DropdownMenu.CheckboxItem>
					<DropdownMenu.ItemIndicator />
				</DropdownMenu.CheckboxItem>
				<DropdownMenu.RadioGroup>
					<DropdownMenu.RadioItem>
						<DropdownMenu.ItemIndicator />
					</DropdownMenu.RadioItem>
				</DropdownMenu.RadioGroup>
				<DropdownMenu.Sub>
					<DropdownMenu.SubTrigger />
					<DropdownMenu.Portal>
						<DropdownMenu.SubContent />
					</DropdownMenu.Portal>
				</DropdownMenu.Sub>
				<DropdownMenu.Separator />
				<DropdownMenu.Arrow />
			</DropdownMenu.Content>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
- Root: data[] props[modal=true]
- Trigger: data[state=[open,closed] disabled=Presentwhendisabled]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center] orientation=[vertical,horizontal]] props[loop=false side="bottom" sideOffset=0 align="center" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-dropdown-menu-content-available-height --radix-dropdown-menu-content-available-width --radix-dropdown-menu-content-transform-origin --radix-dropdown-menu-trigger-height --radix-dropdown-menu-trigger-width]
- Arrow: data[] props[width=10 height=5]
- Item: data[orientation=[vertical,horizontal] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- Group: data[]
- Label: data[]
- CheckboxItem: data[state=[checked,unchecked,indeterminate] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- RadioGroup: data[]
- RadioItem: data[state=[checked,unchecked,indeterminate] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- ItemIndicator: data[state=[checked,unchecked,indeterminate]]
- Separator: data[]
- Sub: data[]
- SubTrigger: data[state=[open,closed] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled] vars[--radix-dropdown-menu-content-available-height --radix-dropdown-menu-content-available-width --radix-dropdown-menu-content-transform-origin --radix-dropdown-menu-trigger-height --radix-dropdown-menu-trigger-width]
- SubContent: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center] orientation=[vertical,horizontal]] props[loop=false sideOffset=0 align="start" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false]

######## form
ANATOMY:
	<Form.Root>
		<Form.Field>
			<Form.Label />
			<Form.Control />
			<Form.Message />
			<Form.ValidityState />
		</Form.Field>
		<Form.Message />
		<Form.ValidityState />
		<Form.Submit />
	</Form.Root>
- Root: data[]
- Field: data[invalid=Presentwhenthefieldisinvalid valid=Presentwhenthefieldisvalid]
- Label: data[invalid=Presentwhenthefieldisinvalid valid=Presentwhenthefieldisvalid]
- Control: data[invalid=Presentwhenthefieldisinvalid valid=Presentwhenthefieldisvalid]
- Message: data[]
- ValidityState: data[]
- Submit: data[]

######## hover-card
ANATOMY:
	<HoverCard.Root>
		<HoverCard.Trigger />
		<HoverCard.Portal>
			<HoverCard.Content>
				<HoverCard.Arrow />
			</HoverCard.Content>
		</HoverCard.Portal>
	</HoverCard.Root>
- Root: data[] props[openDelay=700 closeDelay=300]
- Trigger: data[state=[open,closed]]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[side="bottom" sideOffset=0 align="center" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-hover-card-content-available-height --radix-hover-card-content-available-width --radix-hover-card-content-transform-origin --radix-hover-card-trigger-height --radix-hover-card-trigger-width]
- Arrow: data[] props[width=10 height=5]

######## label
ANATOMY:
export default () => <Label.Root />;
- Root: data[]

######## menubar
ANATOMY:
	<Menubar.Root>
		<Menubar.Menu>
			<Menubar.Trigger />
			<Menubar.Portal>
				<Menubar.Content>
					<Menubar.Label />
					<Menubar.Item />
					<Menubar.Group>
						<Menubar.Item />
					</Menubar.Group>
					<Menubar.CheckboxItem>
						<Menubar.ItemIndicator />
					</Menubar.CheckboxItem>
					<Menubar.RadioGroup>
						<Menubar.RadioItem>
							<Menubar.ItemIndicator />
						</Menubar.RadioItem>
					</Menubar.RadioGroup>
					<Menubar.Sub>
						<Menubar.SubTrigger />
						<Menubar.Portal>
							<Menubar.SubContent />
						</Menubar.Portal>
					</Menubar.Sub>
					<Menubar.Separator />
					<Menubar.Arrow />
				</Menubar.Content>
			</Menubar.Portal>
		</Menubar.Menu>
	</Menubar.Root>
- Root: data[] props[loop=false]
- Menu: data[]
- Trigger: data[state=[open,closed] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[loop=false side="bottom" sideOffset=0 align="center" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-menubar-content-available-height --radix-menubar-content-available-width --radix-menubar-content-transform-origin --radix-menubar-trigger-height --radix-menubar-trigger-width]
- Arrow: data[] props[width=10 height=5]
- Item: data[highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- Group: data[]
- Label: data[]
- CheckboxItem: data[state=[checked,unchecked] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- RadioGroup: data[]
- RadioItem: data[state=[checked,unchecked] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- ItemIndicator: data[state=[checked,unchecked]]
- Separator: data[]
- Sub: data[]
- SubTrigger: data[state=[open,closed] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- SubContent: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center] orientation=[vertical,horizontal]] props[loop=false sideOffset=0 align="start" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-menubar-content-available-height --radix-menubar-content-available-width --radix-menubar-content-transform-origin --radix-menubar-trigger-height --radix-menubar-trigger-width]

######## navigation-menu
ANATOMY:
	<NavigationMenu.Root>
		<NavigationMenu.List>
			<NavigationMenu.Item>
				<NavigationMenu.Trigger />
				<NavigationMenu.Content>
					<NavigationMenu.Link />
				</NavigationMenu.Content>
			</NavigationMenu.Item>
			<NavigationMenu.Item>
				<NavigationMenu.Link />
			</NavigationMenu.Item>
			<NavigationMenu.Item>
				<NavigationMenu.Trigger />
				<NavigationMenu.Content>
					<NavigationMenu.Sub>
						<NavigationMenu.List />
						<NavigationMenu.Viewport />
					</NavigationMenu.Sub>
				</NavigationMenu.Content>
			</NavigationMenu.Item>
			<NavigationMenu.Indicator />
		</NavigationMenu.List>
		<NavigationMenu.Viewport />
	</NavigationMenu.Root>
- Root: data[orientation=[vertical,horizontal]] props[delayDuration=200 skipDelayDuration=300 orientation="horizontal"]
- Sub: data[orientation=[vertical,horizontal]] props[orientation="horizontal"]
- List: data[orientation=[vertical,horizontal]]
- Item: data[]
- Trigger: data[state=[open,closed] disabled=Presentwhendisabled]
- Content: data[state=[open,closed] motion=[to-start,to-end,from-start,from-end] orientation=[vertical,horizontal]]
- Link: data[active=Presentwhenactive]
- Indicator: data[state=[visible,hidden] orientation=[vertical,horizontal]] vars[--radix-navigation-menu-indicator-translate-x --radix-navigation-menu-indicator-translate-y]
- Viewport: data[state=[open,closed] orientation=[vertical,horizontal]] vars[--radix-navigation-menu-viewport-height --radix-navigation-menu-viewport-width]

######## one-time-password-field
ANATOMY:
	<OneTimePasswordField.Root>
		{/* one Input for each character of the value */}
		<OneTimePasswordField.Input />
		{/* single HiddenInput to store the full value */}
		<OneTimePasswordField.HiddenInput />
	</OneTimePasswordField.Root>
- Root: data[orientation=[vertical,horizontal]] props[orientation="vertical" type="text"]
- Input: data[index=Theindexcorrespondingwiththeindexofthecharacterrelativetotherootfieldvalue]
- HiddenInput: data[]

######## password-toggle-field
ANATOMY:
	<PasswordToggleField.Root>
		<PasswordToggleField.Input />
		<PasswordToggleField.Toggle>
			<PasswordToggleField.Icon
				visible={<EyeOpenIcon />}
				hidden={<EyeClosedIcon />}
			/>
		</PasswordToggleField.Toggle>
	</PasswordToggleField.Root>
- Root: data[]
- Input: data[]
- Toggle: data[]
- Slot: data[]
- Icon: data[]

######## popover
ANATOMY:
	<Popover.Root>
		<Popover.Trigger />
		<Popover.Anchor />
		<Popover.Portal>
			<Popover.Content>
				<Popover.Close />
				<Popover.Arrow />
			</Popover.Content>
		</Popover.Portal>
	</Popover.Root>
- Root: data[] props[modal=false]
- Trigger: data[state=[open,closed]]
- Anchor: data[]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[side="bottom" sideOffset=0 align="center" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-popover-content-available-height --radix-popover-content-available-width --radix-popover-content-transform-origin --radix-popover-trigger-height --radix-popover-trigger-width]
- Arrow: data[] props[width=10 height=5]
- Close: data[]

######## progress
ANATOMY:
	<Progress.Root>
		<Progress.Indicator />
	</Progress.Root>
- Root: data[state=[complete,indeterminate,loading] value=Thecurrentvalue max=Themaxvalue]
- Indicator: data[state=[complete,indeterminate,loading] value=Thecurrentvalue max=Themaxvalue]

######## radio-group
ANATOMY:
	<RadioGroup.Root>
		<RadioGroup.Item>
			<RadioGroup.Indicator />
		</RadioGroup.Item>
	</RadioGroup.Root>
- Root: data[disabled=Presentwhendisabled] props[orientation=undefined loop=true]
- Item: data[state=[checked,unchecked] disabled=Presentwhendisabled]
- Indicator: data[state=[checked,unchecked] disabled=Presentwhendisabled]

######## scroll-area
ANATOMY:
	<ScrollArea.Root>
		<ScrollArea.Viewport />
		<ScrollArea.Scrollbar orientation="horizontal">
			<ScrollArea.Thumb />
		</ScrollArea.Scrollbar>
		<ScrollArea.Scrollbar orientation="vertical">
			<ScrollArea.Thumb />
		</ScrollArea.Scrollbar>
		<ScrollArea.Corner />
	</ScrollArea.Root>
- Root: data[] props[type="hover" scrollHideDelay=600]
- Viewport: data[]
- Scrollbar: data[state=[visible,hidden] orientation=[vertical,horizontal]] props[orientation=vertical]
- Thumb: data[state=[visible,hidden]]
- Corner: data[]

######## select
ANATOMY:
	<Select.Root>
		<Select.Trigger>
			<Select.Value />
			<Select.Icon />
		</Select.Trigger>
		<Select.Portal>
			<Select.Content>
				<Select.ScrollUpButton />
				<Select.Viewport>
					<Select.Item>
						<Select.ItemText />
						<Select.ItemIndicator />
					</Select.Item>
					<Select.Group>
						<Select.Label />
						<Select.Item>
							<Select.ItemText />
							<Select.ItemIndicator />
						</Select.Item>
					</Select.Group>
					<Select.Separator />
				</Select.Viewport>
				<Select.ScrollDownButton />
				<Select.Arrow />
			</Select.Content>
		</Select.Portal>
	</Select.Root>
- Root: data[]
- Trigger: data[state=[open,closed] disabled=Presentwhendisabled placeholder=Presentwhenhasplaceholder]
- Value: data[]
- Icon: data[]
- Portal: data[]
- Content: data[state=[open,closed] side=[left,right,bottom,top] align=[start,end,center]] props[position="item-aligned" side="bottom" sideOffset=0 align="start" alignOffset=0 avoidCollisions=true collisionPadding=10 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-select-content-available-height --radix-select-content-available-width --radix-select-content-transform-origin --radix-select-trigger-height --radix-select-trigger-width]
- Viewport: data[]
- Item: data[state=[checked,unchecked] highlighted=Presentwhenhighlighted disabled=Presentwhendisabled]
- ItemText: data[]
- ItemIndicator: data[]
- ScrollUpButton: data[]
- ScrollDownButton: data[]
- Group: data[]
- Label: data[]
- Separator: data[]
- Arrow: data[] props[width=10 height=5]

######## separator
ANATOMY:
export default () => <Separator.Root />;
- Root: data[orientation=[vertical,horizontal]] props[orientation="horizontal"]

######## slider
ANATOMY:
	<Slider.Root>
		<Slider.Track>
			<Slider.Range />
		</Slider.Track>
		<Slider.Thumb />
	</Slider.Root>
- Root: data[disabled=Presentwhendisabled orientation=[vertical,horizontal]] props[orientation="horizontal" min=0 max=100 step=1 minStepsBetweenThumbs=0]
- Track: data[disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Range: data[disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Thumb: data[disabled=Presentwhendisabled orientation=[vertical,horizontal]]

######## switch
ANATOMY:
	<Switch.Root>
		<Switch.Thumb />
	</Switch.Root>
- Root: data[state=[checked,unchecked] disabled=Presentwhendisabled]
- Thumb: data[state=[checked,unchecked] disabled=Presentwhendisabled]

######## tabs
ANATOMY:
	<Tabs.Root>
		<Tabs.List>
			<Tabs.Trigger />
		</Tabs.List>
		<Tabs.Content />
	</Tabs.Root>
- Root: data[orientation=[vertical,horizontal]] props[orientation="horizontal" activationMode="automatic"]
- List: data[orientation=[vertical,horizontal]] props[loop=true]
- Trigger: data[state=[active,inactive] disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Content: data[state=[active,inactive] orientation=[vertical,horizontal]]

######## toast
ANATOMY:
	<Toast.Provider>
		<Toast.Root>
			<Toast.Title />
			<Toast.Description />
			<Toast.Action />
			<Toast.Close />
		</Toast.Root>
		<Toast.Viewport />
	</Toast.Provider>
- Provider: data[] props[duration=5000 label="Notification" swipeDirection="right" swipeThreshold=50]
- Viewport: data[] props[label="Notifications ({hotkey})"]
- Root: data[state=[open,closed] swipe=[start,move,cancel,end] swipe-direction=[up,down,left,right]] props[type="foreground"] vars[--radix-toast-swipe-end-x --radix-toast-swipe-end-y --radix-toast-swipe-move-x --radix-toast-swipe-move-y]
- Title: data[]
- Description: data[]
- Action: data[]
- Close: data[]

######## toggle-group
ANATOMY:
	<ToggleGroup.Root>
		<ToggleGroup.Item />
	</ToggleGroup.Root>
- Root: data[orientation=[vertical,horizontal]] props[orientation=undefined loop=true]
- Item: data[state=[on,off] disabled=Presentwhendisabled orientation=[vertical,horizontal]]

######## toggle
ANATOMY:
export default () => <Toggle.Root />;
- Root: data[state=[on,off] disabled=Presentwhendisabled]

######## toolbar
ANATOMY:
	<Toolbar.Root>
		<Toolbar.Button />
		<Toolbar.Separator />
		<Toolbar.Link />
		<Toolbar.ToggleGroup>
			<Toolbar.ToggleItem />
		</Toolbar.ToggleGroup>
	</Toolbar.Root>
- Root: data[orientation=[vertical,horizontal]] props[orientation="horizontal" loop=true]
- Button: data[orientation=[vertical,horizontal]]
- Link: data[]
- ToggleGroup: data[orientation=[vertical,horizontal]]
- ToggleItem: data[state=[on,off] disabled=Presentwhendisabled orientation=[vertical,horizontal]]
- Separator: data[orientation=[vertical,horizontal]]

######## tooltip
ANATOMY:
	<Tooltip.Provider>
		<Tooltip.Root>
			<Tooltip.Trigger />
			<Tooltip.Portal>
				<Tooltip.Content>
					<Tooltip.Arrow />
				</Tooltip.Content>
			</Tooltip.Portal>
		</Tooltip.Root>
	</Tooltip.Provider>
- Provider: data[] props[delayDuration=700 skipDelayDuration=300]
- Root: data[] props[delayDuration=700 disableHoverableContent=false]
- Trigger: data[state=[closed,delayed-open,instant-open]]
- Portal: data[]
- Content: data[state=[closed,delayed-open,instant-open] side=[left,right,bottom,top] align=[start,end,center]] props[side="top" sideOffset=0 align="center" alignOffset=0 avoidCollisions=true collisionPadding=0 arrowPadding=0 sticky="partial" hideWhenDetached=false] vars[--radix-tooltip-content-available-height --radix-tooltip-content-available-width --radix-tooltip-content-transform-origin --radix-tooltip-trigger-height --radix-tooltip-trigger-width]
- Arrow: data[] props[width=10 height=5]

```

### Inferences
- For a static design tool, a "complete" component sheet can be derived directly from these attribute lists: every part x every listed boolean state. A practical minimum per interactive control is: default, hover, pressed/active, focus-visible, disabled, plus the control-specific states (checked/indeterminate, open, highlighted vs selected item, placeholder vs filled, invalid, read-only, loading/indeterminate).
- Base UI's part names are the better naming convention for layers in a canvas tool (Trigger / Backdrop / Positioner / Popup / Arrow / Title / Description / Close; Track / Indicator / Thumb; Item / ItemText / ItemIndicator; Group / GroupLabel; Separator), because they are uniform across all popup components, whereas Radix mixes Content/Overlay/Viewport/Range/Label per component.
- `data-starting-style`/`data-ending-style` have no static equivalent; in Base UI's own demos they resolve to opacity 0 and scale 0.98, so the "resting" open state is the only one that needs drawing.
- Highlighted and selected being independent in Select/Combobox implies a menu mock-up should show one highlighted row (filled background) and, separately, the selected row's check indicator.

### Gaps
- I did not verify which Radix primitives are still labelled "preview/unstable" in late 2026 (Form, One-Time Password Field and Password Toggle Field were introduced as previews); the docs tree lists them alongside stable components without a status flag in the file names.
- Per-component keyboard-interaction tables were not extracted.
- Base UI CSS-variable lists per part were not captured in the appendix (the extraction missed that table format); the popup variables are listed in section 2.

## 2. Popup positioning model, defaults, nesting

### Takeaway
Both libraries position with side + align + sideOffset + alignOffset + collisionPadding + arrowPadding (Base UI builds on Floating UI); the unstyled defaults are `side=bottom` (tooltips/toasts `top`), `align=center`, offsets 0, with collision padding 5px in Base UI and 0px in Radix (10px for Radix Select), while the styled Radix Themes layer uses sideOffset 4px (tooltip, select, dropdown), 8px (popover, hover card), align `start` and collisionPadding 10px. Nested Base UI dialogs do not get their own backdrop; the parent is shifted and scaled down by a `--nested-dialogs` count.

### Cited Findings
- Base UI Positioner defaults (Popover, Menu, Select, Combobox, Autocomplete, Preview Card, Navigation Menu): `side='bottom'`, `align='center'`, `sideOffset=0`, `alignOffset=0`, `arrowPadding=5`, `collisionPadding=5`, `sticky=false`, `positionMethod='absolute'`. Tooltip and anchored Toast default to `side='top'`. Context Menu defaults to `align='start'`, `side='bottom'` (anchored at the pointer). — [Base UI Popover](https://base-ui.com/react/components/popover); [Base UI Tooltip](https://base-ui.com/react/components/tooltip); [Base UI Context Menu](https://base-ui.com/react/components/context-menu)
- Base UI side values: `top | bottom | left | right | inline-start | inline-end` (logical sides for RTL); align `start | center | end`. Select additionally reports `side='none'` when the popup overlays the trigger. — [Base UI Select](https://base-ui.com/react/components/select)
- Base UI Select defaults to `alignItemWithTrigger=true`: the popup overlaps the trigger so the selected item's text sits exactly over the trigger's value text (macOS-style); in that mode the demo CSS widens the popup to `calc(var(--anchor-width) + 1.75rem)`. — [Base UI Select](https://base-ui.com/react/components/select)
- Base UI Positioner CSS variables: `--anchor-width`, `--anchor-height`, `--available-width`, `--available-height`, `--positioner-width`, `--positioner-height`, `--transform-origin`; Popup: `--popup-width`, `--popup-height`. Docs use `min-width: var(--anchor-width)` for select popups and `max-height: var(--available-height)` for scrollable popups. — [Base UI Popover](https://base-ui.com/react/components/popover); [Base UI Styling](https://base-ui.com/react/handbook/styling)
- Radix equivalents: `--radix-<component>-trigger-width/height`, `--radix-<component>-content-available-width/height`, `--radix-<component>-content-transform-origin`. — [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover)
- Radix Content defaults (Popover, Dropdown Menu, Hover Card, Menubar): `side="bottom"`, `sideOffset=0`, `align="center"`, `alignOffset=0`, `avoidCollisions=true`, `collisionPadding=0`, `arrowPadding=0`, `sticky="partial"`, `hideWhenDetached=false`. Tooltip `side="top"`. Sub-menu content `align="start"`. Select: `position="item-aligned"` default, with popper-mode defaults `align="start"`, `collisionPadding=10`. — [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover); [Radix Select](https://www.radix-ui.com/primitives/docs/components/select)
- Arrow size: Radix `Arrow` defaults `width=10`, `height=5` (px). Base UI's Arrow is an unstyled div; the docs demo draws it as a 12px x 6px box containing a rotated square of side `6px * sqrt(2)` with the popup's 1px border. Base UI Arrow reports `data-uncentered` when collision handling pushed it off-centre. — [Radix Popover](https://www.radix-ui.com/primitives/docs/components/popover); [Base UI Popover demo CSS](https://base-ui.com/react/components/popover)
- Base UI docs demo uses `sideOffset={8}` on the Menu positioner. — [Base UI Styling](https://base-ui.com/react/handbook/styling)
- Radix Themes styled defaults: Tooltip `sideOffset={4}`, `collisionPadding={10}`; Popover and HoverCard `align="start"`, `sideOffset={8}`, `collisionPadding={10}`; DropdownMenu `align="start"`, `sideOffset={4}`, `collisionPadding={10}`; DropdownMenu/ContextMenu sub-content `alignOffset={-size*4}` (i.e. -4px or -8px so the first sub-item lines up with its parent item) and `sideOffset={1}`; Select content `sideOffset={4}`. — [radix-ui/themes components](https://github.com/radix-ui/themes/tree/main/packages/radix-ui-themes/src/components)
- Radix Themes popover/select content use `min-width: var(--radix-popover-trigger-width)` / `var(--radix-select-trigger-width)`: a popup is never narrower than its trigger. — [popover.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/popover.css); [select.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/select.css)
- Open delays: Base UI Tooltip 600ms (Provider grouping timeout 400ms: neighbouring tooltips then show instantly), Preview Card open 600ms / close 300ms, Popover hover-open 300ms, Menu hover trigger and SubmenuTrigger 100ms, Navigation Menu 50ms/50ms. Radix Tooltip `delayDuration=700`, `skipDelayDuration=300`; Hover Card `openDelay=700`, `closeDelay=300`; Navigation Menu `delayDuration=200`, `skipDelayDuration=300`. — [Base UI Tooltip](https://base-ui.com/react/components/tooltip); [Base UI Preview Card](https://base-ui.com/react/components/preview-card); [Radix Tooltip](https://www.radix-ui.com/primitives/docs/components/tooltip); [Radix Hover Card](https://www.radix-ui.com/primitives/docs/components/hover-card)
- Modality defaults: Base UI Dialog `modal=true`, Menu `modal=true`, Select `modal=true`, Popover `modal=false`, Combobox/Autocomplete `modal=false`. Radix Dialog `modal=true`, DropdownMenu `modal=true`, Popover `modal=false`. — [Base UI Dialog](https://base-ui.com/react/components/dialog); [Base UI Popover](https://base-ui.com/react/components/popover); [Radix Dialog](https://www.radix-ui.com/primitives/docs/components/dialog)
- Nested dialogs (Base UI): the parent Popup gets `data-nested-dialog-open` and a numeric `--nested-dialogs`; the child gets `data-nested`. "Backdrops of the child dialogs won't be rendered." The docs demo offsets and shrinks the parent per nesting level: `translate: -50% calc(-50% + 1.25rem * var(--nested-dialogs)); scale: calc(1 - 0.1 * var(--nested-dialogs));` and dims it with an `::after` overlay. — [Base UI Dialog](https://base-ui.com/react/components/dialog)
- Base UI Drawer nesting states: `data-nested-drawer-open`, `data-nested-drawer-swiping`; default `swipeDirection='down'` (bottom sheet); `Drawer.Indent`/`IndentBackground` scale the app behind an open drawer. — [Base UI Drawer](https://base-ui.com/react/components/drawer)
- Base UI submenus: SubmenuTrigger `openOnHover=true`, `delay=100`, `closeDelay=0`, carries `data-popup-open` + `data-highlighted` while its submenu is open; Menubar reports `data-has-submenu-open`. — [Base UI Menu](https://base-ui.com/react/components/menu); [Base UI Menubar](https://base-ui.com/react/components/menubar)
- Radix Themes dialog layout: overlay `--color-overlay` (black-a6 = rgba(0,0,0,0.4) light; black-a8 = rgba(0,0,0,0.6) dark); scroll padding top 32px, bottom max(32px, 6vh), sides 16px; content `--color-panel-solid`, `--shadow-6`; enter animation from `translateY(5px) scale(0.97)`, opacity 0. — [base-dialog.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-dialog.css); [color.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/color.css); [blackA.ts](https://github.com/radix-ui/colors/blob/main/src/blackA.ts)

### Inferences
- A sensible default for drawn popups: gap of 4px for tooltips/menus/select lists and 8px for popovers/hover cards; keep at least 10px from the frame edge; left-align (align start) menus and popovers to their trigger, centre tooltips above it; make list popups at least as wide as the trigger.
- Sub-menus should overlap/abut the parent by about 1px and be shifted up by the popup's inner padding (4 or 8px) so the first sub-item aligns with the parent item.
- To depict a nested dialog: no second scrim; push the parent down ~20px and scale it to 90% per level, slightly dimmed.
- Arrow geometry to draw: 10 x 5 (Radix) or 12 x 6 (Base UI demo), same fill and border as the popup.

### Gaps
- Radix Themes Dialog `maxWidth` default (believed 600px) and AlertDialog width were not found in the files I fetched; unverified.
- Default z-ordering/stacking guidance is not documented by either library beyond portal usage.

## 3. Accessibility rules with a visual consequence

### Takeaway
Neither library enforces focus rings, contrast, labels or target sizes in the unstyled layer: Base UI says explicitly that "it's the developer's responsibility to visually indicate focus" and recommends APCA for contrast, while Radix Themes shows what a conforming implementation looks like: a 2px solid `--focus-8` outline (offset -1px on fields/soft buttons, +2px on solid buttons, checkboxes, radios and switches).

### Cited Findings
- Base UI: "While Base UI components manage focus, it's the developer's responsibility to visually indicate focus", typically via `:focus`/`:focus-visible`, pointing to WCAG 2.2 Focus Appearance. — [Base UI Accessibility](https://base-ui.com/react/overview/accessibility)
- Base UI on contrast: "Unless your application has strict requirements around compliance with current standards, consider adhering to APCA." — [Base UI Accessibility](https://base-ui.com/react/overview/accessibility)
- Base UI: every form control must have an accessible name. Use `Field.Label`/`<label>` for Input, NumberField, OTPField, Autocomplete, Combobox, Checkbox, Radio, Switch; `Select.Label`, `Combobox.Label`, `Slider.Label` for trigger-based controls; `Fieldset.Legend` plus `Field.Item` per option for checkbox/radio groups; `aria-label` only as a fallback when no visible label is rendered. — [Base UI Forms handbook](https://base-ui.com/react/handbook/forms)
- Base UI Field anatomy order is Label, Control, Description, (Item), Error, Validity; `Field.Error` with no children shows the browser's native validation message, and `match` selects messages per validity state. — [Base UI Field](https://base-ui.com/react/components/field); [Base UI Forms handbook](https://base-ui.com/react/handbook/forms)
- Base UI docs demos place Description and Error below the control with a 4px (0.25rem) gap in a column; label is 14px/20px bold; error text is red `oklch(50.5% 0.213 27.518)` (light) / `oklch(70.4% 0.191 22.216)` (dark); description is grey `oklch(43.9% 0 0)`. — [Base UI Field demo CSS](https://base-ui.com/react/components/field)
- Base UI docs demo focus ring: `outline: 2px solid` in the text colour, `outline-offset: -1px` on buttons/inputs/tabs, `+2px` on checkbox, switch and slider thumb. — [Base UI Button demo CSS](https://base-ui.com/react/components/button); [Base UI Checkbox demo CSS](https://base-ui.com/react/components/checkbox)
- Base UI demos raise input text to 16px/24px on coarse pointers (`@media (any-pointer: coarse)`), which avoids iOS zoom-on-focus. — [Base UI Input demo CSS](https://base-ui.com/react/components/input)
- Radix Themes focus ring: `outline: 2px solid var(--focus-8)`; `outline-offset: 2px` for solid and classic buttons, checkbox, radio, switch; `-1px` for soft, ghost, outline, surface buttons, select trigger, text field, card, segmented control; tabs `-2px` on the inner label. Slider thumb uses a double ring: `0 0 0 3px var(--accent-3), 0 0 0 5px var(--focus-8)`. — [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css); [slider.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/slider.css)
- Radix Themes: "Most of the components use `--focus-8` for the focus outline color", and the focus scale follows the component's accent colour (gray-accent components fall back to the theme accent). — [Radix Themes Color](https://www.radix-ui.com/themes/docs/theme/color)
- Radix Themes enlarges the slider thumb hit area to 3x the thumb size with an invisible pseudo-element, and on coarse pointers shows a `0.5em` accent-a4 outline as press feedback on buttons. — [slider.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/slider.css); [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css)
- Radix Primitives: AlertDialog moves focus to the Cancel button on open (the least destructive action is the default). — [Radix Accessibility](https://www.radix-ui.com/primitives/docs/overview/accessibility)
- Keyboard-hint visuals in Radix Themes menus: a shortcut slot is right-aligned (`margin-left: auto`, `padding-left: 16px`), coloured `gray-a11`, and inherits the item colour when the item is highlighted or disabled. `Kbd` is `0.75em` text, min-width 1.75em, line-height 1.7em, 0.5em side padding, radius `0.35em x radius-factor`. — [base-menu.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-menu.css); [kbd.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/kbd.css)
- Toolbar guidance: use at most one text input in a horizontal toolbar and place it last, because arrow keys both move the caret and navigate the toolbar. — [Base UI Toolbar](https://base-ui.com/react/components/toolbar)

### Inferences
- Neither library states a minimum target size. Radix Themes' control heights are 24/32/40/48px, so only size 4 meets a 44px touch target and size 1 exactly meets WCAG 2.2's 24px minimum; size 2 (32px) is the desktop default.
- Neither library ships a "required" asterisk; `data-required` exists on Base UI controls so a required marker is a styling decision. A drawing assistant should add one consistently (label suffix) because the state exists but has no default visual.
- Disabled is drawn by colour, not opacity, in Radix Themes (gray-a8 text on gray-a3 fill), which keeps disabled controls legible on any background.

### Gaps
- No documented minimum target size, required-indicator convention or error-icon convention in either library; statements above about them are inferences.
- Radix Themes has no Field/error-message component; error placement guidance comes only from Base UI.

## 4. Radix Colors

### Takeaway
Radix Colors 3.0 is 31 hues (6 grays + 25 colours), each a 12-step scale with fixed jobs (1-2 backgrounds, 3-5 component fills for rest/hover/active, 6-8 borders, 9-10 solids, 11-12 text), each with light, dark, alpha and Display-P3 variants. Steps 11 and 12 are guaranteed Lc 60 and Lc 90 APCA on a step 2 background of the same scale, and five step-9 solids (Sky, Mint, Lime, Yellow, Amber) need dark text.

### Cited Findings
- Step use cases, exactly: 1 App background; 2 Subtle background; 3 UI element background; 4 Hovered UI element background; 5 Active / Selected UI element background; 6 Subtle borders and separators; 7 UI element border and focus rings; 8 Hovered UI element border; 9 Solid backgrounds; 10 Hovered solid backgrounds; 11 Low-contrast text; 12 High-contrast text. — [use-cases-table.tsx](https://github.com/radix-ui/website/blob/main/components/use-cases-table.tsx); [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Steps 1-2: app background, striped table rows, code block, card, sidebar, canvas area backgrounds; usable interchangeably. Docs suggest white for the light app background and step 1 or 2 in dark mode. — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Steps 3/4/5 = normal/hover/pressed-or-selected component backgrounds; "If your component has a transparent background in its default state, you can use Step 3 for its hover state." — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Step 6 = subtle borders on non-interactive components (sidebars, headers, cards, alerts, separators); step 7 = subtle borders on interactive components; step 8 = stronger borders on interactive components and focus rings. — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Step 9 "has the highest chroma of all steps in the scale"; step 10 is the hover state for a step 9 background. Step 9 is identical in light and dark for most hues (e.g. blue `#0090ff`, red `#e5484d`, green `#30a46c`, indigo `#3e63dd`, amber `#ffc53d` in both modes). — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale); [light.ts](https://github.com/radix-ui/colors/blob/main/src/light.ts); [dark.ts](https://github.com/radix-ui/colors/blob/main/src/dark.ts)
- "Most step 9 colors are designed for white foreground text. Sky, Mint, Lime, Yellow, and Amber are designed for dark foreground text." Radix Themes implements this as `--<hue>-contrast`: `white` for most hues, and step 12 of the paired gray for the five bright ones (sky `#1c2024` slate-12; mint `#1a211e` sage-12; lime `#1d211c` olive-12; yellow and amber `#21201c` sand-12). — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale); [themes tokens/colors](https://github.com/radix-ui/themes/tree/main/packages/radix-ui-themes/src/styles/tokens/colors)
- Contrast guarantee: "Steps 11 and 12, which are designed for text, are guaranteed to Lc 60 and Lc 90 APCA contrast ratio on top of a step 2 background from the same scale." — [Understanding the scale](https://www.radix-ui.com/colors/docs/palette-composition/understanding-the-scale)
- Hue list (31): grays gray, mauve, slate, sage, olive, sand; colours tomato, red, ruby, crimson, pink, plum, purple, violet, iris, indigo, blue, cyan, teal, jade, green, grass, bronze, gold, brown, orange, amber, yellow, lime, mint, sky. Plus overlay scales blackA and whiteA. — [light.ts](https://github.com/radix-ui/colors/blob/main/src/light.ts)
- Gray tints: gray is pure; mauve is purple-based, slate blue-based, sage green-based, olive lime-based, sand yellow-based. "Neutral pairing": gray works with anything. "Natural pairing": pick the gray tinted toward the accent hue. — [Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Automatic gray pairing used by Radix Themes: mauve for tomato, red, ruby, crimson, pink, plum, purple, violet; slate for iris, indigo, blue, sky, cyan; sage for teal, jade, mint, green; olive for grass, lime; sand for yellow, amber, orange, brown, gold, bronze; gray for gray. — [get-matching-gray-color.ts](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/helpers/get-matching-gray-color.ts)
- Semantic pairings suggested: Error red/ruby/tomato/crimson; Success green/teal/jade/grass/mint; Warning yellow/amber/orange; Info blue/indigo/sky/cyan. — [Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Text scale choice: accent 11/12 for a "more colorful vibe" (suggested for marketing sites), gray 11/12 for a "more functional vibe" (suggested for apps). — [Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- "Radix Colors are not intended to be customised"; add custom brand scales alongside rather than editing them. — [Composing a palette](https://www.radix-ui.com/colors/docs/palette-composition/composing-a-palette)
- Alpha scales: every step has an alpha twin (`a1`-`a12`) "designed to appear visually the same when placed over the page background", so tinted fills and borders stay correct on coloured or layered surfaces. — [Radix Themes Color](https://www.radix-ui.com/themes/docs/theme/color)
- blackA / whiteA opacities, steps 1-12: 0.05, 0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 0.95. — [blackA.ts](https://github.com/radix-ui/colors/blob/main/src/blackA.ts); [whiteA.ts](https://github.com/radix-ui/colors/blob/main/src/whiteA.ts)
- Versions: 3.0.0 "A complete rework of all colors" + "Add P3 colorspace versions of all scales"; 2.1.0 added ruby, iris, jade; 2.0.0 reworked all colours for contrast; 1.0.0 moved to hyphenated variables (`--gray-1`, `--gray-a1`). — [Radix Colors releases](https://www.radix-ui.com/colors/docs/overview/releases)
- P3 is applied under `@supports (color: color(display-p3 1 1 1))` + `@media (color-gamut: p3)`, with sRGB hex as the fallback. — [indigo.css in themes tokens](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/colors/indigo.css)
- Exact sRGB values, steps 1-12 (light): 
  - gray `#fcfcfc #f9f9f9 #f0f0f0 #e8e8e8 #e0e0e0 #d9d9d9 #cecece #bbbbbb #8d8d8d #838383 #646464 #202020`
  - slate `#fcfcfd #f9f9fb #f0f0f3 #e8e8ec #e0e1e6 #d9d9e0 #cdced6 #b9bbc6 #8b8d98 #80838d #60646c #1c2024`
  - mauve `#fdfcfd #faf9fb #f2eff3 #eae7ec #e3dfe6 #dbd8e0 #d0cdd7 #bcbac7 #8e8c99 #84828e #65636d #211f26`
  - sage `#fbfdfc #f7f9f8 #eef1f0 #e6e9e8 #dfe2e0 #d7dad9 #cbcfcd #b8bcba #868e8b #7c8481 #5f6563 #1a211e`
  - olive `#fcfdfc #f8faf8 #eff1ef #e7e9e7 #dfe2df #d7dad7 #cccfcc #b9bcb8 #898e87 #7f847d #60655f #1d211c`
  - sand `#fdfdfc #f9f9f8 #f1f0ef #e9e8e6 #e2e1de #dad9d6 #cfceca #bcbbb5 #8d8d86 #82827c #63635e #21201c`
  - blue `#fbfdff #f4faff #e6f4fe #d5efff #c2e5ff #acd8fc #8ec8f6 #5eb1ef #0090ff #0588f0 #0d74ce #113264`
  - indigo `#fdfdfe #f7f9ff #edf2fe #e1e9ff #d2deff #c1d0ff #abbdf9 #8da4ef #3e63dd #3358d4 #3a5bc7 #1f2d5c`
  - red `#fffcfc #fff7f7 #feebec #ffdbdc #ffcdce #fdbdbe #f4a9aa #eb8e90 #e5484d #dc3e42 #ce2c31 #641723`
  - green `#fbfefc #f4fbf6 #e6f6eb #d6f1df #c4e8d1 #adddc0 #8eceaa #5bb98b #30a46c #2b9a66 #218358 #193b2d`
  - amber `#fefdfb #fefbe9 #fff7c2 #ffee9c #fbe577 #f3d673 #e9c162 #e2a336 #ffc53d #ffba18 #ab6400 #4f3422`
  - grayA `#00000003 #00000006 #0000000f #00000017 #0000001f #00000026 #00000031 #00000044 #00000072 #0000007c #0000009b #000000df`
  — [light.ts](https://github.com/radix-ui/colors/blob/main/src/light.ts)
- Exact sRGB values, steps 1-12 (dark):
  - gray `#111111 #191919 #222222 #2a2a2a #313131 #3a3a3a #484848 #606060 #6e6e6e #7b7b7b #b4b4b4 #eeeeee`
  - slate `#111113 #18191b #212225 #272a2d #2e3135 #363a3f #43484e #5a6169 #696e77 #777b84 #b0b4ba #edeef0`
  - blue `#0d1520 #111927 #0d2847 #003362 #004074 #104d87 #205d9e #2870bd #0090ff #3b9eff #70b8ff #c2e6ff`
  - indigo `#11131f #141726 #182449 #1d2e62 #253974 #304384 #3a4f97 #435db1 #3e63dd #5472e4 #9eb1ff #d6e1ff`
  - red `#191111 #201314 #3b1219 #500f1c #611623 #72232d #8c333a #b54548 #e5484d #ec5d5e #ff9592 #ffd1d9`
  - green `#0e1512 #121b17 #132d21 #113b29 #174933 #20573e #28684a #2f7c57 #30a46c #33b074 #3dd68c #b1f1cb`
  - amber `#16120c #1d180f #302008 #3f2700 #4d3000 #5c3d05 #714f19 #8f6424 #ffc53d #ffd60a #ffca16 #ffe7b3`
  - grayA `#00000000 #ffffff09 #ffffff12 #ffffff1b #ffffff22 #ffffff2c #ffffff3b #ffffff55 #ffffff64 #ffffff72 #ffffffaf #ffffffed`
  — [dark.ts](https://github.com/radix-ui/colors/blob/main/src/dark.ts)

### Inferences
- In dark mode step 10 is lighter than step 9 (e.g. blue-dark 10 `#3b9eff` vs 9 `#0090ff`) while in light mode it is darker (`#0588f0`); "hover = step 10" therefore brightens in dark and darkens in light. A generator must not hard-code "hover = darker".
- For the five bright hues, amber/yellow step 10 in dark mode is even brighter, and Themes maps `--yellow-indicator`/`--yellow-track` to step 10 in light mode: bright hues need special handling for fills that must be visible on white.
- The hex lists for the 20 hues not printed above can be read from the same two source files; the shape (two near-whites, three tints, three borders, two solids, two texts) is identical.

### Gaps
- Hex values for the remaining 20 hues and all P3 values were not transcribed here (available in `radix-ui/colors/src/light.ts`, `dark.ts`).
- No APCA guarantee is documented for step 11/12 on step 3-5 fills, or for white on step 9; only the step-2 guarantee is stated.
- The date of Radix Colors 3.0.0 was not captured from the releases page.

## 5. Radix Themes tokens, sizes and variants

### Takeaway
Radix Themes (docs show 3.1.3 as the latest listed release) is a small, strict token system: space 4/8/12/16/24/32/40/48/64, type 12/14/16/18/20/24/28/35/60, radius 3/4/6/8/12/16 multiplied by a theme factor (0, 0.75, 1, 1.5, 1.5+pill), six shadows, and a global `scaling` multiplier of 0.9-1.1. Controls come in heights 24/32/40/48, and every variant is a fixed recipe over the accent scale: solid = 9 fill + contrast text; soft = a3 fill + a11 text; surface = translucent surface + a7 border + a11 text; outline = a8 border + a11 text; ghost = text only with a3 hover.

### Cited Findings

#### Scales (all multiplied by `--scaling`)
- Space: `--space-1` 4px, `-2` 8px, `-3` 12px, `-4` 16px, `-5` 24px, `-6` 32px, `-7` 40px, `-8` 48px, `-9` 64px. — [space.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/space.css)
- Scaling: `90%`=0.9, `95%`=0.95, `100%`=1, `105%`=1.05, `110%`=1.1; it multiplies space, font size, line height and radius. — [scaling.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/scaling.css)
- Type scale (size / line-height / letter-spacing): 1 = 12px / 16px / +0.0025em; 2 = 14 / 20 / 0; 3 = 16 / 24 / 0; 4 = 18 / 26 / -0.0025em; 5 = 20 / 28 / -0.005em; 6 = 24 / 30 / -0.00625em; 7 = 28 / 36 / -0.0075em; 8 = 35 / 40 / -0.01em; 9 = 60 / 60 / -0.025em. — [typography.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/typography.css)
- Heading line heights are tighter at sizes 2-5: 16, 18, 22, 24, 26, 30, 36, 40, 60px for sizes 1-9. — [typography.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/typography.css)
- Weights: light 300, regular 400, medium 500, bold 700. Default text = size 3 (16px), line-height 1.5, regular. Default font stack is the system UI stack (`-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', 'Open Sans', system-ui, sans-serif`); code is Menlo/Consolas at 0.95 size-adjust with -0.007em tracking; `Em` and `Quote` are Times New Roman italic at 1.18 size-adjust, -0.025em. — [typography.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/typography.css)
- Active tab label: medium weight with -0.01em letter-spacing; inactive: regular, 0em (a hidden bold copy reserves width so tabs do not shift). — [typography.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/typography.css); [base-tab-list.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-tab-list.css)
- Radius scale base values: `--radius-1` 3px, `-2` 4px, `-3` 6px, `-4` 8px, `-5` 12px, `-6` 16px, each x `--radius-factor`. — [radius.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/radius.css)
- Radius settings: `none` factor 0; `small` 0.75; `medium` 1; `large` 1.5; `full` 1.5 with `--radius-full: 9999px` (0px in all other settings). `--radius-thumb` is 0.5px for none/small and 9999px for medium/large/full. Resulting px for steps 1-6: small 2.25/3/4.5/6/9/12; medium 3/4/6/8/12/16; large and full 4.5/6/9/12/18/24. — [radius.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/radius.css)
- Controls use `border-radius: max(var(--radius-N), var(--radius-full))`, so only at `full` do buttons, badges, text fields, select triggers and avatars become pills/circles; panels (Card, Dialog, Popover, Menu) use plain `--radius-N` and never become pills; "a Checkbox will never become fully rounded to prevent any confusion" with a radio. — [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css); [Radix Themes Radius](https://www.radix-ui.com/themes/docs/theme/radius)
- Shadows, light mode:
  - 1 (inset, for classic fields): `inset 0 0 0 1px gray-a5, inset 0 1.5px 2px 0 gray-a2, inset 0 1.5px 2px 0 black-a2`
  - 2 (classic card): `0 0 0 1px gray-a3, 0 0 0 0.5px black-a1, 0 1px 1px 0 gray-a2, 0 2px 1px -1px black-a1, 0 1px 3px 0 black-a1`
  - 3: `0 0 0 1px gray-a3, 0 2px 3px -2px gray-a3, 0 3px 12px -4px black-a2, 0 4px 16px -8px black-a2`
  - 4 (hover card): `0 0 0 1px gray-a3, 0 8px 40px black-a1, 0 12px 32px -16px gray-a3`
  - 5 (popover, menu, select): `0 0 0 1px gray-a3, 0 12px 60px black-a3, 0 12px 32px -16px gray-a5`
  - 6 (dialog): `0 0 0 1px gray-a3, 0 12px 60px black-a3, 0 16px 64px gray-a2, 0 16px 36px -20px gray-a7`
  — [shadow.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/shadow.css)
- Shadows, dark mode: the 1px ring becomes `gray-a6` and the blurs use heavier black alphas, e.g. 5: `0 0 0 1px gray-a6, 0 12px 60px black-a5, 0 12px 32px -16px black-a7`; 6: `0 0 0 1px gray-a6, 0 12px 60px black-a4, 0 16px 64px black-a6, 0 16px 36px -20px black-a11`. Every elevation shadow (2-6) includes a 1px hairline ring, so floating panels have no separate border. — [shadow.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/shadow.css)
- Semantic colours: light `--color-background: white`, `--color-panel-solid: white`, `--color-panel-translucent: rgba(255,255,255,0.7)`, `--color-surface: rgba(255,255,255,0.85)`, `--color-overlay: black-a6`; dark `--color-background: gray-1`, `--color-panel-solid: gray-2`, `--color-panel-translucent: gray-a2`, `--color-surface: rgba(0,0,0,0.25)`, `--color-overlay: black-a8`. — [color.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/color.css)
- Panel background setting: `solid` sets `--color-panel` to panel-solid with no backdrop filter; `translucent` sets it to panel-translucent with `backdrop-filter: blur(64px)`. Card and Table (surface variant) use `--color-panel`; Dialog, Popover, HoverCard, Menu and Select content always use `--color-panel-solid`. — [color.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/color.css); [popover.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/popover.css)
- Functional accent tokens: `--accent-contrast` (text on solid), `--accent-surface` (tinted translucent fill; e.g. indigo `#f5f8ffcc` light = 80% alpha, `#171d3b80` dark = 50% alpha), `--accent-indicator` (checkbox/radio/tab indicator, = step 9), `--accent-track` (slider/switch/progress fill, = step 9; for sky, mint, lime, yellow, amber in dark mode it is a mix of step 8 and 9). — [themes tokens/colors](https://github.com/radix-ui/themes/tree/main/packages/radix-ui-themes/src/styles/tokens/colors)
- Selection colour is `--focus-a5`; containers: 448, 688, 880, 1136px; Section vertical padding 24/40/64/80px. — [color.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/color.css); [container.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/container.css); [section.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/section.css)
- Cursors default to `default` for buttons, checkboxes, menu items, switches; `pointer` only for links; `not-allowed` for disabled. — [cursor.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/styles/tokens/cursor.css)

#### Component sizes (px at 100% scaling, medium radius)

| Component | Size 1 | Size 2 | Size 3 | Size 4 |
|---|---|---|---|---|
| Button height | 24 | 32 | 40 | 48 |
| Button padding-x / gap | 8 / 4 | 12 / 8 | 16 / 12 | 24 / 12 |
| Button font / line-height | 12 / 16 | 14 / 20 | 16 / 24 | 18 / 26 |
| Button radius | 3 | 4 | 6 | 8 |
| Ghost button padding x / y | 8 / 4 | 8 / 4 | 12 / 6 | 16 / 8 |
| IconButton | 24x24 | 32x32 | 40x40 | 48x48 |
| TextField height / font / radius | 24 / 12 / 4 | 32 / 14 / 4 | 40 / 16 / 6 | (none) |
| TextField text indent (incl. 1px border) | 6 | 8 | 12 | (none) |
| TextField slot padding-x / gap | 4 / 8 | 8 / 8 | 12 / 12 | (none) |
| Select trigger height / font / radius | 24 / 12 / 3 | 32 / 14 / 4 | 40 / 16 / 6 | (none) |
| Select trigger padding-x / gap | 8 / 4 | 12 / 6 | 16 / 8 | (none) |
| Select/Menu item height / font | 24 / 12 | 32 / 14 | 32 / 16 (select only) | (none) |
| Select/Menu content padding / radius / item radius | 4 / 6 / 3 | 8 / 8 / 4 | 8 / 8 / 4 | (none) |
| Checkbox and Radio box | 14 | 16 | 20 | (none) |
| Checkbox tick icon / radius | 9 / 2.6 | 10 / 3 | 12 / 3.75 | (none) |
| Switch track (w x h) / thumb | 28x16 / 14 | 35x20 / 18 | 42x24 / 22 | (none) |
| Slider track / thumb base | 6 / 10 | 8 / 12 | 10 / 14 | (none) |
| Progress height | 4 | 6 | 8 | (none) |
| Tabs trigger height / font | 32 / 12 | 40 / 14 | (none) | (none) |
| Badge padding y,x / font | 2,6 / 12 | 4,8 / 12 | 4,10 / 14 | (none) |
| Card padding / radius | 12 / 8 | 16 / 8 | 24 / 12 | 32 / 12 (size 5: 48 / 16) |
| Dialog and Popover padding / radius | 12 / 8 | 16 / 8 | 24 / 12 | 32 / 12 |
| HoverCard padding / radius | 12 / 8 | 16 / 8 | 24 / 12 | (none) |
| Callout padding / radius / gap | 12 / 6 / 8 | 16 / 8 / 12 | 24 / 12 / 16 | (none) |
| Table cell padding / min row height | 8 / 36 | 12 / 44 | 12x16 / 48 | (none) |
| Spinner | 12 | 16 | 20 | (none) |

Sources: [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css), [button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/button.css), [icon-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/icon-button.css), [text-field.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/text-field.css), [select.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/select.css), [base-menu.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-menu.css), [base-checkbox.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-checkbox.css), [base-radio.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-radio.css), [switch.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/switch.css), [slider.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/slider.css), [progress.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/progress.css), [base-tab-list.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-tab-list.css), [badge.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/badge.css), [card.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/card.css), [base-dialog.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-dialog.css), [popover.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/popover.css), [hover-card.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/hover-card.css), [callout.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/callout.css), [table.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/table.css), [spinner.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/spinner.css)

- Correction to the brief: Button and IconButton have sizes 1-4, but TextField and Select trigger have only sizes 1-3 (24/32/40px). — [text-field.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/text-field.css); [select.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/select.css)
- Button text is medium weight (500); icons inside non-ghost buttons render at 0.9 opacity. Ghost buttons have no fixed height and use negative margins equal to their padding so their text optically aligns with surrounding content. — [button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/button.css)
- Avatar sizes 1-9: 24, 32, 40, 48, 64, 80, 96, 128, 160px; radius steps 2,2,3,3,4,5,5,6,6; fallback initials uppercase, medium weight. — [avatar.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/avatar.css)
- Switch geometry: width = height x 1.75; thumb inset 1px, thumb = height - 2px, white; pill-shaped whenever radius is medium or larger (`--radius-thumb: 9999px`), nearly square for none/small; thumb slides in 140ms. — [switch.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/switch.css)
- Radio: checked dot is a full-size circle scaled to 0.4 (so 40% of the box diameter), coloured `--accent-contrast`. — [base-radio.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-radio.css)
- Tooltip: padding 4px 8px, fill `gray-12`, text `gray-1` (inverted against the page), radius `--radius-2`, arrow filled `gray-12`, 140ms `cubic-bezier(0.16, 1, 0.3, 1)` slide+fade. — [tooltip.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/tooltip.css)
- Menu details: separator 1px `gray-a6`, 8px vertical margin, inset to the item padding; group label in `gray-a10` at item height; disabled items `gray-a8`; item gap 8px; left padding grows to 24px (20px at size 1) when the menu contains checkbox/radio items so indicators have a gutter; indicator/submenu chevron icons 8px (size 1) or 10px (size 2). — [base-menu.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-menu.css)
- Tabs: list has a 1px bottom hairline `gray-a5`; active tab shows a 2px bar of `--accent-indicator` at the bottom edge; inactive text `gray-a11`, active/hover `gray-12`; hover fills the inner label with `gray-a3`. — [base-tab-list.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-tab-list.css)
- Segmented control: track = `--color-surface` under `gray-a3`; selected segment is an inset (1px) pill of `--color-background` (light) / `gray-a3` (dark); 1px `gray-a4` separators between unselected segments, hidden next to the selected one; selected label medium weight. — [segmented-control.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/segmented-control.css)

#### Variant recipes (accent scale; `aN` = alpha step)

| Variant | Rest | Hover / open | Active (pressed) | Disabled |
|---|---|---|---|---|
| solid | fill accent-9, text accent-contrast | fill accent-10 | accent-10 + filter brightness(0.92) saturate(1.1) (dark: brightness 1.08) | fill gray-a3, text gray-a8 |
| soft | fill accent-a3, text accent-a11 | fill accent-a4 | fill accent-a5 | fill gray-a3, text gray-a8 |
| surface | fill accent-surface, 1px inset ring accent-a7, text accent-a11 | ring accent-a8 | fill accent-a3, ring accent-a8 | fill gray-a2, ring gray-a6, text gray-a8 |
| outline | no fill, 1px inset ring accent-a8, text accent-a11 | fill accent-a2 | fill accent-a3 | ring gray-a7, text gray-a8 |
| ghost | no fill, text accent-a11 | fill accent-a3 | fill accent-a4 | text gray-a8, no fill |
| classic | fill accent-9 with vertical gradient, inner highlight and 1px ring; text accent-contrast | inner layer accent-10 | flattened, content shifts down 1-2px, inset shadow | fill gray-2, text gray-a8 |

Source: [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css)

- High contrast: solid/classic become fill `accent-12` with text `gray-1`; soft/ghost/surface/outline keep their fill and switch text to `accent-12`; outline ring becomes `accent-a7` + `gray-a11`; checkbox/radio checked fill `accent-12` with `accent-1` mark; highlighted menu item `accent-12` fill with `accent-1` text. — [base-button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-button.css); [base-checkbox.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-checkbox.css); [base-menu.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-menu.css)
- TextField variants: surface = `--color-surface` fill + 1px inset `gray-a7` ring, text `gray-12`, placeholder `gray-a10`, slot icons `gray-a11`; classic = same fill with `--shadow-1` inset; soft = `accent-a3` fill, no border, text and placeholder `accent-12` (placeholder at 0.6 opacity). Disabled/read-only: `gray-a2` overlay, ring `gray-a6`, text `gray-a11`. Focus: 2px `--focus-8` outline at -1px offset replaces the border visually. — [text-field.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/text-field.css)
- Select trigger variants: surface = `--color-surface` + `gray-a7` ring (hover/open `gray-a8`), text `gray-12`, placeholder `gray-a10`; soft = `accent-a3` (hover `accent-a4`), text `accent-12`; ghost = transparent (hover `accent-a3`). Select content variants: solid highlights the active item with `accent-9` + contrast text; soft highlights with `accent-a4`. — [select.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/select.css)
- Checkbox surface variant: unchecked = `--color-surface` + 1px `gray-a7` ring; checked/indeterminate = `--accent-indicator` fill + `--accent-contrast` mark; disabled = transparent with `gray-a6` ring and `gray-a8` mark. Soft variant: `accent-a5` fill in all states with `accent-a11` mark. — [base-checkbox.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-checkbox.css)
- Switch surface variant: unchecked track `gray-a3` + `gray-a5` ring; checked track `--accent-track`; thumb white with a soft shadow; disabled track `gray-a3`, thumb `gray-2`. — [switch.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/switch.css)
- Slider surface variant: track `gray-a3` + `gray-a5` ring, range `--accent-track`, white thumb with `0 0 0 1px black-a4`; disabled thumb `gray-1` with `gray-6` ring. — [slider.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/slider.css)
- Badge: solid `accent-9`/contrast; soft `accent-a3`/`accent-a11`; surface `accent-surface` + `accent-a6` ring; outline `accent-a8` ring; medium weight. Callout: soft `accent-a3`; surface `accent-a2` + `accent-a6` ring; outline `accent-a7` ring; text `accent-a11`. — [badge.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/badge.css); [callout.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/callout.css)
- Card surface variant: `--color-panel` fill with a 1px `gray-a5` ring (hover `gray-a7`, active `gray-a6` when the card is a link/button); classic variant uses `--shadow-2`-style layered shadow. Table surface: 1px `gray-a5` border, header row `gray-a2`, row dividers `inset 0 -1px gray-a5`, column headers bold. — [base-card.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-card.css); [table.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/table.css)
- Theme configuration surface: `accentColor`, `grayColor`, `panelBackground` (solid | translucent), `scaling`, `radius` (none | small | medium | large | full). "Variants are visual variations of a component which are used to create visual hierarchies and communicate relative importance." — [Radix Themes theme overview](https://www.radix-ui.com/themes/docs/theme/overview)

### Inferences
- The disabled recipe is uniform across variants (gray-a8 text, gray-a3 or transparent fill, gray-a6/a7 ring): one rule covers every control.
- Borders in Themes are always inset 1px rings (box-shadow), never layout-affecting borders, and almost always alpha steps (a5-a8), so they sit correctly on any surface; in a canvas tool this maps to an inside-aligned 1px stroke with an alpha colour.
- Rest/hover/active for tinted fills is consistently +1 alpha step (a3, a4, a5); for bordered controls hover is +1 border step (a7 to a8). This "+1 step" rule generalises to custom components.
- Inner radius = outer radius - border/padding (TextField input radius = field radius - border width; segmented indicator = radius - 1px): nested shapes should be concentric.
- Panel padding and radius scale together (12/8, 16/8, 24/12, 32/12, 48/16): bigger padding gets bigger radius.

### Gaps
- Theme defaults (believed `accentColor="indigo"`, `grayColor="auto"`, `panelBackground="translucent"`, `radius="medium"`, `scaling="100%"`) were not verified from source in this session (the props file path I tried returned 404).
- Tooltip text size (believed Text size 1 = 12px) and Dialog `maxWidth` default (believed 600px) not verified.
- Radix Themes release dates and whether anything newer than 3.1.3 exists were not captured; the docs' releases page lists 3.1.3 at the top.
- TextArea, DataList, Skeleton, CheckboxCards/RadioCards, ScrollArea metrics were not extracted.

## 6. Base UI's own style guidance, demo styling, history and Radix maintenance status

### Takeaway
Base UI prescribes no visual style, but its current docs demos are a deliberate, minimal monochrome system (near-black `oklch(14.5% 0 0)` 1px borders, zero radius, 32px controls, 14px text, a hard offset shadow `0.25rem 0.25rem 0 rgb(0 0 0 / 12%)`, 100-150ms transitions with scale 0.98) that doubles as a spec for "all the states with the fewest tokens". Base UI reached 1.0.0 on 11 Dec 2025 and has shipped roughly monthly since (1.9.0 on 9 Oct 2026); Radix Primitives is still maintained by WorkOS with releases in June and July 2026, though third-party commentary describes a slower pace and shadcn/ui made Base UI its default in July 2026.

### Cited Findings
- Base UI is "From the creators of Radix, Material UI, and Floating UI ... an unstyled React component library for building accessible user interfaces"; team listed: Colm Tuite, Marija Najdova, Flavien Delangle, James Nelson, Jenna Smith, Michał Dudak, Aarón García. — [About Base UI](https://base-ui.com/react/overview/about)
- "Base UI components are unstyled, don't bundle CSS, and don't prescribe a styling solution"; styling hooks are `className` (string or state function), data attributes, CSS variables and `style`. — [About Base UI](https://base-ui.com/react/overview/about); [Base UI Styling](https://base-ui.com/react/handbook/styling)
- Release history: alphas from Dec 2024 (alpha.4 on 17 Dec 2024), beta.0 on 29 May 2025, rc.0 on 4 Dec 2025, v1.0.0 on 11 Dec 2025; then 1.1.0 (15 Jan 2026), 1.2.0 (12 Feb), 1.3.0 (12 Mar), 1.4.0 (13 Apr), 1.4.1 (20 Apr), 1.5.0 (19 May), 1.6.0 (18 Jun), 1.7.0 (4 Aug), 1.8.0 (4 Sep), 1.9.0 (9 Oct 2026). — [Base UI docs index](https://base-ui.com/llms.txt)
- v1.0.0 renamed the package from `@base-ui-components/react` to `@base-ui/react`. — [v1.0.0 notes](https://base-ui.com/react/overview/releases/v1-0-0)
- v1.3.0: "Drawer is now stable". v1.5.0: Select trigger gained `data-popup-side`. v1.9.0: menu filtering (`Menu.FilterProvider`, `Menu.Input`) added as preview. — [v1.3.0 notes](https://base-ui.com/react/overview/releases/v1-3-0); [v1.5.0 notes](https://base-ui.com/react/overview/releases/v1-5-0); [v1.9.0 notes](https://base-ui.com/react/overview/releases/v1-9-0)
- Demo palette (CSS Modules demos), light / dark: text and border `oklch(14.5% 0 0)` / white; surface white / `oklch(14.5% 0 0)`; hover fill `oklch(97% 0 0)` / `oklch(26.9% 0 0)`; active fill `oklch(92.2% 0 0)` / `oklch(37.1% 0 0)`; disabled text and border `oklch(55.6% 0 0)` / `oklch(70.8% 0 0)`; secondary text `oklch(43.9% 0 0)` / `oklch(70.8% 0 0)`. These match Tailwind v4 neutral-950/100/200/800/700/500/400/600, which the Tailwind versions of the demos use by name. — [Base UI Button demo CSS](https://base-ui.com/react/components/button); [Base UI Styling](https://base-ui.com/react/handbook/styling)
- Demo button: height 2rem (32px), padding 0 0.75rem, gap 0.5rem, 1px border, `border-radius: 0`, 14px text weight 400, line-height 1; trigger-with-chevron variant pads `0 0.5rem 0 0.75rem` with 0.375rem gap; `data-pressed`/`data-popup-open` reuse the hover fill. — [Base UI Button demo CSS](https://base-ui.com/react/components/button); [Base UI Menu demo CSS](https://base-ui.com/react/components/menu)
- Demo input/field: height 2rem, padding 0 0.5rem, 1px border, radius 0, 14px/20px; label 14px/20px weight 700 above with 0.25rem gap; placeholder `oklch(55.6% 0 0)`; field max-width 16rem. — [Base UI Field demo CSS](https://base-ui.com/react/components/field)
- Demo popups (menu, select, popover, tooltip, dialog, toast): 1px border, white fill, `box-shadow: 0.25rem 0.25rem 0 rgb(0 0 0 / 12%)` (no shadow in dark), `transition: transform 100ms ease-out, opacity 100ms ease-out`, enter/exit from `opacity: 0; transform: scale(0.98)` around `var(--transform-origin)`. Popover padding 0.75rem, max-width 500px; tooltip padding 0.25rem 0.5rem, 14px/20px. — [Base UI Popover demo CSS](https://base-ui.com/react/components/popover); [Base UI Tooltip demo CSS](https://base-ui.com/react/components/tooltip)
- Demo menu: popup `padding-block: 0.25rem`; item `padding: 0.5rem 2rem 0.5rem 1rem`, 14px/16px (32px tall); highlighted item is an inverted bar (fill near-black, text white) inset 0.25rem from the popup edges. — [Base UI Menu demo CSS](https://base-ui.com/react/components/menu)
- Demo dialog: width 24rem, `max-width: calc(100vw - 3rem)`, padding 1rem, column gap 1rem, optically raised by `margin-top: -2rem`; title 16px/24px bold; description 14px/20px grey; actions right-aligned with 0.75rem gap; backdrop black at 20% (light) / 50% (dark), 150ms fade. — [Base UI Dialog demo CSS](https://base-ui.com/react/components/dialog)
- Demo switch: 2.25rem x 1.25rem (36x20) track, 1px border, 0.125rem padding, 0.875rem (14px) square thumb translating 1rem; checked inverts track and thumb colours; 150ms ease. Demo checkbox: 1rem box, 1px border, checked/indeterminate filled near-black with white mark; label gap 0.5rem. Demo slider: 14rem wide, 0.25rem track `oklch(92.2% 0 0)`, near-black indicator, 1rem thumb with 1px border, 0.75rem vertical padding on the control for hit area. — [Base UI Switch demo CSS](https://base-ui.com/react/components/switch); [Base UI Checkbox demo CSS](https://base-ui.com/react/components/checkbox); [Base UI Slider demo CSS](https://base-ui.com/react/components/slider)
- Demo tabs: tab height `calc(2rem + 1px)`, padding-inline 0.5rem, gap 0.25rem, inactive text `oklch(43.9% 0 0)`; the Indicator is a bordered white tab shape that slides (`translate`, `width`, 150ms ease-in-out) using `--active-tab-left`/`--active-tab-width`; panel is a bordered box with 1rem padding. — [Base UI Tabs demo CSS](https://base-ui.com/react/components/tabs)
- Demo toast: viewport bottom-right, 1rem inset (2rem and 22.5rem wide above 500px); stacked toasts scale down by 0.1 per index with a 0.75rem peek and expand to a 0.75rem-gap list on hover (`data-expanded`); content padding 0.75rem, gap 1rem; `transition: transform 0.5s cubic-bezier(0.22, 1, 0.36, 1), opacity 0.5s, height 0.15s`. — [Base UI Toast demo CSS](https://base-ui.com/react/components/toast)
- Radix maintenance: the Primitives releases page shows releases dated June 6, June 30, July 6 and July 20, 2026 (controlled Context Menu, new composition APIs, per-primitive subpath imports such as `radix-ui/accordion`, tree-shaking, many Dialog/Slider/Toast/Tooltip fixes). — [Radix Primitives releases](https://www.radix-ui.com/primitives/docs/overview/releases)
- Radix has been owned by WorkOS since the 2022 Modulz acquisition; Radix Icons' README credits "the WorkOS team". I found no official deprecation or end-of-life notice. — [Radix Icons README](https://github.com/radix-ui/icons/blob/master/README.md); [Radix Primitives releases](https://www.radix-ui.com/primitives/docs/overview/releases)
- Secondary/commentary sources (lower reliability, search-result summaries only, not opened): a DEV Community post says many original Radix maintainers have left; 2026 comparison guides position Base UI as the successor for the primitive layer; a releases aggregator reports shadcn/ui made Base UI its default component library on 2 July 2026 with Radix still supported. — [DEV Community](https://dev.to/mashuktamim/is-your-shadcn-ui-project-at-risk-a-deep-dive-into-radixs-future-45ei); [pkgpulse guide](https://www.pkgpulse.com/guides/shadcn-ui-vs-base-ui-vs-radix-components-2026); [releases.sh](https://releases.sh/release/rel_TUxVOyVHeyyTBGlCTm6n4)

### Inferences
- The Base UI demo style is a complete state model in about seven greys: rest, hover, active, pressed/open (= hover), disabled, focus (2px outline), highlighted (inverted). It is a good fallback "wireframe" theme for an assistant that must draw every state without a brand palette.
- The docs demos were restyled at some point after 1.0 (zero radius, hard shadow); older third-party descriptions of Base UI demos with rounded grey styling are out of date. This is inferred from the current CSS only; I did not find the changelog entry for the restyle.
- Radix is not abandoned in 2026 (four dated releases in six weeks mid-year), but momentum and ecosystem defaults have shifted toward Base UI; design knowledge from both remains valid, and Radix Themes/Colors remain the only token systems of the two.

### Gaps
- No release newer than Base UI 1.9.0 (9 Oct 2026) appears in the docs index as of 14 Nov 2026; I did not check GitHub for unreleased work.
- The shadcn/ui default switch and the maintainer-departure claims come from secondary sources that I did not open and verify.
- Radix Themes' and Radix Colors' own 2026 release activity was not established.

## 7. Radix Icons

### Takeaway
Radix Icons is "a crisp set of 15x15 icons": each SVG is `width=15 height=15 viewBox="0 0 15 15"`, drawn as a single filled path using `currentColor`, giving thin, pixel-snapped, outline-style glyphs intended to sit next to 14-15px UI text.

### Cited Findings
- "A crisp set of 15x15 icons designed by the WorkOS team"; MIT licensed; authors Vlad Moroz, Colm Tuite, Pedro Duarte, Lochlan Bunn. — [Radix Icons README](https://github.com/radix-ui/icons/blob/master/README.md)
- File format (check icon): `<svg width="15" height="15" viewBox="0 0 15 15" fill="none">` with one `<path ... fill="currentColor"/>`: strokes are expanded to filled outlines rather than using SVG `stroke`. — [check.svg](https://github.com/radix-ui/icons/blob/master/packages/radix-icons/icons/check.svg)
- In Radix Themes, icons inside menus are 8px (size 1) or 10px (size 2) for check/chevron indicators, the Select trigger chevron is about 11px at size 3, and icons in non-ghost buttons render at 0.9 opacity. — [base-menu.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/_internal/base-menu.css); [select.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/select.css); [button.css](https://github.com/radix-ui/themes/blob/main/packages/radix-ui-themes/src/components/button.css)

### Inferences
- Because the grid is odd (15), glyphs have a true centre pixel and 1px-wide strokes land on whole pixels at 1x, which is the source of the "crisp" look; scaling to 16 or 20px blurs this, so the set is best used at 15px (or exact multiples).
- Icons inherit text colour (`currentColor`), so icon colour should always equal the adjacent label colour for the control's state.

### Gaps
- Icon count, stroke-width convention (visually about 1px) and any 2026 updates to the set were not verified.
