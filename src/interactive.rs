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
            "List        (View locally installed versions)",
            "List-Remote (View all available cloud versions)",
            "Current     (Print the currently active PHP version)",
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

        if choice == 7 {
            break;
        }

        let res = match choice {
            0 => {
                let cmd = commands::use_cmd::Use { version: None };
                cmd.call().await
            }
            1 => {
                let cmd = commands::install::Install { version: None };
                cmd.call().await
            }
            2 => {
                let cmd = commands::uninstall::Uninstall { version: None };
                cmd.call().await
            }
            3 => {
                let cmd = commands::use_cmd::Use { version: None };
                cmd.call().await
            }
            4 => {
                let cmd = commands::ls_remote::LsRemote {
                    version_prefix: None,
                };
                cmd.call().await
            }
            5 => {
                let cmd = commands::current::Current {};
                cmd.call().await
            }
            6 => {
                let cmd = commands::init::Init {};
                cmd.call().await
            }
            _ => break,
        };

        if let Err(e) = res {
            eprintln!("{} Error: {}", "✗".red(), e);
        }
    }

    Ok(())
}
