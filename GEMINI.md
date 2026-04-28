# Gemini AI Assistant Instructions

CRITICAL SAFETY DIRECTIVE:
**NEVER use backticks (\`) inside git commit messages.**

## Why?
When running `git commit -m "..."` in the shell (like Zsh/Bash), backticks are immediately evaluated as command substitution by the user's shell BEFORE the commit is executed.

If a file, branch, or text snippet contains a command-like string inside backticks (e.g. \`rm -rf /\`, \`install\`, \`chmod\`), the shell will execute it silently with the user's full privileges. This is an extreme security and stability risk that can destroy the host system.

## Rule
1. Do not use backticks (\`) in commit bodies or summaries.
2. If you need to quote code, files, or strings in a commit message, use single quotes (e.g. 'filename.rs') or double quotes (e.g. "function_name") instead.
3. Obey this rule forever, until the end of electronics.

## static-php-cli Integration
- **Endpoint:** `https://dl.static-php.dev/static-php-cli/bulk/`
- **Supported OS:** `linux`, `macos` (no Windows support in bulk JSON).
- **Supported Arch:** `x86_64`, `aarch64`.
- **Packages:** `cli`, `fpm`, `micro`.
- **Format:** `tar.gz` only.
- **Cache:** Remote versions are cached in `remote_cache.json` for 24 hours.

## Development Workflow
- **File Edits:** Use `replace` or `write_file` tools. Avoid `cat`, `echo`, or `sed` in shell commands for codebase modifications.
- **Testing:** Run `cargo test` before submitting changes.
- **Commit Messages:** Follow Conventional Commits. No backticks (Rule 1).

