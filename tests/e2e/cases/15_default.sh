#!/usr/bin/env bash
# `pvm default <ver>` persists the version and `pvm env` activates it in new shells.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

PVM_DIR="${PVM_DIR:-$HOME/.local/share/pvm}"

step "pvm default $PREVIOUS — persist + env activation"

"$PVM_BIN" default "$PREVIOUS"
[[ "$(cat "$PVM_DIR/default")" == "$PREVIOUS" ]] \
    || fail "default file does not contain $PREVIOUS"
ok "default file written"

# Non-TTY `pvm default` prints the stored version.
OUT=$("$PVM_BIN" default)
echo "$OUT" | grep -q "$PREVIOUS" || fail "pvm default did not print $PREVIOUS"
ok "pvm default prints $PREVIOUS"

# A fresh shell that evals `pvm env` starts on the default version.
OUT=$(bash --norc --noprofile -c "eval \"\$('$PVM_BIN' env)\"; php -v" 2>&1)
echo "$OUT" | grep -q "$PREVIOUS" \
    || fail "new shell did not start on PHP $PREVIOUS (got: $OUT)"
ok "new shell starts on PHP $PREVIOUS via pvm env"

# `pvm default system` clears it again.
"$PVM_BIN" default system
[[ ! -f "$PVM_DIR/default" ]] || fail "default file still present after clearing"
ok "pvm default system cleared the default"
