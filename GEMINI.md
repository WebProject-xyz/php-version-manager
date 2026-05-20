# Gemini AI Assistant Instructions

CRITICAL SAFETY DIRECTIVE:
**NEVER use backticks (`) inside git commit messages.**

## Why?
When running `git commit -m "..."` in the shell (like Zsh/Bash), backticks are immediately evaluated as command substitution by the user's shell BEFORE the commit is executed. If a file, branch, or text snippet contains a command-like string inside backticks, the shell will execute it silently with the user's full privileges.

## Rule
1. Do not use backticks (`) in commit bodies or summaries.
2. If you need to quote code, files, or strings in a commit message, use single quotes (e.g. 'filename.rs') or double quotes (e.g. "function_name") instead.
3. Obey this rule forever, until the end of electronics.

## Project Architecture (The Map)

### Filesystem Hierarchy
Rooted at `$PVM_DIR` (resolved via `dirs::data_local_dir()`, so it defaults to `~/.local/share/pvm` on Linux and `~/Library/Application Support/pvm` on macOS).
- **$PVM_DIR/bin/pvm**: The `pvm` binary itself.
- **$PVM_DIR/versions/<full-semver>/bin/{php,php-fpm,micro.sfx}**: Installed PHP binaries. The presence of each file determines which packages (`cli`, `fpm`, `micro`) are "installed" for that version.
- **$PVM_DIR/remote_cache.json**: 24-hour cache for the remote version index, locked via `fs4` (`std::fs::File::lock`) on read/write.
- **$PVM_DIR/.env_update[_<shell-pid>]**: Short-lived files written per shell invocation (alternatively designated via `PVM_ENV_UPDATE_PATH` or keyed on PID); the shell wrapper sources them to mutate the parent shell's environment.

### Module Responsibilities
- `src/cli.rs`: Command definitions using `clap`.
- `src/commands/`: Implementation of subcommands. Each command is a `struct` with a `call()` method.
- `src/fs.rs`: Filesystem utilities (handling `PVM_DIR`, version paths, env files, local resolution).
- `src/network.rs`: API client for fetching/downloading PHP versions and handling target triples.
- `src/shell.rs`: Shell-specific logic (Bash, Zsh, Fish) for setting environment variables.
- `src/constants.rs`: Application constants.

### static-php-cli Integration
- **Endpoint:** `https://dl.static-php.dev/static-php-cli/bulk/?format=json`
- **Supported OS:** `linux`, `macos` (filtered via target triple).
- **Supported Arch:** `x86_64`, `aarch64` (filtered via target triple).
- **Packages:** `cli`, `fpm`, `micro`.
- **Format:** `tar.gz` only.
- **Filename Parser:** Package suffixes parsed from filenames like `php-8.4.18-cli-linux-x86_64.tar.gz`.

### Shell Integration Mechanics
A child process cannot mutate its parent shell's environment. PVM solves this with `pvm env` and a wrapper function:
1. **Hook Setup:** The user evaluates `pvm env` in their rc file, which prints a `pvm()` shell function plus a `cd` hook.
2. **Wrapper Execution:** When the user runs `pvm use 8.4`, the wrapper exports `PVM_ENV_UPDATE_PATH=<unique tmpfile>` before invoking the real `pvm` binary.
3. **State Mutation:** The Rust binary writes `export PATH=...; export PVM_MULTISHELL_PATH=...` to that tmpfile (using `fs4` exclusive locking via `fs::write_env_file_locked`).
4. **Environment Application:** The wrapper `eval`s the tmpfile and removes it.
- **Supported Shells:** Bash, Zsh, Fish (defined by `Shell` trait; `detect_shell()` reads `$SHELL`).
- **Auto-Switching:** The `cd` hook reads `.php-version` files and calls `pvm use` automatically.
- **Concurrency Safety:** Parallel shell sessions use distinct PID-keyed tmpfiles, and the cache + env files are locked using `fs4` file locks to prevent state corruption.

## Operational Patterns (The Handbook)

### CLI Commands and Subcommands
PVM acts as a single-binary CLI. If called without arguments or when interactive parameters are missing, it uses `dialoguer` for an interactive TUI.
- **Master Menu:** Running `pvm` without arguments launches `interactive::run_root_menu()`.
- **pvm install <version>** (alias: `pvm i <version>`): Installs a PHP version (supports major.minor alias or exact version). Opens a `MultiSelect` to pick packages (`cli`, `fpm`, `micro`). `cli` is the default. Supports `latest` to fetch the absolute latest version.
- **pvm use <version>**: Uses a version in the current shell. Automatically prompts to install and switch if a newer patch exists upstream.
- **pvm ls** (alias: `pvm list`): Lists installed local versions and active aliases.
- **pvm ls-remote** (alias: `pvm list-remote`): Interactively lists and installs available cloud versions.
- **pvm current**: Prints the active PHP version.
- **pvm uninstall <version>** (aliases: `pvm rm`, `pvm remove`): Interactive uninstall picker if no version is provided. Warns when removing the active version.
- **pvm init**: Interactively picks a version and writes it to a `.php-version` file in the current directory.
- **pvm self-update**: Checks for and applies updates to pvm itself. Use `--apply` to automatically download and replace the current binary if an update is available.

### Adding a New Command
1. Add a new module file in `src/commands/`.
2. Define the command `struct` with `#[derive(Parser, Debug)]`.
3. Export the module in `src/commands/mod.rs`.
4. Register the variant in the `Commands` enum in `src/cli.rs`.
5. Implement the async `call(self) -> Result<()>` method dispatch in `Commands::call`.

### Coding Standards
- **Errors:** Use `anyhow::Result` for all command-level fallible functions.
- **Interactivity:** Use `dialoguer` (`Select`, `MultiSelect`, `Confirm`) with `ColorfulTheme::default()` for menus and confirmations.
- **Icons:** Use `colored` for status icons:
  - `✓` (green) for success
  - `✗` (red) for errors
  - `↻` (blue) for in-progress operations
  - `💡` (yellow) for hints/warnings
- **Async:** Use `tokio` with `features = ["full"]` for runtime and `reqwest` for all network I/O.
- **Data Integrity:** Use file locking (`std::fs::File::lock` / `lock_shared` / `unlock`, stable since Rust 1.89) when writing to env update files or the remote cache. Follow `fs::write_env_file_locked` pattern.

## Build & Release Commands

### Development & Build Commands
- **Toolchain:** Pinned to Rust 1.95.0 with `clippy` and `rustfmt` in `rust-toolchain.toml`.
- **Run from Source:** `cargo run -- <subcommand>`
- **Build Release Binary:** `cargo build --release` (configured size-optimized in `Cargo.toml`: `opt-level = "z"`, LTO, `panic = "abort"`, stripped).
- **Local Install from Source:** `./build.sh` (compiles release, copies to `$PVM_DIR/bin/pvm`).
- **Lint (CI Gate):** `cargo clippy -- -D warnings`
- **Format Check (CI Gate):** `cargo fmt -- --check`

### Release Process
- **Semantic Release:** Driven by `semantic-release` from Conventional Commits on `main`.
- **Cargo.toml and CHANGELOG.md:** Bumped automatically. Do NOT hand-edit them.

## Testing & Validation

### Testing Protocol
- **Integration Tests:** Located in `tests/cli.rs`. Use `assert_cmd` and `predicates` to invoke the compiled binary.
- **Isolation:** Every test that touches the filesystem MUST use `tempfile::tempdir()` and pass its path via `cmd.env("PVM_DIR", temp_dir.path())`. Do not write to the host's real `~/.local/share/pvm`.
- **Unit Tests Concurrency:** Unit tests inside `src/**` that mutate `std::env` must use a `static Mutex<()>` guard (e.g. `src/fs.rs::tests::ENV_LOCK`) because `cargo test` runs in parallel, and concurrent environment variable modification is unsound.
- **Dynamic Versioning:** The build script (`build.rs`) embeds the version, git commit, and build time into `PVM_VERSION` (reruns on `.git/HEAD`, refs, or `Cargo.toml` change). Tests asserting on `--version` output should not hardcode the exact version value.

### Development Workflow
- **Pre-commit:** Run `cargo clippy -- -D warnings` and `cargo fmt --all -- --check`.
- **Tooling:** Use code replacement or write file tools for modifications. Avoid `sed/echo` in shell.
- **Commit Messages:** Follow Conventional Commits (e.g., `feat:`, `fix:`, `chore:`). Never use backticks (Rule 1).
