#!/usr/bin/env bash
# `pvm prune -y` removes superseded patches and re-points the default version.
# Depends on 17_install_packages.sh having installed $LATEST next to $PREVIOUS.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

PVM_DIR="${PVM_DIR:-$HOME/.local/share/pvm}"

if [[ "$LATEST" == "$PREVIOUS" ]]; then
    warn "only one ${VFILTER}.x patch upstream — prune has nothing to do, skipping"
    OUT=$("$PVM_BIN" prune -y)
    echo "$OUT" | grep -qi "nothing to prune" || fail "expected 'Nothing to prune' (got: $OUT)"
    ok "pvm prune reports nothing to prune"
    exit 0
fi

step "pvm prune -y — $PREVIOUS superseded by $LATEST"

# Point the default at the doomed patch to verify the re-point.
"$PVM_BIN" default "$PREVIOUS"

"$PVM_BIN" prune -y

[[ ! -d "$PVM_DIR/versions/$PREVIOUS" ]] || fail "prune left superseded $PREVIOUS in place"
ok "superseded $PREVIOUS removed"

[[ -d "$PVM_DIR/versions/$LATEST" ]] || fail "prune removed the keeper $LATEST"
ok "keeper $LATEST still installed"

[[ "$(cat "$PVM_DIR/default")" == "$LATEST" ]] \
    || fail "default was not re-pointed to $LATEST (got: $(cat "$PVM_DIR/default" 2>/dev/null))"
ok "default re-pointed to $LATEST"

# Cleanup for the following cases.
"$PVM_BIN" default system
