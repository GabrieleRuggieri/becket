#!/usr/bin/env bash
# One-time setup: point Git at the repo-managed hooks in .githooks/
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

chmod +x .githooks/pre-commit
git config core.hooksPath .githooks

echo "Git hooks attivi: core.hooksPath=.githooks"
echo "Il pre-commit esegue 'cargo fmt --all' sui file .rs in stage."
