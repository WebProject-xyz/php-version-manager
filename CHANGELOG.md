## [2.0.1](https://github.com/WebProject-xyz/php-version-manager/compare/v2.0.0...v2.0.1) (2026-07-24)

### Bug Fixes

* stop PATH growing on every activation, guard the bash cd-hook, ship linux-aarch64 ([#41](https://github.com/WebProject-xyz/php-version-manager/issues/41)) ([0b7b39a](https://github.com/WebProject-xyz/php-version-manager/commit/0b7b39a25e83511e1c429f7e07f63d3619e07c09))

## [2.0.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.3.2...v2.0.0) (2026-07-24)

### ⚠ BREAKING CHANGES

* 'pvm ls-remote' no longer prompts to install a
selected version. Use 'pvm install' without arguments for the
interactive picker.

* feat(install): non-interactive installs via --packages, -y and non-TTY defaults

- pvm install gains --packages cli,fpm,micro (validated against the
  remote index) and -y/--yes; without a terminal the package selection
  defaults to cli instead of failing in dialoguer.
- pvm use gains -y/--yes for the install-missing and patch-update
  prompts; patch-update offers are now skipped entirely without a
  terminal so scripts never trigger surprise downloads.
- Patch updates reuse the package set of the version being replaced
  instead of re-prompting.
- New prompt::confirm helper centralizes Confirm handling (assume-yes,
  non-TTY returns the default); uninstall now uses it too.

* feat(install): detect installed packages and preselect missing ones

Installing an already-present version now prints what is installed,
preselects only the missing packages in the interactive selection,
and becomes an idempotent no-op for non-interactive callers when cli
is already there. Explicit --packages still reinstalls (repair).

* feat(default): persistent default version for new shells

pvm default <version> stores the version in PVM_DIR/default; pvm env
activates it on shell startup, so a chosen version finally survives
opening a new terminal. pvm default without argument shows an
interactive picker (or prints the current default when non-TTY), and
pvm default system clears it. Also added to the interactive menu.

* feat(use): support switching back to system PHP

pvm use system writes a deactivation snippet to the env-update file:
it clears PVM_MULTISHELL_PATH and filters every PVM_DIR/versions
entry out of PATH (tr/grep/paste for bash and zsh, string match for
fish). Until now there was no way back to the system PHP without
opening a new shell.

* feat(ls): interactive version switch from list with .php-version save offer

pvm ls on a terminal now shows the installed versions as a picker:
Enter switches the shell to the selection, Esc just exits. After a
picker-driven switch (ls or pvm use without arguments) pvm offers to
save the choice to .php-version. Scripts and pipes still get the
plain list. The activation tail of pvm use moved into a shared
activate() used by both flows, and a missing version without a
terminal now fails with a usage hint instead of a dialoguer error.

* feat(init): confirm .php-version overwrite and prefer installed versions

pvm init no longer overwrites an existing .php-version silently: it
shows the current content and asks first. The selection list now
offers locally installed versions (marked) before the remote
major.minor lines, and falls back to installed-only with a warning
when the remote index is unreachable. Without a terminal init fails
with a hint instead of a dialoguer error.

* feat(cache): add cache clear command for the remote version index

pvm cache clear deletes the cached remote_cache-<target>.json files
so a freshly published upstream patch becomes visible before the 24h
cache expiry, without hand-deleting files in PVM_DIR.

* feat(which): print the path of the active or given PHP binary

pvm which resolves the active version (or an explicit argument like
8.3) and prints the full path of its php binary - a debugging aid for
"which php am I actually running".

* feat(exec): run a command under a specific PHP version

pvm exec <version> <cmd...> prepends the version bin directory to
PATH for a single child process and propagates its exit code -
testing across versions without switching the shell.

* feat(prune): remove superseded patch versions

pvm prune [-y] deletes every installed patch that is no longer the
newest of its minor line, keeping the currently active version. If
the persisted default version gets pruned, it is re-pointed to the
kept patch of the same minor.

* test(e2e): cover default, use system, non-interactive install and prune

New cases 15-18 exercise pvm default persistence + env activation,
pvm use system PATH stripping, install --packages/-y idempotency and
prune with default re-pointing. The uninstall case moves to slot 19
and removes LATEST, since prune already dropped PREVIOUS. Cases guard
the single-upstream-patch situation where LATEST == PREVIOUS.

* docs(readme): document new commands and flows

Covers default/use system, which/exec/prune/cache clear, the
non-interactive install flags, the interactive ls picker and plain
output when piped, and replaces the stale fs4 reference with OS file
locks.

* docs(claude): track CLAUDE.md with current architecture

Previously untracked. Documents command-dispatch conventions incl.
the non-interactive requirements (prompt::confirm, IsTerminal
guards), the default-version and cache files in PVM_DIR, the shared
activate() tail, Shell::deactivate, and the offline test helpers.

* fix(default): keep default lifecycle consistent on uninstall

Uninstalling the persisted default version now clears the default
file with a hint (prune already re-points it); pvm env warns on
stderr when the stored default is not installed instead of silently
falling back to system PHP. Tests cover both paths plus the
non-TTY prune default.

* fix(use): no surprise installs without a terminal

Non-TTY 'pvm use <missing>' without --yes now fails with a hint
instead of auto-confirming a network install (both the argument and
.php-version paths), matching the existing patch-update gate. A
.php-version containing 'system' deactivates instead of offering to
install 'PHP system', --silent suppresses the deactivation message
for cd-hooks, and Esc in the version picker exits quietly as the ls
prompt promises.

* fix(install): make -y fully non-interactive and honest without a TTY

-y now also skips the package MultiSelect (defaults to cli, like a
missing terminal); without a terminal and without -y the trailing
'use now?' question no longer auto-answers yes, which printed a
'Switched to PHP' message although no shell evaluates the env file.

* fix(exec): propagate signal terminations as 128 plus signal number

Signal-killed children previously collapsed to exit code 1; now they
follow the shell convention so callers can distinguish SIGTERM from
a plain failure.

* refactor(use): deduplicate the .php-version save blocks in activate

Both the picker and explicit-argument flows carried the same
confirm-write-report block; save_question() now picks the wording
and one shared block does the writing.

* refactor(fs): consolidate minor-version extraction into fs::minor_of

init.rs and prune.rs carried identical private helpers; fs.rs,
update.rs and the install picker had three more inline split
variants of the same logic.

* refactor(fs): drop dead .exe checks in get_installed_packages

Windows is an unsupported target (get_target_triple bails before any
install), so the .exe fallbacks could never match.

* refactor(interactive): construct and call commands inline in menu arms

Every arm bound the command to a temporary before calling it; call
directly on the struct literal instead.

* chore(gitignore): ignore local .claude directory

Claude Code drops runtime files (locks, worktrees) there; they are
machine-local and must not land in commits.

* fix(review): address CodeRabbit findings on prune, install picker and cd-hook

- pvm prune without a terminal now requires --yes before mass
  deletion, matching the non-TTY guards on use and install.
- Explicit --packages survives the already-installed short-circuit in
  the install picker, so packages can be added to an existing version.
- The silent cd-hook never evaluates the .php-version save prompt; a
  file holding a partial version like 8.3 would otherwise prompt on
  every cd.

### Features

* command-flow overhaul, persistent default, non-interactive installs + de-bloat ([#40](https://github.com/WebProject-xyz/php-version-manager/issues/40)) ([46ca57d](https://github.com/WebProject-xyz/php-version-manager/commit/46ca57dbb8a478d56a278b199cbf1fc25594500c))

## [1.3.2](https://github.com/WebProject-xyz/php-version-manager/compare/v1.3.1...v1.3.2) (2026-06-12)

### Bug Fixes

* **deps:** update deps ([32da128](https://github.com/WebProject-xyz/php-version-manager/commit/32da12857725986b4e11adf2b26e50de20b134f0))

## [1.3.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.3.0...v1.3.1) (2026-05-22)

### Bug Fixes

* **ci:** use x86_64-unknown-linux-gnu as Rust target in build matrix ([58f3b40](https://github.com/WebProject-xyz/php-version-manager/commit/58f3b40dfcb3aeb348ad7f0b4b95b09525ade595))

## [1.3.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.2.1...v1.3.0) (2026-05-22)

### Features

* implement pvm optimizations, robust uninstall logic and php-version auto-detection ([#30](https://github.com/WebProject-xyz/php-version-manager/issues/30)) ([ccaa069](https://github.com/WebProject-xyz/php-version-manager/commit/ccaa0697a6060da8966aebeb5899fb929d14a1f6))

## [1.2.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.2.0...v1.2.1) (2026-05-08)

### Bug Fixes

* **use:** prompt to install when target version missing ([#24](https://github.com/WebProject-xyz/php-version-manager/issues/24)) ([ceec711](https://github.com/WebProject-xyz/php-version-manager/commit/ceec7113d7c3bb8f52db0c69700fcf31dd75060a))

## [1.2.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.2...v1.2.0) (2026-05-07)

### Features

* **self-update:** add self-update command for in-place pvm upgrades ([#19](https://github.com/WebProject-xyz/php-version-manager/issues/19)) ([d77f0ef](https://github.com/WebProject-xyz/php-version-manager/commit/d77f0efd9d7aa8ef6ed5ca55a3d4524c4ab1ab7c))

## [1.1.2](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.1...v1.1.2) (2026-05-06)

### Bug Fixes

* **ci:** pass App token via 'token' input to action-gh-release ([#22](https://github.com/WebProject-xyz/php-version-manager/issues/22)) ([abeeac3](https://github.com/WebProject-xyz/php-version-manager/commit/abeeac3aa6e53cd35c34f20957a9f6b91e232d67)), closes [softprops/action-gh-release#751](https://github.com/softprops/action-gh-release/issues/751) [#20](https://github.com/WebProject-xyz/php-version-manager/issues/20)

## [1.1.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.1.0...v1.1.1) (2026-05-06)

### Bug Fixes

* **ci:** pin build matrix toolchain to 1.95.0 ([4b8fbdd](https://github.com/WebProject-xyz/php-version-manager/commit/4b8fbdd2faa9cb33e2bcd200ad74c63eb35a1c66)), closes [#20](https://github.com/WebProject-xyz/php-version-manager/issues/20)

## [1.1.0](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.4...v1.1.0) (2026-04-29)

### Features

* improve concurrency safety, cross-platform support and dynamic versioning ([85e6c80](https://github.com/WebProject-xyz/php-version-manager/commit/85e6c806e3abc5b8e2c011a48bb1a36b8dac4614))
* improve concurrency safety, cross-platform support and dynamic versioning ([3bc8179](https://github.com/WebProject-xyz/php-version-manager/commit/3bc8179a1323e42add24f9c113ca7d2d086df25a))
* support multiple PHP packages via new bulk API ([5f20206](https://github.com/WebProject-xyz/php-version-manager/commit/5f202060d39a4ec88e2e07828cae75a8c484bfd2))

### Bug Fixes

* address CodeRabbit review findings ([05f9621](https://github.com/WebProject-xyz/php-version-manager/commit/05f9621d423dcd4cfe6458f9132c18efcb3256e4))
* address remaining CodeRabbit findings on PR [#6](https://github.com/WebProject-xyz/php-version-manager/issues/6) ([e6130ed](https://github.com/WebProject-xyz/php-version-manager/commit/e6130ed1fd30f61b84ac179523e98582ddd5b4ad))
* address second-round CodeRabbit review on PR [#6](https://github.com/WebProject-xyz/php-version-manager/issues/6) ([0f9c7f0](https://github.com/WebProject-xyz/php-version-manager/commit/0f9c7f06e46018cce3c5c441b8b696211cff480e))
* resolve clippy::collapsible-if lint in network.rs ([4923dc1](https://github.com/WebProject-xyz/php-version-manager/commit/4923dc12a76dfb0592f0e18893973d053be3ab8c))
* **shell:** add RANDOM entropy to env_file names ([19efe4c](https://github.com/WebProject-xyz/php-version-manager/commit/19efe4cb0444b52dd7476d3c8c9f1eb4b32567bc))

## [1.0.4](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.3...v1.0.4) (2026-02-22)

### Bug Fixes

* add Renovate configuration file ([cb24073](https://github.com/WebProject-xyz/php-version-manager/commit/cb24073f88119813a81c703ec63215dcb1e55486))

## [1.0.3](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.2...v1.0.3) (2026-02-21)

### Bug Fixes

* remove cargo check and allow rust to publish with dirty lockfile ([fefddcd](https://github.com/WebProject-xyz/php-version-manager/commit/fefddcd5f4a43ea95ea9283c684a68d2bf042b3d))
* skip dev profile verification during cargo publish and sync lockfile ([c67edb5](https://github.com/WebProject-xyz/php-version-manager/commit/c67edb57cf24cef830fbd2778d297e6a54edcb4d))

## [1.0.2](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.1...v1.0.2) (2026-02-21)

### Bug Fixes

* update Cargo.lock and allow-dirty to bypass publish error ([be15c3a](https://github.com/WebProject-xyz/php-version-manager/commit/be15c3a762867c57c7a046bedda86cd2f7f0515f))
* update Cargo.lock during semantic-release prepare phase to avoid publish errors ([a92c3de](https://github.com/WebProject-xyz/php-version-manager/commit/a92c3de5e5a74dcb80bb3313f1ed83d19905c99d))

## [1.0.1](https://github.com/WebProject-xyz/php-version-manager/compare/v1.0.0...v1.0.1) (2026-02-21)

### Bug Fixes

* pass github app token to action-gh-release to avoid permission error ([5461490](https://github.com/WebProject-xyz/php-version-manager/commit/546149064c52b2740257607284ab047536bac45c))
* re-architect release pipeline to ensure version correctness ([8636f04](https://github.com/WebProject-xyz/php-version-manager/commit/8636f04932334de987dda03ee9745caf25dd41ad))

## 1.0.0 (2026-02-21)

### Features

* initial commit for pvm ([15ce607](https://github.com/WebProject-xyz/php-version-manager/commit/15ce607227eeb046eeb3fa275221ce4df212342b))

### Bug Fixes

* use GitHub App token for semantic-release ([b8e2354](https://github.com/WebProject-xyz/php-version-manager/commit/b8e235425c8ba2aba1f2bf76d4d82b3d313a679f))
* use macos-latest for x86_64 target to fix unsupported runner error ([98cd2af](https://github.com/WebProject-xyz/php-version-manager/commit/98cd2af7378186cdea35ee904db1c685fc269caf))
