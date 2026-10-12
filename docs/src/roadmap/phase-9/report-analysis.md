# Analysis of the HCI Report and the Design Principles It Yields

| Metadata | Value |
| :--- | :--- |
| **Status** | ✅ Approved 2026-10-12 (decisions recorded in the [phase page](README.md#decisions)) |
| **Date** | 2026-10-12 |
| **Input** | *Clinical Report: Evidence-Based Design Rules for Modern Graphical User Interfaces* (2026), supplied by the project lead as `docs/src/Clinical Report: Evidence-Based Design Rules for Modern Graphical User Interfaces.md` (not yet committed to the repository) |
| **Output** | Principles **UI-P01 … UI-P13**, used as the checklist of [audit](current-ui-audit.md), [design language](design-language.md), [interaction](interaction-and-feel.md), [flow](flow.md) and every Phase 9 milestone |

## 1. What the report is

A synthesis of ten bodies of HCI knowledge into do/don't rules with a compliance matrix: Fitts's, Hick's and Miller's laws (§1), cognitive load (§2), Nielsen's heuristics (§3), Shneiderman's golden rules (§4), Gestalt principles (§5), WCAG 2.2 (§6), dark patterns (§7), peak-end, affordance and the aesthetic-usability effect (§8), a matrix (§9) and eight recommendations (§10).

## 2. How to read it for OPTS — five cautions

1. **Target platform.** The report is written for web and mobile: CSS pixels, 44 px touch targets, ARIA, bottom tab bars, "infinite" screen edges for a mouse cursor. OPTS is a desktop simulator operated with mouse and keyboard, later with a gamepad or a steering wheel with pedals. The *laws* transfer; the *numbers* need translation (§3 below).
2. **Sources.** The closing note says every claim is traceable to cited sources, but the file cites none. Figures such as "+33.6 % task completion" or "92 % / 75 % satisfaction" cannot be verified from it. This proposal therefore uses the report for *rules and mechanisms*, treats its numbers as indicative, and asks that any number we rely on in an acceptance test is sourced independently (an open item, §5).
3. **Internal tension.** Hick's law is presented both as "limit choices" and as "may recommend showing everything at once"; the "≤ 5 choices per screen" (§2) and "≤ 7 navigation items" (§1.3) guidelines differ. OPTS has catalogues of 43 maps and 956 vehicles, where neither a deep category tree nor a flat list of everything works: the answer is *search-first lists with recents, favourites and filters* (UI-P02).
4. **Missing: driving.** OPTS has two distinct modes. In menus the user browses and decides; while driving, attention belongs to the road and the instruments, and interface rules differ (glanceability, no modal interruptions, stable layout). The report does not cover heads-up or in-vehicle interfaces. UI-P13 adds this; the quantitative glance limits (single glances of about 2 s are commonly cited from driver-distraction guidelines) come from outside the report and must be sourced before they become acceptance criteria.
5. **Dark patterns.** OPTS has no commercial layer, so the monetisation patterns do not apply, but the *defaults and consent* rules do (§7): no network request without an explicit choice, no pre-selected sharing, no nagging. The audit found one near miss (the "live weather" panel, [F06](current-ui-audit.md#3-findings)).

## 3. Translating the numbers

| Report value | Meaning for OPTS (logical px at UI scale 100 %, 1080p reference) |
| :--- | :--- |
| 44×44 CSS px touch target | **Buttons and list rows ≥ 40 px tall; primary actions ≥ 44; no pointer target below 32×32** (the current UI has 18×18 and 16×11 targets). WCAG 2.2 AA minimum is 24×24; we choose stricter because a simulator may be used with a wheel-mounted pointer or at 4K scaling. |
| Screen edges as infinite targets | Persistent navigation sits on the left edge (rail), *Back* top-left, the primary action bottom-right of the content area, always in the same place. |
| ≤ 7 navigation items; ≤ 5 choices per screen | **≤ 5 top-level destinations**; ≤ 5 primary actions per screen; lists longer than 7 have search, filters, recents. |
| Contrast 4.5:1 / 3:1 | Same, but measured on *rendered* backgrounds (text over map photos included), not only on tokens. |
| Text size | Body ≥ 14 px, nothing below 12 px; UI scale 75–200 %. |
| Response feedback | ≤ 100 ms for any visible acknowledgement, a progress indicator after 1 s, a cancel option after 5 s. |

## 4. The principles

Each principle has an ID, the report section it rests on, the OPTS rule and a **measurable acceptance** that tests or reviews can check. *Applies to* uses **M** (menus and setup), **H** (in-game HUD), **E** (editor).

| ID | Principle | Rest on | OPTS rule | Acceptance | Applies |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **UI-P01** | Easy to hit | Fitts §1.1 | Targets ≥ 32 px, buttons ≥ 40, primary ≥ 44; hit area ≥ visual glyph; *Back* and the primary action in fixed places; destructive actions separated and never default-focused | a test walks the UI tree and fails on any target below 32×32 | M H E |
| **UI-P02** | Few, well-defaulted choices | Hick §1.2, CLT §2 | ≤ 5 primary choices per screen; smart defaults and "continue where I left off"; long lists get search + recents + favourites + filters; advanced options behind disclosure | every screen has a documented default path of ≤ 2 clicks from Home to driving | M |
| **UI-P03** | Light on memory | Miller §1.3, CLT §2 | ≤ 4–7 chunks per view; the current selection is always visible; context carries across steps; last choices are remembered | the New Drive summary is visible on every step; choices persist across launches | M E |
| **UI-P04** | Always say what is happening | Nielsen 1, Shneiderman 3 | visible feedback ≤ 100 ms; real progress for ≥ 1 s tasks; failures are shown where they happen and are never overwritten | seeded failure tests: load error, missing file, failed save are visible to the user | M H E |
| **UI-P05** | The user is in control | Nielsen 3, Shneiderman 6–7 | Esc means *back / close / pause*, never *destroy*; undo where possible, confirmation only for the irreversible; risky settings can be reverted | no key or click leaves a running session without a way back | M H E |
| **UI-P06** | One system | Shneiderman 1, Nielsen 4 | one component library, one interaction model, one theme, one vocabulary on every screen (HUD and loading screen included) | a screenshot audit shows no hard-coded colours outside the instrument token set | M H E |
| **UI-P07** | Prevent and explain errors | Nielsen 5, 9; Shneiderman 5 | invalid actions are disabled *with a visible reason*; errors say what happened, why, what to do; defaults are the safest common choice | each error message passes a review against the template in [interaction](interaction-and-feel.md#5-errors) | M E |
| **UI-P08** | Recognise, don't recall; shortcuts for experts | Nielsen 6–7 | key hints in context, a help overlay, full keyboard operation, later a command palette | every interactive element reachable and activatable by keyboard (automated traversal test) | M H E |
| **UI-P09** | Group by proximity and hierarchy | Gestalt §5 | spacing from tokens; related controls adjacent; clear figure/ground; never colour alone | spacing audit against the 4-px grid; status never conveyed by colour alone | M H E |
| **UI-P10** | Accessible by design | WCAG 2.2 AA §6 | contrast ≥ 4.5/3, visible focus, keyboard, UI scale 75–200 %, reduced motion, no flashing, colour-blind-safe status | contrast tests over every theme and preset; scale and reduced-motion tests | M H E |
| **UI-P11** | Honest defaults | Dark patterns §7 | no network request, data sharing or nag without an explicit, understandable choice; **a feature that does not work yet is never presented as working**: it stays visible, disabled, with the red text "Feature not available yet!" and no placeholder data | no outgoing request at startup (test); every not-yet-implemented feature is registered as `NotYet` and appears in the generated list; a test fails on controls that are disabled without a reason or marker | M |
| **UI-P12** | Make the peak and the end good | Peak-end, aesthetic-usability §8 | invest in the moment the drive begins (loading → first frame) and in leaving a session (summary); polished first impression | reviewed with users in the usability test (M-0065) | M |
| **UI-P13** | **Driving-safe** *(added; not in the report)* | automotive HMI practice (to be sourced) | while driving the UI is glanceable: fixed positions, large tabular numerals, no layout shifts, no modal interruptions, no steal of input focus; everything else waits for a pause | HUD review checklist in [M-0064](M-0064.md); glance times measured in the usability test | H |

## 5. Open items

- **Sources** for the numbers used in acceptance criteria (touch/pointer target sizes, glance limits, feedback timing). The report does not provide them; the milestones cite them when they are collected.
- **Validation**: the report itself says rules are hypotheses. Phase 9 ends with a small moderated usability test ([M-0065](M-0065.md)), not only with checklists.
- **Editor density**: UI-P01's 32 px floor conflicts with dense editor panels; the editor uses a documented *dense* mode (≥ 24 px, the WCAG floor) instead of a silent exception.
