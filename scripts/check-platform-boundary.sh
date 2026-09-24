#!/usr/bin/env bash
set -euo pipefail

# Platform-specific behaviour must stay behind the platform adapter.
# This check fails when `cfg(target_os ...)` appears outside platform.rs, because
# operating-system differences belong in the adapter, not in domain code.

search_root="src-tauri/src"
adapter="src-tauri/src/platform.rs"

if command -v rg >/dev/null 2>&1; then
    violations="$(rg -n --glob '!**/platform.rs' 'cfg\(target_os' "$search_root" || true)"
else
    violations="$(grep -rn --exclude=platform.rs 'cfg(target_os' "$search_root" || true)"
fi

if [[ -n "$violations" ]]; then
    echo "Platform-boundary violation: cfg(target_os ...) found outside ${adapter}:" >&2
    echo "$violations" >&2
    exit 1
fi

echo "Platform boundary intact: no cfg(target_os ...) outside ${adapter}."
