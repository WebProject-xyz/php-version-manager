#!/usr/bin/env bash
# `pvm current` reports the active version after `pvm use`.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm current — after pvm use"

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

cat > "$WORKDIR/run.sh" <<EOF
#!/bin/bash
set -e
export PVM_DIR='$HOME/.local/share/pvm'
eval "\$('$PVM_BIN' env)"
pvm use $VFILTER >/dev/null
pvm current
EOF
chmod +x "$WORKDIR/run.sh"

OUT=$(run_under_expect "$WORKDIR/run.sh" 2>&1)
echo "$OUT"
echo "$OUT" | grep -q "$PREVIOUS" || fail "pvm current did not show $PREVIOUS"
ok "pvm current → $PREVIOUS"
