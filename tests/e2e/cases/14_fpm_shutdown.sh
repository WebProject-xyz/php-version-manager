#!/usr/bin/env bash
# Send SIGQUIT to the fpm master and wait for clean exit.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "SIGQUIT shutdown"

if [[ ! -f "$E2E_STATE/fpm.pid" ]]; then
    warn "no fpm pid recorded — assuming already stopped"
    exit 0
fi

FPM_PID=$(cat "$E2E_STATE/fpm.pid")
kill -QUIT "$FPM_PID" 2>/dev/null || true

for i in $(seq 1 50); do
    if ! kill -0 "$FPM_PID" 2>/dev/null; then
        ok "fpm master $FPM_PID exited cleanly (after ${i}*100ms)"
        rm -f "$E2E_STATE/fpm.pid"
        exit 0
    fi
    sleep 0.1
done

fail "fpm master $FPM_PID still alive after 5s"
