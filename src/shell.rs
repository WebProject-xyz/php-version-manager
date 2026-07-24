use crate::constants::{ENV_UPDATE_FILE, PVM_DIR_VAR};
use std::path::Path;

pub trait Shell {
    /// Emit a PATH assignment of `bin_dir` followed by `rest`, an already
    /// pvm-free entry list from `fs::path_without_versions` (never `$PATH`).
    fn path(&self, bin_dir: &Path, rest: &[String]) -> String;
    fn set_env_var(&self, name: &str, value: &str) -> String;
    fn use_on_cd(&self) -> String;
    fn wrapper_fn(&self) -> String;
    /// Emit commands that drop every pvm-managed entry from PATH and clear
    /// the multishell marker, returning the shell to the system PHP.
    fn deactivate(&self, rest: &[String]) -> String;
}

/// Quote a string for POSIX shells (bash/zsh) by wrapping it in single quotes
/// and escaping any embedded single quotes via the `'\''` idiom.
fn posix_single_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Quote a string for fish by wrapping it in single quotes and escaping `\` and `'`.
fn fish_single_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            other => out.push(other),
        }
    }
    out.push('\'');
    out
}

/// `head` (if any) followed by `rest`, empty entries dropped: a stray `::` or a
/// trailing `:` in the inherited PATH means "current directory" to the shell.
fn path_entries<'a>(head: Option<&'a str>, rest: &'a [String]) -> impl Iterator<Item = &'a str> {
    head.into_iter()
        .chain(rest.iter().map(String::as_str))
        .filter(|p| !p.is_empty())
}

fn posix_path_assignment<'a>(entries: impl Iterator<Item = &'a str>) -> String {
    let joined = entries.collect::<Vec<_>>().join(":");
    format!("export PATH={}", posix_single_quote(&joined))
}

/// Quote each entry separately: PATH is a list in fish, not a `:`-joined string.
fn fish_path_assignment<'a>(entries: impl Iterator<Item = &'a str>) -> String {
    let words: Vec<String> = entries.map(fish_single_quote).collect();
    format!("set -gx PATH {}", words.join(" "))
}

fn posix_path(bin_dir: &Path, rest: &[String]) -> String {
    let bin = bin_dir.to_string_lossy();
    posix_path_assignment(path_entries(Some(bin.as_ref()), rest))
}

fn posix_set_env_var(name: &str, value: &str) -> String {
    format!("export {}={}", name, posix_single_quote(value))
}

fn posix_wrapper_fn() -> String {
    format!(
        "
case \":$PATH:\" in
  *\":${{{d}}}/bin:\"*) ;;
  *) export PATH=\"${{{d}}}/bin:$PATH\" ;;
esac

pvm() {{
  local command=$1
  if [[ \"$command\" == \"env\" ]]; then
    command pvm \"$@\"
  else
    if [[ -n \"${{{d}}}\" && -d \"${{{d}}}\" ]]; then
      local env_file=\"${{{d}}}/{f}_$$_${{RANDOM}}${{RANDOM}}_$(date +%s)\"
      [[ -f \"$env_file\" ]] && command rm -f \"$env_file\" 2>/dev/null
      PVM_ENV_UPDATE_PATH=\"$env_file\" command pvm \"$@\"
      local exit_code=$?
      if [[ -f \"$env_file\" ]]; then
        eval \"$(cat \"$env_file\")\"
        command rm -f \"$env_file\" 2>/dev/null
      fi
      return $exit_code
    else
      command pvm \"$@\"
    fi
  fi
}}
",
        d = PVM_DIR_VAR,
        f = ENV_UPDATE_FILE
    )
}

fn posix_deactivate(rest: &[String]) -> String {
    format!(
        "export PVM_MULTISHELL_PATH=''\n{}",
        posix_path_assignment(path_entries(None, rest))
    )
}

pub struct Bash;

impl Shell for Bash {
    fn path(&self, bin_dir: &Path, rest: &[String]) -> String {
        posix_path(bin_dir, rest)
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        posix_set_env_var(name, value)
    }

    fn use_on_cd(&self) -> String {
        // Bash has no chpwd hook, so this runs from PROMPT_COMMAND — i.e. after
        // every command, not just after a cd. Bail out unless the directory
        // actually changed, or every prompt would fork a pvm process.
        "
_pvm_cd_hook() {
  [[ \"$PWD\" == \"${__pvm_last_pwd-}\" ]] && return
  __pvm_last_pwd=\"$PWD\"
  if [[ -f .php-version ]]; then
    pvm use --silent \"$(cat .php-version)\" || true
  fi
}
if [[ -n \"$BASH_VERSION\" ]]; then
  if [[ ! \"$PROMPT_COMMAND\" == *\"_pvm_cd_hook\"* ]]; then
    PROMPT_COMMAND=\"_pvm_cd_hook; ${PROMPT_COMMAND:-}\"
  fi
fi
"
        .to_string()
    }

    fn wrapper_fn(&self) -> String {
        posix_wrapper_fn()
    }

    fn deactivate(&self, rest: &[String]) -> String {
        posix_deactivate(rest)
    }
}

pub struct Zsh;

impl Shell for Zsh {
    fn path(&self, bin_dir: &Path, rest: &[String]) -> String {
        posix_path(bin_dir, rest)
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        posix_set_env_var(name, value)
    }

    fn use_on_cd(&self) -> String {
        "
_pvm_cd_hook() {
  if [[ -f .php-version ]]; then
    pvm use --silent \"$(cat .php-version)\" || true
  fi
}
autoload -U add-zsh-hook
add-zsh-hook chpwd _pvm_cd_hook
"
        .to_string()
    }

    fn wrapper_fn(&self) -> String {
        posix_wrapper_fn()
    }

    fn deactivate(&self, rest: &[String]) -> String {
        posix_deactivate(rest)
    }
}

pub struct Fish;

impl Shell for Fish {
    fn path(&self, bin_dir: &Path, rest: &[String]) -> String {
        let bin = bin_dir.to_string_lossy();
        fish_path_assignment(path_entries(Some(bin.as_ref()), rest))
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        format!("set -gx {} {}", name, fish_single_quote(value))
    }

    fn use_on_cd(&self) -> String {
        "
function _pvm_cd_hook --on-variable PWD
    if test -f .php-version
        pvm use --silent (cat .php-version)
    end
end
"
        .to_string()
    }

    fn deactivate(&self, rest: &[String]) -> String {
        format!(
            "set -gx PVM_MULTISHELL_PATH ''\n{}",
            fish_path_assignment(path_entries(None, rest))
        )
    }

    fn wrapper_fn(&self) -> String {
        format!(
            "
if not contains \"${d}/bin\" $PATH
    set -gx PATH \"${d}/bin\" $PATH
end

function pvm
    set command $argv[1]
    if test \"$command\" = \"env\"
        command pvm $argv
    else
        if test -n \"${d}\"; and test -d \"${d}\"
            set env_file \"${d}/{f}_$fish_pid\"_(random)(random)_(date +%s)
            if test -f \"$env_file\"
                command rm -f \"$env_file\" &>/dev/null
            end
            PVM_ENV_UPDATE_PATH=\"$env_file\" command pvm $argv
            set exit_code $status
            if test -f \"$env_file\"
                source \"$env_file\"
                command rm -f \"$env_file\" &>/dev/null
            end
            return $exit_code
        else
            command pvm $argv
        end
    end
end
",
            d = PVM_DIR_VAR,
            f = ENV_UPDATE_FILE
        )
    }
}

pub fn detect_shell() -> Box<dyn Shell> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.ends_with("zsh") {
        Box::new(Zsh)
    } else if shell.ends_with("fish") {
        Box::new(Fish)
    } else {
        Box::new(Bash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stand-in for the non-pvm part of PATH. Deliberately synthetic: nothing
    /// here may depend on how the host or CI runner lays out its directories.
    fn rest() -> Vec<String> {
        vec!["/opt/tool-a/bin".to_string(), "/opt/tool-b/bin".to_string()]
    }

    #[test]
    fn test_bash_path_generation() {
        let bash = Bash;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            bash.path(path, &rest()),
            "export PATH='/home/user/.local/share/pvm/versions/8.3.1/bin:/opt/tool-a/bin:/opt/tool-b/bin'"
        );
    }

    #[test]
    fn test_posix_path_drops_empty_entries() {
        // A trailing/duplicated ':' in PATH means "current directory" — never
        // let one survive into the emitted assignment.
        let bash = Bash;
        let out = bash.path(
            std::path::Path::new("/pvm/versions/8.3.1/bin"),
            &[
                "".to_string(),
                "/opt/tool-a/bin".to_string(),
                "".to_string(),
            ],
        );
        assert_eq!(out, "export PATH='/pvm/versions/8.3.1/bin:/opt/tool-a/bin'");
    }

    #[test]
    fn test_deactivate_drops_empty_entries() {
        // Same guarantee on the way out: 'pvm use system' must not hand back a
        // PATH whose empty entry silently means the current directory.
        let out = Bash.deactivate(&["/opt/tool-a/bin".to_string(), "".to_string()]);
        assert_eq!(
            out,
            "export PVM_MULTISHELL_PATH=''\nexport PATH='/opt/tool-a/bin'"
        );
    }

    #[test]
    fn test_path_does_not_reference_shell_path_var() {
        // The whole point of baking the value in: re-activating must replace the
        // previous pvm entry, not prepend on top of the live $PATH again.
        let bash = Bash;
        let out = bash.path(std::path::Path::new("/pvm/versions/8.3.1/bin"), &rest());
        assert!(!out.contains("$PATH"), "got: {}", out);
    }

    #[test]
    fn test_bash_set_env() {
        let bash = Bash;
        assert_eq!(
            bash.set_env_var("PVM_MULTISHELL_PATH", "/some/path"),
            "export PVM_MULTISHELL_PATH='/some/path'"
        );
    }

    #[test]
    fn test_bash_set_env_escapes_special_chars() {
        let bash = Bash;
        assert_eq!(
            bash.set_env_var("X", "evil$(whoami)`id`\"$PATH\"'quote"),
            "export X='evil$(whoami)`id`\"$PATH\"'\\''quote'"
        );
    }

    #[test]
    fn test_zsh_path_generation() {
        let zsh = Zsh;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            zsh.path(path, &rest()),
            "export PATH='/home/user/.local/share/pvm/versions/8.3.1/bin:/opt/tool-a/bin:/opt/tool-b/bin'"
        );
    }

    #[test]
    fn test_fish_path_generation() {
        let fish = Fish;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            fish.path(path, &rest()),
            "set -gx PATH '/home/user/.local/share/pvm/versions/8.3.1/bin' '/opt/tool-a/bin' '/opt/tool-b/bin'"
        );
    }

    #[test]
    fn test_fish_set_env_escapes_special_chars() {
        let fish = Fish;
        assert_eq!(fish.set_env_var("X", "a'b\\c"), "set -gx X 'a\\'b\\\\c'");
    }

    #[test]
    fn test_bash_deactivate_restores_filtered_path() {
        let bash = Bash;
        let out = bash.deactivate(&rest());
        assert_eq!(
            out,
            "export PVM_MULTISHELL_PATH=''\nexport PATH='/opt/tool-a/bin:/opt/tool-b/bin'"
        );
    }

    #[test]
    fn test_fish_deactivate_restores_filtered_path() {
        let fish = Fish;
        let out = fish.deactivate(&rest());
        assert_eq!(
            out,
            "set -gx PVM_MULTISHELL_PATH ''\nset -gx PATH '/opt/tool-a/bin' '/opt/tool-b/bin'"
        );
    }

    /// Index of `needle`, or a failure naming the snippet that lacked it.
    fn offset_of(haystack: &str, needle: &str) -> usize {
        haystack
            .find(needle)
            .unwrap_or_else(|| panic!("{:?} missing from:\n{}", needle, haystack))
    }

    #[test]
    fn test_bash_cd_hook_skips_unchanged_pwd() {
        // PROMPT_COMMAND fires after every command, not just after a cd, so the
        // hook has to compare $PWD and bail before it ever reads .php-version —
        // otherwise every prompt forks a pvm process.
        let hook = Bash.use_on_cd();
        assert!(
            offset_of(&hook, "__pvm_last_pwd") < offset_of(&hook, ".php-version"),
            "cd hook reads .php-version before the $PWD guard:\n{}",
            hook
        );
    }

    #[test]
    fn test_wrapper_fn_guards_against_duplicate_bin_entry() {
        // Re-sourcing the rc file (nested shells, `exec bash`) must not stack
        // another $PVM_DIR/bin entry onto PATH, so the prepend has to sit behind
        // a membership test rather than run unconditionally.
        let bash = Bash.wrapper_fn();
        assert!(
            offset_of(&bash, "case \":$PATH:\"") < offset_of(&bash, "/bin:$PATH"),
            "bash prepends $PVM_DIR/bin unguarded:\n{}",
            bash
        );
        let fish = Fish.wrapper_fn();
        assert!(
            offset_of(&fish, "if not contains") < offset_of(&fish, "set -gx PATH"),
            "fish prepends $PVM_DIR/bin unguarded:\n{}",
            fish
        );
    }
}
