#!/usr/bin/env bash
# `pvm use <minor>` through the shell wrapper switches PATH.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "pvm use $VFILTER via wrapper — PATH switch + which php-fpm"

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

cat > "$WORKDIR/run.sh" <<EOF
#!/bin/bash
set -e
export PVM_DIR='$HOME/.local/share/pvm'
eval "\$('$PVM_BIN' env)"
pvm use $VFILTER
echo "PATH_AFTER_USE=\$PATH"
echo "WHICH_PHP_FPM=\$(command -v php-fpm)"
php-fpm -v 2>&1 | head -n 1
EOF
chmod +x "$WORKDIR/run.sh"

OUT=$(run_under_expect "$WORKDIR/run.sh" 2>&1)
echo "$OUT"

echo "$OUT" | grep -q "PATH_AFTER_USE=$VDIR" \
    || fail "PATH not switched to $VDIR"
echo "$OUT" | grep -q "WHICH_PHP_FPM=$VDIR/php-fpm" \
    || fail "which php-fpm did not resolve under pvm versions dir"
ok "wrapper switched PATH; php-fpm → $VDIR/php-fpm"
