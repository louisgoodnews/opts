# ISS-051: Sharing and distribution of community content

| Field | Value |
| :--- | :--- |
| **Issue** | ISS-051. Research item RG-10 in the [feature report](../../audit/feature-research-omsi-lotus-2026-11-22.md), section 4 |
| **Severity** | minor |
| **Milestone** | [Phase 6](../../src/roadmap/phase-6/README.md) Modding Ecosystem; depends on [M-0051](../../src/roadmap/phase-6/M-0051.md) Editor Mode (E1 to E6 open, not started). Scheduled after the editor, per the owner 2026-11-26. |
| **Status** | open (proposed 2026-11-22 from the feature research; planned under Phase 6) |

## Problem

OPTS plans a modding ecosystem in Phase 6. The phase is planned but not started, and its first item (M-0031, modding architecture and packaging) is planned ([phase 6](../../src/roadmap/phase-6/README.md)). The vision says modders publish vehicles, maps and scenarios ([vision](../../src/vision.md), line 137).

The gap is the sharing and distribution layer, not the content formats. OMSI 2 has an add-on and mod ecosystem, and its Steam Workshop support is unverified. LOTUS lists a Steam Workshop for extra content and a level editor for sharing. Neither is used as a design source.

## Scope limit

Clean-room rule ([ADR-0002](../../src/adr/0002-clean-room-policy.md)). Packaging and distribution are specified for OPTS's own package format. No OMSI or LOTUS package, Workshop metadata or upload format is copied.

## Work

1. When Phase 6 is picked up, add a sharing and distribution section to the M-0031 scope. It should answer: where packages are hosted, how they are found, how they are verified, and whether a first version has any hosted sharing at all.
2. Record the first-version decision: manual file sharing only, or a hosted channel. A hosted channel needs a separate owner decision because it adds an external service.
3. Keep the Workshop question open until a source for OMSI's Workshop support is verified.

## Done when

- The M-0031 scope has a sharing and distribution section, and the first-version decision is recorded in the Phase 6 page.
