# Investigation — Automatic Dark/Light Switching by Time of Day

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Investigated 2026-10-12; implemented in [M-0061](M-0061.md) as the theme mode **Auto (clock)** |
| **Question** | Project lead, 2026-10-12: keep **dark as the default**, or consider switching between dark and light by the day/night cycle of the device's timezone |
| **Outcome** | **Dark stays the default.** A fourth mode, *Auto (clock)*, switches by the device's local time; *System* follows the operating system; both are opt-in |

## 1. Options examined

| Option | How it works | Needs | Verdict |
| :--- | :--- | :--- | :--- |
| **Fixed default** | always dark | nothing | **default** (a simulator for dim rooms; consistent with loading screen and HUD) |
| **System** | follow the OS dark/light preference | Bevy 0.20 exposes it: `Window::window_theme` reports the current `WindowTheme` and `WindowThemeChanged` events arrive when the user changes it (winit; works where the desktop reports a preference, not guaranteed on every Linux desktop) | **offer** as *System*; falls back to the default when the OS reports nothing |
| **Clock, fixed hours** | light during local daytime, dark otherwise, switching at configurable hours | the device's local time and timezone (`chrono`, already a dependency); no network, no location | **offer** as *Auto (clock)*; defaults 07:00 light, 19:00 dark |
| **Clock, sunrise/sunset** | switch at the real local sunrise and sunset | latitude and longitude; the timezone alone gives only a rough longitude and no latitude; the date's day length depends strongly on latitude and flips with the hemisphere | **not now**: it would need a location, which OPTS does not ask for (UI-P11); the existing sky model assumes 50° N for the *game world*, not for the user's room. Revisit only with an explicit, optional location setting |
| **Follow the in-game time** | UI theme tracks the simulated hour of the drive | the session clock | **rejected**: menus appear before a session exists; a theme that flips while driving would break UI-P13 and UI-P06 |

## 2. Behaviour of *Auto (clock)*

- Evaluated from the **local time of the device** (timezone and daylight-saving rules from the operating system) once per minute and at startup; the switch is a normal live theme change (one frame, no restart).
- **Hysteresis and calm**: the theme changes at most once per transition, never while a modal dialog or the loading screen is shown (it waits until the screen ends), never during a drive (the HUD uses its own instrument tokens, and chrome themes are not changed mid-session).
- **Settings**: *Interface → Theme*: `Dark` (default) · `Light` · `System` · `Auto (clock)`; for *Auto* two time fields (*Light from*, *Dark from*) with 07:00/19:00 as defaults and a preview of the current result.
- **Accessibility**: the *High contrast* theme is a separate choice and is never switched automatically.
- **Tests**: the decision function is pure (`fn theme_for(time: NaiveTime, light_from: NaiveTime, dark_from: NaiveTime) -> Theme`), unit-tested for midnight wrap-around, equal times and daylight-saving days; the system integration test uses a fake clock.

## 3. Open points

- Whether to add the optional location for true sunrise/sunset later (needs a privacy-reviewed setting and a small solar-position function; the sky module already contains one for the game world).
- *System* on desktops that report no preference: documented fallback to the default (dark).
