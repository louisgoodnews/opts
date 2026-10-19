# 🚑 Repo Recovery and Integration Protocol

A previous session audited the repository and identified 7
worktrees containing unmerged work. Before it could save the
uncommitted work or merge the finished branches, the session
terminated.

Your task is to safely recover, validate, and merge this work.
You must prioritize **not losing data** before merging.

---

## 1. Context

- **Main branch:** `master` at `f942568`.
- **Clean branches ready to merge:**
  - `agent/vehicle-input` (3 commits)
  - `agent/bus-mirrors` (2 commits)
  - `docs/roadmap-fidelity-expansion` (2 commits)
- **Uncommitted WIP branches:**
  - `opts-wt-spawn-drift-jitter` (8 files, +1 new test)
  - `opts-wt-hamburg-visuals` (6 files, +1 new file)
  - `opts-wt-fog-look` (7 files)
- **Known Conflict:** `hamburg-visuals` and `fog-look` both edit
  `docs/src/architecture/lighting-system.md`.
- **Stale:** `~/.local/share/opts/benchmark/content-freeze/base`.

---

## 2. Phase A — Rescue the WIP (Uncommitted)

Do not merge anything yet. Do not run builds yet. First, secure the
uncommitted work so a crash or mistake doesn't destroy it.

For each of these three worktrees, enter the directory, stage all
changes (including untracked files), and commit them as WIP.

**A1. `opts-wt-spawn-drift-jitter`**
```bash
cd ../opts-wt-spawn-drift-jitter
git add .
git commit -m "wip: fix parked vehicle drift/jitter (unverified)"
```

**A2. `opts-wt-hamburg-visuals`**
```bash
cd ../opts-wt-hamburg-visuals
git add .
git commit -m "wip: hamburg visuals, glass, view scope (unverified)"
```

**A3. `opts-wt-fog-look`**
```bash
cd ../opts-wt-fog-look
git add .
git commit -m "wip: fog, look, lighting, sun rays (unverified)"
```

**Checkpoint A:** Report that all three WIP branches are committed.
Wait for user confirmation before proceeding.

---

## 3. Phase B — Merge the Clean Branches

These branches have finished commits and should merge cleanly.

**B1. Merge `agent/vehicle-input`**
- `git checkout master`
- `git merge --no-ff agent/vehicle-input`
- Run `./scripts/local-ci.sh`
- If CI passes, keep. If it fails, report the failure.

**B2. Merge `agent/bus-mirrors`**
- `git merge --no-ff agent/bus-mirrors`
- Run `./scripts/local-ci.sh`

**B3. Merge `docs/roadmap-fidelity-expansion`**
- `git merge --no-ff docs/roadmap-fidelity-expansion`
- Run `./scripts/local-ci.sh`

**Checkpoint B:** Report the merge results and CI status for all
three. Wait for user confirmation before proceeding.

---

## 4. Phase C — Merge and Validate the WIP Branches

These branches contain unverified code. They must be merged one at
a time, with tests run after each.

### C1. Merge `spawn-drift-jitter`
- `git merge --no-ff opts-wt-spawn-drift-jitter`
- Run `./scripts/local-ci.sh`
- **If it fails:** The code is unverified. Do not attempt to fix it
  yet. Abort the merge (`git merge --abort`), report the test
  failures, and defer to the user.
- **If it passes:** Keep the merge.

### C2. Merge `hamburg-visuals`
- `git merge --no-ff opts-wt-hamburg-visuals`
- Run `./scripts/local-ci.sh`

### C3. Merge `fog-look`
- `git merge --no-ff opts-wt-fog-look`
- **Expect a conflict in `docs/src/architecture/lighting-system.md`.**
- Resolve the conflict by combining the documentation from both
  branches (they likely cover different aspects of lighting: glass/
  scope vs. fog/sun).
- Run `./scripts/local-ci.sh`

**Checkpoint C:** Report the merges and any CI failures. Wait for
user confirmation before proceeding.

---

## 5. Phase D — Cleanup

Once all merges are complete and `master` is green:

1. Remove the 6 worktrees:
   `git worktree remove ../opts-wt-<name>` for each.
2. Delete the merged branches (optional, ask user):
   `git branch -d <branch-name>`
3. Investigate the stale benchmark worktree:
   `~/.local/share/opts/benchmark/content-freeze/base`.
   - Check if `M-0088` still references it.
   - If not, remove it.

---

## 6. Guardrails

- ❌ **Do not `git reset` or `git clean` in any worktree.** The
  goal is to save the work, not discard it.
- ❌ **Do not force push.**
- ❌ **Do not attempt to write new code** to fix failing tests
  during this session. Your job is integration, not development.
  If a WIP branch fails CI, abort it and report.
- ✅ **Preserve git history.** Use `--no-ff` for all merges.

---

## 7. Exit Criteria

- ✅ All 3 WIP branches are committed and saved.
- ✅ The 3 clean branches are merged to `master`.
- ✅ The 3 WIP branches are merged (or aborted with reported CI
  failures).
- ✅ `master` passes local CI.
- ✅ Worktrees are cleaned up.

When finished, report the final state of `master` and the exact
string:

"RECOVERY COMPLETE — repo consolidated"