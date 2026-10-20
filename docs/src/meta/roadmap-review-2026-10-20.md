# Roadmap Review — 2026-10-20

*Dry run of [P-1010](../../dev/prompts/process/P-1010-roadmap-review.md) for [M-0119](../roadmap/phase-11/M-0119.md). Evidence window: 188 merge commits since 2026-09-20 (187 of them since 2026-10-12), the 80 most recent read one by one (`git log --merges --since=2026-10-12`); milestone status lines of phases 4, 5, 9, 10, 11 and 12 (`grep -m1 '\*\*Status\*\*' docs/src/roadmap/phase-*/M-*.md`); [BOARD](../../dev/BOARD.md); [structure refactoring report](structure-refactoring-2026-10-19.md). Nothing in the roadmap was changed by this review; every item below is a proposal for the project lead.*

## Stale Items

- **M-0119 is 📝 Proposed, but its prompt, index entry, workflow diagram and CONTRIBUTING note exist** — `docs/src/roadmap/phase-11/M-0119.md` vs commit `2c7a03a` — only the dry run was missing; this report is it. **Proposed action:** mark the milestone done after the project lead has read this report (done in this change set as 🟡 until then).
- **M-0062 is ⏳ Planned, but its shell and wizard parts are done** — `docs/src/roadmap/phase-9/M-0062.md` vs BOARD lines "App shell, Home, first run … done 2026-10-12 on `agent/ui-shell`" and "Plan a drive wizard … on `agent/ui-wizard`" — **Proposed action:** status 🟡 In progress with the open parts named.
- **M-0086 says "awaiting merge"** — `phase-9/M-0086.md` vs merge `04fc6a7` (`feat/combobox-search`, M-0086) on master — **Proposed action:** drop "awaiting merge".
- **M-0063 lists view distance as staying unavailable** — `phase-9/M-0063.md` vs merge `7fa91d4` (view distance live, M-0029 part) — **Proposed action:** remove view distance from the open list.
- **Phase 4 page still lists timetables as open** — `phase-4/README.md` status line vs M-0099 "Done 2026-10-19" (merges `9d62853`, `318b2fb`) — **Proposed action:** state what is open: passengers, audio, boarding, bus-stop detection in the world.
- **M-0098 is 🔴 Not started although its predecessor M-0097 is implemented** — `phase-4/M-0098.md` vs merges `c4baba7`, `0a7bae3` — not stale, but it is now the head of the longest dependency chain of Phase 11 (M-0112 → M-0113 need it).
- **The structure ratchet `docs/dev/structure-known.txt` is empty (0 files over 500 lines)** while M-0040 (Phase 5) still says "size splits … open" — **Proposed action:** M-0040 status line re-read against merges `dfc76ee` … `27cc347`.

## Gaps

- **Vehicle assets are not released on despawn or map change** (about 24 images and 29 meshes per spawn, +91 images per map change; BOARD "Vehicle assets are not released on despawn", found by the M-0108 leak check, merge `c933a33`) — no milestone owns it; it limits long sessions that switch vehicles and maps, which M-0068, M-0077 and M-0108 now encourage.
- **Script-driven text is the visible hole in the buses and stops**: BOARD "Scripted scenery (dynamic passenger information) is not functional" (M-0099/M-0100) and "Bus-stop sign fonts are too small" (M-0100, checked 2026-10-20 on Altenheim: not provable as a defect without an OMSI reference) while [M-0100](../roadmap/phase-4/M-0100.md) is 🔴 Not started.
- **No hosted repository, so CI has never run** (M-0052, 🔴, blocked on the project lead's decision, since 2026-10-11): everything is gated by `scripts/local-ci.sh` alone; the Windows and macOS builds are compile-checked only (M-0081).
- **Function-level size is unguarded**: 217 functions over 80 lines ([report](structure-refactoring-2026-10-19.md) §4, up from 214) with no ratchet, unlike the file rule.
- **Open ❓ that recent work touched without a decision**: Q-SV02 (which setvars a script reads after start; blocks a per-option "needs respawn" marker in the new Setvars page, merge `094c766`); the player-versus-`[rule]` question of M-0049; OMSI's text-texture scaling (docs/src/architecture/text-textures.md).
- **Owner column is "Pending Assignment" in every milestone**: the agent workflow picks items by hand each time; the BOARD's In Progress column lists 5 items under a note that says WIP is 4 (target ≤ 1); two of them (M-0047 follow-up, M-0045 screenshot check) have not had a commit message of their own since 2026-10-12.

## Suggestions

- [ ] **Script textures: the `ST*` API, so destination displays and passenger information render** — M-0100 — evidence: the two BOARD items above, the existing `text_texture.rs`/`ScoTextTexture` path, M-0099's route data already delivers the target text to the displays (merge `9d62853`) — scope: large (needs a spec first, ❓ in the architecture page) — depends on M-0096 (done), M-0099 (done) — 🔴
- [ ] **Release vehicle assets on despawn and map change** — new small milestone under M-0089 or Phase 5 — evidence: BOARD leak entry, M-0108 live check — scope: small to medium (track the handles a spawn creates, drop them in `ops::despawn` and at `begin_restore`) — depends on nothing — 🟠
- [ ] **Consist script state: per-part engines, script share, control cables** — M-0098 — evidence: M-0097 done, M-0111/M-0112/M-0113 chain, articulated buses already run (M-0022) — scope: large — depends on M-0096/M-0097 (done) — 🟠
- [ ] **Function-level cycle 2 with a ratchet for the 80-line rule** — Phase 999, new milestone after M-0129 — evidence: 217 functions, the ten named in the report — scope: medium per batch; the first step is the ratchet (small) — 🟡
- [ ] **Decide the hosting question or park CI explicitly** — M-0052 — evidence: blocked since 2026-10-11, every gate runs locally, three platform claims unverified — scope: decision, then small — needs the project lead — 🔴

## Recommendations

1. **Ask for the hosting decision (M-0052)** — a one-line answer unblocks M-0000, M-0042 and every "never run" caveat in the roadmap.
2. **Release vehicle assets on despawn** — small, independent, and every other session feature (configurations, change map, vehicle switching) makes it worse.
3. **M-0100 script textures, starting with its specification** — the largest visible gap in the buses; it also resolves the stop-sign font question with data instead of guesses.
4. **M-0098 consist state**, then M-0111 (AI traffic depth, audit first); both are on the critical path of scheduled traffic (M-0112).
5. **The 80-line ratchet** before the next feature batch grows the 217.
6. Apply the **status corrections** under Stale Items in one docs commit once approved (P-006).

## Notes on the dry run

- The prompt worked as written; two additions were made from this run: an *Evidence recipes* block (the three commands that produced the lists above) and the rule that the review reads the BOARD's In Progress column against the merge log.
- The review itself changed no milestone, BOARD or ROADMAP file; the only other edits of this change set are the P-1010 refinements and M-0119's own page.
- Cost: about one hour of reading; the merge list and the status census are scriptable and are the bulk of it (a candidate for `scripts/` if the review becomes routine).
