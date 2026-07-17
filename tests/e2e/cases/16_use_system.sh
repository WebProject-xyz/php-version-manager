#!/usr/bin/env bash
# `pvm use system` strips pvm-managed entries from PATH via the wrapper.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm use system — back to system PHP"

OUT=$(bash --norc --noprofile -c "
    eval \"\$('$PVM_BIN' env)\"
    pvm use $VFILTER >/dev/null 2>&1
    command -v php || true
    pvm use system >/dev/null 2>&1
    echo '---AFTER---'
    command -v php || true
    pvm current
" 2>&1)

BEFORE=$(echo "$OUT" | sed -n '1p')
echo "$BEFORE" | grep -q "/pvm/versions/" \
    || fail "pvm use $VFILTER did not put a pvm php on PATH (got: $BEFORE)"
ok "pvm use put php on PATH: $BEFORE"

AFTER=$(echo "$OUT" | sed -n '/---AFTER---/,$p')
if echo "$AFTER" | grep -q "/pvm/versions/"; then
    fail "pvm use system left a pvm versions entry on PATH: $AFTER"
fi
ok "no pvm versions entry on PATH after pvm use system"

echo "$AFTER" | grep -q "system" || fail "pvm current is not 'system' after deactivation"
ok "pvm current reports system"
