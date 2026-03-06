use crate::fs;
use anyhow::{Result, bail};
use clap::Parser;
use colored::Colorize;

/// Toggle Xdebug on or off for a specific PHP version
#[derive(Parser, Debug)]
pub struct Xdebug {
    /// Mode: "on" or "off"
    pub mode: String,
    /// Target PHP version (minor or patch). Defaults to current active version.
    pub version: Option<String>,
}

impl Xdebug {
    pub async fn call(self) -> Result<()> {
        let mode = self.mode.to_lowercase();
        if mode != "on" && mode != "off" {
            bail!("Invalid mode '{}'. Expected 'on' or 'off'.", self.mode);
        }

        // Resolve target version
        let target_version = match self.version {
            Some(v) => fs::resolve_local_version(&v)?,
            None => {
                let current = fs::get_current_version();
                if current == "system" {
                    let installed = fs::list_installed_versions()?;
                    if installed.is_empty() {
                        bail!("No PHP versions are currently installed.");
                    }
                    installed
                        .last()
                        .cloned()
                        .expect("installed versions list should not be empty")
                } else {
                    current
                }
            }
        };

        if !fs::is_version_installed(&target_version)? {
            bail!(
                "PHP {} is not installed. Run 'pvm install {}' first.",
                target_version,
                target_version
            );
        }

        let bin_dir = fs::get_version_bin_dir(&target_version)?;
        let xdebug_so = bin_dir.join("ext").join("xdebug.so");
        if mode == "on" && !xdebug_so.exists() {
            bail!(
                "xdebug shared extension not found for PHP {} at {}. Ensure your build includes xdebug.",
                target_version,
                xdebug_so.display()
            );
        }

        let php_ini_path = fs::get_version_php_ini_path(&target_version)?;
        if let Some(parent) = php_ini_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !php_ini_path.exists() {
            std::fs::write(&php_ini_path, "; php.ini managed by pvm\n")?;
        }

        let contents = std::fs::read_to_string(&php_ini_path).unwrap_or_default();
        let mut lines: Vec<String> = contents.lines().map(|s| s.to_string()).collect();

        let xdebug_line = format!("zend_extension=\"{}\"", xdebug_so.to_string_lossy());

        if mode == "on" {
            // Remove any existing xdebug zend_extension lines, then add a clean one.
            lines.retain(|line| {
                let l = line.trim();
                !(l.starts_with("zend_extension") && l.to_lowercase().contains("xdebug"))
            });
            lines.push(xdebug_line);

            println!(
                "{} Enabled Xdebug for PHP {} using {}",
                "✓".green(),
                target_version.bold(),
                php_ini_path.display()
            );
        } else {
            let before_len = lines.len();
            lines.retain(|line| {
                let l = line.trim();
                !(l.starts_with("zend_extension") && l.to_lowercase().contains("xdebug"))
            });

            if before_len == lines.len() {
                println!(
                    "{} No Xdebug zend_extension entry found in {}; nothing to disable.",
                    "💡".yellow(),
                    php_ini_path.display()
                );
            } else {
                println!(
                    "{} Disabled Xdebug for PHP {} in {}",
                    "✓".green(),
                    target_version.bold(),
                    php_ini_path.display()
                );
            }
        }

        let new_contents = if lines.is_empty() {
            String::from("; php.ini managed by pvm\n")
        } else {
            let mut s = String::new();
            for line in lines {
                s.push_str(&line);
                s.push('\n');
            }
            s
        };
        std::fs::write(&php_ini_path, new_contents)?;

        Ok(())
    }
}
