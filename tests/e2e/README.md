# pvm e2e tests (Linux)

Locks the README "Running PHP-FPM" walkthrough and the core `pvm` flows in lockstep with the upstream static-php-cli FPM tarballs. Runs on every PR via `release.yml` job **e2e tests - linux**.

## Layout

```text
tests/e2e/
├── Dockerfile          sandbox image (ubuntu:24.04 + expect + libfcgi-bin)
├── run.sh              driver — version resolution + per-case execution
├── _lib.sh             shared helpers (run_under_expect, fcgi_call, sandbox guard)
├── README.md           this file
└── cases/
    ├── 01_install.sh           real `pvm install` with interactive MultiSelect
    ├── 02_ls.sh                `pvm ls` discovers the version + package tags
    ├── 03_use_wrapper.sh       `pvm use` via shell wrapper switches PATH
    ├── 04_current.sh           `pvm current`
    ├── 05_php_version_hook.sh  `.php-version` cd-hook
    ├── 06_use_missing.sh       missing-version install prompt + decline (#24)
    ├── 07_patch_update.sh      patch-update detection
    ├── 08_fpm_config.sh        README php-fpm.conf + pool.d/{www,sock}.conf, `-t`
    ├── 09_fpm_run.sh           `php-fpm -F` listens on TCP + unix socket
    ├── 10_fcgi_tcp.sh          FastCGI roundtrip over TCP
    ├── 11_fcgi_sock.sh         FastCGI roundtrip over unix socket
    ├── 12_php_ini_effective.sh `-c php.ini` is effective inside the worker
    ├── 13_pid_log.sh           pid file + error log written
    ├── 14_fpm_shutdown.sh      SIGQUIT clean shutdown
    └── 15_uninstall.sh         `pvm uninstall` removes the version dir
```

The driver runs each `cases/NN_*.sh` as a fresh bash subprocess so state from one case cannot mask bugs in the next. (The previous monolith silently passed because pvm's 24h `.update_check_guard` suppressed the patch-update prompt for any case after the first one to use it.)

## Why the safety check

`run.sh` mutates `$HOME/.local/share/pvm`, `/tmp/php-fpm.*`, and `~/.config/php-fpm`. Running on a dev machine would clobber whatever real pvm install lives there. So the driver refuses to run unless one of these is true:

- `/.dockerenv` exists (you're inside a container)
- `GITHUB_ACTIONS=true` (you're on a hosted runner)
- `/proc/1/cgroup` shows a container runtime (docker, containerd, kubepods)
- `PVM_E2E_FORCE=1` is set (manual override at your own risk)

## Running locally

Build the sandbox image once (rebuild only when `Dockerfile` changes):

```bash
docker build -t pvm-e2e tests/e2e
```

Build pvm from source:

```bash
cargo build --release
```

Run the full suite:

```bash
docker run --rm \
    -v "$(pwd)/tests/e2e:/home/tester/e2e:ro" \
    -v "$(pwd)/target/release/pvm:/home/tester/pvm:ro" \
    -e PVM_BIN=/home/tester/pvm \
    pvm-e2e bash /home/tester/e2e/run.sh
```

## Useful overrides

| Env var | Default | Effect |
|---------|---------|--------|
| `PVM_BIN` | _unset_ | Path to a pre-built pvm. Unset → driver runs `install.sh` and pulls the latest GitHub release. |
| `PVM_VERSION_MAJOR_MINOR` | `8.5` | Major.minor line to test. Both `LATEST` and `PREVIOUS` patches must exist upstream. |
| `PVM_E2E_ONLY` | _unset_ | Space-separated case files to run, e.g. `"01_install.sh 07_patch_update.sh"`. Useful for reproducing one failure. |
| `FPM_TCP_ADDR` | `127.0.0.1:9000` | Override the TCP listener if `:9000` is busy. |
| `PVM_E2E_FORCE` | _unset_ | Set to `1` to bypass the sandbox guard. Don't. |

### Run a single case

```bash
docker run --rm \
    -v "$(pwd)/tests/e2e:/home/tester/e2e:ro" \
    -v "$(pwd)/target/release/pvm:/home/tester/pvm:ro" \
    -e PVM_BIN=/home/tester/pvm \
    -e PVM_E2E_ONLY="07_patch_update.sh" \
    pvm-e2e bash /home/tester/e2e/run.sh
```

Note: cases 09–13 depend on case 08 having written the FPM config and on a running FPM process — running them in isolation will fail unless you also include the cases that set up that state.

### Test PHP 8.4 instead of 8.5

```bash
docker run --rm ... \
    -e PVM_VERSION_MAJOR_MINOR=8.4 \
    pvm-e2e bash /home/tester/e2e/run.sh
```

## In CI

The job lives in `.github/workflows/release.yml` as `e2e tests - linux`, chained `tests` → `e2e tests - linux` → `release`. It builds pvm from source, installs `expect` + `libfcgi-bin` + `netcat-openbsd`, and invokes `tests/e2e/run.sh` directly on the runner — no Docker layer needed because the runner *is* the sandbox (`GITHUB_ACTIONS=true`).
