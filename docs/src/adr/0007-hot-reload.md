# ADR-0007: Hot Reload of OMSI Content

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0007 |
| **Status** | 🟡 Proposed |
| **Date** | 2026-10-10 |
| **Deciders** | Project lead, dev agent |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

Content authors and the dev loop both edit OMSI files (`.cfg`, `.bus`, `.sco`, `.sli`, `.map`, `.o3d`,
textures) and restart the game to see the effect, which costs minutes on large maps. OPTS loads OMSI content
through its own `ContentLoader`/`ContentCache` outside Bevy's `AssetServer`, so Bevy's asset hot reload does
not apply. Editors save in bursts, and intermediate saves are often broken files.

---

## 🎯 Decision

**OPTS will watch the files of the running session with one `notify`-based watcher, debounce events, validate
the new file off the main thread and only then replace the affected content.**

- Scope: meshes, textures, configs (`.cfg/.bus/.sco/.sli/.hum`) and map tiles. `.rs` hot reload is out of
  scope; an embedded scripting language is a separate decision.
- A failed parse keeps the previous state and logs a `WARN` (`ReloadFailed`).
- Reloads are also triggerable through the control protocol (`reload.*`).
- A reverse index `path → entities/materials` makes invalidation precise; tiles, vehicles and objects are
  respawned, textures are updated in place.
- The watcher is on by default for the session's own folders and can be disabled with `--no-hot-reload`.

---

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **Fast iteration** | Seconds instead of minutes per change for authors, agents and developers. |
| **One mechanism** | A single watcher and pipeline instead of per-type special cases. |
| **Robust against bad saves** | Validate-then-apply makes mid-edit files harmless. |
| **Reuses existing machinery** | Tile re-queueing, vehicle spawning and cache invalidation already exist. |

---

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **Bevy `AssetServer` hot reload** | OMSI content does not go through it. |
| **Polling modification times** | Latency and cost on large trees; `notify` is already a dependency of Bevy. |
| **Manual reload only** | Keeps the loop manual; kept as the `reload.*` commands. |
| **Dylib code reload** | Out of scope; much larger project. |
| **Embedded scripting now** | Needs its own sandbox/API design; deferred. |

---

## 🌊 Consequences

### ✅ Positive
- Edits show up in the running game; the control protocol and the watcher together close the agent loop.
- A broken edit never kills the session.

### ⚠️ Negative
- A reverse index must be maintained by every spawner; missing registrations mean stale content.
- Respawning loses transient entity state unless snapshotted (done for the vehicle only).
- File watching has platform quirks (inotify limits on very large trees).

### 🔄 Neutral
- The catalogue is not updated live; menus pick changes up on the next scan.

---

## 🔗 References

- [Hot Reload](../architecture/hot-reload.md)
- [Control Protocol](../architecture/control-protocol.md)
- [ADR-0006: Control Protocol](0006-control-protocol.md)
