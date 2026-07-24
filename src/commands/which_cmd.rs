use crate::fs;
use anyhow::Result;
use clap::Parser;

/// Print the path of the active or given PHP binary
#[derive(Parser, Debug)]
pub struct Which {
    /// The version to inspect (defaults to the active one)
    pub version: Option<String>,
}

impl Which {
    pub async fn call(self) -> Result<()> {
        let version = match self.version {
            Some(ref v) => fs::resolve_local_version(v)?,
            None => {
                let current = fs::get_current_version();
                if current == "system" {
                    anyhow::bail!("no pvm-managed PHP is active (try: pvm which <version>)");
                }
                current
            }
        };

        let php_path = fs::get_version_bin_dir(&version)?.join("php");
        if !php_path.exists() {
            anyhow::bail!("PHP {} has no cli package installed", version);
        }

        // Plain output (no icons) so scripts can consume the path directly.
        println!("{}", php_path.display());
        Ok(())
    }
}
