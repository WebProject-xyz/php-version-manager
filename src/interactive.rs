use crate::commands;
use anyhow::Result;
use colored::Colorize;
use dialoguer::{Select, theme::ColorfulTheme};

pub async fn run_root_menu() -> Result<()> {
    loop {
        println!();
        let options = vec![
            "Use         (Switch active PHP version)",
            "Install     (Install a PHP version)",
            "Uninstall   (Remove a PHP version)",
            "Prune       (Remove superseded patch versions)",
            "List        (View locally installed versions)",
            "List-Remote (View all available cloud versions)",
            "Current     (Print the currently active PHP version)",
            "Default     (Set the default PHP version for new shells)",
            "Init        (Initialize a .php-version file)",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("PVM Interactive Menu")
            .default(0)
            .items(&options)
            .interact_opt()?;

        let choice = match selection {
            Some(idx) => idx,
            None => break, // Esc/Q exits the menu entirely
        };

        if choice == options.len() - 1 {
            break;
        }

        let res = match choice {
            0 => {
                commands::use_cmd::Use {
                    version: None,
                    silent: false,
                    yes: false,
                }
                .call()
                .await
            }
            1 => {
                commands::install::Install {
                    version: None,
                    packages: vec![],
                    yes: false,
                }
                .call()
                .await
            }
            2 => {
                commands::uninstall::Uninstall {
                    version: None,
                    yes: false,
                }
                .call()
                .await
            }
            3 => commands::prune::Prune { yes: false }.call().await,
            4 => commands::ls::Ls.call().await,
            5 => {
                commands::ls_remote::LsRemote {
                    version_prefix: None,
                }
                .call()
                .await
            }
            6 => commands::current::Current {}.call().await,
            7 => {
                commands::default_cmd::DefaultCmd { version: None }
                    .call()
                    .await
            }
            8 => commands::init::Init {}.call().await,
            _ => break,
        };

        if let Err(e) = res {
            eprintln!("{} Error: {}", "✗".red(), e);
        }
    }

    Ok(())
}
