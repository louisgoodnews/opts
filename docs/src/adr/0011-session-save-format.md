# ADR-0011: Session saves are a versioned container of independent sections

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0011 |
| **Status** | ✅ Accepted |
| **Date** | 2026-10-13 |
| **Deciders** | Project lead, Claude (implementation of [M-0069](../roadmap/phase-10/M-0069.md)) |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

OPTS needs to save a running drive and restore it ([M-0069](../roadmap/phase-10/M-0069.md), field report R-05). The state that belongs to a session will keep growing: weather, timetables, passengers, AI traffic, scripts. Every one of those arrives in a different milestone from a different crate. A save format that is one big struct, or a save system that knows what is in it, would be rewritten with every feature and would break every old save.

## 🎯 Decision

**We will save a session as one JSON file holding a small versioned container and any number of independent sections, each owned by the feature that registers it.**

* The **container** (`magic`, `format_version`, `opts_version`, `meta`, `fingerprint`, `sections`) is the only thing the save system understands. `sections` maps a section id to `{version, data}`; `data` is opaque JSON to the container.
* A **section registry** (`SaveRegistry`, same idea as the settings and HUD registries) holds one `SaveSection` per id: *capture* (world to JSON), *apply* (JSON to world), *migrate* (older version to current) and the stage in which it is applied. Feature code registers; the save system never names a section.
* **Compatibility rules.** A section unknown to this build is kept verbatim in memory and written back on an overwrite, but never interpreted. A section missing from the file is skipped, so the feature keeps its defaults. A section older than the build's version is migrated step by step. A section newer than the build's version is treated as unknown (kept, ignored, warned). A container with a newer `format_version` is refused with a clear error, never half-read.
* **Files.** `<user data dir>/saves/<id>.save.json`, written atomically (temporary file, `fsync`, rotate the old file to `<id>.save.json.bak`, rename into place). Reading falls back to the backup when the main file is missing or corrupt. Files above 16 MiB are refused.
* **Encoding.** Pretty-printed UTF-8 JSON, no compression.

The design is in [Session save](../architecture/session-save.md).

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **Features own their state** | A new feature adds a section in its own crate; no edit to the save system, the container or other sections. A test in the save module registers a section and proves it. |
| **Old saves keep working** | Per-section versions and migrations mean a change to the weather section never invalidates the vehicles. |
| **New saves survive old builds** | Unknown sections are preserved, so opening and re-saving with an older build does not destroy a newer build's data. |
| **Inspectable** | The project already stores `drive_history.json` and `viewpoints.json` as JSON, `serde_json` is a dependency, and a save is a bug-report attachment people can read and diff. |
| **Small enough not to compress** | Measured saves are a few kilobytes to tens of kilobytes (poses, counters, script variables of a handful of vehicles). Compression would cost inspectability and a dependency for nothing. A later section that is large (AI traffic) can compress its own `data`; the container does not change. |
| **Crash safe** | Temp file plus rename never leaves a half-written main file; the backup covers a crash between the two renames and a bad write. |

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **One `Save` struct with serde defaults** | Every feature edits it; unknown fields from newer builds are dropped on re-save; no per-feature migration. |
| **Binary (bincode, postcard)** | Not inspectable, no forward compatibility without extra machinery. |
| **RON or TOML** | The project's data files are JSON; RON/TOML give nothing here and handle large nested arrays worse. |
| **SQLite** | A save is one document; transactions and queries are not needed, and a file can be copied and attached. |
| **Compressed container (zstd, gzip)** | See rationale: saves are small; revisit when a section needs it. |
| **A save system that serialises the whole ECS world (reflection scene)** | Couples saves to component layouts, includes runtime entities and asset handles, and cannot migrate. |

## 🌊 Consequences

### ✅ Positive
- A new section is one `register_save_section` call; nothing else changes.
- Old saves load after sections change; newer saves are never destroyed by older builds.

### ⚠️ Negative
- Each feature must write a stable capture/apply pair and keep migrations; the registry cannot check that a section's JSON is sensible.
- JSON floats are decimal text: poses round-trip to `f32` precision, not bit-exact.

### 🔄 Neutral
- AI traffic and pedestrians are not saved; they are re-seeded from the restored map and time. That is a documented limit, and a `traffic` section can be added later without touching the container.

## 🔗 References

- [Session save](../architecture/session-save.md)
- [M-0069](../roadmap/phase-10/M-0069.md)
- [ADR-0009: UI architecture](0009-ui-architecture.md) (registries)
- [Session vehicles](../architecture/session-vehicles.md)
