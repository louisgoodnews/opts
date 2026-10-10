# 🛑 Universal Guardrails

**These rules apply to every prompt in this library.** They are
restated in each prompt for redundancy, but the agent should treat
them as invariant.

1. **Never inspect OMSI binaries.** No `OMSI.exe`, no DLLs, no
   decompilation, no Ghidra/IDA/dnSpy. See [ADR-0002](../src/adr/0002-clean-room-policy.md).

2. **Never copy GPL code.** Apply the "read, close, write" discipline
   when consulting community tools. Cite concepts, not code.

3. **Never distribute copyrighted assets.** No `.o3d`, `.sco`, `.dll`,
   or map files in git history.

4. **Documentation-Driven Development.** Write the spec before the
   code. If a spec doesn't exist, the code doesn't get written.

5. **Honest Open Questions.** A partial spec with honest unknowns is
   worth more than a complete spec built on guesswork.

6. **The ROADMAP is the source of truth.** Never invent scope. If
   the ROADMAP doesn't cover it, propose an addition via P-006.

---
