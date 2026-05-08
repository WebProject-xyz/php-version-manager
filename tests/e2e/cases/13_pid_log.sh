#!/usr/bin/env bash
# Pid file written + error log present.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pid file + error log"

[[ -s "$FPM_PID_FILE" ]] || fail "pid file $FPM_PID_FILE not written"
PID_IN_FILE=$(cat "$FPM_PID_FILE")
EXPECTED_PID=$(cat "$E2E_STATE/fpm.pid")
[[ "$PID_IN_FILE" -eq "$EXPECTED_PID" ]] \
    && ok "pid file matches master ($PID_IN_FILE)" \
    || warn "pid file $PID_IN_FILE != master $EXPECTED_PID"

[[ -s "$FPM_LOG_FILE" ]] && ok "error log $FPM_LOG_FILE has content" \
    || warn "error log empty (ok if startup was quiet)"
