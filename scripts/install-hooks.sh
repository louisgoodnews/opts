#!/usr/bin/env bash
# Installs the repository's Git hooks: points `core.hooksPath` of THIS repository (every worktree
# of it) at `scripts`, where `pre-commit` lives (M-0052). Undo: `git config --unset core.hooksPath`.
set -euo pipefail
ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"
chmod +x scripts/pre-commit
git config core.hooksPath scripts
echo "Hooks installed: core.hooksPath = scripts (pre-commit checks the staged tree)."
echo "Set OPTS_PRECOMMIT_TESTS=1 to run the tests in the hook too."
