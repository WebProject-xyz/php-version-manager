#!/usr/bin/env bash
# `.php-version` cd-hook auto-switches the active version.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step ".php-version cd-hook"

WORKDIR=$(mktemp -d)
trap 'rm -rf "$WORKDIR"' EXIT

PROJ="$WORKDIR/proj"
mkdir -p "$PROJ"
echo "$VFILTER" > "$PROJ/.php-version"

cat > "$WORKDIR/run.sh" <<EOF
#!/bin/bash
set -e
unset PVM_MULTISHELL_PATH
export PVM_DIR='$HOME/.local/share/pvm'
eval "\$('$PVM_BIN' env)"
type _pvm_cd_hook >/dev/null || { echo "_pvm_cd_hook not defined"; exit 1; }
cd '$PROJ'
_pvm_cd_hook
pvm current
EOF
chmod +x "$WORKDIR/run.sh"

OUT=$(run_under_expect "$WORKDIR/run.sh" 2>&1)
echo "$OUT"
echo "$OUT" | grep -q "$PREVIOUS" \
    || fail ".php-version did not switch to $VFILTER → $PREVIOUS"
ok ".php-version cd-hook switched to $PREVIOUS"
