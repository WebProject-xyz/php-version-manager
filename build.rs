use std::process::Command;

fn main() {
    let commit_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let build_time = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let is_ci = std::env::var("GITHUB_ACTIONS").is_ok();

    let version = if is_ci {
        let tag = Command::new("git")
            .args(["describe", "--tags", "--always"])
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        format!("{} (built at: {})", tag, build_time)
    } else {
        let pkg_version =
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".to_string());
        format!(
            "{} (commit: {}, built at: {})",
            pkg_version, commit_hash, build_time
        )
    };

    println!("cargo:rustc-env=PVM_VERSION={}", version);
}
