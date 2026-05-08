#!/usr/bin/env bash
# End-to-end test for the "Running PHP-FPM" section of README.md.
#
# Validates that the documented config samples and CLI flags actually work
# against the static-php-cli FPM tarball that pvm distributes.
#
# Usage:
#   PVM_BIN=/path/to/pvm bash tests/e2e/fpm-readme.sh   # use a pre-built pvm (CI)
#   bash tests/e2e/fpm-readme.sh                        # download via install.sh
#
# Optional env:
#   PVM_VERSION_FILTER  major.minor to install (default: 8.4)
set -euo pipefail

GREEN='\033[0;32m'; RED='\033[0;31m'; BLUE='\033[0;34m'; YEL='\033[0;33m'; NC='\033[0m'
ok()   { echo -e "${GREEN}✓${NC} $*"; }
fail() { echo -e "${RED}✗${NC} $*" >&2; exit 1; }
step() { echo -e "\n${BLUE}==>${NC} $*"; }
warn() { echo -e "${YEL}!${NC} $*"; }

for tool in curl tar nc; do
    command -v "$tool" >/dev/null 2>&1 || fail "missing required tool: $tool"
done

VERSION_FILTER="${PVM_VERSION_FILTER:-8.4}"

# ---------------------------------------------------------------------------
step "1. Stage pvm binary"
if [[ -n "${PVM_BIN:-}" ]]; then
    [[ -x "$PVM_BIN" ]] || fail "PVM_BIN set but not executable: $PVM_BIN"
    : "${PVM_DIR:=$HOME/.local/share/pvm}"
    mkdir -p "$PVM_DIR/bin"
    cp "$PVM_BIN" "$PVM_DIR/bin/pvm"
    PVM_BIN="$PVM_DIR/bin/pvm"
    ok "using pre-built pvm at $PVM_BIN"
else
    curl -fsSL https://raw.githubusercontent.com/WebProject-xyz/php-version-manager/main/install.sh | bash
    PVM_BIN="$HOME/.local/share/pvm/bin/pvm"
    ok "installed via install.sh"
fi
"$PVM_BIN" --version

# ---------------------------------------------------------------------------
step "2. Download php-fpm static binary (bypass interactive MultiSelect)"
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)  TGT="linux-x86_64" ;;
    aarch64|arm64) TGT="linux-aarch64" ;;
    *) fail "unsupported arch $ARCH" ;;
esac

INDEX_URL="https://dl.static-php.dev/static-php-cli/bulk/?format=json"
ESCAPED_FILTER="${VERSION_FILTER//./\\.}"
LATEST_FPM=$(curl -fsSL "$INDEX_URL" \
    | grep -o "\"php-${ESCAPED_FILTER}\\.[0-9]*-fpm-${TGT}\\.tar\\.gz\"" \
    | sort -V | tail -n 1 | tr -d '"')
[[ -n "$LATEST_FPM" ]] || fail "no php-${VERSION_FILTER}.x fpm tarball for $TGT"
RESOLVED_VER=$(echo "$LATEST_FPM" \
    | sed -E "s/php-(${ESCAPED_FILTER}\\.[0-9]+)-fpm-${TGT}\\.tar\\.gz/\\1/")
ok "resolved $LATEST_FPM (version $RESOLVED_VER)"

DEST="$HOME/.local/share/pvm/versions/$RESOLVED_VER/bin"
mkdir -p "$DEST"
curl -fsSL "https://dl.static-php.dev/static-php-cli/bulk/$LATEST_FPM" \
    | tar -xzf - -C "$DEST"
chmod 0755 "$DEST"/*
[[ -x "$DEST/php-fpm" ]] || fail "php-fpm not extracted to $DEST"
ok "php-fpm extracted to $DEST/php-fpm"

# ---------------------------------------------------------------------------
step "3. pvm sees the new version"
"$PVM_BIN" ls
"$PVM_BIN" ls | grep -q "$RESOLVED_VER" \
    && ok "pvm ls includes $RESOLVED_VER" \
    || fail "pvm ls did not list $RESOLVED_VER"

# ---------------------------------------------------------------------------
step "4. README §3 — flags: -v, -m"
"$DEST/php-fpm" -v && ok "php-fpm -v works" || fail "php-fpm -v failed"
MODULES_OUT=$("$DEST/php-fpm" -m)
echo "$MODULES_OUT" | head -n 20
MOD_COUNT=$(echo "$MODULES_OUT" | grep -cE '^[a-z]' || true)
ok "php-fpm -m listed $MOD_COUNT modules"

# ---------------------------------------------------------------------------
step "5. README §2 — write minimal config"
USER_NAME="$(whoami)"
mkdir -p "$HOME/.config/php-fpm/pool.d"

cat > "$HOME/.config/php-fpm/php-fpm.conf" <<EOF
[global]
pid = /tmp/php-fpm.pid
error_log = /tmp/php-fpm.log
daemonize = no

include = $HOME/.config/php-fpm/pool.d/*.conf
EOF

cat > "$HOME/.config/php-fpm/pool.d/www.conf" <<EOF
[www]
user = $USER_NAME
group = $USER_NAME
listen = 127.0.0.1:9000

pm = dynamic
pm.max_children = 5
pm.start_servers = 2
pm.min_spare_servers = 1
pm.max_spare_servers = 3

catch_workers_output = yes
clear_env = no
EOF
ok "wrote $HOME/.config/php-fpm/{php-fpm.conf,pool.d/www.conf}"

# ---------------------------------------------------------------------------
step "6. README §3 — php-fpm -t (validate config)"
"$DEST/php-fpm" -y "$HOME/.config/php-fpm/php-fpm.conf" -t \
    && ok "config validation OK" \
    || fail "config validation failed"

# ---------------------------------------------------------------------------
step "7. README §3 — foreground run + listen check"
"$DEST/php-fpm" -y "$HOME/.config/php-fpm/php-fpm.conf" -F \
    > /tmp/fpm.stdout 2> /tmp/fpm.stderr &
FPM_PID=$!
trap 'kill $FPM_PID 2>/dev/null || true' EXIT

for i in $(seq 1 50); do
    if nc -z 127.0.0.1 9000 2>/dev/null; then
        ok "php-fpm listening on 127.0.0.1:9000 (after ${i}*100ms)"
        break
    fi
    sleep 0.1
done

if ! nc -z 127.0.0.1 9000 2>/dev/null; then
    echo "--- stdout ---"; cat /tmp/fpm.stdout || true
    echo "--- stderr ---"; cat /tmp/fpm.stderr || true
    fail "php-fpm did not listen on :9000 within 5s"
fi

WORKER_COUNT=$(pgrep -P "$FPM_PID" 2>/dev/null | wc -l || echo 0)
[[ "$WORKER_COUNT" -ge 1 ]] && ok "$WORKER_COUNT worker(s) spawned" \
    || warn "no workers detected"

kill -QUIT "$FPM_PID" 2>/dev/null || true
wait "$FPM_PID" 2>/dev/null || true
ok "php-fpm shutdown clean"

# ---------------------------------------------------------------------------
step "8. README §3 — custom php.ini via -c flag"
cat > "$HOME/.config/php-fpm/php.ini" <<'EOF'
memory_limit = 256M
expose_php = Off
EOF
"$DEST/php-fpm" -c "$HOME/.config/php-fpm/php.ini" \
                -y "$HOME/.config/php-fpm/php-fpm.conf" -t \
    && ok "php-fpm -c custom-ini -y conf -t works" \
    || fail "php-fpm with -c failed"

# ---------------------------------------------------------------------------
echo
echo -e "${GREEN}All README PHP-FPM steps validated end-to-end.${NC}"
echo "Resolved version:  $RESOLVED_VER"
echo "Binary path:       $DEST/php-fpm"
echo "Config files:      $HOME/.config/php-fpm/"
