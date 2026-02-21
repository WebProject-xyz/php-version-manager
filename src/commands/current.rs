use crate::fs;
use anyhow::Result;
use clap::Parser;

/// Print the currently active PHP version
#[derive(Parser, Debug)]
pub struct Current;

impl Current {
    pub async fn call(self) -> Result<()> {
        let current = fs::get_current_version();
        println!("{}", current);
        Ok(())
    }
}
