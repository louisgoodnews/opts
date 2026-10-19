# 📄 Text Formats

OMSI's configuration formats are plain text. Most use the same keyword grammar: a line that is exactly `[keyword]` starts a directive, the following lines are its parameters, and everything else (banners, indented lines, lines starting with an apostrophe) is a comment. Encodings vary per file — CP1252, UTF-8 (with or without BOM) and UTF-16LE all occur — so detect per file.

| Group | Specs |
| :--- | :--- |
| Objects and splines | [`.sco`](sco.md), [`.sli`](sli.md), [audit](sco-sli-audit.md), [texture property files](texture-cfg.md) |
| Vehicles | [`.bus`](vehicles/bus.md), [`.cfg`](cfg.md), [`model.cfg`](vehicles/model_cfg.md), [paths](vehicles/paths_cfg.md), [passenger cabin](vehicles/passengercabin_cfg.md), [sound](vehicles/sound_cfg.md) |
| Transit data | [`.hof`](vehicles/hof.md), [timetables](timetable.md), [`.org`](org.md) |
| Scripts | [`.osc`](vehicles/osc.md), [variable lists](vehicles/varnamelist.md), [constant files](vehicles/const.md), [audit](vehicles/vehicle-corpus-audit.md) |
| Everything else | [misc. types](misc-types.md) |
| Map files | [map & scenario files](../map/global-cfg.md) |

See the [formats overview](../overview.md) for the evidence markers used in every spec.
