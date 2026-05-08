#!/usr/bin/env bash
# `pvm uninstall <ver>` removes the version dir and pvm ls drops the entry.
# Driver runs this AFTER fpm has been shut down.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm uninstall $PREVIOUS"

"$PVM_BIN" uninstall "$PREVIOUS"

if [[ ! -d "${PVM_DIR:-$HOME/.local/share/pvm}/versions/$PREVIOUS" ]]; then
    ok "versions/$PREVIOUS directory removed"
else
    fail "uninstall left versions/$PREVIOUS in place"
fi

if "$PVM_BIN" ls 2>&1 | grep -q "$PREVIOUS"; then
    fail "pvm ls still shows $PREVIOUS after uninstall"
fi
ok "pvm ls no longer lists $PREVIOUS"
