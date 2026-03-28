#!/usr/bin/env bash
set -euo pipefail

# Usage: ./build-php.sh [build] [PHP_MINOR]
# Example: ./build-php.sh build 8.4

COMMAND="${1:-}"
PHP_MINOR="${2:-8.3}"
IMAGE_NAME="pvm-build-env"

if [ "$COMMAND" == "build" ]; then
    if [ -z "${GITHUB_TOKEN:-}" ]; then
        echo "Error: GITHUB_TOKEN is not set."
        echo "Please set it by running: export GITHUB_TOKEN=your_token"
        exit 1
    fi

    echo "Building PHP $PHP_MINOR in Docker..."
    docker build --network host \
        --build-arg GITHUB_TOKEN="${GITHUB_TOKEN}" \
        --build-arg PHP_MINOR="${PHP_MINOR}" \
        -f build/Dockerfile -t "$IMAGE_NAME" build

    echo "Extracting build artifact..."
    mkdir -p output
    
    # Create a temporary container to copy files out
    TMP_CONTAINER=$(docker create "$IMAGE_NAME")
    
    # Get the actual tarball name from the container
    TARBALL_NAME=$(docker cp "$TMP_CONTAINER:/tarball-name.txt" - | tar -xO)
    
    echo "Artifact name: $TARBALL_NAME"
    docker cp "$TMP_CONTAINER:/php-artifact.tar.gz" "output/$TARBALL_NAME"
    docker rm "$TMP_CONTAINER"

    echo "Build finished! Check output/$TARBALL_NAME"
    stat "output/$TARBALL_NAME"
    exit 0
fi

echo "Usage: $0 build [PHP_MINOR]"
echo "Example: $0 build 8.3"
exit 1
