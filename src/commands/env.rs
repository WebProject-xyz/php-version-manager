use crate::constants::{MULTISHELL_PATH_VAR, PVM_DIR_VAR};
use crate::shell;
use anyhow::Result;
use clap::Parser;

/// Print and set up required environment variables for pvm
///
/// This command generates a series of shell commands that
/// should be evaluated by your shell to create a pvm-ready environment.
///
/// Evaluating pvm on Bash and Zsh looks like `eval "$(pvm env)"`.
/// In Fish, evaluating looks like `pvm env | source`.
#[derive(Parser, Debug)]
pub struct Env {
    /// Override the detected shell (bash, zsh, fish)
    #[arg(long)]
    pub shell: Option<String>,
}

impl Env {
    pub async fn call(self) -> Result<()> {
        let pvm_dir = crate::fs::get_pvm_dir()?;
        let s: Box<dyn shell::Shell> = match self.shell.as_deref() {
            Some("bash") => Box::new(shell::Bash),
            Some("zsh") => Box::new(shell::Zsh),
            Some("fish") => Box::new(shell::Fish),
            _ => shell::detect_shell(),
        };

        println!("{}", s.set_env_var(PVM_DIR_VAR, &pvm_dir.to_string_lossy()));
        println!("{}", s.wrapper_fn());
        println!("{}", s.use_on_cd());

        // Activate the persisted default version ('pvm default') so every new
        // shell starts on it instead of the system PHP.
        if let Some(version) = crate::fs::get_default_version()? {
            if crate::fs::is_version_installed(&version)? {
                let bin_dir = crate::fs::get_version_bin_dir(&version)?;
                println!(
                    "{}",
                    s.set_env_var(MULTISHELL_PATH_VAR, &bin_dir.to_string_lossy())
                );
                println!("{}", s.path(&bin_dir));
            } else {
                // stderr so the eval'd stdout stays clean.
                eprintln!(
                    "pvm: default version {} is not installed; run 'pvm default' to fix",
                    version
                );
            }
        }

        Ok(())
    }
}
