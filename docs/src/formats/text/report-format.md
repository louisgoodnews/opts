# `.report` — F1 bug report files

Written by the F1 reporting feature ([architecture](../../architecture/reporting-feature.md)) next to a screenshot of the same name. OPTS-defined format, not an OMSI format.

| Property | Value |
| :--- | :--- |
| Location | `<data dir>/opts/reports/` — Linux `~/.local/share/opts/reports`, Windows `%APPDATA%\opts\reports`, macOS `~/Library/Application Support/opts/reports`; overridable with `OPTS_REPORTS_DIR` |
| File names | `YYYY-MM-DD_HH-MM-SS.report` and `.png` (local time, sorts lexically); a second report in the same second becomes `…-1`, `…-2`, … ; existing files are never overwritten |
| Encoding | UTF-8 without BOM, `\n` line endings on every platform |
| Screenshot | PNG of the primary window at its current resolution, taken before the modal opened |

## Layout

```
OPTS Report
===========

Timestamp:   2026-10-10T14:30:52+02:00
Version:     0.1.0-alpha
Git:         abc1234
Platform:    linux
Map:         Berlin-Spandau
Camera:      3 External
Vehicle:     MAN_NG272
Focused UI:  TextArea

--- Description ---

The user's text appears here verbatim,
preserving newlines and any UTF-8 characters.
```

- `Timestamp` is RFC 3339 **local** time with offset.
- Every header field is best effort; unknown values are written as `(none)`.
- `Camera` is the key and name of the active camera mode (see [camera system](../../architecture/camera-system.md)).
- The description is the text area content, `\r\n` and `\r` normalised to `\n`, otherwise untouched (never parsed). It ends with one trailing newline.
