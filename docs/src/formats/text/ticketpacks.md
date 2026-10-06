# Ticket Packs — Layout, Ticket Meshes and Voice Folders

| Metadata | Value |
| :--- | :--- |
| **Format Family** | Convention / content pack (text `.otp` + meshes + audio) |
| **File Extensions** | `.otp` (56), `.o3d` (470), `.wav` (11,842), `.bmp`/`.jpg` ticket textures |
| **Parser Crate** | `opts-core` |
| **Parser Module** | `opts_core::formats::ticketpack` |
| **Encoding** | CP1252 / UTF-8 (`.otp`) |
| **Status** | 🟡 Draft — layout and vocabulary corpus-verified; event semantics inferred |
| **Last Updated** | 2026-10-04 |

---

## 📖 Overview

A **ticket pack** defines how passengers buy tickets in a map and era: the ticket types and prices (`.otp`), the 3D ticket models shown at the cash desk, and the **voices** passengers speak. The `.otp` grammar (probabilities, `[ticket]`, `[ticket_2]`, `[voicepath]`) is in [misc types](misc-types.md#otp); this page specifies the **pack as a whole**. Evidence tags: **[C]** counted over the corpus, **[H]** seen in files, 📖 shipped manual, 📚 community wiki, ❓ inferred. No behaviour was tested.

Packs are selected by the map (`[ticketpack]` in [`global.cfg`](../map/global-cfg.md)) or by a chrono entry ([timetables](../map/timetables.md)), which is how prices change over the simulated years 📖.

## 🔤 General Structure

### Layout of `TicketPacks/` [C]

40 pack folders, 56 `.otp` files, 11,842 voice WAVs (4.2 GB). A pack folder contains:

```
TicketPacks/<Pack>/
  <Pack>.otp                 the ticket definitions (several per pack for era variants)
  Ticket_<n>_single.o3d      mesh of ticket type n as a single ticket
  Ticket_<n>_block.o3d       mesh of ticket type n as a ticket block (pad)
  Fahrscheine_*.bmp|jpg      ticket textures (36 each in the install)
  <VoiceId>/                 voice folders (only in packs that carry their own voices)
```

- **Several `.otp` per pack:** `HB_76_Bremen` has 15 (one per era), `Hamburg_109` and `VRR 2025_AL` two each; most packs have one.
- **Ticket meshes follow the ticket list:** in 47 of 54 pack/`.otp` pairs the number of tickets (`[ticket]` + `[ticket_2]` blocks, median 5, maximum 32) equals the highest mesh index + 1, so **mesh index *n* = the *n*-th ticket type** (0-based) ❓; in 7 pairs some tickets have no mesh (e.g. 4 tickets, meshes up to 6). Each type has a `_single` and a `_block` variant (the two forms of the ticket the driver tears off or prints — manual 📖: ticket pads before 1993, printers after).
- **`[voicepath]`** (present in 29 of 56 `.otp`) names a folder **relative to the OMSI root** (`TicketPacks\Berlin_1\`); **all 29 resolve**. Several packs reuse another pack's voices (e.g. `Aachen33`, `Berlin_91`, `Berlin_93` and `Berlin_94` all point to `Berlin_1`), so voices are shared content, not tied to one `.otp`. The remaining 27 `.otp` omit `[voicepath]` ❓ (voices then presumably come from a default or from the pack's own folder).
- **`.mp3` files** (a handful) sit among the WAVs of some voice folders; see [wav](../wav.md) for codec handling.

### Voice folders

Inside a voice location there is **one folder per voice id**: `F1`–`F4`, `FY1`, `FO1`, `M1`–`M6`, `MY1` (the full set exists in 31 packs; `M5`/`M6` in 3 and 2). Reading *F* female, *M* male, *Y* young, *O* old is natural ❓. Each folder holds about 1,050 WAVs. A human definition selects its voice with `[voice]` ([misc types](misc-types.md#hum)): the observed values are exactly these ids — `M2` ×13, `M4` ×9, `M3` ×7, `F3` ×5, `F1` ×4, `M1` ×3, `F4` ×3, `M5` ×3, `M6` ×3, `F2` ×2, `MY1` ×1, `FY1` ×1 ✅. So **`[voice] <id>` → `<voicepath>/<id>/`** ❓.

### Utterance vocabulary

All voice files share a small vocabulary of name patterns (digits normalised to `#`); **76 distinct patterns** occur in the corpus. The 24 that account for almost everything [C]:

| Pattern | Files | Probable situation ❓ |
| :--- | ---: | :--- |
| `ticket_<n>_<m>` | 2,169 | passenger asks for ticket type *n*, spoken variant *m* |
| `pardon_<n>` | 989 | asks the driver to repeat / "pardon" |
| `pardonticket_<n>` (+ misspelling `padonticket_<n>`) | 341 (+5) | asks again for the ticket |
| `hello_<n>` (+ `hallo_<n>`) | 700 (+3) | greeting on boarding |
| `goodmorning_<n>` / `goodevening_<n>` | 347 each | time-of-day greeting |
| `thanks_<n>` / `thanks` | 343 / 73 | thanks after receiving the ticket |
| `toodark_<n>`, `toocold_<n>`, `toohot_<n>`, `toolate_<n>` | ≈ 690 each | complaints: too dark, cold, hot, late |
| `toowet_<n>` | 343 | complaint: wet/rain (one `toodry_<n>`) |
| `toobad_a`, `toobad_b`, `toobad_c` | 343 each | complaints about rough driving |
| `badchange_<n>` | 329 | wrong change |
| `badticket_a`, `badticket_b` | 322 each | wrong ticket |
| `toofew_a`, `toofew_b` | 322 each | too little money/change |
| `toomuch_a`, `toomuch_b` | 321 / 320 | too much money/change |

Rarer tails: `*_alt` / `*_alt<digit>` alternates (`toolate_#_alt`, `toobad_a_alt`, …), a few extra letters (`toobad_d/e`, `badticket_c/d`, `toofew_c/d`, `toomuch_c/d`), and German one-offs (`gruppenticket_<n>`, `nachtticket_<n>`, `fragezeichen_<n>`, `w_guten_abend`, `ohne_nachtticket`). The groups line up with the four probabilities of `[ticketpack]` (stamping, buying, **talking** → greetings/thanks, **complaining** → `too*`, `bad*`) ([misc types](misc-types.md#otp)). How OMSI picks `<n>` and `a/b/c` ❓.

## 📚 Field Reference

See the layout and vocabulary above; the `.otp` keyword fields (`[ticketpack]` 4 values, `[ticket]` 7, `[ticket_2]` 9, `[voicepath]` 1) are in [misc types](misc-types.md#otp). 📚 The wiki adds that for day tickets the purchase probability rises linearly between 00:00 and 09:00.

## 🧪 Examples

```
TicketPacks\Berlin_1\Berlin_1.otp           [voicepath] TicketPacks\Berlin_1\
TicketPacks\Berlin_1\Ticket_0_single.o3d    ticket type 0, single
TicketPacks\Berlin_1\Ticket_0_block.o3d     ticket type 0, pad
TicketPacks\Berlin_1\M2\hello_1.wav         voice id M2, greeting
```

## ❓ Open Questions

| ID | Question | Priority |
| :--- | :--- | :--- |
| Q-TP01 | How are the numbers (`<n>`, `<m>`) and letters (`a/b/c`) in voice names chosen — random, by passenger age, by ticket index? | 🟡 Medium |
| Q-TP02 | What happens for the 27 `.otp` without `[voicepath]`? | 🟡 Medium |
| Q-TP03 | Do `.alt` voice files replace or supplement the standard ones? | 🟢 Low |
| Q-TP04 | Tickets without meshes (7 pairs): is the mesh optional? | 🟢 Low |

## 📝 Provenance

- **Corpus (2026-10-04):** layout census of all 40 packs; keyword census and ticket count of all 56 `.otp`; `[voicepath]` resolution (29/29); mesh-index vs ticket-count comparison (54 pairs); normalised file-name census of all voice folders (76 patterns); `[voice]` values of all `.hum`.
- **Shipped manual (📖), community wiki (📚):** ticket pads vs printers; probabilities and day-ticket curve. Paraphrased.

**Clean-room attestation (ADR-0002):**
- No OMSI binaries were inspected.
- No GPL source code was read.
- No OMSI assets were committed.

## 🔗 Related Documents

- [misc types (`.otp`, `.hum`)](misc-types.md#otp), [money](money.md), [`global.cfg`](../map/global-cfg.md), [wav](../wav.md), [predefined variables](vehicles/predefined-variables.md) (`GetTicketName`, `GetTicketValue`)
