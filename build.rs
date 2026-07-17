use std::process::Command;

fn cmd_stdout(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");
    if let Ok(head) = std::fs::read_to_string(".git/HEAD")
        && let Some(ref_path) = head.strip_prefix("ref: ")
    {
        println!("cargo:rerun-if-changed=.git/{}", ref_path.trim());
    }
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-env-changed=GITHUB_ACTIONS");
    println!("cargo:rerun-if-env-changed=CI");
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

    // Only Linux/macOS are supported targets, so `date -u` is always available.
    let build_time =
        cmd_stdout("date", &["-u", "+%Y-%m-%dT%H:%M:%SZ"]).unwrap_or_else(|| "unknown".to_string());

    let is_ci = std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok();

    let version = if is_ci {
        let tag = cmd_stdout("git", &["describe", "--tags", "--always"])
            .unwrap_or_else(|| "unknown".to_string());
        format!("{} (built at: {})", tag, build_time)
    } else {
        let commit_hash = cmd_stdout("git", &["rev-parse", "--short", "HEAD"])
            .unwrap_or_else(|| "unknown".to_string());
        let pkg_version =
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
        format!(
            "{} (commit: {}, built at: {})",
            pkg_version, commit_hash, build_time
        )
    };

    println!("cargo:rustc-env=PVM_VERSION={}", version);
}
