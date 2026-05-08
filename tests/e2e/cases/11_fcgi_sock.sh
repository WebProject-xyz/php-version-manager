#!/usr/bin/env bash
# FastCGI roundtrip over Unix socket.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "FastCGI roundtrip over $FPM_SOCK"
RESPONSE=$(fcgi_call "$FPM_SOCK" 'echo "FCGI_OK\n";' 2>&1)
echo "$RESPONSE"
echo "$RESPONSE" | grep -q "FCGI_OK" || fail "unix socket roundtrip failed"
ok "Unix socket FastCGI executed PHP"
