#!/usr/bin/env bash
# Real `pvm install` flow with interactive MultiSelect via expect.
# Toggles `fpm` on top of the default `cli` selection.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm install $PREVIOUS — toggle fpm in MultiSelect"

expect <<EXPECT_EOF
set timeout 240
log_user 1
spawn $PVM_BIN install $PREVIOUS
expect {
    -re "Select packages" { }
    timeout { puts "TIMEOUT_MULTISELECT" }
    eof     { puts "EOF_MULTISELECT" }
}
# Packages alphabetical: cli (default), fpm, micro. Down-arrow + space toggles fpm.
send "\033\[B"
send " "
send "\r"
expect {
    -re "Successfully installed" { }
    timeout { puts "TIMEOUT_INSTALL" }
    eof     { puts "EOF_INSTALL" }
}
expect {
    -re {Do you want to use} { send "n\r"; exp_continue }
    eof
}
EXPECT_EOF

[[ -x "$VDIR/php" ]]     || fail "cli binary missing at $VDIR/php"
[[ -x "$VDIR/php-fpm" ]] || fail "fpm binary missing at $VDIR/php-fpm"
ok "cli + fpm binaries installed at $VDIR"
