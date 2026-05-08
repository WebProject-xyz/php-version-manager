#!/usr/bin/env bash
# Custom php.ini via -c flag — assert ini_get values inside the FPM worker.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "-c php.ini effective inside worker (memory_limit, expose_php)"
RESPONSE=$(fcgi_call "$FPM_TCP_ADDR" \
    'echo "MEM=" . ini_get("memory_limit") . "\n"; echo "EXPOSE=" . (ini_get("expose_php") ? "On" : "Off") . "\n";' \
    2>&1)
echo "$RESPONSE"
echo "$RESPONSE" | grep -q "MEM=256M" \
    || fail "memory_limit not 256M inside worker — -c flag not effective"
echo "$RESPONSE" | grep -q "EXPOSE=Off" \
    || fail "expose_php not Off inside worker"
ok "php.ini applied inside worker"
