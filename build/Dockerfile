FROM ubuntu:24.04
ARG GITHUB_TOKEN
ARG PHP_MINOR=8.3

# Avoid interactive prompts during build
ENV DEBIAN_FRONTEND=noninteractive
ENV GITHUB_TOKEN=${GITHUB_TOKEN}
ENV PHP_MINOR=${PHP_MINOR}
ENV SPC_TARGET="native-native-gnu"
ENV SPC_BUILD_SHARED="xdebug"

# Install all build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    autoconf \
    automake \
    autopoint \
    bison \
    bzip2 \
    ca-certificates \
    cmake \
    curl \
    flex \
    g++ \
    gcc \
    git \
    jq \
    libtool \
    make \
    patch \
    pkg-config \
    re2c \
    sudo \
    unzip \
    xz-utils \
    libunistring-dev \
    libidn2-dev \
    libedit-dev \
    && rm -rf /var/lib/apt/lists/*

USER root
WORKDIR /app
COPY --chmod=755 build-prepare-build.sh .
COPY --chmod=755 build-php-local.sh .
COPY --chmod=644 php-versions.json .
COPY --chmod=644 craft.yml .

RUN --mount=type=cache,target=/app/downloads \
    --mount=type=cache,target=/app/source \
    ./build-prepare-build.sh

RUN --mount=type=cache,target=/app/downloads \
    --mount=type=cache,target=/app/source \
    spc doctor --auto-fix

# Dynamically update php-version in craft.yml based on PHP_MINOR
RUN sed -i "s/^php-version: .*/php-version: ${PHP_MINOR}/" craft.yml

# Download all required sources and extensions
RUN --mount=type=cache,target=/app/downloads \
    --mount=type=cache,target=/app/source \
    PHP_BUILD=$(jq -r --arg minor "${PHP_MINOR}" '.[$minor].build' php-versions.json) && \
    spc download --with-php="$PHP_BUILD" --for-extensions="apcu,bcmath,calendar,ctype,curl,dba,dom,exif,fileinfo,filter,gd,iconv,igbinary,mbstring,mbregex,msgpack,mysqli,mysqlnd,opcache,openssl,pcntl,pdo,pdo_mysql,pdo_sqlite,phar,posix,readline,redis,session,simplexml,sockets,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,xsl,zip,zlib,xdebug" --prefer-pre-built

# Build PHP using spc build to explicitly control SAPIs and avoid embed failures
RUN --mount=type=cache,target=/app/downloads \
    --mount=type=cache,target=/app/source \
    spc build --build-cli \
    --build-shared="xdebug" \
    --enable-zts \
    --with-suggested-libs \
    --with-suggested-exts \
    "apcu,bcmath,calendar,ctype,curl,dba,dom,exif,fileinfo,filter,gd,iconv,igbinary,mbstring,mbregex,msgpack,mysqli,mysqlnd,opcache,openssl,pcntl,pdo,pdo_mysql,pdo_sqlite,phar,posix,readline,redis,session,simplexml,sockets,sodium,sqlite3,tokenizer,xml,xmlreader,xmlwriter,xsl,zip,zlib" \
    --debug

# Verify PHP execution (without and with xdebug)
RUN ./buildroot/bin/php --version
RUN ./buildroot/bin/php -d zend_extension=./buildroot/modules/xdebug.so --version

# Package result
RUN PHP_BUILD=$(jq -r --arg minor "${PHP_MINOR}" '.[$minor].build' php-versions.json) && \
    DIST_DIR="dist/${PHP_MINOR}" && \
    mkdir -p "$DIST_DIR/ext" && \
    cp buildroot/bin/php "$DIST_DIR/php" && \
    cp buildroot/modules/xdebug.so "$DIST_DIR/ext/xdebug.so" && \
    TARBALL="php-version-${PHP_BUILD}-linux-x86_64.tar.gz" && \
    tar -C "$DIST_DIR" -czf "/php-artifact.tar.gz" . && \
    echo "$TARBALL" > /tarball-name.txt

