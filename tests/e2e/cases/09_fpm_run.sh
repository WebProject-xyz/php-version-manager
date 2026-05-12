#!/usr/bin/env bash
# Start php-fpm in foreground; verify TCP + unix socket listeners come up.
# Leaves the master running for fcgi roundtrip cases; driver kills on exit.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "Start php-fpm -F; verify listeners"

rm -f "$FPM_PID_FILE" "$FPM_LOG_FILE" "$FPM_SOCK"

"$VDIR/php-fpm" \
    -c "$HOME/.config/php-fpm/php.ini" \
    -y "$HOME/.config/php-fpm/php-fpm.conf" \
    -F \
    > /tmp/fpm.stdout 2> /tmp/fpm.stderr &

FPM_PID=$!
echo "$FPM_PID" > "$E2E_STATE/fpm.pid"

TCP_HOST="${FPM_TCP_ADDR%:*}"
TCP_PORT="${FPM_TCP_ADDR##*:}"

for i in $(seq 1 60); do
    if nc -z "$TCP_HOST" "$TCP_PORT" 2>/dev/null && [[ -S "$FPM_SOCK" ]]; then
        ok "fpm listening on TCP $FPM_TCP_ADDR + unix $FPM_SOCK (after ${i}*100ms)"
        break
    fi
    sleep 0.1
done

if ! nc -z "$TCP_HOST" "$TCP_PORT" 2>/dev/null; then
    echo "--- stdout ---"; cat /tmp/fpm.stdout || true
    echo "--- stderr ---"; cat /tmp/fpm.stderr || true
    fail "TCP $FPM_TCP_ADDR not listening"
fi
[[ -S "$FPM_SOCK" ]] || fail "unix socket $FPM_SOCK not created"

WORKER_COUNT=$(pgrep -P "$FPM_PID" 2>/dev/null | wc -l || true)
if [[ "$WORKER_COUNT" -ge 2 ]]; then
    ok "$WORKER_COUNT workers spawned"
else
    warn "only $WORKER_COUNT workers detected"
fi
