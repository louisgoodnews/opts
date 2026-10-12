# Accessibility Audit (WCAG 2.2 AA) — M-0065

| Metadata | Value |
| :--- | :--- |
| **Status** | Automatic part done 2026-10-12; human and hardware parts **open** (see section 5) |
| **Milestone** | [M-0065](M-0065.md) |
| **Method** | automatic rules over the laid-out UI tree, run in CI on self-authored fixture content; no OMSI assets, no GPU, no screen reader, no people |
| **Where** | rules: `opts_gui::testing::rules`; screens: `crates/opts-game/tests/a11y_matrix.rs` and `golden_screens.rs`; library: `crates/opts-gui/tests/ci_rules.rs`, `opts_gui::theme::{audit, cvd}`; languages: `crates/opts-game/tests/localisation.rs` |

> **Honesty note.** "Verified" below means *a test in this repository checks it and passes*. The tests run Bevy's real UI layout without a window, so sizes are the ones a player would see, but **nothing was looked at on a real display, with a real screen reader, a real gamepad or a real steering wheel, and no person took part** (the usability protocol is written but not run: [protocol](../../meta/usability-test-protocol.md)). Every claim outside the automatic tests is listed as open.

## 1. What the rules check

| Rule (`rules::…`) | Checks | Per |
| :--- | :--- | :--- |
| `small_targets` | no focusable element below 32 x 32 logical px (UI-P01) | screen, window size |
| `small_text` | no text below 12 px (design language section 5) | screen |
| `text_contrast` | every visible, enabled text against the colour really behind it (ancestor fills composited, the not-yet label's chip and scrim included): 4.5:1, large text (>= 24 px) 3:1; high contrast 7:1 and 4.5:1; every focusable control border 3:1 (4.5:1); `OutlineVariant` borders are decorative and exempt | screen x 3 themes x 5 accents |
| `text_overflow` | text wider than its container, cut sideways by a clipping container (a tooltip exempts), outside the window, or overlapping other text; texts scrolled out of a scroll container are not "visible" | screen x window size x locale (`en`, `de`, `en-XA`) x UI scale |
| `disabled_without_reason` | a disabled control carries a reason or the not-yet marker | screen |
| `keyboard_traversal` | Tab reaches every interactive element and Confirm activates it | library, screens (existing tests) |
| `directional_traversal` | starting at the first focus and using only the four directions (a gamepad or D-pad has no Tab) reaches every interactive element | 9 screens |
| `unnamed_controls`, `missing_accessibility_nodes` | every interactive element has an accessible name and an `AccessibilityNode` carrying it, in `en` and `de` | 9 screens x 2 languages |
| `theme::audit` (unit tests) | every colour pair of every theme and accent (4,050 pairs) | theme |
| `theme::cvd` (unit tests) | the same pairs after a deuteranopia, protanopia and tritanopia simulation (Machado 2009) stay above the control/large-text threshold | theme |
| `hardcoded_colours`, string-literal source tests | no colour or user-visible string outside the token and key tables | source |
| golden screenshots | the layout, colours and text of 8 scenes x dark/light at 1280x720 (and 4 scenes at 1920x1080) did not change beyond a tolerance | see [goldens](#4-golden-screenshots) |

The nine screens of the matrix are Home, Help, Settings, Plan a drive, the pause overlay, the three first-run dialogs and Home with the session summary. Not in the matrix: the HUD (M-0064, another agent), the loading screen (needs an installation; it has its own rule tests without contrast and overflow), the report dialog, the legacy in-game vehicle menu. They are open items.

## 2. WCAG 2.2 AA checklist

Legend: ✅ verified by an automatic test; 🟡 partly verified or by reading only; ⬜ not verified (needs a human or hardware); — not applicable to a desktop application.

| Criterion | Status | Evidence or gap |
| :--- | :---: | :--- |
| 1.1.1 Non-text content | 🟡 | every icon-only control has a name (rule `unnamed_controls`); decorative images are not part of the tree |
| 1.3.1 Info and relationships | ✅ | roles, names and states come from one place (`ui_id.rs`) into `AccessibilityNode`; tests compare tree and nodes |
| 1.3.4 Orientation, 1.3.5 Identify input purpose | — | desktop window |
| 1.4.1 Use of colour | 🟡 | status uses icon + text + colour by design; checked by reading Home, pause, loading; no automatic rule yet |
| 1.4.3 Contrast (minimum) | ✅ | `text_contrast`, 9 screens x 3 themes x 5 accents, built *and* switched; **text over pictures is not measurable from the tree** (open) |
| 1.4.4 Resize text | ✅ | UI scale 75 %, 100 %, 150 %, 200 % at 1024x640, 1280x720, 1920x1080: no text leaves the window, no overlaps, targets >= 32 logical px (`ui_scale_from_75_to_200_percent_keeps_text_inside_at_three_window_sizes`) |
| 1.4.5 Images of text | 🟡 | none known; text is real text (by reading) |
| 1.4.10 Reflow | — | desktop; large scale scrolls instead (finding A6) |
| 1.4.11 Non-text contrast | ✅ | control borders measured; `OutlineVariant` is decorative and documented as such |
| 1.4.12 Text spacing | ⬜ | no user setting for line height or letter spacing |
| 1.4.13 Content on hover or focus | 🟡 | tooltips appear on focus and hover, never cover the pointed control; *dismissible with Esc* and *hoverable* are not implemented (open) |
| 2.1.1 Keyboard | ✅ | `keyboard_traversal` on library and shell screens, wizard steps, settings categories |
| 2.1.2 No keyboard trap | 🟡 | modals trap focus by design and release on close (library test); other screens have no traps; no test over all screens |
| 2.1.4 Character key shortcuts | 🟡 | `/`, `[`, `]` are shortcuts only outside text entry; no remapping UI yet (M-0063 follow-up) |
| 2.2.1, 2.2.2 Timing, moving content | ✅ | nothing times out; only progress indicators move; reduced motion zeroes transitions (`motion.rs` tests) |
| 2.3.1 Three flashes | ✅ | the only periodic visual change of the widgets is the caret at 1 Hz (`the_caret_flashes_well_below_three_times_a_second`); simulated vehicle indicators are scene content at 1.4 Hz |
| 2.4.1 Bypass blocks | ✅ | regions (rail, content) with Tab (library test); arrows now cross regions (`navigation.rs` tests) |
| 2.4.2 Page titled | 🟡 | every screen shows a title (by reading; no rule) |
| 2.4.3 Focus order | ✅ | layout order; the directional rule reaches the same set |
| 2.4.6 Headings and labels | 🟡 | by reading |
| 2.4.7 Focus visible | 🟡 | the ring is an `Outline` (not painted by the software painter); unit-tested in the library, not in the screenshots |
| 2.4.11 Focus not obscured | ✅ | new: focus scrolls its container (`scroll_into_view`); modals cannot hide focus (trap) |
| 2.5.1, 2.5.2, 2.5.4 Pointer gestures, cancellation, motion actuation | 🟡 | activation on release is a library test; no drag-only or motion input in the menus (by reading) |
| 2.5.7 Dragging movements | 🟡 | none required (by reading) |
| 2.5.8 Target size (minimum 24 px) | ✅ | the project uses 32 px (`small_targets`) |
| 3.1.1 Language of page | ✅ | the language is a setting and can be chosen at first run (`the_language_can_be_chosen_on_every_first_run_step_and_back`) |
| 3.2.1–3.2.4 Predictable, consistent navigation | 🟡 | one shell, one rail, Back/Esc behaviour in `interaction-and-feel.md` section 6 (shell tests, by reading) |
| 3.3.1–3.3.3 Error identification and suggestion | 🟡 | inline errors and banners exist (settings, first run); no automatic rule |
| 3.3.7 Redundant entry, 3.3.8 Authentication | 🟡 / — | the wizard remembers choices (wizard tests); there is no authentication |
| 4.1.2 Name, role, value | ✅ | `unnamed_controls` + `missing_accessibility_nodes`, `en` and `de` |
| 4.1.3 Status messages | 🟡 | toasts map to the *status* role; whether a screen reader announces them is unverified |

## 3. Findings

🔴 blocks a criterion for some users, 🟠 degrades it, 🟡 noted. "Fixed" means the change and a test that fails without it are in this milestone's commits.

| ID | Sev. | Finding | Status |
| :--- | :---: | :--- | :--- |
| A1 | 🟠 | The red *Feature not available yet!* label on a disabled filled control measured 4.47:1 (needs 4.5) because it sits over the primary fill through the disabled scrim | **Fixed**: the label has its own `Surface` chip (`not_yet.rs`) |
| A2 | 🟠 | In the high-contrast theme the elevation tint moved the toast surface off `InverseSurface`; the action text measured 6.89:1 (needs 7:1) | **Fixed**: no tonal tint in high contrast; `InversePrimary` on `InverseSurface` joined the theme audit |
| A3 | 🔴 | A gamepad or wheel D-pad could not leave a region: from Home's content the rail was unreachable, from the rail nothing (arrows stayed in the region, there is no Tab on a pad) | **Fixed**: directional moves cross into another region when it is clearly nearer or the current one has no target (`navigation.rs`); `directional_traversal` passes on 9 screens |
| A4 | 🟠 | The Settings *Vertical sync* switch had no accessible name | **Fixed**: the setting's title is its name and tooltip |
| A5 | 🟠 | Focus could move to a control outside its scrolling container (2.4.11) | **Fixed**: `scroll_focus_into_view` for every scrolling ancestor |
| A6 | 🔴 | At UI scale 150 to 200 % (and in small windows) the rail lost Settings, Help, theme and Quit below the window edge; the settings category list lost *Content* and *Advanced*; long dialogs (pause) were taller than the window; banners could take the whole window and leave the page 0 px high | **Fixed**: rail, content area, banners (max half the window), category list and modals scroll (wheel and focus); rail items no longer shrink |
| A7 | 🟠 | Tooltips were clamped to the window size in window pixels, not UI units, so at 150 % and more they appeared below the window | **Fixed** (`tooltip.rs`) |
| A8 | 🟠 | At the minimum window (1024x640) the *Plan a drive* summary strip clipped its three parts sideways without ellipsis or tooltip | **Fixed**: the strip wraps |
| A9 | 🟡 | Colour-vision simulation: 72 of the 4,050 pairs fall below the *text* threshold (lowest 3.11:1; mostly `Error` on the Light surfaces under deuteranopia and tritanopia, `Primary` on `SurfaceDim` under protanopia, `Warning` on `SurfaceVariant` in Dark under protanopia, `InversePrimary` on `InverseSurface`) | **Open, accepted for now**: the test requires the control/large-text floor (3:1; 4.5:1 in high contrast) because WCAG has no criterion for simulated vision and the matrices are approximations; status is never colour alone (icon + text). A theme revision could lift them |
| A10 | 🟡 | `Select` -> `Help` and `Start` -> `Menu` are in the gamepad table but nothing consumes `UiAction::Help` and `UiAction::Menu` (F1 and Esc open Help and the pause overlay; the pad's B button already opens the pause overlay in a drive) | **Open**: belongs to the Help/pause wiring (agent `ui-wiring`); one line each |
| A11 | 🟡 | Text over pictures (the map hero) cannot be measured from the tree; `scrim_for_text` computes a scrim, but no test renders image plus text | **Open** |
| A12 | 🟡 | Community language packs from the user directory are specified ([expandability section 6](expandability.md#6-other-registries-and-services)) but not loaded: the tables are compiled in; the *Language* setting offers `en` and `de` | **Open**; the process for contributing a language is in [translating](../../reference/translating.md) |

## 4. Golden screenshots

`crates/opts-game/tests/golden/*.png`: Home, Help, Settings (Graphics, Interface), Plan a drive (Where, What, Review) and the pause overlay, dark and light at 1280x720; Home, Settings Graphics, Plan a drive (Where) and pause also at 1920x1080. The content is **self-authored** (invented maps "Fixture Valley", "Test Hills 01", vehicles "Fixture City Bus A", no OMSI names or assets, ADR-0002). 2560x1440 and UI scale 150 % from the specification are not stored (the matrix tests cover scales; sizes beyond 1920x1080 add nothing for a layout painter).

The pictures are **not GPU screenshots**. `opts_gui::testing::golden::paint` paints Bevy's computed layout in software: filled and rounded boxes, borders and the real glyphs (the font atlas Bevy rasterises on the CPU), in stack order and clipped like the renderer. It does not paint images, shadows, the focus ring or gradients. The comparison allows 6/255 per channel on up to 0.2 % of the pixels. A failing test writes `<name>.actual.png` and `<name>.diff.png` next to the goldens (`_failed/`, ignored by git). Regenerate after an intended change: `UPDATE_GOLDEN=1 cargo test -p opts-game --test golden_screens`, then look at the pictures.

## 5. Open items (not done, with owner)

| Item | Owner |
| :--- | :--- |
| Run the usability test ([protocol](../../meta/usability-test-protocol.md)): 5 participants, 6 tasks; acceptance "at least 4 of 5 complete every task without help" is **unmet** until then | project lead (people needed) |
| Screen reader (Orca, NVDA) on a real desktop: the tree and names exist and are tested; announcement is unverified | project lead / accessibility reviewer |
| Gamepad and steering wheel with hardware: the action table and traversal are tested with synthetic input only; wheel button mappings by gilrs differ per device | project lead |
| GPU screenshots at 2560x1440 and the focus ring, shadows and images in the goldens | UI owner |
| HUD, loading screen, report dialog and vehicle menu in the matrix (contrast, overflow, scale) | M-0064 owner / UI owner |
| Text spacing setting (1.4.12), dismissible hoverable tooltips (1.4.13), shortcut remapping (2.1.4) | UI owner |
| A9, A10, A11, A12 above | as listed |
