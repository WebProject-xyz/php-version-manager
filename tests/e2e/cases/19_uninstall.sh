#!/usr/bin/env bash
# `pvm uninstall <ver>` removes the version dir and pvm ls drops the entry.
# Runs last: 18_prune keeps only $LATEST, so that is what gets removed here.
# Driver runs this AFTER fpm has been shut down.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm uninstall $LATEST"

"$PVM_BIN" uninstall "$LATEST"

if [[ ! -d "${PVM_DIR:-$HOME/.local/share/pvm}/versions/$LATEST" ]]; then
    ok "versions/$LATEST directory removed"
else
    fail "uninstall left versions/$LATEST in place"
fi

if "$PVM_BIN" ls 2>&1 | grep -q "$LATEST"; then
    fail "pvm ls still shows $LATEST after uninstall"
fi
ok "pvm ls no longer lists $LATEST"
