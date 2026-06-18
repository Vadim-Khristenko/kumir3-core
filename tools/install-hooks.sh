#!/usr/bin/env bash
# =============================================================================
#         KITE :: HOOK INSTALLER (bash)
# =============================================================================
# Points git at the version-controlled .githooks/ directory and makes the
# hooks executable. Run once after cloning:  bash tools/install-hooks.sh
# =============================================================================

set -euo pipefail

cd "$(dirname "$0")/.."

git config core.hooksPath .githooks
chmod +x .githooks/* 2>/dev/null || true

echo "✔ KITE git hooks installed (core.hooksPath -> .githooks)."
