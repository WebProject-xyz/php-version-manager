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
