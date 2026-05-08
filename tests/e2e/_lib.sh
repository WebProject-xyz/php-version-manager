#!/usr/bin/env bash
# Shared helpers for tests/e2e cases.
# Source from a case via: source "$(dirname "$0")/../_lib.sh"
# The driver pre-exports PVM_BIN, PREVIOUS, LATEST, VFILTER, VDIR, etc.

GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; YEL='\033[0;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*" >&2; exit 1; }
step() { echo -e "${BLUE}==>${NC} $*"; }
warn() { echo -e "${YEL}!${NC} $*"; }

# Spawn a script under expect, auto-decline the three known prompts.
# Always exits 0; caller asserts on captured stdout for markers.
run_under_expect() {
    local script="$1"
    expect <<EXPECT_EOF
set timeout 240
log_user 1
spawn bash --norc --noprofile $script
expect {
    -re {patch version is available.*Do you want to install} { send "n\r"; exp_continue }
    -re {Do you want to use PHP.*now} { send "n\r"; exp_continue }
    -re {\.php-version file is present.*Do you want to apply} { send "n\r"; exp_continue }
    eof
}
EXPECT_EOF
}

# Send <?php …?> body to fpm via cgi-fcgi and echo the response.
# Args: connect_target (e.g. "127.0.0.1:9000" or "/tmp/sock"), php_body
fcgi_call() {
    local connect="$1"
    local php_body="$2"
    local script_dir
    script_dir=$(mktemp -d)
    echo "<?php $php_body" > "$script_dir/run.php"
    SCRIPT_FILENAME="$script_dir/run.php" \
    SCRIPT_NAME=/run.php \
    REQUEST_METHOD=GET \
    QUERY_STRING="" \
        cgi-fcgi -bind -connect "$connect"
    rm -rf "$script_dir"
}
