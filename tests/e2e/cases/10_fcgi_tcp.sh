#!/usr/bin/env bash
# FastCGI roundtrip over TCP — execute <?php echo "FCGI_OK"; ?>.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "FastCGI roundtrip over TCP 127.0.0.1:9000"
RESPONSE=$(fcgi_call "127.0.0.1:9000" 'echo "FCGI_OK\n";' 2>&1)
echo "$RESPONSE"
echo "$RESPONSE" | grep -q "FCGI_OK" || fail "TCP roundtrip did not return FCGI_OK"
ok "TCP FastCGI executed PHP"
