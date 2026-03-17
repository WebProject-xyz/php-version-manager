use crate::constants::{ENV_UPDATE_FILE, PVM_DIR_VAR};
use std::path::Path;

pub trait Shell {
    fn path(&self, path: &Path) -> String;
    fn set_env_var(&self, name: &str, value: &str) -> String;
    fn use_on_cd(&self) -> String;
    fn wrapper_fn(&self) -> String;
}

pub struct Bash;

impl Shell for Bash {
    fn path(&self, path: &Path) -> String {
        format!("export PATH=\"{}:$PATH\"", path.display())
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        format!("export {}=\"{}\"", name, value)
    }

    fn use_on_cd(&self) -> String {
        "
_pvm_cd_hook() {
  if [[ -f .php-version ]]; then
    pvm use \"$(cat .php-version)\" || true
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
        format!(
            "
export PATH=\"${{{}}}/bin:$PATH\"

pvm() {{
  local command=$1
  if [[ \"$command\" == \"env\" ]]; then
    command pvm \"$@\"
  else
    if [[ -n \"${{{}}}\" && -d \"${{{}}}\" ]]; then
      local env_file=\"${{{}}}/{}_$(date +%s)_$$\"
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
            PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, ENV_UPDATE_FILE
        )
    }
}

pub struct Zsh;

impl Shell for Zsh {
    fn path(&self, path: &Path) -> String {
        format!("export PATH=\"{}:$PATH\"", path.display())
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        format!("export {}=\"{}\"", name, value)
    }

    fn use_on_cd(&self) -> String {
        "
_pvm_cd_hook() {
  if [[ -f .php-version ]]; then
    pvm use \"$(cat .php-version)\" || true
  fi
}
autoload -U add-zsh-hook
add-zsh-hook chpwd _pvm_cd_hook
"
        .to_string()
    }

    fn wrapper_fn(&self) -> String {
        format!(
            "
export PATH=\"${{{}}}/bin:$PATH\"

pvm() {{
  local command=$1
  if [[ \"$command\" == \"env\" ]]; then
    command pvm \"$@\"
  else
    if [[ -n \"${{{}}}\" && -d \"${{{}}}\" ]]; then
      local env_file=\"${{{}}}/{}_$(date +%s)_$$\"
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
            PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, ENV_UPDATE_FILE
        )
    }
}

pub struct Fish;

impl Shell for Fish {
    fn path(&self, path: &Path) -> String {
        format!("set -gx PATH \"{}\" $PATH", path.display())
    }

    fn set_env_var(&self, name: &str, value: &str) -> String {
        format!("set -gx {} \"{}\"", name, value)
    }

    fn use_on_cd(&self) -> String {
        "
function _pvm_cd_hook --on-variable PWD
    if test -f .php-version
        pvm use (cat .php-version)
    end
end
"
        .to_string()
    }

    fn wrapper_fn(&self) -> String {
        format!(
            "
set -gx PATH \"${{{}}}/bin\" $PATH

function pvm
    set command $argv[1]
    if test \"$command\" = \"env\"
        command pvm $argv
    else
        if test -n \"${{{}}}\"; and test -d \"${{{}}}\"
            set env_file \"${{{}}}/{}_$(date +%s)_$fish_pid\"
            if test -f \"$env_file\"
                command rm -f \"$env_file\" &>/dev/null
            end
            PVM_ENV_UPDATE_PATH=\"$env_file\" command pvm $argv
            set exit_code $status
            if test -f \"$env_file\"
                eval (cat \"$env_file\")
                command rm -f \"$env_file\" &>/dev/null
            end
            return $exit_code
        else
            command pvm $argv
        end
    end
end
",
            PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, PVM_DIR_VAR, ENV_UPDATE_FILE
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

    #[test]
    fn test_bash_path_generation() {
        let bash = Bash;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            bash.path(path),
            "export PATH=\"/home/user/.local/share/pvm/versions/8.3.1/bin:$PATH\""
        );
    }

    #[test]
    fn test_bash_set_env() {
        let bash = Bash;
        assert_eq!(
            bash.set_env_var("PVM_MULTISHELL_PATH", "/some/path"),
            "export PVM_MULTISHELL_PATH=\"/some/path\""
        );
    }

    #[test]
    fn test_zsh_path_generation() {
        let zsh = Zsh;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            zsh.path(path),
            "export PATH=\"/home/user/.local/share/pvm/versions/8.3.1/bin:$PATH\""
        );
    }

    #[test]
    fn test_fish_path_generation() {
        let fish = Fish;
        let path = std::path::Path::new("/home/user/.local/share/pvm/versions/8.3.1/bin");
        assert_eq!(
            fish.path(path),
            "set -gx PATH \"/home/user/.local/share/pvm/versions/8.3.1/bin\" $PATH"
        );
    }
}
