# ADR-0006: Control Protocol — JSON-RPC over Loopback TCP

| Metadata | Value |
| :--- | :--- |
| **ADR ID** | 0006 |
| **Status** | 🟡 Proposed |
| **Date** | 2026-10-10 |
| **Deciders** | Project lead, dev agent |
| **Supersedes** | None |
| **Superseded by** | None |

---

## 📌 Context

Development of OPTS is a loop: change something, start the game, look, press keys, take a screenshot, read
the log. Every turn of that loop needs a person at the keyboard, which also makes it impossible for an AI
agent to iterate on the running application. The game needs a way to be observed and driven from outside.
Requirements: works on Linux and Windows without administrator rights; usable from a shell, scripts and agents;
cannot stall rendering; cannot become a remote-code-execution hole; small enough to maintain. The engine
(Bevy 0.14) offers no built-in remote protocol (`bevy_remote` arrives in 0.15).

---

## 🎯 Decision

**OPTS will expose an opt-in control server speaking JSON-RPC 2.0, one message per line, over TCP bound to
`127.0.0.1`.**

- Off unless `--cli-port <N>` is passed (`0` = ephemeral port, logged); default port `44201`.
- Non-loopback binds need both `--cli-bind` and `--cli-token`; the token is checked by an `auth` call.
- The server runs on plain `std` threads (one listener, one per connection); it communicates with Bevy only
  through channels, drained by one system per frame. No async runtime is added.
- Methods are specified in [Control Protocol](../architecture/control-protocol.md); results and error codes
  are the public contract.
- The client is the `opts-cli` binary (a `client` feature of the existing `opts-cli` crate).
- Filesystem access is restricted to the OMSI root, the reports directory and `--cli-allow-dir` directories.

---

## 🧠 Rationale

| Reason | Explanation |
| :--- | :--- |
| **Standard, self-describing** | JSON-RPC 2.0 is understood by agents and tools; errors have codes. |
| **Debuggable** | Line-delimited JSON works with `nc` and `jq`; no schema compiler needed. |
| **Cross-platform** | TCP on loopback needs no named-pipe/UDS split and no admin rights. |
| **No render stalls** | Threads plus channels keep socket I/O off the frame loop. |
| **Small** | Hand-written framing on `serde_json` is a few hundred lines and adds no heavy dependency. |
| **Safe by default** | Opt-in, loopback, path-restricted; no command runs code. |

---

## ⚖️ Alternatives Considered

| Alternative | Reason for Rejection |
| :--- | :--- |
| **Unix domain socket / named pipe** | Two implementations; awkward on Windows. |
| **stdin/stdout sidecar** | One client only; collides with log output. |
| **HTTP server** | Heavier than needed; no streaming advantage for v1. |
| **WebSocket** | Upgrade handshake and a dependency for a feature (push events) deferred past v1. |
| **gRPC** | Code generation and proto files for a debugging interface. |
| **MessagePack** | Opaque to humans and agents. |
| **`tokio` server** | A second runtime next to Bevy's task pools for ~5 connections; threads are enough. |
| **`bevy_remote`** | Not available on Bevy 0.14; can later back the introspection methods (§10 of the protocol doc). |

---

## 🌊 Consequences

### ✅ Positive
- Agents and humans can observe and drive a live game; screenshots, reports and reloads become scriptable.
- Input injected as Bevy events exercises the real input paths, which makes it a regression-test tool too.

### ⚠️ Negative
- A new attack surface (mitigated: opt-in, loopback, path checks, no code execution).
- Introspection (`entity.get`) is shallow until components derive `Reflect` or the engine is upgraded.

### 🔄 Neutral
- The protocol version becomes a compatibility promise; additive changes only within a major version.
- One more `opts-cli` responsibility: it is both the game's argument parser and the control client.

---

## 🔗 References

- [Control Protocol](../architecture/control-protocol.md)
- [ADR-0004: Unified Logging Architecture](0004-logging-architecture.md)
- [ADR-0005: Workspace Crate Architecture](0005-crate-architecture.md)
- [JSON-RPC 2.0 specification](https://www.jsonrpc.org/specification)
