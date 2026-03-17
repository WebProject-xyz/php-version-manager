use crate::commands;
use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pvm",
    version = env!("PVM_VERSION"),
    author,
    about = "Fast and simple PHP version manager",
    disable_version_flag = true,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short = 'v', short_alias = 'V', long = "version", action = clap::ArgAction::Version)]
    pub version: Option<bool>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install a specific PHP version
    #[clap(name = "install", visible_aliases = &["i"])]
    Install(commands::install::Install),

    /// Change PHP version
    #[clap(name = "use")]
    Use(commands::use_cmd::Use),

    /// Print and set up required environment variables for pvm
    #[clap(name = "env")]
    Env(commands::env::Env),

    /// List all locally installed PHP versions
    #[clap(name = "list", visible_aliases = &["ls"])]
    Ls(commands::ls::Ls),

    /// List all remote available PHP versions
    #[clap(name = "ls-remote", visible_aliases = &["list-remote"])]
    LsRemote(commands::ls_remote::LsRemote),

    /// Print the currently active PHP version
    #[clap(name = "current")]
    Current(commands::current::Current),

    /// Uninstall a PHP version
    #[clap(name = "uninstall", visible_aliases = &["rm", "remove"])]
    Uninstall(commands::uninstall::Uninstall),

    /// Initialize a .php-version file in the current directory
    #[clap(name = "init")]
    Init(commands::init::Init),
}

impl Commands {
    pub async fn call(self) -> Result<()> {
        match self {
            Self::Install(cmd) => cmd.call().await,
            Self::Use(cmd) => cmd.call().await,
            Self::Env(cmd) => cmd.call().await,
            Self::Ls(cmd) => cmd.call().await,
            Self::LsRemote(cmd) => cmd.call().await,
            Self::Current(cmd) => cmd.call().await,
            Self::Uninstall(cmd) => cmd.call().await,
            Self::Init(cmd) => cmd.call().await,
        }
    }
}
