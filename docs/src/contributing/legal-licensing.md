# Legal & Licensing

> 🚧 **Not yet written.** Dual MIT/Apache-2.0 licensing, clean-room reverse engineering adherence (ADR-0002), and asset distribution boundaries.

## Bundled fonts (M-0061)

Fonts are compiled into `opts-gui` (`crates/opts-gui/assets/fonts/`, each with its licence text next to it). Only open licences are accepted: SIL Open Font License 1.1 and permissive licences; nothing else may be bundled.

| Font | Files | Use | Licence | Licence file |
| :--- | :--- | :--- | :--- | :--- |
| **Noto Sans** (Noto Project Authors, 2022, v2.015) | `NotoSans-Regular.ttf` (400), `NotoSans-Medium.ttf` (500), `NotoSans-Bold.ttf` (700) | the UI typeface | SIL OFL 1.1 | `LICENSE-OFL-NotoSans.txt` |
| **JetBrains Mono** (JetBrains Mono Project Authors, 2020, v2.304) | `JetBrainsMono-Regular.ttf` | monospaced companion (diagnostics, logs) | SIL OFL 1.1 | `LICENSE-OFL-JetBrainsMono.txt` |
| **DejaVu Sans** (Bitstream, Tavmjong Bah; DejaVu changes public domain) | `DejaVuSans.ttf` | fallback for glyphs Noto Sans lacks (arrows, geometric shapes, check marks) | Bitstream Vera / DejaVu licence | `LICENSE-DejaVu.txt` |
| **Material Icons** (Google) | `MaterialIcons-Regular.ttf` | pictograms | Apache 2.0 | `LICENSE-MaterialIcons.txt` |

The OFL permits bundling and redistribution with the software; the fonts are not sold on their own and keep their names (the OFL's reserved-name rule only applies to modified versions). The files were taken unmodified from the licensed distributions available on the build machine (Noto Sans from the `noto-fonts` package, JetBrains Mono from the JetBrains Runtime). The decision record and the upgrade path to a variable font are in the [design system](../architecture/gui-design-system.md#32-typography).
