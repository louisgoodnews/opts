# 🚌 Phase 16: Feature parity with OMSI: bus operation

*Goal: close the gaps between OPTS and OMSI 2 in bus operation (boarding, stops, tickets, timetable discipline, breakdowns and service), in the order of player impact.*

**Status:** 📝 Proposed 2026-11-18 from the [OMSI parity matrix](../../../audit/omsi-parity-matrix-2026-11-18.md) (GAP-01 to GAP-20). No milestone of this phase has code yet. The checklist is in [`ROADMAP.md`](../../../../ROADMAP.md) (Phase 16); this page holds the details.

## Why this phase

The parity pass found that OPTS covers OMSI's driving model well, and that bus operation is the thinnest layer: a first passenger slice exists (M-0101), but the default free drive takes no passengers, there is no cash desk or change, no stop request, no failures or repair, and no depot wash. The gaps below are ranked by player impact.

## Priority order

Ranked by the parity gap rank (GAP number). Earlier rows first.

| Order | Milestone | Gap | Size | Notes |
| ---: | :--- | :--- | :--- | :--- |
| 1 | [M-0152](M-0152.md) Free-drive boarding rule | GAP-01 | S | Rule decided 2026-11-22 (board anyone whose destination the stop serves); done 2026-11-22 on `agent/free-drive` (rule) and `agent/free-drive-rig` (boarding-path tests). In-game check open. Issue ISS-036. |
| 2 | [M-0157](M-0157.md) Breakdowns, failures and repair | GAP-03 | L | Owner decision first: whether breakdowns are wanted at all. |
| 3 | [M-0156](M-0156.md) Cash desk and change | GAP-06 | M | Depends on the M-0101 money model. |
| 4 | [M-0155](M-0155.md) Timetable circuits by day type and night | GAP-07 | M | Uses TTData from M-0099. |
| 5 | [M-0153](M-0153.md) Stop request and stop-door interlocks | GAP-08 | S | Stop request light and button. |
| 6 | [M-0154](M-0154.md) Alighting on the real stop sequence | GAP-14 | M | Couples alighting to `bus.set_stop`. |
| 7 | [M-0159](M-0159.md) Passenger greetings and voices | GAP-16 | M | Audio payload events (P6). |
| 8 | [M-0158](M-0158.md) Depot wash and tanking | GAP-19 | M | Depot zones; fuel from M-0123. |

## Existing milestones this phase depends on

- [M-0101](../phase-4/M-0101.md) Doors, Passengers, Tickets and Cash: first slice only (2026-10-21); cash desk open (free-drive boarding done, M-0152).
- [M-0099](../phase-4/M-0099.md) Route, timetable and IBIS data layer (done).
- [M-0102](../phase-4/M-0102.md) and [M-0140](../phase-4/M-0140.md) sound events and AI sounds (first slices).
- [M-0118](../phase-11/M-0118.md) OMSI event system research (in progress).
- [M-0123](../phase-12/M-0123.md) fuel and battery (in progress).

## Not in this phase

- Real-weather observations ([M-0116](../phase-11/M-0116.md)) are owner-directed, not OMSI parity.
- Editors (GAP-20, [M-0051](../phase-6/M-0051.md)) are planned separately.
- Overhead lines and trolleybuses are in [Phase 17](../phase-17/README.md).

## Checklist

*Mirrored from the [roadmap](../README.md), which is canonical; update it there first.*

*Goal: close the OMSI 2 bus-operation gaps in the order of player impact (boarding, failures, cash, timetable circuits, stops, stop sequence, voices, depot service). Status: proposed 2026-11-18 from the [OMSI parity matrix](../../../audit/omsi-parity-matrix-2026-11-18.md); no milestone of this phase has code yet. Documents: [phase page](README.md).*

- [x] **🎫 Free-drive boarding rule** — [M-0152](M-0152.md) — rule decided 2026-11-22; done 2026-11-22 on `agent/free-drive` (rule) and `agent/free-drive-rig` (boarding-path tests); in-game check open. (GAP-01, ISS-036)
  - [x] Rule: a free drive boards anyone whose destination the stop serves; implemented, with an `admits` test for a bus with no timetable run.
- [ ] **🔧 Breakdowns, failures and repair** — [M-0157](M-0157.md) — owner decision first (GAP-03)
  - [ ] Decision memo; if approved, one failure class with repair, off by default.
- [ ] **💶 Cash desk and change** — [M-0156](M-0156.md) (GAP-06)
  - [ ] Money model, change action, `GivenTicket` and `GiveChangeCoin` acting on it.
- [ ] **📅 Timetable circuits by day type and night** — [M-0155](M-0155.md) (GAP-07)
  - [ ] Circuit selection from `TTData` by day type and time; night lines only at night.
- [ ] **🛑 Stop request and stop-door interlocks** — [M-0153](M-0153.md) (GAP-08)
  - [ ] Stop request light and button; rear-door and baby-switch behaviour from the scripts, tested.
- [ ] **🗺️ Alighting on the real stop sequence** — [M-0154](M-0154.md) (GAP-14)
  - [ ] Passengers alight at their stop, coupled to `bus.set_stop`.
- [ ] **🔊 Passenger greetings and voices** — [M-0159](M-0159.md) (GAP-16)
  - [ ] Greeting on boarding, voice per ticket pack, through the audio system.
- [ ] **🧽 Depot wash and tanking** — [M-0158](M-0158.md) (GAP-19)
  - [ ] Wash in a depot zone (`veh_wash`); tanking takes a modelled time at a depot or station.
