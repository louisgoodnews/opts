# Usability Test Protocol — Phase 9 (M-0065)

| Metadata | Value |
| :--- | :--- |
| **Status** | Protocol ready; **no sessions have been run** |
| **Milestone** | [M-0065](../roadmap/phase-9/M-0065.md) |
| **Rests on** | [Report analysis](../roadmap/phase-9/report-analysis.md) (the report's own advice: rules are hypotheses until validated with users), principles UI-P01 to UI-P13 |
| **Results report** | `docs/src/meta/usability-test-results-<date>.md` (to be written after the sessions; the template is in section 9) |

> **Honesty note.** This document was written by an AI agent working in the repository. It **cannot recruit, observe or time real participants**, so *every result field in this document is empty on purpose*, and the acceptance criterion of M-0065 ("at least 4 of 5 participants complete every task without help") is **not met until a human moderator runs the sessions** and fills the results report. Nothing here may be quoted as evidence that the interface is usable. What the automatic tests cover instead is listed in section 10.

## 1. Purpose

Find out, with people who did not build the interface, whether the Phase 9 rules actually make the application easy to learn and safe to use: first start, planning and starting a drive, changing a setting, finding controls, reporting a problem, leaving a drive; and whether the HUD can be read at a glance while driving.

## 2. Method

| Item | Decision |
| :--- | :--- |
| Type | moderated, think-aloud, one participant at a time, about 45 minutes |
| Participants | **5**; at least 2 who have never used OPTS or OMSI, at least 1 who drives simulators regularly, at least 1 who relies on keyboard or a gamepad or uses a large UI scale. Not project contributors. Recruiting is a human task |
| Environment | a release build on the participant's or the lab's machine; **a fresh configuration** (empty `XDG_CONFIG_HOME`/`XDG_DATA_HOME`) so the first start is real; an OMSI 2 installation present for tasks 2 to 6 (the participant's own, or the lab's; **no OMSI assets are recorded**, see ADR-0002) |
| Devices | mouse and keyboard; a gamepad is available and offered in task 4 |
| Recording | screen and voice **only with written consent**; recordings stay on the lab machine and are deleted after the report; notes are anonymous (P1 to P5). The moderator must not capture OMSI content in anything that is committed |
| Moderator rules | read the task card verbatim; do not explain the interface; answer only "what would you do?"; give a hint only after 3 minutes of no progress (the task then counts as *with help*) or on the participant's request; stop a task after 5 minutes (*failed*) |
| Data protection | no personal data beyond age band and driving-simulator experience; participation is voluntary and can be stopped at any time |

## 3. Tasks

Each task has a start state, a card text read to the participant, a success criterion the moderator can observe, and the principles it exercises.

| # | Task | Start state | Card text | Success | Principles |
| :--- | :--- | :--- | :--- | :--- | :--- |
| T1 | **First start** | fresh configuration, OMSI installation present | "You have just installed the game and start it for the first time. Get to the point where you could begin using it." | the participant reaches Home with the OMSI folder accepted, language and theme as they wish, without asking what a message means | UI-P02, UI-P11, UI-P12 |
| T2 | **Plan and start a drive** | Home | "Drive any bus on any map you like for a few minutes." | the drive starts (first HUD frame) after choosing map, vehicle and start point; the participant can say what they chose before starting | UI-P02, UI-P03, UI-P04, UI-P12 |
| T3 | **Change a setting** | Home | "The text is a bit small for you. Make the interface larger." (then) "Switch to the light appearance." | UI scale and theme changed through Settings, effect seen immediately, participant knows how to undo it | UI-P05, UI-P07, UI-P10 |
| T4 | **Find the controls** | Home, then in a drive | "You forgot which key opens the doors. Find out." (variant for the gamepad participant: "Find out how to steer the menus with the gamepad.") | the Help page or the help overlay is opened and the answer read out | UI-P08, UI-P10 |
| T5 | **Report a problem** | in a drive | "Something looks wrong with the vehicle. Tell the developers." | the report dialog is opened and a report sent or saved; the participant knows where it went | UI-P07, UI-P05 |
| T6 | **Leave a drive** | in a drive | "You have driven enough. Stop and get back to the start." | pause overlay opened, leaving confirmed knowingly, the session summary seen, Home reached; no input was lost without a warning | UI-P05, UI-P12 |

Tasks run in the order T1 to T6. A *with help* or *failed* task does not stop the session; the moderator resets the start state of the next task.

## 4. HUD glance test (during T2, after 2 minutes of driving)

The moderator asks three questions at random moments while the vehicle is moving at least 20 km/h and writes down the **glance time**: from the question to the participant's answer, and, where an eye-tracker or a second camera is available, the time the eyes left the road.

| Question | Source |
| :--- | :--- |
| "How fast are you going right now?" | speed readout |
| "Is the parking brake on?" | brake indicator (status never by colour alone) |
| "Which gear are you in?" | gear readout |

Targets from the HUD review checklist ([M-0064](../roadmap/phase-9/M-0064.md)): a single glance of at most 2 seconds for each of these. The numbers are hypotheses from automotive HMI practice and need sources (open item of the [report analysis](../roadmap/phase-9/report-analysis.md#5-open-items)).

## 5. Measures

| Measure | How | Per |
| :--- | :--- | :--- |
| **Success** | *completed without help* / *with help* / *failed* | task and participant |
| **Time** | seconds from the end of the card text to the success criterion (stopwatch) | task and participant |
| **Errors** | wrong clicks or keys that led to an unintended screen or state, counted by the moderator; each is described in a few words | task and participant |
| **Perceived effort** | the single-item *Single Ease Question*, 1 (very difficult) to 7 (very easy), asked after each task | task and participant |
| **Satisfaction** | the 10 items of the *System Usability Scale*, once at the end | participant |
| **Glance time** | section 4 | question and participant |
| **Observations** | quotes, hesitations, workarounds | free text |

## 6. Acceptance (from M-0065)

- At least **4 of 5** participants complete **every** task without help.
- Every failed or helped task is logged as a **finding** with a severity (blocker, major, minor) and the principle it contradicts.
- The rules of the report that the tests could not cover are listed as **open items with owners** (section 10 starts that list).

## 7. Session script

1. Welcome, purpose ("we test the program, not you"), consent, anonymity, permission to stop (3 min).
2. Background questions: age band, simulator experience, input devices (2 min).
3. Fresh configuration, task cards T1 to T6 (30 min). After each task: the SEQ question and one open question ("what did you expect to happen?").
4. HUD glance questions during T2.
5. SUS questionnaire, final open questions ("what was the best / the most annoying moment?") (5 min).
6. Thanks; the moderator writes the session notes the same day.

## 8. Analysis

1. A table of all measures per participant and task (template below).
2. Findings: each problem observed in two or more participants is at least *major*; a problem that stopped a participant is a *blocker*.
3. Fixes: a list "finding, fix, owner, milestone", appended to the results report; fixes are re-tested with one participant per blocker.
4. Report in `docs/src/meta/`, with no recordings and no OMSI content.

## 9. Results report template

| Participant | Task | Result (done / help / failed) | Time (s) | Errors | SEQ (1 to 7) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| P1 | T1 | *not run* | | | |

| Finding | Task | Severity | Principle | Fix | Owner |
| :--- | :--- | :--- | :--- | :--- | :--- |
| *none: no sessions held* | | | | | |

SUS per participant, glance times per question, summary against the acceptance criteria: all **not run**.

## 10. What the automatic tests already cover, and what they cannot

Covered without people (see [accessibility audit](../roadmap/phase-9/accessibility-audit.md)): target size, text size, contrast in every theme and accent, keyboard traversal, disabled controls with a reason, German and English completeness, text expansion, the layout goldens. **Open items for a human** (owner: project lead until assigned): all of sections 3 to 5; screen-reader behaviour; real pixel-level screenshots on real GPUs; perceived calm, trust and aesthetics (UI-P12); gamepad and wheel feel with hardware; glance times.
