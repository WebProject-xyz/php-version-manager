#!/usr/bin/env bash
# Patch-update detection: `pvm use <minor>` offers the newer patch.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

if [[ "$LATEST" == "$PREVIOUS" ]]; then
    warn "only one ${VFILTER}.x patch upstream — skipping patch-update detection"
    exit 0
fi

# pvm rate-limits the patch-update check to once per 24h via this guard file;
# clear it so the prompt actually fires when this case runs after another `pvm use`.
rm -f "$HOME/.local/share/pvm/.update_check_guard"

step "pvm use $VFILTER offers $LATEST over $PREVIOUS"

OUT=$(expect <<EXPECT_EOF 2>&1
set timeout 90
log_user 1
spawn $PVM_BIN use $VFILTER
expect {
    -re {patch version is available.*Do you want to install} { send "n\r" }
    timeout { puts "TIMEOUT_PATCH_PROMPT" }
    eof     { puts "EOF_PATCH_PROMPT" }
}
expect eof
EXPECT_EOF
)
echo "$OUT"
echo "$OUT" | grep -q "$LATEST" \
    || fail "patch-update prompt did not mention $LATEST"
ok "patch-update prompt offered $PREVIOUS → $LATEST"
