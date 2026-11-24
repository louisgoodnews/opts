# Help controls table: screenshot check (M-0164, ISS-015), 2026-11-24

Build and run: master `26980ff6`, `opts-game` debug build, windowed, no map (main menu, Help opened with `ui.navigate help`), control protocol on loopback. Screenshots are kept outside the repository (scratch folder `ui-help`), not committed.

## Captures

| Size | Screenshot file (scratch) | Content |
| :--- | :--- | :--- |
| 1920x1080 (window 1920x1013 physical) | `help-desktop-1920x1080-2026-11-24.png` | Top of the Controls list (vehicle systems, driving with mouse, bus driving) |
| 1920x1080 | `help-desktop-1920x1080-p1-2026-11-24.png` | Bus rows end, tram/train driving, camera, mini map, menus and tools |
| 1920x1080 | `help-desktop-1920x1080-p2-2026-11-24.png` | Bottom of the list (last row reached) |
| 1024x640 (minimum window size) | `help-phone-1024x640-p1-2026-11-24.png`, `...-p2-...png` | Same list at the small size |

## Checks

| # | Check | Result |
| :-- | :--- | :--- |
| 1 | Help screen opens (`ui.navigate help`; screen reported as `help`) | PASS |
| 2 | Driving (buses) rows present: throttle and brake, steer incl. arrow alias, parking brake, refuel, drive/neutral/reverse | PASS |
| 3 | Driving (trams and trains) rows present: direction, power lever, zero, points, other cab, arrow alias for route choice | PASS |
| 4 | Panels rows present: mini map F7 / Shift+F7 / Ctrl+F7 chip, traffic lanes F11 / Shift+F11, F10 / Shift+F10 chip, Shift+F12 last row | PASS |
| 5 | Camera rows present (1-5, 0, Shift+Space, driver's view Tab) | PASS |
| 6 | No clipped chips at 1920x1080 (all chip text fully inside its chip) | PASS |
| 7 | Every row readable at 1920x1080, last row reached by scrolling | PASS |
| 8 | Key chip column aligned at 1920x1080: the driver's view (Tab) row is wider than the others and its description starts about 38 px left of the description column used by all other rows | FAIL |
| 9 | Key chip column aligned at 1024x640: chips are sized to their text, so the description column starts at a different x in almost every row (about 6 different start positions in one screen) | FAIL |
| 10 | No clipped chips and no wrapped chip text at 1024x640: the mini-map row chip (F7 / Shift+F7 / Ctrl+F7) wraps onto two lines inside its chip | FAIL (wrapping; text not cut) |
| 11 | Every row readable at 1024x640 with no horizontal page scroll | PASS |

## Defects (not fixed in this run)

1. 1920x1080, Camera section, driver's view row (Tab): the chip is wider than the other chips and the description starts at a smaller x than the rest of the column; the description wraps onto two lines.
2. 1024x640, all sections: chip width follows the text, so the description column does not line up; the row for the driving rows with the longest chips pushes its description right. The layout has no fixed chip column width.
3. 1024x640, Mini map section, first row: the chip label wraps onto two lines.

## Not checked in this run

- The F1 key itself (the screen was opened with `ui.navigate help`, not the F1 key).
- Reverse test and chip-vs-chord test (code-level, separate from the screenshot).
