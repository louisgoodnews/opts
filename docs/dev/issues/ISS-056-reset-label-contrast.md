# ISS-056: The Reset label is nearly invisible on the light theme

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-056. Owner report 2026-11-22, with screenshots (light theme, Einstellungen, Grafik page). Category QoL with an accessibility angle |
| **Severity** | minor (the owner did not rate it; minor is proposed here and can be raised) |
| **Milestone** | [M-0065](../../src/roadmap/phase-9/M-0065.md) Accessibility, Localisation and Validation (scope addendum 2026-11-22) |
| **Status** | open; a scope addendum is recorded in M-0065. M-0065 keeps its status |

## Problem

On the light theme, the *Zurücksetzen* (Reset) button on the right of each stored setting is nearly invisible: light grey on white. The button is disabled while the setting is at its default, and the disabled style is what makes it hard to read. The owner reads it as "no change to reset", which is the intent, but the label fails contrast.

## Evidence

- The string is `settings-reset` in `crates/opts-game/src/settings_registry/messages_de.ftl` (line 196; `messages_en.ftl` line 197 holds `Reset`). The disabled reason is `settings-reset-disabled` (line 197 of the German file).
- The button is built in `spawn_reset` in `crates/opts-game/src/settings_screen/rows/texts.rs` (lines 52 to 75): a text button with `with_disabled(at_default)`. The sync after a change is `sync_reset` in `crates/opts-game/src/settings_screen/page/refresh.rs` (lines 117 to 133).
- A text button has a `Surface` background and a `Primary` label (`crates/opts-gui/src/components/button/style.rs`, the `ButtonVariant::Text` arm).
- The disabled treatment is a scrim: `crates/opts-gui/src/interaction/layer/update.rs` (lines 85 to 92) draws a `Surface` layer at alpha `1 - DISABLED_CONTENT` over the control, where `DISABLED_CONTENT = 0.38` (`crates/opts-gui/src/interaction/state.rs`, line 104). The scrim is drawn over the label, so the label is pulled toward the background colour.

Estimate (not measured): with the scrim, the disabled label colour is `0.62 x Surface + 0.38 x Primary` against `Surface`, blended in sRGB. For the light palette (`Surface` #FDF8FD, `crates/opts-gui/src/theme/palette.rs`), this gives about 2.7:1 even when `Primary` is black, and about 1.7 to 1.8:1 for mid-blue or grey primaries. The exact accent value is set by `crates/opts-gui/src/theme/accent.rs`. The enabled label is not affected by the scrim.

## Why the audit does not catch it

The WCAG 2.2 audit in [M-0065](../../src/roadmap/phase-9/accessibility-audit.md) runs `text_contrast` on "every visible, enabled text" (its rule table, line 18). The disabled reset label is therefore never measured. WCAG 1.4.3 exempts inactive components, but the project's bar for text is 4.5:1 on each theme and 7:1 in high contrast, as the audit's own rule states. The design language sets the same themes table ([design language](../../src/roadmap/phase-9/design-language.md), line 44).

## Fix direction

Keep the disabled look, but do not let the scrim reduce the reset label below the audit's floor on either theme. Options, for the owner or the UI owner to choose: a dedicated disabled-text role that meets 4.5:1 on light and dark; or a disabled style that dims the container rather than the label. Extend the audit so that disabled labels of reset buttons are measured on both themes.

## Done when

- The disabled and enabled reset labels meet 4.5:1 on the light and the dark theme (and 7:1 in the high-contrast theme, as the audit requires).
- The audit measures the disabled reset label on each theme, and the result is in the audit table.
