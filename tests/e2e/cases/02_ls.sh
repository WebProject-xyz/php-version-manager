#!/usr/bin/env bash
# pvm ls discovers the installed version with [cli, fpm] tags.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm ls — version discovery + package tags"
"$PVM_BIN" ls
"$PVM_BIN" ls | grep -q "$PREVIOUS" || fail "pvm ls missing $PREVIOUS"
"$PVM_BIN" ls | grep -E "$PREVIOUS.*cli.*fpm" >/dev/null \
    && ok "pvm ls shows [cli, fpm] for $PREVIOUS" \
    || warn "pvm ls did not show both [cli, fpm] tags"
