use std::process::Command;

fn main() {
    // Get the git tag if it exists on the current commit
    let git_tag = Command::new("git")
        .args(["describe", "--exact-match", "--tags", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .and_then(|tag| tag.strip_prefix('v').map(|s| s.to_string()).or(Some(tag)));

    let version = if let Some(tag) = git_tag {
        // Use the git tag (with 'v' prefix stripped)
        tag
    } else {
        // Use cargo package version + git short hash
        let pkg_version = env!("CARGO_PKG_VERSION");
        let git_hash = Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    String::from_utf8(output.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        format!("{}-{}", pkg_version, git_hash)
    };

    println!("cargo:rustc-env=ALLIUM_VERSION={}", version);
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
}
