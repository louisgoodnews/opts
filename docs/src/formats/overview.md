# 📄 OMSI Formats — Overview

This section specifies the OMSI 2 file formats OPTS needs to read. Every page follows the same template (metadata table, overview, structure, field reference, examples, **Open Questions**, **Provenance**, related documents) and every page is currently a 🟡 **Draft**.

## How to read a spec

| Marker | Meaning |
| :--- | :--- |
| **[C]** | counted or walked over the whole installed corpus |
| **[S]** | statistics over a random sample |
| **[H]** | seen in individual files (hex or text dump) |
| 📚 | taken from the public community wiki, restated in our own words (see [Community Cross-Check](community-crosscheck.md)) |
| ❓ | hypothesis or unverified; also listed under the page's Open Questions |

Specs are derived from black-box observation of the files and from public documentation, under [ADR-0002](../adr/0002-clean-room-policy.md). **No mutation tests in OMSI have been run yet**, so *what a value does* is weaker evidence than *how the bytes are laid out*.

## Index

### Binary and mesh

| Spec | Covers |
| :--- | :--- |
| [`.o3d`](binary/o3d.md), [chunk reference](binary/o3d-chunks.md) | the native mesh format |
| [`.x`](binary/x.md) | legacy DirectX meshes |
| [Textures](textures.md) | `.dds`, `.bmp`, `.tga`, `.png`, `.jpg` |

### Text — objects and splines

| Spec | Covers |
| :--- | :--- |
| [`.sco`](text/sco.md), [`.sli`](text/sli.md), [corpus audit](text/sco-sli-audit.md) | scenery objects and splines |
| [Texture property files](text/texture-cfg.md) | `*.bmp.cfg` surface properties |

### Text — vehicles and scripts

| Spec | Covers |
| :--- | :--- |
| [`.bus`](text/vehicles/bus.md), [`.cfg`](text/cfg.md) | vehicle definitions |
| [`model.cfg`](text/vehicles/model_cfg.md), [paths](text/vehicles/paths_cfg.md), [passenger cabin](text/vehicles/passengercabin_cfg.md), [sound](text/vehicles/sound_cfg.md) | vehicle sub-configurations |
| [`.hof`](text/vehicles/hof.md) | destinations, stops, routes |
| [`.osc`](text/vehicles/osc.md), [variable lists](text/vehicles/varnamelist.md), [constant files](text/vehicles/const.md) | scripts |
| [Predefined variables & callbacks](text/vehicles/predefined-variables.md), [script sets](text/vehicles/script-sets.md), [scenery scripts](text/scenery-scripts.md) | the engine-side script vocabulary and how real vehicles/scenery organise their scripts |
| [Repaints](text/repaints.md) | `[CTC]`, `[CTCTexture]`, `.cti` item lists, `.rpc` templates |
| [Environment](text/environment.md), [engine config](text/engine-config.md), [`.opl`](text/opl.md) | sun/sky, weather, scenarios and previews; key bindings, controllers, options; plug-in variable declarations |
| [Ticket packs](text/ticketpacks.md), [money](text/money.md), [fonts](text/fonts.md) | passenger content: pack layout and voices, currencies, the font registry |
| [Vehicle-side corpus audit](text/vehicles/vehicle-corpus-audit.md) | checks of the above against the whole install |
| [`.org`](text/org.md), [misc. types](text/misc-types.md) | number lists, `.ovh`, `.hum`, `.cti`, `.oft`, `.otp`, … |

### Maps and scenarios

| Spec | Covers |
| :--- | :--- |
| [Map tiles](map/tiles.md) | `.map`, `.terrain`, `.water`, light maps |
| [`global.cfg` and friends](map/global-cfg.md) | map manifest, saved situations |
| [Lists and calendars](map/lists.md) | AI lists, traffic density, holidays, signal routes |
| [Timetables](map/timetables.md) | chrono, `.ttl`, `.ttp`, `.ttr`, stops, links |

### Audio and other data

| Spec | Covers |
| :--- | :--- |
| [`.wav`](wav.md) | sound files: variants, chunks, robustness findings, config references |
| [Inventory](inventory.md) | every extension in the install with coverage |
| [`.dat`](dat.md), [`.xsli`](xsli.md), [`.itx`](itx.md), [`.ipr`](ipr.md), [`.kyb`](kyb.md), [`.kennz`](kennz.md), [`.html`](html.md), [`.off`](off.md), [`.sichtbar`](sichtbar.md) | small stubs from the inventory pass |

### Meta

| Page | Purpose |
| :--- | :--- |
| [Community Cross-Check](community-crosscheck.md) | what the public wiki adds, confirms or contradicts |

## Not yet specified

Audio decoding details beyond the [`.wav` analysis](wav.md), the `.dll` plug-in interface (deliberately not analysed; only the text [`.opl`](text/opl.md) declaration is specified), and the coordinate-system document, which is blocked on mutation tests (see the [roadmap](../roadmap/README.md)).
