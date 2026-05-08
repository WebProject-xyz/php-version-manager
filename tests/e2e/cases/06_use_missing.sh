#!/usr/bin/env bash
# `pvm use <missing-version>` prompts to install (#24); decline cancels cleanly.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm use $MISSING_VER (uninstalled) → install prompt → decline"

OUT=$(expect <<EXPECT_EOF 2>&1
set timeout 60
log_user 1
spawn $PVM_BIN use $MISSING_VER
expect {
    -re {is not installed locally.*Do you want to install} { send "n\r" }
    timeout { puts "TIMEOUT_MISSING_PROMPT" }
    eof     { puts "EOF_MISSING_PROMPT" }
}
expect eof
EXPECT_EOF
)
echo "$OUT"
echo "$OUT" | grep -q "is not installed locally" \
    || fail "missing-version install prompt not shown"
echo "$OUT" | grep -q "Operation cancelled" \
    || fail "decline did not cancel cleanly"
ok "missing-version prompt + decline path works (#24)"
