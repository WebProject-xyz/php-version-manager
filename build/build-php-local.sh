#!/usr/bin/env bash
set -euox pipefail

# Usage: ./build-php-local.sh [PHP_MINOR]
# Example: ./build-php-local.sh 8.3

PHP_MINOR="${1:-8.3}"
PHP_VERSIONS_FILE="php-versions.json"

if [ ! -f "$PHP_VERSIONS_FILE" ]; then
    echo "Error: $PHP_VERSIONS_FILE not found." >&2
    exit 1
fi

# Extract build version using jq
PHP_BUILD=$(jq -r --arg minor "$PHP_MINOR" '.[$minor].build' "$PHP_VERSIONS_FILE")

if [ "$PHP_BUILD" == "null" ] || [ -z "$PHP_BUILD" ]; then
    echo "Error: No build field for $PHP_MINOR in $PHP_VERSIONS_FILE" >&2
    exit 1
fi

echo "Building PHP $PHP_MINOR ($PHP_BUILD)..."
echo "Running as user: $(whoami) (UID: $(id -u), GID: $(id -g))"

# Download static-php-cli (spc) if not present
if ! command -v spc &> /dev/null; then
    echo "Error: spc not found" >&2
    exit 1
fi

if ! command -v zig &> /dev/null; then
    echo "Error: zig not found" >&2
    exit 1
fi

# Build PHP
echo "Running spc doctor..."
spc doctor --auto-fix

echo "Downloading PHP sources for required extensions..."
# Extensions list from build-php.yml
EXTENSIONS="bcmath,bz2,calendar,ctype,curl,dom,exif,fileinfo,filter,gd,iconv,igbinary,intl,mbregex,mbstring,mysqli,mysqlnd,opcache,openssl,pcntl,pdo,pdo_mysql,pdo_pgsql,pdo_sqlite,pgsql,phar,posix,readline,redis,session,simplexml,sockets,sqlite3,swoole,tokenizer,xml,xmlreader,xmlwriter,zip,zlib"

spc download --with-php="$PHP_BUILD" --for-extensions="$EXTENSIONS,xdebug" --prefer-pre-built

echo "Building PHP with extensions..."

# Use a dynamic target on Linux so xdebug can be built as a shared extension
export SPC_TARGET="native-native-gnu"

spc build "$EXTENSIONS" --with-suggested-libs --with-suggested-exts --enable-zts --with-micro-fake-cli --build-cli --build-shared="xdebug" --debug

# Package result
echo "Packaging PHP $PHP_MINOR..."

if [ ! -f buildroot/bin/php ]; then
    echo "Error: buildroot/bin/php not found" >&2
    exit 1
fi

if [ ! -f buildroot/modules/xdebug.so ]; then
    echo "Error: buildroot/modules/xdebug.so not found" >&2
    exit 1
fi

DIST_DIR="dist/$PHP_MINOR"
mkdir -p "$DIST_DIR/ext"
cp buildroot/bin/php "$DIST_DIR/php"
cp buildroot/modules/xdebug.so "$DIST_DIR/ext/xdebug.so"

TARBALL="php-${PHP_MINOR}-linux-x86_64.tar.gz"
tar -C "$DIST_DIR" -czf "$TARBALL" .

mkdir -p /output
# Fix permissions if HOST_UID/HOST_GID are set
if [ -n "${HOST_UID:-}" ] && [ -n "${HOST_GID:-}" ]; then
    echo "Fixing file permissions for host user ($HOST_UID:$HOST_GID)..."
    chown -R "$HOST_UID:$HOST_GID" dist/ .spc/ "$TARBALL" spc buildroot/ downloads/ source/ /output/ || true
fi

cp "$TARBALL" /output/
echo "Successfully built and packaged PHP $PHP_MINOR to /output/$TARBALL"
