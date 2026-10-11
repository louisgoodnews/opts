#!/usr/bin/env bash
# Creates a separate work station (git worktree + branch) for one agent session.
# Usage: scripts/agent-worktree.sh <topic> [base-branch]
# Result: ../opts-wt-<topic> on branch agent/<topic>, based on <base-branch> (default: the
# branch checked out in the main worktree). See CONTRIBUTING.md, "AI Agent Sessions".
set -euo pipefail
topic=${1:?usage: scripts/agent-worktree.sh <topic> [base-branch]}
root=$(git rev-parse --show-toplevel)
main=$(git -C "$root" worktree list --porcelain | awk '/^worktree /{print $2; exit}')
base=${2:-$(git -C "$main" rev-parse --abbrev-ref HEAD)}
dir="$(dirname "$main")/opts-wt-$topic"
branch="agent/$topic"
if [ -e "$dir" ]; then
  echo "error: $dir already exists" >&2
  exit 1
fi
git -C "$main" worktree add "$dir" -b "$branch" "$base"
echo
echo "Work station ready: $dir (branch $branch, from $base)."
echo "Build and commit there only. When done: merge $branch, then"
echo "  git worktree remove \"$dir\" && git branch -d $branch"
