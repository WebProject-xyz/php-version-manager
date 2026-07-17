#!/usr/bin/env bash
# Non-interactive install: --packages skips the MultiSelect, rerun is idempotent.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

PVM_DIR="${PVM_DIR:-$HOME/.local/share/pvm}"

step "pvm install $LATEST --packages cli — non-interactive"

"$PVM_BIN" install "$LATEST" --packages cli --yes </dev/null
[[ -x "$PVM_DIR/versions/$LATEST/bin/php" ]] \
    || fail "php binary missing after --packages cli install"
ok "installed $LATEST with cli only, no prompt"

step "pvm install $LATEST again — idempotent without TTY"
OUT=$("$PVM_BIN" install "$LATEST" </dev/null)
echo "$OUT" | grep -q "already installed" \
    || fail "second install did not short-circuit (got: $OUT)"
ok "re-install short-circuits with 'already installed'"
