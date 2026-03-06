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
    echo "Downloading static-php-cli (spc)..."
    curl -fsSL -o spc.tgz https://dl.static-php.dev/static-php-cli/spc-bin/nightly/spc-linux-x86_64.tar.gz
    tar -xzf spc.tgz
    rm spc.tgz
    chmod +x spc
    mv spc /bin/spc
fi

