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
- **$PVM_DIR**: Root directory. Resolved via `dirs::data_local_dir()`, so defaults to `~/.local/share/pvm` on Linux and `~/Library/Application Support/pvm` on macOS.
- **$PVM_DIR/versions/<version>**: Installation directory for specific PHP versions.
- **$PVM_DIR/bin/pvm**: The `pvm` binary itself.
- **$PVM_DIR/remote_cache-<target-triple>.json**: 24-hour cache for the remote version index, scoped per target triple (e.g. `linux-x86_64`).
- **$PVM_DIR/.env_update[_<shell-pid>]**: Short-lived files written per shell invocation; the shell wrapper sources them to mutate the parent shell's environment.

### Module Responsibilities
- `src/cli.rs`: Command definitions using `clap`.
- `src/commands/`: Implementation of subcommands. Each command is a `struct` with a `call()` method.
- `src/fs.rs`: Filesystem utilities (handling `PVM_DIR`, version paths, env files).
- `src/network.rs`: API client for fetching and downloading PHP versions.
- `src/shell.rs`: Shell-specific logic for setting environment variables.

### static-php-cli Integration
- **Endpoint:** `https://dl.static-php.dev/static-php-cli/bulk/`
- **Supported OS:** `linux`, `macOS`.
- **Supported Arch:** `x86_64`, `aarch64`.
- **Packages:** `cli`, `fpm`, `micro`.
- **Format:** `tar.gz` only.

## Operational Patterns (The Handbook)
### Adding a New Command
1. Add a new module file in `src/commands/`.
2. Define the command `struct` with `#[derive(Parser, Debug)]`.
3. Export the module in `src/commands/mod.rs`.
4. Register the variant in the `Commands` enum in `src/cli.rs`.
5. Implement the `call()` method logic.

### Coding Standards
- **Errors:** Use `anyhow::Result` for all command-level fallible functions.
- **Interactivity:** Use `dialoguer` for menus and confirmations.
- **Icons:** Use `colored` for status icons: `✓` (green), `✗` (red), `↻` (blue), `💡` (yellow).
- **Async:** Use `tokio` for runtime and `reqwest` for all network I/O.
- **Data Integrity:** Use file locking (`std::fs::File::lock` / `lock_shared` / `unlock`, stable since Rust 1.89) when writing to env update files or the remote cache.

## Testing & Validation
### Testing Protocol
- **Isolation:** Every integration test MUST use `tempfile::tempdir()` and set `PVM_DIR` to that path.
- **CLI Verification:** Use `assert_cmd` and `predicates` for output and exit code verification.

### Development Workflow
- **Pre-commit:** Run `cargo clippy -- -D warnings` and `cargo fmt --all -- --check`.
- **Tooling:** Use `replace` or `write_file` for codebase modifications. Avoid `sed/echo` in shell.
- **Commit Messages:** Follow Conventional Commits. No backticks (Rule 1).
