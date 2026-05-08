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
    # RETURN trap fires even when set -e in the caller would otherwise abort
    # mid-call due to a non-zero cgi-fcgi exit, so the temp dir is always removed.
    trap 'rm -rf "$script_dir"' RETURN
    echo "<?php $php_body" > "$script_dir/run.php"
    SCRIPT_FILENAME="$script_dir/run.php" \
    SCRIPT_NAME=/run.php \
    REQUEST_METHOD=GET \
    QUERY_STRING="" \
        cgi-fcgi -bind -connect "$connect"
}

# Safety: refuse to mutate the user's local pvm state outside Docker or GitHub Actions.
# Both run.sh and any case sourcing this lib hit $HOME/.local/share/pvm, /tmp, and
# $HOME/.config/php-fpm — running on a dev machine would clobber the user's setup.
e2e_require_sandbox() {
    if [[ "${PVM_E2E_FORCE:-}" == "1" ]]; then
        warn "PVM_E2E_FORCE=1 — running outside Docker / GitHub Actions on caller's request"
        return 0
    fi
    if [[ -f /.dockerenv ]]; then
        return 0
    fi
    if [[ "${GITHUB_ACTIONS:-}" == "true" ]]; then
        return 0
    fi
    if grep -qE '(/docker/|containerd|kubepods)' /proc/1/cgroup 2>/dev/null; then
        return 0
    fi
    fail "tests/e2e mutates \$HOME/.local/share/pvm, /tmp/php-fpm.*, and ~/.config/php-fpm.
       Run inside Docker (see tests/e2e/README.md) or GitHub Actions.
       Override at your own risk with PVM_E2E_FORCE=1."
}
