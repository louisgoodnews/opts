# OMSI Event System: Consolidation Design

| Metadata | Value |
| :--- | :--- |
| **Status** | 🟡 Design proposal (2026-11-16, M-0118 Phase B/C). No code written; the design is not approved |
| **Milestone** | [M-0118](../roadmap/phase-11/M-0118.md) |
| **Evidence** | [OMSI events and triggers](../formats/text/vehicles/events.md) (taxonomy, census, ordering facts), [bus script integration](bus-script-integration.md) (D6, Q-BSI-4, Q-BSI-5) |
| **Consumers** | [M-0102](../roadmap/phase-4/M-0102.md) (sound), [M-0098](../roadmap/phase-4/M-0098.md) (trigger routing), [M-0115](../roadmap/phase-11/M-0115.md) (zones) |
| **Evidence marks** | ✅ measured or read in code, 📚 documented on an existing OPTS page, ❓ unverified (as in [events.md](../formats/text/vehicles/events.md)) |

---

## 1. Problem

OPTS has one channel per feature. Each has its own queue, its own cap and its own raise function (✅ code):

| Channel | Where | Behaviour today |
| :--- | :--- | :--- |
| script events | `VehicleScripts.raised: Vec<String>` (`crates/opts-bevy/src/vehicle_script/system.rs`) | keeps the **name only**; the event kind (`Local`, `Front`, `Sound`) and the part are dropped at `raised.push(e.name)`; **cap 256 per step, silent** |
| script events of scenery | `SceneryScripts` (`crates/opts-bevy/src/scenery_runtime/scripts.rs`) | own queue, lower-case names |
| AI events | `AiEvents(Vec<String>)` (`crates/opts-game/src/vehicle_sound/events.rs`) | **cap 8 per frame, silent** |
| key and mouse triggers | `vehicle_triggers::raise` (`crates/opts-bevy/src/vehicle_triggers.rs`) | calls `engine.trigger(name)`; unknown name logs at debug only |
| protocol triggers | `vehicle_remote::raise_trigger`, `control_drive::raise` | own paths |

Consequences that the corpus data show (📚 events.md section 5): (a) the `(T.F)` payload never reaches the sound layer, which is why M-0102 lists it as open; (b) events beyond the caps disappear, which contradicts principle D6 ("nothing is dropped silently"); (c) nothing can say which names reached a consumer and which did not.

The VM already has the right envelope, `ScriptEvent { kind, name }` (`crates/opts-sim/src/script/mod.rs`), but it has **no payload field and no part id**. The design extends that type rather than inventing a second one.

---

## 2. Event types

Names are **normalised once** to lower case and interned (OMSI names match case-insensitively, 📚 [OSC](../formats/text/vehicles/osc.md)). Every type below carries the owning vehicle entity and the part index.

### 2.1 Inbound: triggers

```text
Trigger {
    source: Key | Mouse { mesh, drag: Option<(f32, f32)> } | Engine | Protocol | ScriptLocal,
    owner:  VehicleEntity,
    name:   NameId,            // lower-case, interned
    frame:  u64,
}
```

`ScriptLocal` is the `(T.L)` raise when the flag `ScriptRaiseRunsTrigger` is on (default off, 📚 [OSC runtime](osc-runtime.md)). It is listed so that Q-EV-4 can be answered without a new type.

### 2.2 Outbound: script events

```text
ScriptEvent {                  // extends crates/opts-sim/src/script/mod.rs
    kind:    Local | Front | Sound,   // existing EventKind
    name:    NameId,
    part:    u16,                     // new: which part's engine raised it
    payload: Payload,                 // new: None | Text(String)
}
Payload::Text  carries the top string for kind = Front (announcement file, 📚 OSC Q-OSC-008)
```

### 2.3 Engine events

Engine-raised names are not in any script. They are a **small closed set** that the host raises, one variant each, with the payload documented per variant as it is measured (events.md Q-EV-2):

```text
EngineEvent::AiScheduledSetTarget { owner }    // 📚 reads AI_Target_Index, SetLineTo
EngineEvent::Collision { owner, part }         // ❓ parameters
EngineEvent::MalfunctionReset | MalfunctionGetTime { owner }   // ❓
EngineEvent::VehTank | VehWash { owner }       // ❓
EngineEvent::ParkingBrakeSet | ParkingBrakeRelease { owner }   // ❓
```

Each variant is **stubbed explicitly** (M-0102 acceptance 3) and logged the first time it fires. Its name maps to the trigger table of section 4.

### 2.4 Zone events (M-0115)

```text
ZoneEntered { vehicle: VehicleEntity, zone: ZoneId, frame: u64 }
ZoneLeft    { vehicle: VehicleEntity, zone: ZoneId, frame: u64 }
```

These use the same envelope (`frame`, plus a sequence number, section 3) so that one log orders them with script events. No OMSI name is involved; M-0115 shapes them.

---

## 3. The bus

One **event log per world**, a Bevy message channel in `opts-bevy` fed by the adapters of each source. The record:

```text
LogEntry { frame: u64, seq: u32, event: Trigger | ScriptEvent | EngineEvent | Zone... }
```

* `seq` is assigned at push time from one counter per frame. The order `(frame, seq)` is the single delivery order.
* The log is **append only within a frame**. Consumers read it after the producing step (section 5, R2) and cannot remove entries; a consumer that does not match an entry simply skips it.
* The `opts-sim` crate keeps the plain envelope types (no Bevy dependency), `opts-bevy` owns the log and the adapter. **No new crate** is proposed, so no crate-boundary ADR is required by this design. If the project lead prefers a separate `opts-events` crate, that is a boundary change and needs an ADR first (ADR-0005 and ADR-0003 govern the layering).

---

## 4. Mapping layer

| Input | Mapping | Result |
| :--- | :--- | :--- |
| `ScriptEvent` name | the names bound by the vehicle's `sound.cfg` `[trigger]` (data-driven, per vehicle) | a sound consumer subscribes to the names it has |
| `ScriptEvent` name, `kind = Front` | payload resolved as an announcement file | announcement consumer |
| `EngineEvent` | closed enum, section 2.3 | engine consumer and trigger routing |
| any other name | none | `Unmapped(name)`: counted per vehicle, listed in the report (section 7) |

The mapping is **not** a static table of OMSI names. The corpus has 2,194 raised names and new add-ons add more (events.md section 6.1); a static list would be stale on arrival. Only the engine names are static, because they are the only ones the host raises.

**Content-added registration.** A script set needs no registration. Its events are routed by name to whichever consumer subscribes, and the subscription comes from the data the vehicle already ships (`sound.cfg`, the key and mouse bindings). That is why the mapping can be data-driven.

**Extension point.** A consumer is a predicate and a handler:

```text
trait EventConsumer {
    fn accepts(&self, entry: &LogEntry) -> bool;   // by kind, name, or owner
    fn handle(&mut self, entry: &LogEntry);
}
```

---

## 5. Ordering and dispatch rules

These rules can be checked by unit tests. Each rule names the OMSI fact it follows.

| # | Rule | Test that checks it | Follows |
| :--- | :--- | :--- | :--- |
| R1 | Entries of one owner are delivered in `seq` order | feed raises `[a, b, a]` from parts `[0, 1, 0]`; every consumer sees the three entries in that order | events.md O3 |
| R2 | Events raised during step *n* are visible to consumers after step *n* and before step *n + 1* | a consumer reading in `Update` after `run_frame` sees exactly the entries of that step | events.md O3 |
| R3 | Input triggers of one frame run before the step *n + 1*, in binding order; within a frame keys come before mouse and engine (design choice, OMSI order ❓) | a fixture with one key, one mouse and one engine trigger yields that order | events.md O7; order between sources ❓ |
| R4 | Fan-out without priority: every consumer whose `accepts` is true receives each entry; no consumer can suppress another | two consumers on one name both receive it; a consumer that drops its entry does not change the other's count | events.md O5 |
| R5 | Routing: key → head part only; mouse → the part whose mesh was hit; engine → head and every part that defines the name; scripted click → first engine that has the trigger | the routing table as a test of 4 cases (one per row) | [M-0098](../roadmap/phase-4/M-0098.md) (trigger routing, measured on the NF6D, 📚 [OSC runtime](osc-runtime.md)); events.md O10 |
| R6 | No silent drop: there is no cap that discards. A queue that grows over a limit counts the overflow and logs once per name per second | 1,000 raises in one step: all 1,000 are in the log, or the overflow counter equals the missing count | events.md O4, O9 (current defects) |
| R7 | All blocks of one trigger name run in file order | two blocks of one name write `1` then `2` to a variable; the variable reads `2` | events.md O2 |
| R8 | A trigger name no block answers is `Unmapped` and counted, never ignored | a `Trigger` with an unknown name appears in the unmapped count | events.md O8 |

**Priority** is absent by design: no consumer has a priority, and no entry can be consumed. If OMSI has a priority rule (Q-EV-5), it is added as a per-consumer filter, not as a change to R4.

---

## 6. Consumers and the event types each needs

| Consumer | Milestone | Needs | Ordering it relies on | Notes |
| :--- | :--- | :--- | :--- | :--- |
| Sound | [M-0102](../roadmap/phase-4/M-0102.md) | `ScriptEvent` with `kind`, `name`, `part`, `payload`; the sound binding of `sound.cfg` `[trigger]` (viewpoint, `[conditionSingle]`) | R1, R4, R6 | every match starts (events.md O5); the payload is the announcement path |
| Trigger routing | [M-0098](../roadmap/phase-4/M-0098.md) | `Trigger` (key, mouse, engine, protocol) and the consist's part list; `handled` result | R3, R5, R8 | replaces the three raise paths of section 1 with one `Trigger` |
| Tunnel and zones | [M-0115](../roadmap/phase-11/M-0115.md) | `ZoneEntered` and `ZoneLeft` with vehicle and zone id | R1, R2 | no OMSI name is needed; OMSI's own tunnel events are unsourced |

---

## 7. Debug and report

Per vehicle and per run, the integration report lists for every name:

| Column | Meaning |
| :--- | :--- |
| raised | count of entries with the name |
| consumed | count of entries that at least one consumer accepted |
| unmapped | `true` when no consumer accepted it |
| never raised | for names bound by a `[trigger]` or a key but not raised in the run (the Q-BSI-4 list) |

The report is the answer to M-0102 acceptance 2 ("the report lists events without a sound mapping") and to Q-BSI-4 (the engine-raised candidates of events.md section 3.2).

---

## 8. Migration

| Step | Change | Milestone | Done when |
| :--- | :--- | :--- | :--- |
| 1 | `ScriptEvent` gains `part` and `payload`; `VehicleScripts.raised` becomes the log; caps replaced by R6 | M-0102 | R1, R4, R6 tests pass; `(T.F)` payload reaches the announcement consumer |
| 2 | `AiEvents` cap replaced by R6 | M-0102 | R6 test for AI vehicles |
| 3 | `vehicle_triggers::raise`, `vehicle_remote::raise_trigger` and `control_drive::raise` produce `Trigger`s | M-0098 | R3, R5, R8 tests pass; E8 result recorded |
| 4 | `ZoneEntered` and `ZoneLeft` added to the log | M-0115 | the zone detector emits through the log; event shape equal to M-0115 line 43 |
| 5 | Scenery events move to the log (optional) | later | scenery golden run unchanged |

---

## 9. Acceptance mapping (M-0118 acceptance 4)

| Item of the consumer milestone | Section of this design | Section of events.md |
| :--- | :--- | :--- |
| M-0102 acceptance 1: every `[trigger]` of NG272, SD200, HH109 reached in a scripted run; ≥ 90 % of corpus `sound.cfg` triggers matched | §4 (sound subscription), §6, §7 (raised column) | §1 item 5 (98.1 % `(T.L)` match, all vehicles); §6.2 and appendix A (per vehicle) |
| M-0102 acceptance 2: no event dropped silently; report lists events without a sound mapping | R6, R8, §7 (unmapped) | §5 O4, O8, O9 (the current drops) |
| M-0102 acceptance 3: engine-raised events listed and stubbed | §2.3 (closed enum), §4 | §3.2 (engine candidates), §6.2 (residual per vehicle), Q-EV-1 and Q-EV-2 |
| M-0102 open item: payload of `(T.F)` events | §2.2 (`Payload::Text`), migration step 1 | §4 (`(T.F)` payload), Q-OSC-008 |
| M-0102 dependency: `ScriptEvent` from every part's engine | §2.2 (`part`), §3 | §3.1 (sources) |
| M-0098 trigger routing: keys to head, hot-spot to the owner part, scripted click to the first engine with the trigger | R5, §6 (routing) | §5 O10 |
| M-0098 E8 / Q-BSI-5 (one trigger in several parts) | R5 (engine case), §8 step 3 | Q-EV-3 (order between parts); protocol E8 |
| M-0115 zone enter and leave messages | §2.4, §6, migration step 4 | not applicable (no OMSI name) |

---

## 10. Open questions

| ID | Question | Blocks |
| :--- | :--- | :--- |
| Q-EV-1 | Which residual names does the engine raise? (EV-1) | §2.3 completeness |
| Q-EV-2 | Do the engine triggers carry parameters? (EV-2) | §2.3 payloads |
| Q-EV-3 | Order of raisers of one name in one frame (EV-3) | R1 across parts |
| Q-EV-4 | Does `(T.L)` run a same-named trigger? (EV-4) | §2.1 `ScriptLocal` flag default |
| Q-EV-5 | Priority between sounds on one name (EV-5) | R4 exception, if any |
| Q-EV-6 | `(T.S)` meaning; missing announcement file (EV-6) | §2.2 `Sound` kind |
| D-Q1 | Cap or no cap for the log? The design says no cap, with counted overflow (R6). The project lead decides whether a hard limit is needed for memory | R6 |
| D-Q2 | Is the zone event shape of M-0115 final? | §2.4 |

---

## 11. Review (Phase C)

* **Self-check against M-0102 and M-0098 (done here, section 9).** Every acceptance item of both milestones that touches events has a row.
* **Independent adversarial review (P-003): not yet done.** It needs a reviewer who did not write this page. Until it is done, the design is a proposal.
* **Crate boundary and ADR:** the design keeps the envelope in `opts-sim` and the log in `opts-bevy`, so no ADR is needed for the current proposal (section 3). An ADR is needed if the log moves to a new crate.
* **Milestone tracker updates** (ROADMAP, BOARD, phase page) are the coordinator's, not this page's.

---

## Provenance

* Code facts as listed in [events.md, Provenance](../formats/text/vehicles/events.md#provenance), read 2026-11-16 on branch `agent/m0118`.
* Design choices (R3 source order, R6 log growth, the closed engine enum) are the author's proposals, marked above where they go beyond OMSI's data.

**Clean-room attestation (ADR-0002):** no OMSI binaries inspected; no OMSI asset committed; names are identifiers only.

## Related Documents

[OMSI events and triggers](../formats/text/vehicles/events.md) · [Bus script integration](bus-script-integration.md) · [OSC runtime](osc-runtime.md) · [Script VM](script-vm.md) · [M-0118](../roadmap/phase-11/M-0118.md) · [M-0102](../roadmap/phase-4/M-0102.md) · [M-0098](../roadmap/phase-4/M-0098.md) · [M-0115](../roadmap/phase-11/M-0115.md) · [ADR-0002](../adr/0002-clean-room-policy.md)
