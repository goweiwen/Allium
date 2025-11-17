use anyhow::{Context, Result};
use common::constants::{ALLIUM_SD_ROOT, ALLIUM_UPDATE_SETTINGS};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::LazyLock;
use tokio::sync::mpsc;

const GITHUB_REPOSITORY: &str = "goweiwen/Allium";
const RELEASE_FILE: &str = "allium-armv7-unknown-linux-gnueabihf.zip";

static UPDATE_FILE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| ALLIUM_SD_ROOT.join("allium-ota.zip"));

/// Update channel selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpdateChannel {
    #[default]
    Stable,
    Nightly,
}

/// Update settings that are persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSettings {
    pub channel: UpdateChannel,
}

impl UpdateSettings {
    pub fn load() -> Result<Self> {
        if ALLIUM_UPDATE_SETTINGS.exists() {
            debug!("found update settings, loading from file");
            let file = File::open(ALLIUM_UPDATE_SETTINGS.as_path())?;
            if let Ok(json) = serde_json::from_reader(file) {
                return Ok(json);
            }
            warn!("failed to read update settings file, removing");
            fs::remove_file(ALLIUM_UPDATE_SETTINGS.as_path())?;
        }
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<()> {
        let file = File::create(ALLIUM_UPDATE_SETTINGS.as_path())?;
        serde_json::to_writer(file, &self)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub assets: Vec<GitHubAsset>,
    #[serde(default)]
    pub prerelease: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    #[serde(default)]
    pub digest: Option<String>,
}

/// GitHub API response for git reference (tag)
#[derive(Debug, Clone, Deserialize)]
struct GitRef {
    object: GitObject,
}

#[derive(Debug, Clone, Deserialize)]
struct GitObject {
    sha: String,
    #[serde(rename = "type")]
    object_type: String,
}

/// GitHub API response for tag object (annotated tags)
#[derive(Debug, Clone, Deserialize)]
struct GitTag {
    object: GitTagObject,
}

#[derive(Debug, Clone, Deserialize)]
struct GitTagObject {
    sha: String,
}

/// Check if an update is available for the given channel
/// Returns the GitHubRelease if an update is available
pub async fn check_for_update(channel: UpdateChannel) -> Result<Option<GitHubRelease>> {
    let current_version = common::constants::ALLIUM_VERSION;
    info!("Current version: {}", current_version);

    let release = match channel {
        UpdateChannel::Stable => get_latest_stable_release().await?,
        UpdateChannel::Nightly => get_latest_nightly_release().await?,
    };

    let latest_version = release.tag_name.trim_start_matches('v');
    info!("Latest version: {}", latest_version);

    // For nightly, always show as available if it's a different commit
    // For stable, compare version strings
    let is_different = match channel {
        UpdateChannel::Stable => current_version != latest_version,
        UpdateChannel::Nightly => {
            // For nightly, consider it different if:
            // 1. The base version is different, OR
            // 2. We're on stable and there's any nightly available
            let base_version = latest_version.split('-').next().unwrap_or(latest_version);
            current_version != latest_version
                && (current_version != base_version || !current_version.contains('-'))
        }
    };

    if is_different {
        Ok(Some(release))
    } else {
        Ok(None)
    }
}

/// Get the version string for a release (fetches commit hash for nightly/prerelease)
pub async fn get_release_version(release: &GitHubRelease) -> String {
    let base_version = release.tag_name.trim_start_matches('v');

    if release.prerelease {
        // For nightly/prerelease, fetch and append short commit hash
        if let Ok(commit_sha) = get_tag_commit_sha(&release.tag_name).await {
            let short_hash = if commit_sha.len() >= 7 {
                &commit_sha[..7]
            } else {
                &commit_sha
            };
            return format!("{}-{}", base_version, short_hash);
        }
    }

    base_version.to_string()
}

/// Get the latest stable release from GitHub
async fn get_latest_stable_release() -> Result<GitHubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPOSITORY
    );

    let client = reqwest::Client::builder()
        .user_agent("Allium-OTA-Updater")
        .build()?;

    let release: GitHubRelease = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch latest release")?
        .json()
        .await
        .context("Failed to parse release JSON")?;

    Ok(release)
}

/// Get the latest nightly (prerelease) from GitHub
async fn get_latest_nightly_release() -> Result<GitHubRelease> {
    let url = format!(
        "https://api.github.com/repos/{}/releases",
        GITHUB_REPOSITORY
    );

    let client = reqwest::Client::builder()
        .user_agent("Allium-OTA-Updater")
        .build()?;

    let releases: Vec<GitHubRelease> = client
        .get(&url)
        .query(&[("per_page", "20")]) // Get recent releases
        .send()
        .await
        .context("Failed to fetch releases")?
        .json()
        .await
        .context("Failed to parse releases JSON")?;

    // Find the first prerelease (they're returned in date order, newest first)
    releases
        .into_iter()
        .find(|r| r.prerelease)
        .context("No nightly release found")
}

/// Fetch the commit SHA for a given tag
async fn get_tag_commit_sha(tag_name: &str) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("Allium-OTA-Updater")
        .build()?;

    let url = format!(
        "https://api.github.com/repos/{}/git/ref/tags/{}",
        GITHUB_REPOSITORY, tag_name
    );

    let git_ref: GitRef = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch tag reference")?
        .json()
        .await
        .context("Failed to parse tag reference JSON")?;

    // If the object type is "tag" (annotated tag), we need to dereference it
    if git_ref.object.object_type == "tag" {
        let tag_url = format!(
            "https://api.github.com/repos/{}/git/tags/{}",
            GITHUB_REPOSITORY, git_ref.object.sha
        );

        let git_tag: GitTag = client
            .get(&tag_url)
            .send()
            .await
            .context("Failed to fetch tag object")?
            .json()
            .await
            .context("Failed to parse tag object JSON")?;

        Ok(git_tag.object.sha)
    } else {
        // Lightweight tag - points directly to commit
        Ok(git_ref.object.sha)
    }
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

impl DownloadProgress {
    pub fn percentage(&self) -> u8 {
        if self.total == 0 {
            0
        } else {
            ((self.downloaded as f64 / self.total as f64) * 100.0) as u8
        }
    }
}

/// Download event - either progress or completion/error
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    Progress(DownloadProgress),
    Completed,
    Error(String),
}

/// Download a release to /mnt/SDCARD/allium-ota.zip, reporting the progress to event_tx
pub async fn download_update_with_progress(
    release: &GitHubRelease,
    event_tx: Option<mpsc::UnboundedSender<DownloadEvent>>,
) -> Result<()> {
    // Check if there's enough space (need 300MB)
    #[cfg(feature = "miyoo")]
    {
        let output = std::process::Command::new("df")
            .args(["-m", "/mnt/SDCARD"])
            .output()
            .context("Failed to check disk space")?;

        let output_str = String::from_utf8_lossy(&output.stdout);
        let available_space: i32 = output_str
            .lines()
            .nth(1)
            .and_then(|line| line.split_whitespace().nth(3))
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if available_space < 300 {
            anyhow::bail!(
                "Insufficient disk space. Need 300MB, have {}MB",
                available_space
            );
        }
    }

    let version = release.tag_name.trim_start_matches('v');
    info!("Downloading update version {}", version);

    // Find the asset with the expected filename
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == RELEASE_FILE)
        .context(format!("Asset '{}' not found in release", RELEASE_FILE))?;

    let expected_hash = asset
        .digest
        .as_ref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .context("SHA256 digest not found for release asset")?;

    info!("Downloading from: {}", asset.browser_download_url);
    let mut response = reqwest::get(&asset.browser_download_url)
        .await
        .context("Failed to download update")?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download update: HTTP {}", response.status());
    }

    // Get content length for progress reporting
    let total_size = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;

    // Create file and hasher
    info!("Writing update to {}", UPDATE_FILE_PATH.display());
    let file = File::create(&*UPDATE_FILE_PATH).context("Failed to create update file")?;
    let mut writer = BufWriter::new(file);
    let mut hasher = Sha256::new();

    // Stream download to file while calculating hash
    while let Some(chunk) = response.chunk().await.context("Failed to read chunk")? {
        writer
            .write_all(&chunk)
            .context("Failed to write to file")?;
        hasher.update(&chunk);

        downloaded += chunk.len() as u64;

        // Send progress update
        if let Some(ref tx) = event_tx {
            info!(
                "Downloaded {}% ({}/{} bytes)",
                downloaded * 100 / total_size,
                downloaded,
                total_size
            );
            let _ = tx.send(DownloadEvent::Progress(DownloadProgress {
                downloaded,
                total: total_size,
            }));
        }
    }

    writer.flush().context("Failed to flush file")?;

    // Verify SHA256 checksum
    info!("Verifying SHA256 checksum...");
    let calculated_hash = format!("{:x}", hasher.finalize());

    if calculated_hash != expected_hash {
        // Delete the file if verification fails
        let _ = std::fs::remove_file(&*UPDATE_FILE_PATH);
        let error_msg = format!(
            "SHA256 checksum mismatch!\nExpected: {}\nCalculated: {}",
            expected_hash, calculated_hash
        );
        if let Some(ref tx) = event_tx {
            let _ = tx.send(DownloadEvent::Error(error_msg.clone()));
        }
        anyhow::bail!(error_msg);
    }

    info!("Update downloaded successfully");
    if let Some(ref tx) = event_tx {
        let _ = tx.send(DownloadEvent::Completed);
    }
    Ok(())
}

pub fn update_file_exists() -> bool {
    UPDATE_FILE_PATH.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stable_release(tag: &str) -> GitHubRelease {
        GitHubRelease {
            tag_name: tag.to_string(),
            assets: vec![],
            prerelease: false,
        }
    }

    #[tokio::test]
    async fn test_get_release_version_stable() {
        let release = make_stable_release("v0.29.0");
        let version = get_release_version(&release).await;
        assert_eq!(version, "0.29.0");
    }

    #[tokio::test]
    async fn test_get_release_version_stable_without_v_prefix() {
        let release = make_stable_release("0.29.0");
        let version = get_release_version(&release).await;
        assert_eq!(version, "0.29.0");
    }

    #[tokio::test]
    async fn test_stable_version_format_matches_semver() {
        let release = make_stable_release("v1.2.3");
        let version = get_release_version(&release).await;
        // Should match semver format: X.Y.Z
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[0].parse::<u32>().is_ok());
        assert!(parts[1].parse::<u32>().is_ok());
        assert!(parts[2].parse::<u32>().is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_check_for_update_stable() {
        let result = check_for_update(UpdateChannel::Stable).await;
        assert!(result.is_ok(), "Failed to check for update: {:?}", result);
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_check_for_update_nightly() {
        let result = check_for_update(UpdateChannel::Nightly).await;
        assert!(result.is_ok(), "Failed to check for update: {:?}", result);
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_stable_release_version_format() {
        let release = get_latest_stable_release()
            .await
            .expect("Failed to get stable release");

        let version = get_release_version(&release).await;
        println!("Stable version: {}", version);

        // Stable should not have a dash (no commit hash)
        assert!(
            !version.starts_with('v'),
            "Stable version should start without 'v': {}",
            version
        );

        // Should be semver format
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "Version should have 3 parts (semver): {}",
            version
        );
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_nightly_release_version_format() {
        let release = get_latest_nightly_release()
            .await
            .expect("Failed to get nightly release");

        let version = get_release_version(&release).await;
        println!("Nightly version: {}", version);

        // Nightly should have format: X.Y.Z-HHHHHHH
        assert!(
            version.contains('-'),
            "Nightly version should contain commit hash: {}",
            version
        );

        let parts: Vec<&str> = version.split('-').collect();
        assert_eq!(
            parts.len(),
            2,
            "Nightly version should have base and hash: {}",
            version
        );

        // Commit hash should be 7 characters
        assert_eq!(
            parts[1].len(),
            7,
            "Commit hash should be 7 characters: {}",
            parts[1]
        );
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_latest_stable_release() {
        let result = get_latest_stable_release().await;
        assert!(result.is_ok(), "Failed to get latest release: {:?}", result);

        let release = result.unwrap();
        assert!(!release.tag_name.is_empty(), "Tag name should not be empty");
        assert!(!release.assets.is_empty(), "Assets should not be empty");

        // Check if the expected asset exists
        let asset = release.assets.iter().find(|a| a.name == RELEASE_FILE);
        assert!(asset.is_some());

        // Check if SHA256 digest exists
        let asset = asset.unwrap();
        assert!(asset.digest.as_ref().unwrap().starts_with("sha256:"));
    }

    #[tokio::test]
    #[ignore] // Requires network access and downloads ~250MB
    async fn test_download_and_verify() {
        use std::fs;

        let _ = fs::remove_file(&*UPDATE_FILE_PATH);
        if let Some(parent) = UPDATE_FILE_PATH.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let release = get_latest_stable_release()
            .await
            .expect("Failed to get latest release");

        let version = release.tag_name.trim_start_matches('v');
        println!("Testing download of version: v{}", version);
        println!("Downloading update file to {}", UPDATE_FILE_PATH.display());

        download_update_with_progress(&release, None).await.unwrap();
        assert!(update_file_exists());

        std::fs::remove_file(&*UPDATE_FILE_PATH).expect("Failed to remove test file");
        assert!(!update_file_exists());

        println!("Download and verification successful!");
    }
}
