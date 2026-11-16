//! UI screen commands of the table, in protocol order.

use crate::commands::{c, CommandSpec};

pub(super) const PART: &[CommandSpec] = &[
    c(
        "ui.screen",
        "ui",
        "Where the app is: state, screen, open dialogs, toasts, focus, scrollables",
        "",
    )
    .since("1.1")
    .result("`{app_state, screen, loading?, modal_open, dialogs:[{id, tag?, label, visible, controls:[{id, label, role, enabled, focused}]}], toasts:[{id, label}], focus, back_stack, scrollables}`")
    .doc("One call that answers \"what is on screen and what can I do now\". `dialogs` lists every open modal (confirmations, the pause overlay, settings) with the ids of its controls, so a script can answer a confirmation without reading the whole tree; `loading` (stage and progress) is present while a drive loads, which is when the screen has nothing to click."),
    c(
        "ui.scrollables",
        "ui",
        "List the scrollable views with offset, content size and position",
        "visible_only:b?",
    )
    .since("1.1")
    .result("`{count, scrollables:[{id, owner, rect, offset, viewport, content, max, at_top, at_bottom, items:[id...]}]}`")
    .doc("`id` is what `ui.scroll target` takes: the element's own id, else `<nearest id>/scroll` (suffix `-2`, `-3` for duplicates). `items` are the ids of the elements inside the view in layout order, so `ui.scroll item` can reveal any of them."),
    c(
        "ui.scroll",
        "ui",
        "Scroll a view by an amount, by pages, to the top or bottom, or to an item",
        "target:s? by:f? dx:f? pages:f? to:s[top|bottom|left|right]? item:s?",
    )
    .since("1.1")
    .result("the scrollable after the move (as in `ui.scrollables`) plus `moved`")
    .doc("Without `target` the view under focus is used, else the first visible one; while a dialog is open only views inside it can be scrolled (`1009` otherwise). Exactly one of `by` (logical pixels, positive scrolls down), `dx`, `pages` (viewport heights), `to` or `item` is needed. `item` reveals the element with that id (the same effect as focusing it); `1004` when the item is not inside the view."),
    c(
        "ui.text",
        "ui",
        "Type text into a text field (optionally by id; replaces the content by default)",
        "text:s@0 id:s@1? replace:b? submit:b?",
    )
    .since("1.1")
    .result("`{ok, id, value}` - the field's content once the text has arrived")
    .doc("Focuses the field `id` (like a click), clears it when `replace` is true (the default) and types the text as keyboard events, so validation, search-as-you-type and the settings search react exactly as for a person. `submit` presses Enter afterwards. Without `id` the focused field receives the text. `1001` when the field is disabled, hidden or behind a dialog."),
    c(
        "ui.select",
        "ui",
        "Choose an option of a combo box, a tab, a radio or a list row",
        "id:s@0 option:s@1? index:i?",
    )
    .since("1.1")
    .result("`{ok, id, selected}`")
    .doc("For a combo box `option` is the visible text (or `index` the position): the drop-down is opened and the entry activated like clicks. For a tab, radio or list row no option is needed. Disabled targets answer `1001` with the reason."),
    c(
        "ui.toggle",
        "ui",
        "Set a switch, check box or radio to on or off (or flip it)",
        "id:s@0 value:b@1?",
    )
    .since("1.1")
    .result("`{ok, id, checked, changed}`")
    .doc("Only activates the element when its state differs from `value`; without `value` it flips. Settings with a revert prompt open the prompt as for a click."),
    c(
        "ui.hover",
        "ui",
        "Move the pointer over an element (tooltips, hover states)",
        "id:s@0",
    )
    .since("1.1")
    .result("`{ok, id, pos}` with the pointer position in logical pixels")
    .doc("Places the cursor at the centre of the element's rectangle and reports it to the window like a mouse move, so picking, hover states and tooltips behave as for a person. `1001` for an element without a rectangle (not laid out or hidden)."),
    c(
        "ui.key",
        "ui",
        "Press a key with modifiers (Ctrl, Shift, Alt) through the input layer",
        "key:s@0 ctrl:b? shift:b? alt:b? duration_ms:i?",
    )
    .since("1.1")
    .result("`{ok, key, modifiers}`")
    .doc("Presses the modifiers, then the key, and releases everything after `duration_ms` (default 50). The events are ordinary keyboard events, so shortcuts (`Ctrl+F`, `Ctrl+Tab`) and gating are the same as for a keyboard."),
    c(
        "ui.drag",
        "ui",
        "Drag from one element to another element or by an offset",
        "from:s@0 to:s@1? dx:f? dy:f? steps:i?",
    )
    .since("1.1")
    .result("`{ok, from, to?, from_pos, to_pos}` once the button is released")
    .doc("Presses the left button on the centre of `from`, moves the cursor in `steps` frames (default 8) to the centre of `to` or by `(dx, dy)` logical pixels and releases. Answers after the release."),
    c(
        "ui.wait_for",
        "ui",
        "Wait until a condition holds (element, screen, dialog, app state, speed, ...)",
        "until:s[exists|visible|gone|enabled|disabled|focused|checked|text|screen|dialog|no_dialog|app_state|hud_ready|speed_above|speed_below|distance_above|paused|running|sim_time_above|items_above]@0 id:s@1? value:s? number:f? timeout_ms:i?",
    )
    .since("1.1")
    .result("`{ok, until, waited_ms}` on success; error `1005` with `data.last` (the last observed value) when `timeout_ms` (default 10000, at most 120000) passes first")
    .doc("Polled once per frame on the main thread. `exists`, `visible`, `gone`, `enabled`, `disabled`, `focused` and `checked` need `id`; `text` needs `id` and `value` (the field or label contains it); `screen` takes the shell id in `value` (`settings`), `dialog` a dialog tag or id in `value` (`no_dialog` when none is open); `app_state` takes `MainMenu`, `Loading`, `InGame`, ...; `speed_above`/`speed_below` take km/h in `number`, `distance_above` the metres driven since the session started (see `drive.state`), `sim_time_above` seconds of simulation time, `items_above` the number of entries a scrollable `id` lists (a list fills in after the catalogue scan). Use it instead of sleeping."),
    // ---- protocol 1.1: driving --------------------------------------------------------------
];
