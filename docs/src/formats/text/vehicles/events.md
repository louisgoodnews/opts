# OMSI Events and Triggers: Taxonomy, Names and Ordering

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Research draft (2026-11-16, M-0118 Phase A/B). Corpus census run; ordering facts partly from code; nothing verified against a running OMSI 2 |
| **Milestone** | [M-0118](../../../roadmap/phase-11/M-0118.md) |
| **Applies to** | `.osc` scripts ([`.osc`](osc.md)), `sound.cfg` ([`sound_cfg`](sound_cfg.md)), `keyboard.cfg` and `gamectrler.cfg` ([key bindings](../../../architecture/key-bindings.md)), `[mouseevent]` in `model.cfg` ([`model_cfg`](model_cfg.md)) and in `.sco` map objects ([`.sco`](../sco.md)) |
| **Used by** | [Event system design](../../../architecture/event-system.md), [callbacks](callbacks.md), [bus script integration](../../../architecture/bus-script-integration.md) |
| **Evidence marks** | ✅ **measured**: counted by the corpus census, or read in OPTS code (source given). 📚 **documented**: stated on an existing OPTS page, cited. ❓ **unverified**: inference, or not sourced at all |

> **Clean-room attestation (ADR-0002):** no OMSI executable was inspected. All counts come from shipped text files (`.osc`, `.cfg`) read as data. The name tables list **identifiers only**; no script text, sound file, texture or other asset is quoted or committed. The census script is kept outside the repository (scratch) because M-0118 is a documents milestone.

---

## 1. Summary

1. OMSI has **two directions**: **triggers** go into a script (`{trigger:name}`), **events** come out of it (`(T.L.name)`, `(T.F.name)`, `(T.S.name)`). Sounds and announcements are the consumers of events; keys, mouse hot-spots and the engine are the sources of triggers. ✅ (census, code) 📚 ([`.osc`](osc.md), [`osc-guide`](osc-guide.md))
2. Triggers and events share one **name table** in the VM, matched case-insensitively. A trigger and an event of the same name therefore carry the same identifier. ✅ code (`crates/opts-sim/src/script/run.rs` `trigger`, `names.events`) 📚 ([`osc-guide`](osc-guide.md))
3. **Events never run triggers in OPTS today.** `(T.L.name)` only queues an event. Whether OMSI runs a same-named `{trigger:name}` is ❓ (Q-OSC-007; 2 names in the corpus overlap, so the question matters little for the measured data).
4. The corpus uses **2,194 distinct `(T.L.…)` names** in 64,669 call sites, **15 `(T.F.…)` payload names** in 709 sites and **one `(T.S.…)` name** in 6 sites (✅ census). The 2026-11-16 scratch census found no event name built at run time (0 non-literal names). The committed script, re-run on 2026-11-24, does not reproduce that: it finds 1,043 hyphenated accessors it cannot classify as literal (38 distinct names); see section 6.4.
5. **98.1 %** of the 73,521 `sound.cfg` `[trigger]` entries name an event that some script of the corpus raises as `(T.L)` (✅ census, all vehicles and add-ons; `(T.F)` payload events are not counted here). The earlier research report measured 90.3 % over the player buses only (📚 [report §4.5](../../../meta/bus-script-integration-2026-10-14.md)); the difference is scope, not a correction.
6. **About 1,900 trigger names have no visible binding**: no script raises them, no key in `keyboard.cfg` or `gamectrler.cfg` names them, no mouse hot-spot in the scanned models carries them, and no sound binds them. Of these, the names OPTS already treats as engine-raised are `ai_scheduled_settarget`, `collision`, `malfunction_reset`, `malfunction_gettime`, `veh_tank` and `veh_wash` (📚 [OSC runtime](../../../architecture/osc-runtime.md), [OSC guide](osc-guide.md)). Their **raise rule is unverified** (❓, see section 7).
7. **Ordering is partly known**: triggers run as their own top-level runs between steps, in the order the host fires them; a same-named trigger block runs in file order; events are delivered after the step that raised them, in raise order. **Priority does not exist** in the data or in OPTS code, and OMSI's own priority rule is unknown (❓).

---

## 2. Method

| Step | What | Evidence |
| :--- | :--- | :--- |
| Corpus | `Vehicles/` and `Addons/` of the installed OMSI 2 (4,918 `.osc`, 1,853 `model*.cfg`, 1,035 `sound*.cfg`), plus `Inputs/keyboard.cfg`, `gamectrler.cfg`, `keyboard_reset.cfg` | ✅ census |
| Event call sites | Literal `(T.L.name)`, `(T.F.name)`, `(T.S.name)` matched case-insensitively; any accessor whose name is not a literal (contains `$`, a bracket, a space or is empty) would be counted as dynamic | ✅ census 2026-11-16 (0 dynamic); committed script 2026-11-24 counts 1,043 non-literal, all hyphenated (section 6.4) |
| Trigger blocks | `{trigger:name}` blocks in every `.osc` (51,703 blocks, 5,705 distinct names) | ✅ census |
| Sound triggers | the line after `[trigger]` in every `sound*.cfg` (73,521 entries) | ✅ census |
| Keys | the line after `[entry]` in `keyboard.cfg`, `gamectrler.cfg`, `keyboard_reset.cfg`: 129 distinct names | ✅ census (same count as [report §4.6](../../../meta/bus-script-integration-2026-10-14.md)) |
| Mouse | the first non-blank line after `[mouseevent]` in every `model*.cfg`: 2,919 distinct mesh-trigger names | ✅ census (names only; the `_drag` and `_off` variants are derived, not counted separately) |
| Reference vehicles | `MAN_NL_NG` (NG272), `MAN_SD200` (SD200), `HH109_Stadtbus_HHA` (HH109; the HH109 family has more than one folder, this one was chosen because it has a complete script set ❓) | ✅ census |

**Derived names.** A key `k` produces triggers `k` and `k_off` (`_off` only for held bindings, 📚 [key bindings](../../../architecture/key-bindings.md)). A mesh `m` produces `m`, `m_off` and `m_drag` (📚 [OSC guide](osc-guide.md)). The residual below removes these derived names; the derivation is approximate, because a mesh name in one model file may be bound by another file of the same vehicle.

**Limits of the census.** The census reads text as data. It cannot tell whether OMSI keeps a name it never reads, nor whether an engine-raised name exists in the binary. Every "unbound" count below is therefore a candidate list, not a finding.

---

## 3. Taxonomy

### 3.1 Sources and sinks

| Kind | Direction | Syntax | Raised or read by | Evidence |
| :--- | :--- | :--- | :--- | :--- |
| Key trigger | in | `{trigger:name}`, `name_off` on release | `keyboard.cfg` `[entry]` (`[vehicles]` section) | ✅ census (129 names); 📚 [key bindings](../../../architecture/key-bindings.md) |
| Mouse trigger | in | `{trigger:mesh}`, `mesh_drag` (with `mouse_x`, `mouse_y`), `mesh_off` | `[mouseevent]` in `model.cfg`, and in `.sco` map objects (`SceneryMouseTrigger`, 60 m) | ✅ census (2,919 names); 📚 [`.sco`](../sco.md), [OSC guide](osc-guide.md) |
| Engine trigger | in | `{trigger:name}` | the host: `ai_scheduled_settarget`, `collision`, `malfunction_*`, `veh_tank`, `veh_wash`, `parking_brake_*` (names only; see section 7) | ✅ census (residual); 📚 [OSC guide](osc-guide.md) for the first four |
| Protocol trigger | in | `{trigger:name}` | the control protocol (`vehicle_remote::raise_trigger`, `control_drive`) | ✅ code (`crates/opts-bevy/src/vehicle_remote/triggers.rs`) |
| Local event | out | `(T.L.name)` | script; consumed by `sound.cfg` `[trigger]` and by the sound layer | ✅ census (64,669 sites); ✅ code (`EventKind::Local`) |
| Front event | out | `(T.F.name)` with the top string as payload | script; announcements (`ev_IBIS_Ansagen` and similar) | ✅ census (709 sites, 15 names); ✅ code (`EventKind::Front`); 📚 [OSC](osc.md) |
| Sound event | out | `(T.S.name)` | script; meaning unverified | ✅ census (6 uses, one name, `smetak`); ✅ code (`EventKind::Sound`); ❓ meaning |
| Script-raised trigger | in | `(T.L.name)` runs `{trigger:name}` | ❓ (Q-OSC-007). OPTS does not do it | 📚 [OSC runtime](../../../architecture/osc-runtime.md), [OSC](osc.md) |

### 3.2 Base-engine versus content-added (classification)

The data does not say which names the **base game** raises and which an **add-on** defines. The classification below is therefore by evidence pattern, not by origin:

| Class | Rule (all counted over the corpus) | Count of names | Status |
| :--- | :--- | ---: | :--- |
| **Content-added, consumed by sound** | raised by some `(T.L.name)` and bound by some `sound.cfg` `[trigger]` | 1,558 | ✅ census; origin of the raising script set is not separated |
| **Content-added, not consumed by sound** | raised by a `(T.L.name)`, no sound binds it | 636 | ✅ census; these events have no sound consumer in the data |
| **Payload (announcement) event** | raised as `(T.F)` or `(T.S)` | 16 | ✅ census |
| **Sound trigger never raised as `(T.L)`** | bound by `sound.cfg`, no `(T.L)` in the corpus (some are raised as `(T.F)` payloads, e.g. `ev_ibis_ansagen`) | 78 (1,369 entries) | ✅ census; candidates for engine-raised events, e.g. `ev_ticketsale_takemoney`, `ev_stamper`, `ev_matrix_lang` |
| **Engine-raised candidate** | trigger block with no script raise, key or mouse binding, and no sound binding | 1,905 (8,713 blocks) | ✅ census (residual); the raise rule is ❓ |
| **Base-engine (confirmed)** | — | 0 | ❓ no name can be shown to come from the base game alone with the data at hand |

The reference-vehicle table in the appendix gives the class of each name per vehicle.

---

## 4. Parameter formats

| Event or trigger | Parameter | Format | Evidence |
| :--- | :--- | :--- | :--- |
| `(T.L.name)` | none | the name only; the VM keeps no payload | ✅ code (`ScriptEvent { kind, name }`, no payload field) |
| `(T.F.name)` | payload | the top string of the string stack (a `.wav` path relative to the vehicle, resolved case-insensitively); the GN92 builds `..\..\Announcements\…` | ✅ census (709 sites); 📚 [OSC](osc.md) Q-OSC-008. **OPTS does not carry it:** `ScriptEvent` has no payload field and the Bevy layer keeps the name only (✅ code, `crates/opts-sim/src/script/mod.rs`, `crates/opts-bevy/src/vehicle_script/system.rs`) |
| `(T.S.name)` | payload | ❓ (6 uses, one name) | ✅ census |
| key trigger | none | pressed: `name`; released (held bindings only): `name_off` | 📚 [OSC guide](osc-guide.md); ✅ code (`vehicle_triggers/keys.rs`) |
| mouse trigger | position | `mouse_x`, `mouse_y` during `_drag` | 📚 [OSC](osc.md) |
| `ai_scheduled_settarget` | inputs | reads the float `AI_Target_Index` and the string `SetLineTo` | 📚 [callbacks](callbacks.md) |
| `[trigger]` in `sound.cfg` | — | one event name per line; several lines may follow one `[sound]` | ✅ census; 📚 [sound_cfg](sound_cfg.md) |
| engine triggers in general | — | ❓ no parameter format is documented for `collision`, `malfunction_*`, `veh_tank`, `veh_wash`, `parking_brake_*` | ❓ |

---

## 5. Ordering and dispatch

### 5.1 Facts with their sources

| # | Fact | Mark | Source |
| :--- | :--- | :--- | :--- |
| O1 | A trigger runs as its own top-level run with an empty stack and a zero register file; it runs between steps, in the order the host fires it, and runs to completion (no preemption) | 📚 ✅ code | [OSC runtime](../../../architecture/osc-runtime.md); `crates/opts-sim/src/script/run.rs` `trigger` |
| O2 | All blocks of one trigger name run, in file order | 📚 ✅ code | [OSC guide](osc-guide.md); `compiler.rs` `close` (cited there) |
| O3 | Events raised during a run are queued and handed to the host **after** the step (`drain_events`), in raise order. The OSC runtime design says they carry the part id and payload (📚); the code's `ScriptEvent` carries the kind and the name only (✅ code, so the payload and part are not delivered today) | ✅ code | `crates/opts-bevy/src/vehicle_script/system.rs` (drain after `run_frame`); `crates/opts-sim/src/script/mod.rs` |
| O4 | The vehicle path keeps **at most 256 events per step** and drops the rest without a log line | ✅ code (**finding**) | `crates/opts-bevy/src/vehicle_script/system.rs:80-86` (`if raised.len() < 256`); the same cap in `consist_scripts.rs:80` |
| O5 | The player sound layer takes the raised names in order; for each name **every** voice with a matching `[trigger]` starts (no exclusivity, no priority), gated by playback mode, viewpoint and `[conditionSingle]` | ✅ code | `crates/opts-game/src/player_sound/triggers.rs` `fire_player_triggers` |
| O6 | The sound layer keeps the last 64 raised events for the diagnostics, and drops older ones | ✅ code | same file (`stats.events`) |
| O7 | Key triggers are raised in binding order, once per frame, on exact modifiers (Shift and Ctrl must match exactly, Alt never matches) | ✅ code | `crates/opts-bevy/src/vehicle_triggers/keys.rs` (`just_pressed`, `just_released`) |
| O8 | A trigger name no block answers returns "not handled"; the caller logs it at debug level and nothing else happens | ✅ code | `crates/opts-bevy/src/vehicle_triggers.rs` `raise`; `crates/opts-sim/src/script/run.rs` `trigger` returns `false` |
| O9 | An AI vehicle keeps at most **8** pending events per frame; the next raise is dropped without a log | ✅ code (**finding**) | `crates/opts-game/src/vehicle_sound/events.rs` `AiEvents::raise` |
| O10 | Key triggers reach the head part only; hot-spot clicks reach the part whose mesh was hit; engine triggers reach the head and every part that defines them | 📚 ✅ code | [OSC runtime](../../../architecture/osc-runtime.md) "Trigger routing"; `MouseTrigger.owner` |
| O11 | **Priority** among events or triggers: none in the data, none in OPTS code | ✅ (absence) | this census; ❓ for OMSI |
| O12 | **Several scripts raising one name in the same frame**: the order between the raisers (parts, vehicles) is ❓ | ❓ | Q-VS-1 and experiment EV-3 below |

### 5.2 Consequence for OPTS

O4 and O9 break the "nothing is dropped" principle (D6 in the [bus script integration strategy](../../../architecture/bus-script-integration.md)). They are listed as defects for [M-0102](../../../roadmap/phase-4/M-0102.md) to fix, and the design in [event-system.md](../../../architecture/event-system.md) removes them.

---

## 6. Name tables

### 6.1 Corpus (census)

| Table | Distinct names | Sites or entries | Top names (sites) |
| :--- | ---: | ---: | :--- |
| `(T.L.…)` | 2,194 | 64,669 | `ev_ibis_piep` 1,598; `ev_printer_button_on` 1,307; `ev_schaltruck` 1,267; `ev_printer_button_off` 1,258; `ev_wischerhebel` 907 |
| `(T.F.…)` | 15 | 709 | `ev_ibis_ansagen` 315; `ev_funkspruch` 98; `ev_efad_ansage` 58; `ev_an_int` 52; `ev_sonderansage` 48 |
| `(T.S.…)` | 1 | 6 | `smetak` 6 |
| `sound.cfg` `[trigger]` | 1,636 | 73,521 | `ev_doortriggerclose_2` 1,332; `ev_doortriggeropen_2` 1,330; `ev_doorhitclose_1` 1,307; `ev_doortriggeropen_1` 1,290 |
| `{trigger:…}` blocks | 5,705 | 51,703 | about 3,800 names are derived from keys or mouse; 2 are also raised by a script; about 1,900 are residual (categories overlap slightly, so the parts do not add up exactly) |
| keys (`[entry]`) | 129 | — | `automatic_d`, `bus_doorfront0`, `view_set_driver`, `ibis_eingabe` |

The corpus-wide identifier list is committed as [`event-identifiers.txt`](event-identifiers.txt) (identifiers only, 8,279 lines, run of 2026-11-24; see section 6.4). It is reproducible with `scripts/omsi_event_census.py --names-out`. The list is not embedded in this page, which keeps the reference tables short. Acceptance criterion 1 of M-0118 is met for the identifier list (the reference vehicles, section 6.2, are complete).

### 6.2 Reference vehicles

Appendix A lists every name found in the three reference vehicles: raised (`(T.L)`, `(T.F)`, `(T.S)`), bound by `sound.cfg`, and trigger blocks. Columns: **E** raised by a script of the vehicle, **S** bound in its sound file, **B** trigger block in the vehicle, **M** mesh trigger in the vehicle. Counts over the corpus follow for each name.

| Reference vehicle | Raised names | Sound-bound names | Payload (`T.F`) names | Trigger blocks (distinct) | Residual (no script, key, mouse or sound) |
| :--- | ---: | ---: | ---: | ---: | ---: |
| MAN_NL_NG (NG272) | 134 | 105 (101 also raised) | 1 | 262 | 42 |
| MAN_SD200 (SD200) | 108 | 83 (80 also raised) | 0 | 256 | 27 |
| HH109_Stadtbus_HHA (HH109) | 129 | 108 (103 also raised) | 1 | 263 | 25 |

The residual names of the three vehicles include the engine-raised candidates named in section 1, plus names such as `cp_dummy_sw…_toggle`, `kr_play`, `rollband_*` and `railbond_*` that no key or mouse binding of the scanned files reaches. For these the census cannot separate "engine-raised", "bound in a file not scanned" and "not used"; they stay ❓.

### 6.3 Sound events not raised by the scripts

Names that a reference vehicle binds in `sound.cfg` but never raises as `(T.L)`:

* in all three vehicles: `ev_ticketsale_givemoney`, `ev_ticketsale_takemoney`;
* NG272 and SD200: `ev_stamper`; HH109: `ev_matrix_lang`, `ev_tastersound`;
* raised only as `(T.F)` payloads, so not in the `(T.L)` list: `ev_ibis_ansagen` (NG272), `ev_efad_ansage` (HH109).

These are the mapping questions for [M-0102](../../../roadmap/phase-4/M-0102.md). They are probably raised by the engine (❓, the same way as the ticket-sale names), which is what experiment EV-1 tests.

---

### 6.4 Re-run of the committed census script on the installed data (2026-11-24)

**Command** (run on the installed OMSI 2 data root, output to a scratch file outside the repository; counts only, no names printed): `python3 scripts/omsi_event_census.py "<OMSI 2 install>/" --out <scratch>/census.md`. Exit status 0; the run finished.

| Source (script output) | Count |
| :--- | ---: |
| files scanned | 7,809 |
| `(T.L)` sites (literal) | 63,005 |
| `(T.F)` sites | 657 |
| `(T.S)` sites | 6 |
| non-literal event accessors | 1,043 |
| sound trigger entries | 73,521 |
| key entries (distinct) | 129 |
| mouse trigger entries (distinct) | 2,919 |

| Classification (section 3.2) | Count |
| :--- | ---: |
| raised names `(T.L)` | 2,150 |
| of which bound by a sound trigger | 1,528 |
| of which without sound binding | 622 |
| payload names (`T.F` or `T.S`) | 15 |
| sound names never raised as `(T.L)` | 108 |
| trigger-block names (distinct) | 5,633 |
| residual names (engine-raised candidates) | 1,900 |
| residual block sites | 8,693 |

**Comparison with the 2026-11-16 scratch census (sections 3.2 and 6.1).** Files scanned (7,809 = 4,918 + 1,853 + 1,035 + 3) and sound entries (73,521), keys (129) and mouse names (2,919) agree. The event counts do not:

| Figure | 2026-11-16 | 2026-11-24 (committed script) |
| :--- | ---: | ---: |
| `(T.L)` sites | 64,669 | 63,005 |
| `(T.L)` distinct names | 2,194 | 2,150 |
| `(T.F)` sites | 709 | 657 |
| non-literal accessors | 0 | 1,043 |
| distinct trigger-block names | 5,705 | 5,633 |
| sound names never raised | 78 | 108 |
| residual names | 1,905 | 1,900 |

Findings. (a) The 1,043 non-literal accessors are all hyphenated names (38 distinct). The script's literal rule (`[a-z0-9_]+`) excludes hyphens, and the scratch census probably did not; that explains the zero-dynamic result and is a rule choice, not a data fact. Only 5 of the 38 are bound by a sound trigger, and none has a hyphen-free twin among the literal names. (b) Literal sites are 1,664 lower than in the scratch census, of which 1,043 are the hyphenated ones; the remaining 621 are not explained, because the scratch census rules are not recorded in the repository. (c) The 98.1 % sound-match figure of section 1 is from the scratch census and has not been recomputed. The residual, sound-never-raised and classification counts in section 3.2 and in the M-0118 card are therefore to be read with these differences; the counts above are the committed script's output and should be the reference.

**Acceptance criterion 1 (corpus-wide list), met 2026-11-24 for the identifier list.** The run gives 2,150 distinct raised names (1,528 bound by a sound trigger, 622 unbound), 15 payload names and 108 sound-only names under the script's rules. The owner decided that identifiers may be committed. The list is `docs/src/formats/text/vehicles/event-identifiers.txt`: 8,279 lines, one identifier each, sorted and distinct (all name categories; the 2,150 raised names are all in it). It was produced by `python3 scripts/omsi_event_census.py <data-root> --names-out <file>`; the default run still prints counts only. 79 names that do not match `[a-z0-9_]+` (63 with a hyphen) are left out. The origin class and parameter shape of each name are in the tables above, not in the list.

## 7. Open questions and experiment proposals

Experiments use the [OMSI experiment protocol](../../../architecture/omsi-experiment-protocol.md) format. They are **proposals**; none has been run. Numbers EV-n do not collide with the protocol's E-numbers.

| ID | Question | Mark | Experiment (needs OMSI, copies of vehicles only) | Records | Learns |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Q-EV-1 | Which of the 1,905 residual names does the engine raise? | ❓ | EV-1: drive one bus with the sound on for 10 minutes, then open the doors, switch on the wipers and the blinkers; record each `sound.cfg` effect and the `Snd_*` outputs | screen recording, sound log line per item | which residual names produce a sound (the engine raised them) |
| Q-EV-2 | Do the engine triggers take parameters? | ❓ | EV-2: in a copy of a vehicle, a `{trigger:collision}` block shows the values of the variables it reads on a display; hit a kerb and a pole and note which values change | the display values per event | whether `collision`, `malfunction_*`, `veh_tank` carry data in variables |
| Q-EV-3 | In which order do several raisers of one name run in one frame? | ❓ | EV-3: two parts of one articulated bus raise the same `ev_*` name in one step; each writes its own counter; compare the counters after a step | counter values per part | ordering between parts (Q-VS-1 link) |
| Q-EV-4 | Does a `(T.L.name)` run a same-named `{trigger:name}`? | ❓ (Q-OSC-007) | EV-4: in a copy, add `{trigger:ev_test}` that shows a value; raise `(T.L.ev_test)` from another block; repeat with `(T.F.ev_test)` | displayed value | the script-raised trigger rule |
| Q-EV-5 | Is there a priority between two `[trigger]` blocks of one sound, or between two sounds on one name? | ❓ | EV-5: two `[sound]` entries on one `[trigger]` name, different volumes; listen and record the level | recording | whether both play (current OPTS: both) |
| Q-EV-6 | What do `(T.S.smetak)` and the `(T.F)` payload with a missing file do? | ❓ | EV-6: in a copy, raise `(T.S.smetak)` and `(T.F.ev_test)` with an existing and a missing file name | sound log, display | the meaning of `T.S` and the missing-file behaviour |

---

## Appendix A. Names of the reference vehicles

Columns: class as in section 3.2; **NG272**, **SD200**, **HH109** give the flags **E** (raised by a script), **S** (bound by `sound.cfg`), **B** (trigger block), **M** (mesh trigger); `-` means none. The last three columns count the corpus: raised sites `(T.L)`, sound entries, payload sites `(T.F)` and `(T.S)`.

| Name | Origin class | NG272 | SD200 | HH109 | Corpus `(T.x)` sites | Corpus sound entries | Corpus payload sites |
| :--- | :--- | :---: | :---: | :---: | ---: | ---: | ---: |
| `ai_scheduled_settarget` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `automatic_1` | key trigger (keyboard.cfg) | B | BM | BM | 0 | 0 | 0 |
| `automatic_1_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `automatic_2` | key trigger (keyboard.cfg) | B | BM | BM | 0 | 0 | 0 |
| `automatic_2_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `automatic_3` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `automatic_3_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `automatic_d` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `automatic_d_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `automatic_n` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `automatic_n_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `automatic_r` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `automatic_r-frei` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `automatic_r-frei_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `automatic_r_mouse` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `automatic_r_mouse_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `automatic_r_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `blinker_left_move` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `blinker_left_set` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `blinker_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `blinker_right_move` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `blinker_right_set` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `blinker_warn_toggle` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `blinker_warn_toggle_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `bremsescharf` | script event, sound-bound | - | - | ES | 21 | 27 | 0 |
| `bus_20h-switch` | key trigger (keyboard.cfg) | BM | B | - | 0 | 0 | 0 |
| `bus_dooraft` | key trigger (keyboard.cfg) | BM | BM | B | 0 | 0 | 0 |
| `bus_doorfront0` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `bus_doorfront0_external` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `bus_doorfront0_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `bus_doorfront1` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `bus_doorfront1_lock` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `bus_doorfront1_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `bus_doorfront1close_off` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `bus_matrix_change_steckschild` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `bus_rollband_change_steckschild` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `bus_rollband_dn` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_dn_step` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_dn_step_off` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_setl1` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_setl2` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_setl3` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_sett` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_up` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_up_step` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `bus_rollband_up_step_off` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `cashdesk_changer_0_05` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_05_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_0_10` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_10_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_0_15` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_15_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_0_20` | mouse/hot-spot trigger | BM | - | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_20_off` | mouse/hot-spot trigger | B | - | B | 0 | 0 | 0 |
| `cashdesk_changer_0_30` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_30_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_0_40` | mouse/hot-spot trigger | BM | - | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_40_off` | mouse/hot-spot trigger | B | - | B | 0 | 0 | 0 |
| `cashdesk_changer_0_50` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_0_50_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_15_00` | key trigger (keyboard.cfg) | BM | B | - | 0 | 0 | 0 |
| `cashdesk_changer_15_00_off` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `cashdesk_changer_1_00` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_1_00_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_1_50` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_1_50_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_2_00` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_2_00_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_4_00` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_4_00_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cashdesk_changer_5_00` | key trigger (keyboard.cfg) | BM | B | - | 0 | 0 | 0 |
| `cashdesk_changer_5_00_off` | key trigger (keyboard.cfg) | B | B | - | 0 | 0 | 0 |
| `cashdesk_changer_8_00` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cashdesk_changer_8_00_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `collision` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `cp_airride_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_airride_toggleoff_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_airride_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_airride_toggleon_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_asroffsw_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_asroffsw_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_batterietrennschalter_toggle` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cp_batterietrennschalter_toggle_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cp_blende_opn_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_blende_opn_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_dachluke_1` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_dachluke_2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_dachluke_3` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_doorentriegelung_01` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `cp_doorentriegelung_23` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `cp_doorentriegelung_45` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_driverventsw_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_driverventsw_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_dummy_sw01_toggle` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `cp_dummy_sw02_toggle` | mouse/hot-spot trigger | BM | B | - | 0 | 0 | 0 |
| `cp_dummy_sw03_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw04_toggle` | mouse/hot-spot trigger | BM | B | - | 0 | 0 | 0 |
| `cp_dummy_sw05_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw06_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw07_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw08_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw09_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_dummy_sw10_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_fahrerfenster_opn_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_fahrerfenster_opn_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_fahrerlicht_toggle` | key trigger (keyboard.cfg) | BM | BM | B | 0 | 0 | 0 |
| `cp_fahrertuer1_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_fahrertuer1_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_fahrertuer_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_fahrertuer_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_fahrgastpendel` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendel_2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendel_2_drag` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendel_2_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendel_drag` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendel_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_fahrgastpendell` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_fahrgastpendell_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_fahrgastpendell_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_fahrgastpendelr` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_fahrgastpendelr_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_fahrgastpendelr_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_flapsdrivervent_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_flapsdrivervent_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_flapsroomvent_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_flapsroomvent_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_flapswindshieldvent_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_flapswindshieldvent_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_floorventsw_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_floorventsw_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw1_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw1_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw2_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw2_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw3_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_frontventsw3_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_funk_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_funk_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_heizluefter_toggle` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `cp_heizregler_bug_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_heizregler_defrost_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_heizregler_fussraum_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_heizregler_temp_drag` | mouse/hot-spot trigger | - | B | B | 0 | 0 | 0 |
| `cp_heizregler_umluft_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_heizregler_umluft_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_hornl_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_hornm_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_hornu_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_ignitionkey_mov_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_ignitionkey_mov_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_ignitionkey_toggle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsdriver_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsdriver_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsfront_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsfront_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsrear_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_intlightsrear_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_klappfenster_1` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_klappfenster_2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_klappfenster_3` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_klappfenster_4` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_klappfenster_ol1` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ol2` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ol3` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ol4` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_or1` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_or2` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_or3` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_or4` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ul1` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ul2` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ur1` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ur2` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ur3` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfenster_ur4` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `cp_klappfensterl` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_klappfensterr` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_klima_toggleoff` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_klima_toggleon` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_klimator_sw_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `cp_kneeling_toggle` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_kneeling_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_kneeling_toggleoff_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_kneeling_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_kneeling_toggleon_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_kombil_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_kombim_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_kombiu_toggle` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `cp_lenkrad_toggle` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `cp_licht_nebelschw_sw` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `cp_licht_oberdeck_toggle` | key trigger (keyboard.cfg) | B | BM | B | 0 | 0 | 0 |
| `cp_licht_untenrechts_toggle` | key trigger (keyboard.cfg) | BM | BM | B | 0 | 0 | 0 |
| `cp_licht_unterdeck_toggle` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `cp_lockdoors_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_lockdoors_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_locksingledoor_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_locksingledoor_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_microphone` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `cp_microphone_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cp_motorkuehlung_toggle` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `cp_nebel_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_nebel_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_rampe_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_rampe_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_retarder1_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_retarder1_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_retarder1_toggleon_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_retarder2_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_retarder2_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_retarder_direkt_toggle` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `cp_retarder_toggle` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `cp_rollo2_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_rollo2_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_rollo2_retract` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_rollo2_retract_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_rollo_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_rollo_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_rollo_retract` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `cp_rollo_retract_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_roofvent_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_roofvent_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_schalter_asr_off_toggle` | mouse/hot-spot trigger | BM | B | - | 0 | 0 | 0 |
| `cp_schalter_hub_dn_toggle` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_schalter_hub_dn_toggle_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_schalter_hub_mstr_toggle` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_schalter_hub_up_toggle` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_schalter_hub_up_toggle_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_schalter_kinderwagen` | key trigger (keyboard.cfg) | BM | BM | B | 0 | 0 | 0 |
| `cp_schalter_knick_ovrd_toggle` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `cp_schalter_knick_ovrd_toggle_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `cp_schluessel_mov_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_schluessel_mov_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_schluessel_toggle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton1` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton1_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton2` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton3l` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton3l_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton3u` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton3u_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton4l` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton4l_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton4u` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton4u_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton5l` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton5l_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton5u` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_seatbutton5u_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_seatbutton6` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_sidewindowheatsw_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_sidewindowheatsw_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_spiegelheizung_toggle` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `cp_stopbrake_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_stopbrake_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_tacho2_drag` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_tacho2_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `cp_tachoschloss_toggle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_windshieldheat_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_windshieldheat_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_wiper_turnswitch_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_wiper_turnswitch_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `cp_wiperlever_drag` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `cp_wiperlever_off` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `cp_wischer_intervall_toggle` | key trigger (keyboard.cfg) | B | BM | B | 0 | 0 | 0 |
| `cp_wischer_schnell_toggle` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `cp_wischer_wascher_button` | key trigger (keyboard.cfg) | BM | BM | B | 0 | 0 | 0 |
| `cp_wischer_wascher_button_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `cp_zusatzheizung_toggleoff` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `cp_zusatzheizung_toggleon` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `door0_drag` | mouse/hot-spot trigger | B | B | BM | 0 | 0 | 0 |
| `door1_drag` | mouse/hot-spot trigger | B | B | BM | 0 | 0 | 0 |
| `door2_drag` | mouse/hot-spot trigger | B | B | BM | 0 | 0 | 0 |
| `door3_drag` | mouse/hot-spot trigger | B | B | BM | 0 | 0 | 0 |
| `door4_drag` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `door5_drag` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `door_aussenoeffner` | mouse/hot-spot trigger | BM | B | - | 0 | 0 | 0 |
| `door_aussenoeffner_2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `door_haltewunsch` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `door_haltewunsch_2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `door_haltewunsch_2_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `door_haltewunsch_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `door_kinderwagenwunsch` | mouse/hot-spot trigger | BM | BM | B | 0 | 0 | 0 |
| `door_rampenwunsch` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k0` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k1` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k2` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k3` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k4` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k5` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k6` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k7` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k8` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_k9` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kcity` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kdown` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kentf` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kfunktion` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kfunktion_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `efad_kgesamt` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kgross3` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kgruppe` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kkind` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kkurz` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_klikrs` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_knah` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_knein` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_knein_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `efad_kok` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kok_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `efad_ksonderz` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kstorno` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_ktag` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kubr` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_kup` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_modul_toggle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `efad_toggleticket` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `ev_an_int` | payload event (T.F/T.S) | - | - | - | 0 | 1 | 52 |
| `ev_anfahrheulen` | script event, no sound.cfg trigger | - | - | E | 12 | 0 | 0 |
| `ev_ansage` | payload event (T.F/T.S) | - | - | - | 0 | 16 | 13 |
| `ev_blende_cls` | script event, no sound.cfg trigger | - | - | E | 40 | 0 | 0 |
| `ev_blende_opn` | script event, no sound.cfg trigger | - | - | E | 40 | 0 | 0 |
| `ev_bremse_abs_0l_off` | script event, sound-bound | ES | E | - | 136 | 227 | 0 |
| `ev_bremse_abs_0l_on` | script event, sound-bound | ES | E | - | 136 | 227 | 0 |
| `ev_bremse_abs_0r_off` | script event, sound-bound | ES | E | - | 152 | 227 | 0 |
| `ev_bremse_abs_0r_on` | script event, sound-bound | ES | E | - | 152 | 227 | 0 |
| `ev_bremse_abs_1l_off` | script event, sound-bound | ES | E | - | 136 | 225 | 0 |
| `ev_bremse_abs_1l_on` | script event, sound-bound | ES | E | - | 136 | 225 | 0 |
| `ev_bremse_abs_1r_off` | script event, sound-bound | ES | E | - | 136 | 225 | 0 |
| `ev_bremse_abs_1r_on` | script event, sound-bound | ES | E | - | 136 | 225 | 0 |
| `ev_bremse_abs_2l_off` | script event, sound-bound | ES | - | - | 56 | 20 | 0 |
| `ev_bremse_abs_2l_on` | script event, sound-bound | ES | - | - | 55 | 20 | 0 |
| `ev_bremse_abs_2r_off` | script event, sound-bound | ES | - | - | 56 | 20 | 0 |
| `ev_bremse_abs_2r_on` | script event, sound-bound | ES | - | - | 56 | 20 | 0 |
| `ev_bremse_abs_off` | script event, sound-bound | - | - | ES | 148 | 12 | 0 |
| `ev_bremse_abs_on` | script event, sound-bound | - | - | ES | 148 | 12 | 0 |
| `ev_bremse_feststell_aus` | script event, sound-bound | ES | ES | ES | 560 | 391 | 0 |
| `ev_bremse_feststell_ein` | script event, sound-bound | ES | ES | ES | 568 | 409 | 0 |
| `ev_bremse_hup` | script event, sound-bound | E | ES | E | 156 | 215 | 0 |
| `ev_bremszischen` | script event, sound-bound | ES | ES | ES | 149 | 346 | 0 |
| `ev_contactshoe` | payload event (T.F/T.S) | - | - | - | 0 | 30 | 20 |
| `ev_crash` | script event, sound-bound | - | - | ES | 11 | 16 | 0 |
| `ev_dachluke_change` | script event, sound-bound | ES | - | - | 76 | 28 | 0 |
| `ev_dachluke_close` | script event, sound-bound | ES | - | - | 38 | 33 | 0 |
| `ev_dachluke_open` | script event, sound-bound | ES | - | - | 38 | 33 | 0 |
| `ev_doorentriegelung_01_active_off` | script event, sound-bound | ES | E | - | 65 | 12 | 0 |
| `ev_doorentriegelung_01_active_on` | script event, no sound.cfg trigger | E | E | - | 65 | 0 | 0 |
| `ev_doorentriegelung_01_off` | script event, sound-bound | ES | E | ES | 109 | 225 | 0 |
| `ev_doorentriegelung_01_off_final` | script event, sound-bound | E | E | - | 96 | 1 | 0 |
| `ev_doorentriegelung_01_on` | script event, sound-bound | ES | ES | ES | 109 | 394 | 0 |
| `ev_doorentriegelung_23_active_off` | script event, sound-bound | ES | E | - | 65 | 12 | 0 |
| `ev_doorentriegelung_23_active_on` | script event, no sound.cfg trigger | E | E | - | 65 | 0 | 0 |
| `ev_doorentriegelung_23_off` | script event, sound-bound | ES | E | ES | 104 | 222 | 0 |
| `ev_doorentriegelung_23_off_final` | script event, sound-bound | E | E | - | 96 | 1 | 0 |
| `ev_doorentriegelung_23_on` | script event, sound-bound | ES | ES | ES | 107 | 393 | 0 |
| `ev_doorentriegelung_45_active_off` | script event, sound-bound | ES | - | - | 59 | 2 | 0 |
| `ev_doorentriegelung_45_active_on` | script event, no sound.cfg trigger | E | - | - | 59 | 0 | 0 |
| `ev_doorentriegelung_45_off` | script event, sound-bound | ES | - | - | 87 | 161 | 0 |
| `ev_doorentriegelung_45_off_final` | script event, no sound.cfg trigger | E | - | - | 81 | 0 | 0 |
| `ev_doorentriegelung_45_on` | script event, sound-bound | ES | - | - | 90 | 275 | 0 |
| `ev_doorhitclose_0` | script event, sound-bound | ES | ES | ES | 234 | 1288 | 0 |
| `ev_doorhitclose_1` | script event, sound-bound | ES | ES | ES | 220 | 1307 | 0 |
| `ev_doorhitclose_2` | script event, sound-bound | ES | ES | ES | 233 | 228 | 0 |
| `ev_doorhitclose_3` | script event, sound-bound | E | E | E | 204 | 1140 | 0 |
| `ev_doorhitclose_4` | script event, sound-bound | E | - | - | 206 | 107 | 0 |
| `ev_doorhitclose_5` | script event, sound-bound | E | - | - | 182 | 949 | 0 |
| `ev_doorhitopen_0` | script event, sound-bound | E | ES | ES | 234 | 1269 | 0 |
| `ev_doorhitopen_1` | script event, sound-bound | E | ES | ES | 220 | 1216 | 0 |
| `ev_doorhitopen_2` | script event, sound-bound | E | ES | ES | 233 | 189 | 0 |
| `ev_doorhitopen_3` | script event, sound-bound | E | E | E | 215 | 1112 | 0 |
| `ev_doorhitopen_4` | script event, sound-bound | E | - | - | 202 | 115 | 0 |
| `ev_doorhitopen_5` | script event, sound-bound | E | - | - | 180 | 909 | 0 |
| `ev_doorlock1_off` | script event, sound-bound | E | E | E | 84 | 15 | 0 |
| `ev_doorlock1_on` | script event, sound-bound | E | E | E | 84 | 15 | 0 |
| `ev_doortriggerclose_0` | script event, sound-bound | ES | ES | E | 327 | 1249 | 0 |
| `ev_doortriggerclose_1` | script event, sound-bound | ES | ES | ES | 264 | 1290 | 0 |
| `ev_doortriggerclose_2` | script event, sound-bound | ES | ES | ES | 435 | 1332 | 0 |
| `ev_doortriggerclose_4` | script event, sound-bound | ES | - | - | 233 | 454 | 0 |
| `ev_doortriggeropen_0` | script event, sound-bound | ES | ES | E | 443 | 1249 | 0 |
| `ev_doortriggeropen_1` | script event, sound-bound | ES | ES | ES | 372 | 1290 | 0 |
| `ev_doortriggeropen_2` | script event, sound-bound | ES | ES | ES | 402 | 1330 | 0 |
| `ev_doortriggeropen_4` | script event, sound-bound | ES | - | - | 219 | 453 | 0 |
| `ev_downshift_2-1` | script event, sound-bound | - | - | ES | 36 | 23 | 0 |
| `ev_downshift_3-2` | script event, sound-bound | - | - | ES | 64 | 42 | 0 |
| `ev_downshift_4-3` | script event, sound-bound | - | - | ES | 64 | 40 | 0 |
| `ev_downshift_5-4` | script event, sound-bound | - | - | ES | 98 | 28 | 0 |
| `ev_drucker_piep` | script event, sound-bound | - | - | ES | 5 | 15 | 0 |
| `ev_druckschaltersound` | script event, sound-bound | - | - | ES | 64 | 12 | 0 |
| `ev_efad_ansage` | payload event (T.F/T.S) | - | - | S | 0 | 33 | 58 |
| `ev_efad_modulsteck` | script event, sound-bound | - | - | ES | 5 | 6 | 0 |
| `ev_efad_printer` | script event, sound-bound | - | - | ES | 15 | 6 | 0 |
| `ev_engine_brake_actuator_off` | script event, sound-bound | E | E | - | 113 | 11 | 0 |
| `ev_engine_brake_actuator_on` | script event, sound-bound | E | E | - | 113 | 11 | 0 |
| `ev_engine_ingnition` | script event, sound-bound | ES | ES | - | 189 | 716 | 0 |
| `ev_engine_shutdown_actuator_off` | script event, sound-bound | E | E | - | 156 | 4 | 0 |
| `ev_engine_shutdown_actuator_on` | script event, sound-bound | E | E | - | 170 | 4 | 0 |
| `ev_engine_starter` | script event, sound-bound | ES | ES | - | 262 | 719 | 0 |
| `ev_engineshutdown` | script event, sound-bound | ES | ES | ES | 235 | 879 | 0 |
| `ev_enginestart` | script event, sound-bound | - | - | ES | 110 | 102 | 0 |
| `ev_ext_announce` | payload event (T.F/T.S) | - | - | - | 0 | 4 | 1 |
| `ev_fahrerfenster_cls` | script event, sound-bound | ES | ES | ES | 171 | 241 | 0 |
| `ev_fahrerfenster_opn` | script event, sound-bound | ES | ES | ES | 167 | 241 | 0 |
| `ev_fahrertuer1_auf` | script event, no sound.cfg trigger | - | - | E | 31 | 0 | 0 |
| `ev_fahrertuer1_zu` | script event, no sound.cfg trigger | - | - | E | 31 | 0 | 0 |
| `ev_fahrertuer_auf` | script event, sound-bound | ES | ES | ES | 307 | 299 | 0 |
| `ev_fahrertuer_zu` | script event, sound-bound | ES | ES | ES | 313 | 301 | 0 |
| `ev_fahrgastpendel_2_zu` | script event, sound-bound | ES | - | - | 53 | 122 | 0 |
| `ev_fahrgastpendel_zu` | script event, sound-bound | ES | - | - | 65 | 126 | 0 |
| `ev_fahrgastpendell_zu` | script event, sound-bound | - | - | E | 24 | 9 | 0 |
| `ev_fahrgastpendelr_zu` | script event, no sound.cfg trigger | - | - | E | 34 | 0 | 0 |
| `ev_funkpiep` | script event, sound-bound | - | - | ES | 10 | 6 | 0 |
| `ev_funkspruch` | payload event (T.F/T.S) | - | - | - | 0 | 11 | 98 |
| `ev_gangwahltaster` | script event, sound-bound | ES | ES | ES | 505 | 162 | 0 |
| `ev_gangwahltaster_n` | script event, sound-bound | ES | ES | - | 125 | 141 | 0 |
| `ev_gangwahltaster_old_off` | script event, sound-bound | E | ES | - | 180 | 17 | 0 |
| `ev_gangwahltaster_old_on` | script event, sound-bound | E | ES | - | 146 | 17 | 0 |
| `ev_gear_downshift2` | script event, sound-bound | ES | E | - | 138 | 142 | 0 |
| `ev_gear_downshift3` | script event, sound-bound | E | - | - | 164 | 122 | 0 |
| `ev_gear_upshift2` | script event, sound-bound | ES | ES | - | 235 | 155 | 0 |
| `ev_gear_upshift3` | script event, sound-bound | ES | ES | - | 172 | 221 | 0 |
| `ev_gear_upshift4` | script event, sound-bound | E | - | - | 98 | 175 | 0 |
| `ev_haltebremse_an` | script event, sound-bound | ES | ES | - | 192 | 259 | 0 |
| `ev_haltebremse_aus` | script event, sound-bound | ES | ES | - | 241 | 354 | 0 |
| `ev_haltebremse_sw_an` | script event, sound-bound | ES | ES | E | 327 | 246 | 0 |
| `ev_haltebremse_sw_aus` | script event, sound-bound | ES | ES | E | 323 | 246 | 0 |
| `ev_handrampe` | script event, sound-bound | - | - | ES | 80 | 36 | 0 |
| `ev_hochschalt_1-2` | script event, sound-bound | - | - | ES | 79 | 48 | 0 |
| `ev_hochschalt_1-2_high` | script event, sound-bound | - | - | ES | 54 | 48 | 0 |
| `ev_hochschalt_2-3_high` | script event, sound-bound | - | - | ES | 54 | 46 | 0 |
| `ev_hochschalt_2-3_low` | script event, sound-bound | - | - | ES | 54 | 46 | 0 |
| `ev_hochschalt_3-4_high` | script event, sound-bound | - | - | ES | 27 | 27 | 0 |
| `ev_hochschalt_3-4_low` | script event, sound-bound | - | - | ES | 27 | 25 | 0 |
| `ev_hochschalt_4-5_high` | script event, sound-bound | - | - | ES | 64 | 19 | 0 |
| `ev_hochschalt_4-5_low` | script event, sound-bound | - | - | ES | 64 | 19 | 0 |
| `ev_hochschalt_5-6_high` | script event, no sound.cfg trigger | - | - | E | 12 | 0 | 0 |
| `ev_hochschalt_5-6_low` | script event, no sound.cfg trigger | - | - | E | 12 | 0 | 0 |
| `ev_hupe_an` | script event, sound-bound | ES | ES | ES | 161 | 345 | 0 |
| `ev_hupe_aus` | script event, sound-bound | ES | ES | ES | 164 | 344 | 0 |
| `ev_ibis-startupsound` | script event, sound-bound | ES | ES | - | 147 | 106 | 0 |
| `ev_ibis_ansagen` | payload event (T.F/T.S) | S | - | - | 0 | 336 | 315 |
| `ev_ibis_ansagen_ext` | payload event (T.F/T.S) | - | - | - | 0 | 8 | 16 |
| `ev_ibis_ansagen_fahrer` | payload event (T.F/T.S) | - | - | - | 0 | 4 | 6 |
| `ev_ibis_entwerter` | script event, sound-bound | ES | ES | - | 368 | 106 | 0 |
| `ev_ibis_entwerter_weiter` | script event, sound-bound | ES | ES | - | 110 | 97 | 0 |
| `ev_ibis_piep` | script event, sound-bound | ES | ES | - | 1598 | 251 | 0 |
| `ev_ibis_pieperror` | script event, sound-bound | ES | ES | - | 202 | 113 | 0 |
| `ev_kickdown_2-1` | script event, sound-bound | - | - | ES | 36 | 41 | 0 |
| `ev_kickdown_3-2` | script event, sound-bound | - | - | ES | 27 | 42 | 0 |
| `ev_kickdown_4-3` | script event, sound-bound | - | - | ES | 27 | 40 | 0 |
| `ev_kickdown_5-4` | script event, sound-bound | - | - | ES | 49 | 31 | 0 |
| `ev_kippschalter_aus` | script event, sound-bound | ES | ES | E | 554 | 226 | 0 |
| `ev_kippschalter_ein` | script event, sound-bound | ES | ES | E | 542 | 226 | 0 |
| `ev_klappfenster_cls` | script event, sound-bound | ES | ES | ES | 128 | 302 | 0 |
| `ev_klappfenster_cls_t` | script event, sound-bound | ES | - | - | 38 | 12 | 0 |
| `ev_klappfenster_opn` | script event, sound-bound | ES | ES | ES | 132 | 302 | 0 |
| `ev_klappfenster_opn_t` | script event, sound-bound | ES | - | - | 38 | 12 | 0 |
| `ev_kompressor_aus` | script event, sound-bound | ES | ES | ES | 165 | 343 | 0 |
| `ev_kompressor_ein` | script event, sound-bound | E | ES | E | 161 | 95 | 0 |
| `ev_kr_play` | script event, sound-bound | E | ES | - | 71 | 39 | 0 |
| `ev_kr_stop` | script event, sound-bound | E | ES | - | 72 | 39 | 0 |
| `ev_lights_blinker_off` | script event, sound-bound | ES | ES | ES | 179 | 259 | 0 |
| `ev_lights_blinker_on` | script event, sound-bound | ES | ES | ES | 369 | 420 | 0 |
| `ev_lights_blinker_swoff` | script event, sound-bound | ES | ES | ES | 790 | 307 | 0 |
| `ev_lights_blinker_swon` | script event, sound-bound | ES | E | ES | 527 | 281 | 0 |
| `ev_loesezischen` | script event, sound-bound | ES | ES | ES | 149 | 397 | 0 |
| `ev_logfile` | payload event (T.F/T.S) | - | - | - | 0 | 7 | 13 |
| `ev_lueftung_an` | script event, sound-bound | - | - | ES | 13 | 13 | 0 |
| `ev_lueftung_aus` | script event, sound-bound | - | - | ES | 13 | 13 | 0 |
| `ev_matrix_kurz` | script event, sound-bound | - | - | ES | 24 | 13 | 0 |
| `ev_matrix_lang` | sound trigger, never raised by a script | - | - | S | 0 | 13 | 0 |
| `ev_matrix_line_change` | script event, sound-bound | ES | ES | - | 21 | 96 | 0 |
| `ev_matrix_terminus_change` | script event, sound-bound | ES | ES | - | 127 | 96 | 0 |
| `ev_nbs_an` | script event, sound-bound | - | - | ES | 27 | 28 | 0 |
| `ev_nbs_aus` | script event, sound-bound | - | - | ES | 54 | 28 | 0 |
| `ev_nbs_on` | payload event (T.F/T.S) | - | - | - | 1 | 4 | 38 |
| `ev_printer_button_off` | script event, sound-bound | ES | ES | - | 1258 | 72 | 0 |
| `ev_printer_button_on` | script event, sound-bound | ES | ES | - | 1307 | 93 | 0 |
| `ev_printer_printing` | script event, sound-bound | ES | ES | - | 138 | 239 | 0 |
| `ev_retarder_aus` | script event, sound-bound | ES | ES | E | 172 | 59 | 0 |
| `ev_rollo2_end` | script event, sound-bound | - | - | E | 47 | 31 | 0 |
| `ev_rollo2_retract` | script event, sound-bound | - | - | ES | 26 | 157 | 0 |
| `ev_rollo_end` | script event, sound-bound | ES | ES | E | 186 | 140 | 0 |
| `ev_rollo_retract` | script event, sound-bound | ES | ES | ES | 131 | 266 | 0 |
| `ev_schaltruck` | script event, sound-bound | ES | ES | ES | 1267 | 229 | 0 |
| `ev_schluessel_dreh` | script event, sound-bound | ES | ES | ES | 489 | 169 | 0 |
| `ev_schluessel_raus` | script event, sound-bound | - | - | ES | 58 | 38 | 0 |
| `ev_schluessel_steck` | script event, sound-bound | ES | ES | ES | 262 | 159 | 0 |
| `ev_schluessel_weg` | script event, sound-bound | - | - | ES | 39 | 45 | 0 |
| `ev_sonderansage` | payload event (T.F/T.S) | - | - | - | 0 | 10 | 48 |
| `ev_stamper` | sound trigger, never raised by a script | S | S | - | 0 | 241 | 0 |
| `ev_stop` | script event, sound-bound | E | E | ES | 193 | 239 | 0 |
| `ev_stopbrake_aus` | script event, sound-bound | - | - | ES | 29 | 75 | 0 |
| `ev_stopbrake_ein` | script event, sound-bound | - | - | ES | 29 | 68 | 0 |
| `ev_stoss_dec` | script event, sound-bound | - | - | ES | 14 | 17 | 0 |
| `ev_stoss_inc` | script event, sound-bound | - | - | ES | 14 | 17 | 0 |
| `ev_stroman` | script event, sound-bound | - | - | ES | 40 | 64 | 0 |
| `ev_stromaus` | script event, sound-bound | - | - | ES | 40 | 52 | 0 |
| `ev_tacho2_auf` | script event, sound-bound | - | - | ES | 12 | 12 | 0 |
| `ev_tacho2_zu` | script event, sound-bound | - | - | ES | 12 | 12 | 0 |
| `ev_tastersound` | sound trigger, never raised by a script | - | - | S | 0 | 12 | 0 |
| `ev_ticketsale_givechange` | script event, sound-bound | ES | ES | ES | 806 | 410 | 0 |
| `ev_ticketsale_givechange2` | script event, sound-bound | ES | ES | ES | 701 | 399 | 0 |
| `ev_ticketsale_givechange3` | script event, sound-bound | ES | ES | ES | 701 | 399 | 0 |
| `ev_ticketsale_givechange_solo` | script event, sound-bound | ES | ES | ES | 774 | 410 | 0 |
| `ev_ticketsale_givechange_solo2` | script event, sound-bound | ES | ES | ES | 702 | 399 | 0 |
| `ev_ticketsale_givechange_solo3` | script event, sound-bound | ES | ES | ES | 702 | 399 | 0 |
| `ev_ticketsale_givemoney` | script event, sound-bound | S | S | S | 10 | 300 | 0 |
| `ev_ticketsale_giveticket` | script event, sound-bound | ES | ES | ES | 203 | 296 | 0 |
| `ev_ticketsale_takemoney` | sound trigger, never raised by a script | S | S | S | 0 | 408 | 0 |
| `ev_warning_hublift` | script event, sound-bound | ES | - | - | 17 | 19 | 0 |
| `ev_warning_rampe` | script event, sound-bound | ES | - | - | 25 | 25 | 0 |
| `ev_wippschalteraussound` | script event, sound-bound | - | - | ES | 385 | 40 | 0 |
| `ev_wippschaltereinsound` | script event, sound-bound | - | - | ES | 389 | 40 | 0 |
| `ev_wischerdrehschalter` | script event, sound-bound | - | - | ES | 220 | 44 | 0 |
| `ev_wischerhebel` | script event, sound-bound | ES | E | E | 907 | 271 | 0 |
| `handrampe_drag` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `hatch1handle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `hatch1lock` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `hatch2handle` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `hatch2lock` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `horn` | key trigger (keyboard.cfg) | BM | BM | BM | 0 | 0 | 0 |
| `horn_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `ibis_0` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_1` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_2` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_3` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_4` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_5` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_6` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_7` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_8` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_9` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_eingabe` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_loeschen` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_rueck` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ibis_setmode_linie_kurs` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_setmode_route` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_setmode_ziel` | key trigger (keyboard.cfg) | BM | BM | - | 0 | 0 | 0 |
| `ibis_vor` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ibis_vor_stumm` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `int_haltewunsch` | unbound in scan (engine-raised candidate or unscanned binding) | - | - | B | 0 | 0 | 0 |
| `kickdown` | script event, sound-bound | - | - | ES | 10 | 7 | 0 |
| `kneeling_ab` | script event, sound-bound | - | - | ES | 56 | 53 | 0 |
| `kneeling_auf` | script event, sound-bound | - | - | ES | 64 | 53 | 0 |
| `kr_play` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `kr_play_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `kr_stop` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `kr_stop_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `kw_batterietrennschalter` | mouse/hot-spot trigger | B | BM | B | 0 | 0 | 0 |
| `kw_batterietrennschalter_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `kw_fernlicht_toggle` | key trigger (keyboard.cfg) | B | BM | B | 0 | 0 | 0 |
| `kw_fernlicht_toggle_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `kw_m_engine_startbutton` | mouse/hot-spot trigger | BM | BM | BM | 0 | 0 | 0 |
| `kw_m_engine_startbutton_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `kw_m_engine_stopbutton` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `kw_m_engine_stopbutton_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `kw_m_engineshutdown` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `kw_m_engineshutdown_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `kw_m_enginestart` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `kw_m_enginestart_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `kw_scheinwerfer_toggle` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `kw_standlicht_toggle` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `kw_wipermode_up` | key trigger (keyboard.cfg) | B | BM | B | 0 | 0 | 0 |
| `luftsitz_rast` | script event, sound-bound | - | - | ES | 50 | 10 | 0 |
| `luftsitz_tast` | script event, sound-bound | - | - | ES | 40 | 10 | 0 |
| `malfunction_gettime` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | - | 0 | 0 | 0 |
| `malfunction_reset` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `matrix_seitenschildklemme_drag` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `matrix_seitenschildklemme_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `micro_lautsprecherknack_end` | script event, sound-bound | ES | ES | ES | 122 | 78 | 0 |
| `micro_lautsprecherknack_start` | script event, sound-bound | ES | ES | ES | 122 | 78 | 0 |
| `nextstop_announcement` | payload event (T.F/T.S) | - | - | - | 0 | 6 | 24 |
| `notloese_anfahrsperre` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `notloese_emr` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `parking_brake_mouse_drag` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `parking_brake_mouse_off` | mouse/hot-spot trigger | B | B | B | 0 | 0 | 0 |
| `parking_brake_release` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `parking_brake_set` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `parking_brake_toggle` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `rampe_raus` | script event, sound-bound | - | - | ES | 7 | 11 | 0 |
| `rampe_rein` | script event, sound-bound | - | - | ES | 7 | 11 | 0 |
| `ramplift_wunsch` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `rascheln` | script event, sound-bound | - | - | ES | 21 | 12 | 0 |
| `rlbnd_klick` | script event, sound-bound | E | ES | - | 125 | 32 | 0 |
| `rlbnd_linie_select_100_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_100_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_select_100_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_100_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_select_10_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_10_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_select_10_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_10_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_select_1_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_1_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_select_1_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_select_1_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_linie_start` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_linie_start_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_seitenschildklemme_drag` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_seitenschildklemme_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_taste_aus` | script event, sound-bound | E | ES | - | 229 | 19 | 0 |
| `rlbnd_taste_ein` | script event, sound-bound | E | ES | - | 231 | 19 | 0 |
| `rlbnd_ziel_select_100_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_100_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_100_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_100_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_10_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_10_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_10_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_10_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_1_+` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_1_+_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_1_-` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_select_1_-_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rlbnd_ziel_start` | mouse/hot-spot trigger | - | BM | - | 0 | 0 | 0 |
| `rlbnd_ziel_start_off` | mouse/hot-spot trigger | - | B | - | 0 | 0 | 0 |
| `rollband_ab` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `rollband_ab_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `rollband_auf` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `rollband_auf_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `rollband_sync` | mouse/hot-spot trigger | B | BM | - | 0 | 0 | 0 |
| `rollband_sync_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `smetak` | payload event (T.F/T.S) | - | - | - | 0 | 0 | 6 |
| `stamper_tone` | payload event (T.F/T.S) | - | - | - | 0 | 3 | 5 |
| `taster_heiz_def` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_def_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_l0` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_l0_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_l1` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_l1_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_l2` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_l2_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_l3` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_l3_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_misch` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_misch_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_smog` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_smog_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_tm` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_tm_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_tp` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_tp_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_v_f` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_v_f_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_v_ft` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_v_ft_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_v_t` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_v_t_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_heiz_v_w` | mouse/hot-spot trigger | BM | - | - | 0 | 0 | 0 |
| `taster_heiz_v_w_off` | mouse/hot-spot trigger | B | - | - | 0 | 0 | 0 |
| `taster_nebelschluss` | key trigger (keyboard.cfg) | BM | B | BM | 0 | 0 | 0 |
| `taster_nebelschluss_off` | key trigger (keyboard.cfg) | B | B | B | 0 | 0 | 0 |
| `taster_standheizung` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `taster_standheizung_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `taster_test` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `taster_test_off` | mouse/hot-spot trigger | - | - | B | 0 | 0 | 0 |
| `ticketer_newmessage` | payload event (T.F/T.S) | - | - | - | 0 | 2 | 2 |
| `ticketprinter_button_cancel` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_cancel_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_0` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_0_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_1` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_1_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_2` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_2_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_3` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_3_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_4` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_dummy_4_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_enter` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_enter_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_0` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_0_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_1` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_1_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_2` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_2_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_3` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_3_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_4` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `ticketprinter_button_ticket_4_off` | mouse/hot-spot trigger | B | B | - | 0 | 0 | 0 |
| `ticketprinter_getticket` | mouse/hot-spot trigger | BM | BM | - | 0 | 0 | 0 |
| `timetable_switch` | mouse/hot-spot trigger | - | - | BM | 0 | 0 | 0 |
| `veh_tank` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `veh_wash` | unbound in scan (engine-raised candidate or unscanned binding) | B | B | B | 0 | 0 | 0 |
| `wiper_start` | script event, sound-bound | ES | ES | ES | 207 | 277 | 0 |
| `wiper_start_dry` | script event, sound-bound | ES | ES | ES | 201 | 284 | 0 |
| `wiper_wash_start` | script event, sound-bound | ES | ES | E | 101 | 245 | 0 |

---

## Provenance

- Corpus census of `Vehicles/`, `Addons/` and `Inputs/` of the installed OMSI 2 (read-only, text files as data), 2026-11-16; rules in section 2.
- Committed script `scripts/omsi_event_census.py` run on the installed data, 2026-11-24 (branch `agent/m0118-census`); counts in section 6.4.
- Code facts: `crates/opts-sim/src/script/mod.rs` (`ScriptEvent`), `crates/opts-sim/src/script/compiler/bytecode.rs` (`EventKind`), `crates/opts-sim/src/script/run.rs` (`trigger`), `crates/opts-bevy/src/vehicle_script/system.rs`, `crates/opts-bevy/src/vehicle_triggers.rs`, `crates/opts-bevy/src/vehicle_triggers/keys.rs`, `crates/opts-game/src/player_sound/triggers.rs`, `crates/opts-game/src/vehicle_sound/events.rs` (2026-11-16, branch `agent/m0118`).
- Documented facts are cited to existing OPTS pages (section 3 and 5 of this page).

**Clean-room attestation (ADR-0002):** no OMSI binaries inspected; no OMSI asset, script text or sound committed; identifiers only.
