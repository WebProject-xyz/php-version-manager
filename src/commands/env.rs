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
        let s: Box<dyn shell::Shell> = match self.shell.as_deref() {
            Some("bash") => Box::new(shell::Bash),
            Some("zsh") => Box::new(shell::Zsh),
            Some("fish") => Box::new(shell::Fish),
            _ => shell::detect_shell(),
        };

        println!("{}", s.wrapper_fn());
        println!("{}", s.use_on_cd());

        Ok(())
    }
}
