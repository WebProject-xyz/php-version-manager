#!/usr/bin/env bash
# Write README config (TCP + Unix socket pools) and validate via `php-fpm -t`.
set -euo pipefail
source "$(dirname "$0")/../_lib.sh"

step "Write README php-fpm config and validate via -t"

USER_NAME="$(whoami)"
mkdir -p "$HOME/.config/php-fpm/pool.d"

cat > "$HOME/.config/php-fpm/php-fpm.conf" <<EOF
[global]
pid = $FPM_PID_FILE
error_log = $FPM_LOG_FILE
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

cat > "$HOME/.config/php-fpm/pool.d/sock.conf" <<EOF
[sock]
user = $USER_NAME
group = $USER_NAME
listen = $FPM_SOCK
listen.owner = $USER_NAME
listen.group = $USER_NAME
listen.mode = 0660

pm = static
pm.max_children = 2

clear_env = no
EOF

cat > "$HOME/.config/php-fpm/php.ini" <<'EOF'
memory_limit = 256M
expose_php = Off
EOF

ok "wrote php-fpm.conf, pool.d/{www,sock}.conf, php.ini"

"$VDIR/php-fpm" -y "$HOME/.config/php-fpm/php-fpm.conf" -t \
    && ok "php-fpm -t OK" \
    || fail "php-fpm -t failed"

"$VDIR/php-fpm" -v
MOD_COUNT=$("$VDIR/php-fpm" -m | grep -cE '^[a-z]' || true)
ok "php-fpm -m listed $MOD_COUNT modules"
