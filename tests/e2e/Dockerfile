# Sandbox image for the pvm e2e suite.
# Build:  docker build -t pvm-e2e tests/e2e
# Run:    docker run --rm \
#             -v "$(pwd)/tests/e2e:/home/tester/e2e:ro" \
#             -v "$(pwd)/target/release/pvm:/home/tester/pvm:ro" \
#             -e PVM_BIN=/home/tester/pvm \
#             pvm-e2e bash /home/tester/e2e/run.sh
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

# Runtime deps mirror what release.yml installs:
#   curl, ca-certificates  — for install.sh fallback + upstream tarball download
#   bash                   — driver/case scripts
#   netcat-openbsd         — listener readiness check
#   procps                 — pgrep for worker counting
#   expect                 — drives interactive dialoguer prompts
#   libfcgi-bin            — supplies cgi-fcgi for FastCGI roundtrip cases
RUN apt-get update && apt-get install -y --no-install-recommends \
        curl ca-certificates bash netcat-openbsd procps \
        expect libfcgi-bin \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/bash tester
USER tester
WORKDIR /home/tester
ENV SHELL=/bin/bash
CMD ["/bin/bash"]
