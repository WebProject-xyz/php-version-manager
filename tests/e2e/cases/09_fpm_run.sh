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

for i in $(seq 1 60); do
    if nc -z 127.0.0.1 9000 2>/dev/null && [[ -S "$FPM_SOCK" ]]; then
        ok "fpm listening on TCP :9000 + unix $FPM_SOCK (after ${i}*100ms)"
        break
    fi
    sleep 0.1
done

if ! nc -z 127.0.0.1 9000 2>/dev/null; then
    echo "--- stdout ---"; cat /tmp/fpm.stdout || true
    echo "--- stderr ---"; cat /tmp/fpm.stderr || true
    fail "TCP :9000 not listening"
fi
[[ -S "$FPM_SOCK" ]] || fail "unix socket $FPM_SOCK not created"

WORKER_COUNT=$(pgrep -P "$FPM_PID" 2>/dev/null | wc -l || echo 0)
[[ "$WORKER_COUNT" -ge 2 ]] && ok "$WORKER_COUNT workers spawned" \
    || warn "only $WORKER_COUNT workers detected"
