# OPTS project audit, 2026-11-18

- Master commit: 295df09f
- Scope: architecture, process and records, control-server safety, parser robustness, dependencies and licensing
- Feature findings: docs/audit/feature-audit-2026-11-18.md (written separately; not present when this report was written). Feature findings are referenced below only where a project finding depends on them.
- Status: untracked, not committed. No other repo file was edited.

## Method and caveat

This audit was read-only. Source was read, and greps and the repository's own Python gates were used to confirm claims. Nothing was executed by this audit: no cargo build, no test run, no game run, no `cargo audit` or `cargo deny` against an advisory database (none is cached offline). Where a claim depends on a test failing or passing, it is marked NOT VERIFIED. Severity uses three levels: blocker, major, minor. Evidence is marked VERIFIED (checked by reading or a grep in this audit or a recorded gate run) or SUSPECTED (inferred, not run).

## Executive summary

1. The project's controls accepted a fix that did nothing. The trailer-front joint (3420545e) was merged on a green unrelated suite result, and it stayed inert for about a day (PRJ-01).
2. The control server is safe on loopback by default, but it cannot safely bind to a LAN. Failed token guesses are not throttled, the token check accepts an empty token, and connections are unbounded (PRJ-02, PRJ-07).
3. The OMSI boundary has not shrunk since the decision to shrink it (ADR-0014, 2026-10-19). The ratchet counts 180 uses at HEAD, the same number as at the ADR commit, and the largest crate sits outside the ratchet (PRJ-03, PRJ-05).
4. The parsers of untrusted OMSI files have no fuzz or property tests, and at least one text-parser path allocates from an unbounded file-supplied count (PRJ-04).
5. Branch and record hygiene is weak. Two unmerged branches carry duplicate work, one has uncommitted changes that exist nowhere else, 115 of 165 done items cite no commit, and the status of the milestone in progress differs across three files (PRJ-09 to PRJ-11).

## Scorecard

| Area | Status | Key evidence |
|---|---|---|
| OMSI boundary (engine crates) | Failing | 180 uses at HEAD, equal to the count at ADR-0014 acceptance; ratchet protects nothing (PRJ-03) |
| Structure ratchet | Acceptable | 0 regressions; 7 files exactly at 150 code lines; no total-line budget (PRJ-14) |
| Parser robustness | Weak | No fuzz or property tests; corpus and pseudo-random tests exist for most formats (PRJ-04) |
| Verification process | Failing | Inert fix accepted on a green unrelated suite; no fails-on-parent rule (PRJ-01) |
| Hook and local gates | Weak | One recorded `--no-verify` (e79a98ca); tests run locally only when opted in (PRJ-08) |
| Board and milestone records | Weak | 115 of 165 `[x]` items cite no hash; M-0053 has three different statuses (PRJ-10, PRJ-11) |
| Worktrees and branches | Weak | Unique unmerged work on a branch with dirty files; merged branches with dirty worktrees (PRJ-09) |
| Control server, loopback default | Acceptable | No token by default, reachable by any local account when enabled; opt-in (PRJ-06) |
| Control server, LAN bind | Failing | Unthrottled auth, empty token accepted, no read timeout, no connection cap (PRJ-02, PRJ-07) |
| Write paths and file safety | Acceptable | Capture prune is safe; symlink and save-path issues are minor (PRJ-16, PRJ-17) |
| Network use and telemetry | Sound | No HTTP or telemetry crate in Cargo.lock (grep); enforced only in one crate |
| OMSI installation read-only | Sound | Non-test writes go to OPTS-owned directories (grep, not exhaustive) |
| Unsafe code | Sound | One `unsafe` block (opts-game/src/entry.rs:46), with a SAFETY comment and a deny lint elsewhere |
| Dependencies and licensing | Acceptable | 787 locked packages; duplicate majors; advisories and font notices not checked (PRJ-18, PRJ-19) |

## Findings

### PRJ-01: inert fix accepted on an unrelated green test (BLOCKER, governance)

- Claim: the workspace suite result is accepted as evidence that a behaviour fix works.
- Evidence: the board, line 555, records stage 3 (3420545e, 2026-11-17) as merged on "Full workspace suite 2950 passed, 0 failed". The correction on the same line says the joint read unit 1, which does not exist on a one-trailer bus, and stayed at identity. The fix 17473f64 reads unit 0 and adds crates/opts-bevy/tests/trailer_front_live.rs (present; drives the articulated head and asserts the joint yaw). The fix also records 2951 passed. The correction is recorded in BOARD.md line 555 and in M-0053.md, which is good practice.
- Status: VERIFIED that the inert join and the correction exist in the record. NOT VERIFIED that the live test fails on the parent commit, as the fix commit message says. No cargo was run.
- Why it matters: a green suite does not show that a behaviour changed. The defect was live for a day and was fixed only because a later run looked at the bellow. The Definition of Done (docs/dev/BOARD.md lines 63-78) has no rule for behaviour or visual claims and no requirement for a test that fails on the parent commit.
- Next action: run the live test on 3420545e with the test file copied in, and record the failing output in the commit or board. Add a Definition of Done item: a behaviour fix names a test that fails on its parent commit, and a "verified" claim cites that test or a dated in-game check. "Suite passed" is not accepted as evidence for a fix. See the Governance section.

### PRJ-02: control server cannot safely bind to a LAN (BLOCKER for any non-loopback bind; latent while loopback-only)

- Claim: failed token guesses are never throttled, and the token check is presence-only.
- Evidence: crates/opts-bevy/src/control/server/dispatch.rs:75-77 and 119-120 return from `auth` before `gate()` and the limiter, so failed `auth` attempts are never limited. server.rs:64 checks only `token.is_none()` for a non-loopback bind, so an empty or short token is accepted. server.rs:70 sends the token in plain TCP.
- Impact: with `--cli-bind 0.0.0.0` and a weak token, a LAN peer can brute-force the token and then run `shutdown`, input and UI injection, `screenshot`, `log.tail`, `session.delete_save`, config, profile and formation deletes, and `capture.prune` with `dry_run:false`.
- Status: VERIFIED by code reading. Not exploited or run.
- Next action: reject empty and short tokens (at least 32 random characters). Throttle failed `auth` per peer and globally. Put `auth` under the same limiter. Until these are done, document that a non-loopback bind needs an SSH tunnel or TLS, or refuse it.

### PRJ-03: the OMSI boundary has not shrunk since ADR-0014 (MAJOR)

- Claim: the engine crates still use OMSI types at the same rate as when the decision to shrink them was accepted.
- Evidence: scripts/omsi_uses.py at HEAD reports opts-bevy 130, opts-sim 31, opts-traffic 14, opts-catalogue 5, opts-gui 0, total 180. The architecture audit reports the identical per-crate counts at commit 1e155242, the ADR-0014 acceptance commit (2026-10-19), across 1157 commits. The jump from 55 to 130 in docs/dev/omsi-uses-known.txt (commit e709b80e) is a change of counting method, not growth. opts-sim depends on the facade opts-core, which re-exports opts_omsi::{formats, setvars}. Traffic leaks through crates/opts-traffic/src/graph/lane_rules.rs and source.rs. The native model is still a stub: opts-model is 168 lines, opts-importer 83, opts-core 20.
- Status: counts VERIFIED by the architecture audit's run of the gate and its counts at 1e155242. Not re-run here.
- Why it matters: the ratchet only blocks growth, and growth has been zero, so the ratchet has not been tested against a real target. Without a burn-down target, the migration to native types has no end date.
- Next action: set a dated burn-down target per crate, starting with opts-sim and opts-traffic. Route each format through one adapter instead of scattering `formats::` imports. Make the ratchet fail when a phase makes no progress. Remove the facade dependency from opts-sim.

### PRJ-04: no fuzz or property testing for untrusted OMSI parsers (MAJOR)

- Claim: the parsers that read game-supplied files have no fuzz target and no property tests, and at least one allocates from file-supplied counts.
- Evidence: there is no fuzz directory and no proptest, quickcheck or arbitrary dependency in crates/. opts-omsi has about 105 index or slice expressions and 70 `as usize` casts in non-test code. Some guards exist: map/terrain.rs:33, map/water.rs:48, and the o3d header version check at o3d/header.rs:64. Pseudo-random tests cover operational, x, world, model and o3d. Corpus tests cover hof, osn, sli, timetable, tile rules, weather and water. The text X reader calls `Vec::with_capacity(n_vertices)` at formats/x/parse/mesh.rs:22 and `with_capacity(n_faces)` at line 32, with no bound. The binary X reader has `MAX_LIST = 50_000_000` (x/binary.rs:26, checked at line 104); the text reader does not.
- Status: guards and missing tests VERIFIED. The allocation abort is SUSPECTED until a test with a large count is run. A count near 4 billion would request tens of gigabytes, and Rust aborts on allocation failure, so it is a crash rather than a panic.
- Next action: apply the `MAX_LIST` bound, or bound the count by the remaining token count, to the text X path. Add a huge-count test. Add a cargo-fuzz target for .x, .o3d, .bus and the map formats, and run it when the shared build lock is free.

### PRJ-05: the largest crate is outside the boundary ratchet (MAJOR)

- Claim: opts-game holds 48% of workspace Rust lines and is exempt from the OMSI ratchet as a composition root, so the real translation layer is not measured.
- Evidence: opts-game is 109,703 of 230,065 workspace .rs lines. scripts/omsi_uses.py excludes it. 199 opts-game files reach opts_omsi or opts_core::formats. The OMSI-to-engine translation is in crates/opts-game/src/in_game/omsi_world/.
- Status: VERIFIED by the architecture audit's greps; line counts from the audit.
- Next action: extend the ratchet to in_game/omsi_world, or move that tree into an adapter crate, and record its count.

### PRJ-06: loopback control access needs no token by default (MAJOR when enabled; opt-in)

- Claim: any local account can drive the game while the control server is on.
- Evidence: the ControlConfig default is loopback with `token: None` (crates/opts-bevy/src/control/mod.rs:40-48). The game starts the server only with `--cli-port` (crates/opts-game/src/app.rs:108-115). TCP on 127.0.0.1 has no per-user restriction. The commands are the same as in PRJ-02, and they run with the victim's file rights.
- SUSPECTED, not tested: a web page could send a text/plain POST to 127.0.0.1. The reader is line-based and has no HTTP parsing (connection.rs:15-27), so a JSON line in a body could run. Browser private-network rules may block this.
- Next action: require a token on loopback too, or use a Unix socket with mode 0600. Reject any first line that is not JSON-RPC. Test the browser case directly before concluding either way.

### PRJ-07: unbounded connections and no read timeout (MAJOR)

- Evidence: the server sets no read timeout. `set_read_timeout` appears only in the client (opts-cli/src/transport.rs:61-62). connection.rs:33-50 loops until EOF. server.rs:83-86 spawns a thread per connection with no cap. The stop flag cannot wake a thread blocked in `read_line`.
- Impact: a few hundred silent connections each hold a thread and a file descriptor.
- Status: VERIFIED by reading. Not run.
- Next action: a read timeout (for example 30 seconds idle), a concurrency cap (for example 8), and shutdown of the socket when stopping.

### PRJ-08: hook bypass and local test gate (MAJOR)

- Evidence: e79a98ca (docs(m0053)) records "Hook skipped: docs-only change; the non-cargo gates were run by hand." The commit does not name `--no-verify`, and no record exists of the by-hand run. The architecture audit re-ran the six Python gates on master and all pass, so the commit is currently clean. The hook (scripts/pre-commit) runs the test suite only when OPTS_PRECOMMIT_TESTS=1 (line 61), so DoD item 2 is enforced locally only when opted in, and in CI always. Nothing in the repository forbids `--no-verify`.
- Next action: have the hook log every skip with a reason, and require the reason in the commit body. Run the test suite by default for commits that touch crates/.

### PRJ-09: unmerged duplicate work and dirty worktrees (MAJOR)

- Evidence: agent/pitchfix (12254a0e) and agent/residual (4a353125) are unmerged. Each carries one unique commit with the same subject, "test(opts-game): measure the trailer-front seam residual under pitch and roll", and their trees differ. Neither appears on the board. The pitchfix worktree has five modified trailer-joint files uncommitted; that is the only copy of that work. Merged branches still have worktrees with uncommitted changes (bellow, sign, doors). A worktree directory (opts-wt-trackdoc) is absent from `git worktree list`. Eight branches are ancestors of master and can be deleted after the worktree check.
- Status: VERIFIED by the process audit's git reads.
- Next action: the owner chooses between pitchfix and residual. Commit the five dirty files to a branch before anything else. Commit or discard the other dirty changes, then remove those worktrees, run `git worktree prune`, and delete merged branches. Move docs/audit out of the shared checkout or commit it.

### PRJ-10: most done items cite no evidence (MAJOR, records)

- Evidence: docs/dev/BOARD.md has 165 `[x]` lines. 115 cite no commit hash. 55 of those name a branch instead, and branch names are not durable once deleted or rewritten. The 99 distinct hashes cited all exist, and the process audit reports that the sampled subjects match their claims (its own list names 14 hashes against a stated sample of 15, so the sample is not fully itemised). The hashes are honest; the gap is the uncited items. "Tests pass" was the sole evidence in very few lines.
- Next action: every Done item cites a commit hash. roadmap_audit should fail a `[x]` line with no hash.

### PRJ-11: status drift and work-in-progress overload (MAJOR, records)

- Evidence: docs/src/roadmap/phase-4/README.md line 44 says M-0053 is Planned. M-0053.md line 7 says In progress. The board lists M-0053 under Backlog. In Progress and Review together hold 17 items against a WIP target of 1. The health table in the board is a 2026-10-29 snapshot and was not refreshed. roadmap_audit passes with five findings, all waived. The waiver list was not checked. ROADMAP.md percentages were not checked against milestone counts.
- Next action: correct the phase-4 README status. Refresh the health table. Cap In Progress at the stated limit, or change the stated limit.

### PRJ-12: production panic on a per-frame path (MINOR)

- Evidence: crates/opts-game/src/in_game/omsi_world/rail/player_track.rs:22 `expect("lane exists")`. The lane comes from `net.0.nearest` on the same snapshot, so a race is SUSPECTED unlikely. Tile streaming could still make it panic. The function already returns false for the empty-network case at line 15.
- Other production expect calls checked (profile store, scramble.rs:11) are acceptable.
- Next action: `let Some(l) = ... else { return false };`.

### PRJ-13: test support ships in the production library (MINOR)

- Evidence: `pub mod testing;` at crates/opts-game/src/catalogue_service.rs:146 is not gated by cfg(test). It builds a fake OMSI installation. There are 25 `#[allow]` attributes in the workspace, including non-test dead_code allows at crates/opts-bevy/src/control/mod.rs:57 and crates/opts-game/src/help/controls.rs:16.
- Next action: gate `testing` behind `cfg(any(test, feature = "test-support"))`. Remove or justify the two dead-code allows.

### PRJ-14: structure ratchet at its ceiling (MINOR)

- Evidence: `structure_audit.py check` passes with zero exceptions and zero regressions, after tightening in commits 52df5197 and 17d8a78b (2026-11-16). 7 files sit at exactly 150 code lines and 78 are at or above 140. The limit counts code lines only, so a 150-code-line file can be about 220 lines. Functions at 80 lines include update_clouds, build (opts-traffic/src/graph/build.rs) and compute_neighbours.
- Next action: add a total-line budget. Target 40 lines on the top 20 functions.

### PRJ-15: token on the command line and in history (MINOR-MAJOR)

- Evidence: `--cli-token` is a plain argument (crates/opts-cli/src/args.rs:107). It is visible in /proc to other local users unless hidepid is set, and it lands in shell history. The `auth` command in the client takes the token as a positional argument (opts-cli/src/commands/table/basics.rs:19-20).
- Next action: read the token from an environment variable or a 0600 file. Do not echo it in help text or logs.

### PRJ-16: symlink followed at the write target (MINOR)

- Evidence: check_write in crates/opts-bevy/src/control/paths.rs:56 canonicalises the parent only, so an existing symlink at the target name is followed. Callers include viewpoint export (opts-game/src/viewpoints/protocol/transfer.rs:22), trace export and drive snapshot. The reports writer is unaffected (create_new at reporting/save.rs:50-60).
- Status: code path VERIFIED. Exploitation needs a prior write into an allowed directory, so impact is low.
- Next action: refuse a symlink target in check_write, or open with O_NOFOLLOW.

### PRJ-17: save-file paths used without a single-component check (MINOR)

- Evidence: opts-game/src/save/sections/session.rs:80 joins `map_folder` from save content with no check for `..` or absolute paths. save/sections.rs:58-70 uses an absolute `saved` path if it names a file. The effect is reading and hashing files outside the installation. No writes.
- Next action: require `map_folder` to be one normal path component, and reject absolute paths from saves.

### PRJ-18: dependency health (MINOR; advisory status unknown)

- Evidence: 787 locked packages. Duplicate majors: syn (three), ron, toml, thiserror, rand, notify, nix, hashbrown (four), windows-sys (six). deny.toml sets multiple-versions to warn. cargo-audit is not installed, and no advisory database is cached, so no advisory check was made. The network-crate grep of Cargo.lock found no HTTP client. tokio appears only as a transitive entry.
- Next action: run `cargo deny check advisories` in CI or when network access is available. Consider consolidating the build-time duplicates (syn, thiserror).

### PRJ-19: licensing notices (MINOR; not checked)

- Evidence: six cached crates outside the deny.toml allow-list are not in Cargo.lock. Bundled fonts ship licence files. It is not checked whether the binary shows the font notices in an About or credits screen.
- Next action: confirm the binary shows the notices, or add them to the documentation.

### PRJ-20: agent rules live outside the repository (MINOR, records)

- Evidence: the repository has no CLAUDE.md. Working rules (separate worktrees, sequential builds, no /tmp, no stash in worktrees, board updates at start and end) live in personal memory and docs/dev/PROMPTS.md. The board header (line 3) is a run-on paragraph of about 3,000 characters and belongs in the changelog section. Board status markers are mixed.
- Next action: move the shared rules into a repository CLAUDE.md or the Definition of Done. Move the header into the changelog.

## Governance: why the controls missed the stage-3 failure

The stage-3 joint was merged with a passing suite, and the suite did not test the joint. The project's Definition of Done requires passing tests and clippy, and it has no rule that a fix must be shown to change behaviour. Three things would have caught it:

1. A non-inert control: every behaviour fix names a test, and the test must fail on the parent commit. For 3420545e, that is the live test now in 17473f64, run against the parent.
2. A dated in-game check with a named control for anything visual or physical. The review checklist (docs/dev/review-checklist.md) exists but is not linked from the Definition of Done.
3. A rule that a suite result is not evidence for a fix.

The same gap shows up in the other instances. e79a98ca skipped the hook with no record of the by-hand gate run. The pitchfix and residual branches contain the same result in two unmerged copies that the board does not mention. 115 `[x]` items have no hash. M-0053 is Planned, In progress and Backlog in three files. Each instance is a case where a claim was recorded without a check that would fail if the claim were false.

## Security posture: control server

Safe by default:
- The server is off unless `--cli-port` is given (opts-game/src/app.rs:108-115).
- It binds to loopback by default, and a non-loopback bind is refused without a token (server.rs:64). The token check is weak (PRJ-02).
- Capture pruning is dry-run by default and deletes only under the canonical captures root.
- Non-test code writes only to OPTS-owned directories, and the control write policy excludes the OMSI root.

Unsafe if enabled on a LAN:
- Failed auth is not throttled, the token may be empty or short, and the token travels in plain TCP. A LAN peer can brute-force and then control the game (PRJ-02).
- No read timeout or connection cap, so idle clients exhaust threads and descriptors (PRJ-07).

Unsafe on a shared machine even on loopback:
- No token is required by default, and any local account can connect while the server runs (PRJ-06). The token flag appears in the process list (PRJ-15).

Minimum before any non-loopback bind:
1. Refuse empty and short tokens. Require at least 32 random characters.
2. Throttle `auth` per peer and globally, under the same limiter as other commands.
3. Add a read timeout and a connection cap.
4. Require a token on loopback as well, or use a 0600 Unix socket.
5. Move the token out of argv.
6. Either document TLS or an SSH tunnel as the only supported remote path, or refuse non-loopback binds.

## Recommended order of work

1. Run the live trailer test on the parent commit and record the result (PRJ-01). Cheapest, and it tests the central claim.
2. Refuse non-loopback binds until PRJ-02 and PRJ-07 are fixed. This is a one-line guard.
3. Clean the worktrees and branches, and commit the five dirty pitchfix files to a branch (PRJ-09). This prevents loss of work.
4. Add the Definition of Done rule for behaviour fixes, and the hash requirement for `[x]` items (PRJ-01, PRJ-10).
5. Fix the M-0053 status and the In Progress count (PRJ-11).
6. Bound the text X allocation and add a huge-count test (PRJ-04).
7. Add a fuzz target for the entry parsers.
8. Set a dated burn-down target for the OMSI boundary and extend the ratchet to opts-game's omsi_world tree (PRJ-03, PRJ-05).
9. Make the smaller fixes: the player_track expect, the testing gate, the symlink check, and the save-path check (PRJ-12, PRJ-13, PRJ-16, PRJ-17).
10. Run cargo deny for advisories and confirm the licence notices (PRJ-18, PRJ-19).

## Sources

- Project auditor reports, architecture, process and safety, from the audit scratch folder (opts-scratch/audit). Their claims were checked against the repository as noted above.
- Repository files: docs/dev/BOARD.md (lines 3, 63-78, 555), docs/src/roadmap/phase-4/README.md (line 44), docs/dev/M-0053.md and docs/dev/omsi-uses-known.txt, scripts/omsi_uses.py, scripts/pre-commit (line 61), crates/opts-bevy/src/control/server.rs (lines 64, 70, 83-87), crates/opts-bevy/src/control/server/dispatch.rs (lines 75-79, 115-122), crates/opts-bevy/src/control/mod.rs (lines 40-48), crates/opts-bevy/src/control/paths.rs (line 56), crates/opts-bevy/tests/trailer_front_live.rs, crates/opts-game/src/app.rs (lines 108-115), crates/opts-game/src/catalogue_service.rs (line 146), crates/opts-game/src/in_game/omsi_world/rail/player_track.rs (line 22), crates/opts-omsi/src/formats/x/parse/mesh.rs (lines 22, 32), crates/opts-omsi/src/formats/x/binary.rs (lines 26, 104), crates/opts-cli/src/args.rs (line 107), git branch and worktree listings, and Cargo.lock.
- Not used: any cargo output, test output, game run, or advisory database.
