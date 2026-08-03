# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Critical Safety Rule

**Never use backticks (`` ` ``) inside `git commit -m` messages.** Backticks are evaluated as command substitution by the user's shell before git sees them, executing arbitrary code under their privileges. Use single or double quotes to wrap code/file references in commit bodies. This applies to all commits, PRs, and any string passed to a shell.

## Commands

Toolchain is pinned in `rust-toolchain.toml` (currently Rust 1.97.1, edition 2024) with `clippy` and `rustfmt`; Renovate bumps it, so check the file rather than assuming.

- Build release binary: `cargo build --release` (size-optimized: `opt-level = "z"`, LTO, `panic = "abort"`, stripped)
- Run from source: `cargo run -- <subcommand>`
- All tests: `cargo test`
- Single test: `cargo test test_use_silent_export` (or any function name in `tests/cli.rs` / `src/**/tests`)
- Lint (CI gate): `cargo clippy -- -D warnings`
- Format check (CI gate): `cargo fmt -- --check`
- Local install from source: `./build.sh` (compiles release, copies to `$PVM_DIR/bin/pvm`)

CI lives in `.github/workflows/release.yml` as a chained pipeline: `tests` (fmt + clippy + `cargo test`) → `e2e tests - linux` (`tests/e2e/run.sh`) → `release`; failures block merge. New CI gates belong inside `release.yml` in that chain, not as standalone workflow files. Releases are driven by `semantic-release` from Conventional Commits on `main` — `Cargo.toml` version and `CHANGELOG.md` are bumped automatically, so do not hand-edit them.

## Architecture

PVM is a single-binary Rust CLI that downloads pre-built static PHP binaries from `dl.static-php.dev` and switches `PATH` per-shell. Three concerns drive the design:

### 1. Command dispatch (`src/cli.rs` → `src/commands/*`)

`Cli` is a `clap` `Parser` with an `Option<Commands>` subcommand. If absent, `main.rs` falls through to `interactive::run_root_menu()` (a `dialoguer` TUI). Every subcommand is its own struct in `src/commands/<name>.rs` with `#[derive(Parser, Debug)]` and an async `call(self) -> Result<()>` method. To add a subcommand: create the module, add it to `src/commands/mod.rs`, register the variant in the `Commands` enum in `src/cli.rs`, add the dispatch arm in `Commands::call`, and (if menu-worthy) extend `interactive.rs` — its match arms are index-based, so renumber everything after the insertion point.

Non-interactive behavior is a hard requirement: every command must work without a terminal. Yes/no questions go through `prompt::confirm(prompt, default, assume_yes)` (`src/prompt.rs`), which auto-answers with `default` on a non-TTY stdin and short-circuits on `--yes`. Pickers (`dialoguer::Select`) are guarded by `std::io::IsTerminal` checks that bail with a usage hint or fall back to plain output (`ls`, `ls-remote`). The patch-update offer in `use` runs only on a TTY so scripts never trigger downloads.

### 2. Filesystem layout (`src/fs.rs`, `src/constants.rs`)

Rooted at `$PVM_DIR` (defaults to `dirs::data_local_dir()/pvm`, i.e. `~/.local/share/pvm` on Linux). Layout:

- `$PVM_DIR/bin/pvm` — the binary itself
- `$PVM_DIR/versions/<full-semver>/bin/{php,php-fpm,micro.sfx}` — installed PHP binaries; presence of each file determines which packages are "installed" (`fs::get_installed_packages`)
- `$PVM_DIR/remote_cache-<target>.json` — 24h cache of the remote version index (per target triple), file-locked on read/write; `pvm cache clear` deletes it
- `$PVM_DIR/default` — version activated by `pvm env` in every new shell (`pvm default`); `prune` re-points it when it removes the referenced patch
- `$PVM_DIR/.update_check_guard` — 24h throttle for the "newer patch available" check (`src/update.rs`); its presence suppresses update prompts, which matters when testing update flows
- `$PVM_DIR/.env_update_<timestamp>_<pid>` — short-lived file the shell wrapper evals to mutate the parent shell

Version resolution is two-staged: `network::resolve_version` matches a user input like `8.4` against the remote list (latest patch wins), `fs::resolve_local_version` does the same against installed versions. `latest` is a special alias on both sides.

### 3. Shell integration (`src/shell.rs`, `src/commands/env.rs`)

The hard part: a child process cannot mutate its parent shell's environment. PVM solves this with `pvm env` + a wrapper function:

1. The user evaluates `pvm env` in their rc file, which prints a `pvm()` shell function plus a `cd` hook.
2. When the user runs `pvm use 8.4`, the wrapper exports `PVM_ENV_UPDATE_PATH=<unique tmpfile>` before invoking the real `pvm` binary.
3. The Rust binary writes `export PATH=...; export PVM_MULTISHELL_PATH=...` to that tmpfile (with an exclusive file lock — see `fs::write_env_file_locked`).
4. The wrapper `eval`s the tmpfile and removes it.

The `Shell` trait (`Bash`, `Zsh`, `Fish`) emits the syntax for each shell; `detect_shell()` reads `$SHELL`. Bash and Zsh share the `posix_*` free functions; only `use_on_cd` differs. The `cd` hook reads `.php-version` and calls `pvm use --silent` automatically. `deactivate()` (used by `pvm use system`) writes an env-file snippet that clears `PVM_MULTISHELL_PATH` and restores the filtered PATH.

PATH is never manipulated in shell code: `Shell::path` and `Shell::deactivate` take an already-filtered entry list from `fs::path_without_versions()` and bake the whole value in literally. Because the wrapper runs `pvm` as a direct child, the Rust process sees exactly the PATH its output will be eval'd into, so it can drop stale `$PVM_DIR/versions` entries itself. Emitting `PATH=<new>:$PATH` instead would stack a duplicate on every activation — and the bash `cd` hook runs from `PROMPT_COMMAND`, i.e. after *every* command, so that grows without bound (hence the `__pvm_last_pwd` guard in `Bash::use_on_cd` and the `case`/`contains` guard around the `$PVM_DIR/bin` prepend in `wrapper_fn`).

The activation tail (patch-update offer, `.php-version` prompts, env-file write) lives in `use_cmd::activate(version, ActivateOpts)`; both `pvm use` and the interactive `pvm ls` picker call it. `ActivateOpts.offer_save` distinguishes picker flows (offer to write `.php-version`) from explicit-argument flows (offer to update an existing differing file).

Concurrency: every shell session uses its own tmpfile keyed on PID, and the cache + env files are taken under OS file locks (std `File::lock`/`lock_shared` — the `fs4` crate was dropped) so parallel `pvm` invocations don't corrupt state.

### 4. Networking (`src/network.rs`)

Single endpoint: `https://dl.static-php.dev/static-php-cli/bulk/?format=json` returns the file index. `get_target_triple()` filters by `{linux,macos}-{x86_64,aarch64}`; package suffixes `cli`/`fpm`/`micro` are parsed from filenames like `php-8.4.18-cli-linux-x86_64.tar.gz`. Downloads stream via `reqwest` with an `indicatif` progress bar, then untar into `$PVM_DIR/versions/<v>/bin/` and `chmod 0755` on Unix. Other platforms/architectures are unsupported and will bail.

## Conventions

- All fallible command-level functions return `anyhow::Result<T>`.
- User-visible status uses `colored` icons: `✓` (green = success), `✗` (red = error), `↻` (blue = in-progress), `💡` (yellow = hint).
- Async runtime is `tokio` with `features = ["full"]`; all network I/O uses `reqwest`.
- Interactive prompts are `dialoguer` (`Select`, `MultiSelect`, `Confirm`) with `ColorfulTheme::default()`.
- File writes that other processes may read concurrently MUST take an OS file lock via std `File::lock` — follow the pattern in `fs::write_env_file_locked` (cache reads use `lock_shared`, see `src/network.rs`).
- Commits follow Conventional Commits (`feat:`, `fix:`, `chore:`, `docs:`, …); the `feat:`/`fix:` prefixes determine version bumps via semantic-release.

## Testing

- Integration tests live in `tests/cli.rs` and use `assert_cmd` + `predicates` to invoke the compiled binary.
- Every test that touches the filesystem MUST call `tempfile::tempdir()` and pass its path via `cmd.env("PVM_DIR", ...)`. Never assume the host's real `~/.local/share/pvm` is empty or writable.
- Network-dependent paths are tested offline via the helpers at the top of `tests/cli.rs`: `seed_remote_cache()` pre-fills `remote_cache-<target>.json` with fake versions, `seed_installed_version()` fakes `versions/<v>/bin` layouts. Tests that must not see a pvm-active shell use `cmd.env_remove("PVM_MULTISHELL_PATH")`.
- Unit tests inside `src/**` that mutate `std::env` use a `static Mutex<()>` guard (see `src/fs.rs::tests::ENV_LOCK`) — `cargo test` runs in parallel by default and concurrent `set_var` is unsound.
- The build script (`build.rs`) embeds version + git commit + build time into `PVM_VERSION`; it reruns when `.git/HEAD`, refs, or `Cargo.toml` change. Tests that assert on `--version` output should not hardcode the value.

### E2E suite (`tests/e2e/`)

Bash-based Linux end-to-end tests: `run.sh` is the driver, one case per file in `cases/NN_*.sh` (install → use → FPM config/run/FastCGI → uninstall), shared helpers in `_lib.sh`. Each case runs as a fresh bash subprocess so state can't mask bugs (e.g. `.update_check_guard` suppressing later prompts). Key points:

- The driver mutates `$HOME/.local/share/pvm` and refuses to run outside a sandbox (container or `GITHUB_ACTIONS=true`); `PVM_E2E_FORCE=1` overrides — don't use it on a dev machine.
- Run locally via Docker: `docker build -t pvm-e2e tests/e2e`, then mount `tests/e2e` + `target/release/pvm` and set `PVM_BIN` (full commands in `tests/e2e/README.md`).
- `PVM_E2E_ONLY="07_patch_update.sh"` runs a single case; cases 09–13 depend on state from 08.
- `PVM_VERSION_MAJOR_MINOR` (default `8.5`) picks the PHP line; both latest and previous patches must exist upstream.
