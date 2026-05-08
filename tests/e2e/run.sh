#!/usr/bin/env bash
# Driver for tests/e2e — runs each cases/NN_*.sh as a fresh bash subprocess
# so state from one case does not leak into the next.
#
# Usage:
#   PVM_BIN=/path/to/pvm bash tests/e2e/run.sh   # CI: use a pre-built pvm
#   bash tests/e2e/run.sh                        # local: download via install.sh
#
# Optional env:
#   PVM_VERSION_MAJOR_MINOR  major.minor to test (default: 8.5)
#   PVM_E2E_ONLY             space-separated case file names to run (e.g. "01_install.sh 02_ls.sh")
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
source "$HERE/_lib.sh"

for tool in curl tar nc expect cgi-fcgi pgrep; do
    command -v "$tool" >/dev/null 2>&1 \
        || fail "missing required tool: $tool (install: expect libfcgi-bin)"
done

# ---------------------------------------------------------------------------
# Stage pvm binary
# ---------------------------------------------------------------------------
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
# Resolve target versions from upstream index
# ---------------------------------------------------------------------------
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)  TGT="linux-x86_64" ;;
    aarch64|arm64) TGT="linux-aarch64" ;;
    *) fail "unsupported arch $ARCH" ;;
esac

VFILTER="${PVM_VERSION_MAJOR_MINOR:-8.5}"
ESCAPED_VFILTER="${VFILTER//./\\.}"

INDEX_JSON=$(curl -fsSL "https://dl.static-php.dev/static-php-cli/bulk/?format=json")
ALL_VERS=$(echo "$INDEX_JSON" \
    | grep -oE "\"php-${ESCAPED_VFILTER}\\.[0-9]+-cli-${TGT}\\.tar\\.gz\"" \
    | sed -E "s|\"php-(${ESCAPED_VFILTER}\\.[0-9]+)-cli-${TGT}\\.tar\\.gz\"|\\1|" \
    | sort -V -u)
LATEST=$(echo "$ALL_VERS" | tail -n 1)
PREVIOUS=$(echo "$ALL_VERS" | tail -n 2 | head -n 1)
[[ -n "$LATEST" ]] || fail "no ${VFILTER}.x versions found upstream"
[[ -n "$PREVIOUS" && "$PREVIOUS" != "$LATEST" ]] || PREVIOUS="$LATEST"

VDIR="$HOME/.local/share/pvm/versions/$PREVIOUS/bin"
MISSING_VER="${VFILTER}.99999"

ok "latest ${VFILTER}.x = $LATEST  /  previous = $PREVIOUS"

# ---------------------------------------------------------------------------
# Shared state dir + fpm config paths (used across cases)
# ---------------------------------------------------------------------------
E2E_STATE=$(mktemp -d)
FPM_PID_FILE=/tmp/php-fpm.pid
FPM_LOG_FILE=/tmp/php-fpm.log
FPM_SOCK=/tmp/php-fpm-www.sock

cleanup() {
    if [[ -f "$E2E_STATE/fpm.pid" ]]; then
        kill -QUIT "$(cat "$E2E_STATE/fpm.pid")" 2>/dev/null || true
    fi
    rm -rf "$E2E_STATE"
    rm -f "$FPM_SOCK" "$FPM_PID_FILE" "$FPM_LOG_FILE"
}
trap cleanup EXIT

export PVM_BIN VFILTER LATEST PREVIOUS VDIR MISSING_VER E2E_STATE
export FPM_PID_FILE FPM_LOG_FILE FPM_SOCK

# ---------------------------------------------------------------------------
# Run each case as a fresh bash subprocess
# ---------------------------------------------------------------------------
CASES=()
if [[ -n "${PVM_E2E_ONLY:-}" ]]; then
    for c in $PVM_E2E_ONLY; do
        CASES+=("$HERE/cases/$c")
    done
else
    while IFS= read -r f; do
        CASES+=("$f")
    done < <(find "$HERE/cases" -maxdepth 1 -name '[0-9][0-9]_*.sh' | sort)
fi

PASSED=0
FAILED=0
for case_file in "${CASES[@]}"; do
    case_name=$(basename "$case_file" .sh)
    echo
    echo -e "${BLUE}── ${case_name} ──${NC}"
    if bash "$case_file"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
        echo -e "${RED}✗ ${case_name} failed${NC}" >&2
        # Stop on first failure — later cases depend on earlier state (fpm, install).
        echo
        echo -e "${RED}aborting: $FAILED failed, $PASSED passed before failure${NC}" >&2
        exit 1
    fi
done

echo
echo -e "${GREEN}All $PASSED e2e cases passed on Linux.${NC}"
echo "Tested version:  $PREVIOUS (latest upstream: $LATEST)"
echo "Pools:           TCP 127.0.0.1:9000 + unix $FPM_SOCK"
