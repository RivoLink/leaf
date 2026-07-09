use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const REPOS: &[&str] = &["RivoLink/leaf", "leaf-mg/leaf"];
const CHECKSUMS_ASSET_NAME: &str = "checksums.txt";
const HTTP_TIMEOUT: Duration = Duration::from_secs(20);

#[cfg(windows)]
const INSTALLER_NAME: &str = "install.ps1";

#[cfg(not(windows))]
const INSTALLER_NAME: &str = "install.sh";

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub(crate) fn run_update() -> Result<()> {
    println!("Updating leaf...");

    let current_version = env!("CARGO_PKG_VERSION");
    let asset_name = current_asset_name()?;
    let release = fetch_latest_release(asset_name)?;
    let latest_version = normalize_version_tag(&release.tag_name);

    if !is_newer_version(current_version, latest_version)? {
        println!("leaf {current_version} is already up to date");
        return Ok(());
    }

    let download_url = expected_asset_download_url(&release.tag_name, &release.assets, asset_name)?;
    let checksums_url =
        expected_asset_download_url(&release.tag_name, &release.assets, CHECKSUMS_ASSET_NAME)?;
    let checksums = download_text_asset(checksums_url)?;
    let expected_checksum = find_expected_checksum(&checksums, asset_name)?;
    let current_exe = match std::env::var("LEAF_CURRENT_EXE") {
        Ok(path) => PathBuf::from(path),
        Err(_) => std::env::current_exe().context("Cannot locate current executable")?,
    };
    let temp_path = temp_download_path(&current_exe);

    download_asset(download_url, &temp_path)?;
    verify_download_checksum(&temp_path, expected_checksum)?;

    match replace_binary(&current_exe, &temp_path) {
        Ok(()) => {
            println!("leaf updated from {current_version} to {latest_version}");
            Ok(())
        }
        Err(err) => {
            cleanup_file_if_exists(&temp_path);
            Err(err)
        }
    }
}

fn current_asset_name() -> Result<&'static str> {
    asset_name_for_target(std::env::consts::OS, std::env::consts::ARCH).ok_or_else(|| {
        anyhow::anyhow!(
            "Unsupported platform: {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        )
    })
}

pub(crate) fn asset_name_for_target(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        ("macos", "x86_64") => Some("leaf-macos-x86_64"),
        ("macos", "aarch64") => Some("leaf-macos-arm64"),
        ("linux", "x86_64") => Some("leaf-linux-x86_64"),
        ("linux", "aarch64") => Some("leaf-linux-arm64"),
        ("android", "aarch64") => Some("leaf-android-arm64"),
        ("windows", "x86_64") => Some("leaf-windows-x86_64.exe"),
        _ => None,
    }
}

fn fetch_latest_release(asset_name: &str) -> Result<GithubRelease> {
    let mut last_err: Option<anyhow::Error> = None;

    for repo in REPOS {
        match fetch_release_from_api(repo) {
            Ok(release) => return Ok(release),
            Err(err) => last_err = Some(err),
        }
    }
    for repo in REPOS {
        match fetch_release_from_redirect(repo, asset_name) {
            Ok(release) => return Ok(release),
            Err(err) => last_err = Some(err),
        }
    }

    let last = last_err.unwrap_or_else(|| anyhow::anyhow!("Unable to fetch latest leaf release"));
    Err(last.context(format!(
        "All GitHub sources failed. Try reinstalling via {INSTALLER_NAME}."
    )))
}

fn fetch_release_from_api(repo: &str) -> Result<GithubRelease> {
    let client = http_client()?;

    let response = client
        .get(format!(
            "https://api.github.com/repos/{repo}/releases/latest"
        ))
        .header(reqwest::header::USER_AGENT, "leaf-updater")
        .send()
        .context("Cannot reach GitHub releases API")?;

    let status = response.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!("GitHub API request was forbidden or rate-limited");
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("Latest GitHub release was not found");
    }
    if !status.is_success() {
        bail!("GitHub releases API returned HTTP {status}");
    }

    response
        .json::<GithubRelease>()
        .context("Cannot parse GitHub release metadata")
}

fn fetch_release_from_redirect(repo: &str, asset_name: &str) -> Result<GithubRelease> {
    let client = http_client_no_redirect()?;

    let response = client
        .head(format!("https://github.com/{repo}/releases/latest"))
        .header(reqwest::header::USER_AGENT, "leaf-updater")
        .send()
        .context("Cannot reach GitHub releases page")?;

    let status = response.status();
    if !status.is_redirection() {
        bail!("GitHub releases page returned unexpected HTTP {status}");
    }

    let location = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| anyhow::anyhow!("GitHub releases redirect missing Location header"))?;

    let tag_name = extract_tag_from_release_url(location)?.to_string();
    let assets = build_synthetic_assets(repo, &tag_name, asset_name);

    Ok(GithubRelease { tag_name, assets })
}

pub(crate) fn extract_tag_from_release_url(url: &str) -> Result<&str> {
    let marker = "/releases/tag/";
    let idx = url
        .rfind(marker)
        .ok_or_else(|| anyhow::anyhow!("Release URL missing /releases/tag/ segment: {url}"))?;
    let tail = &url[idx + marker.len()..];
    let tag = tail.split(['/', '?', '#']).next().unwrap_or("");
    if tag.is_empty() {
        bail!("Release URL contains an empty tag: {url}");
    }
    Ok(tag)
}

pub(crate) fn build_download_url(repo: &str, tag: &str, asset_name: &str) -> String {
    format!("https://github.com/{repo}/releases/download/{tag}/{asset_name}")
}

fn build_synthetic_assets(repo: &str, tag: &str, asset_name: &str) -> Vec<GithubAsset> {
    [asset_name, CHECKSUMS_ASSET_NAME]
        .into_iter()
        .map(|name| GithubAsset {
            name: name.to_string(),
            browser_download_url: build_download_url(repo, tag, name),
        })
        .collect()
}

pub(crate) fn expected_asset_download_url<'a>(
    tag_name: &str,
    assets: &'a [impl AsRefAsset],
    expected_asset: &str,
) -> Result<&'a str> {
    let _ = normalize_version_tag(tag_name);
    assets
        .iter()
        .find(|asset| asset.name() == expected_asset)
        .map(|asset| asset.download_url())
        .ok_or_else(|| anyhow::anyhow!("Release does not contain asset {expected_asset}"))
}

pub(crate) trait AsRefAsset {
    fn name(&self) -> &str;
    fn download_url(&self) -> &str;
}

impl AsRefAsset for GithubAsset {
    fn name(&self) -> &str {
        &self.name
    }

    fn download_url(&self) -> &str {
        &self.browser_download_url
    }
}

pub(crate) fn is_newer_version(current: &str, remote: &str) -> Result<bool> {
    let current = Version::parse(normalize_version_tag(current))
        .with_context(|| format!("Invalid current version: {current}"))?;
    let remote = Version::parse(normalize_version_tag(remote))
        .with_context(|| format!("Invalid remote version: {remote}"))?;
    Ok(remote > current)
}

fn normalize_version_tag(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

fn download_asset(url: &str, destination: &Path) -> Result<()> {
    cleanup_file_if_exists(destination);
    let _cleanup = TempFileGuard::new(destination.to_path_buf());
    let client = http_client()?;
    let mut response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "leaf-updater")
        .send()
        .with_context(|| format!("Cannot download release asset: {url}"))?;

    let expected_len = validate_download_response(response.status(), response.content_length())?;
    let mut file = fs::File::create(destination)
        .with_context(|| format!("Cannot create temporary file: {}", destination.display()))?;
    let copied = response
        .copy_to(&mut file)
        .with_context(|| format!("Cannot write downloaded asset: {}", destination.display()))?;
    validate_download_size(expected_len, copied)?;
    file.sync_all()
        .with_context(|| format!("Cannot flush temporary file: {}", destination.display()))?;
    _cleanup.disarm();
    Ok(())
}

fn download_text_asset(url: &str) -> Result<String> {
    let client = http_client()?;
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "leaf-updater")
        .send()
        .with_context(|| format!("Cannot download release metadata asset: {url}"))?;

    validate_download_response(response.status(), response.content_length())?;
    response
        .text()
        .with_context(|| format!("Cannot read release metadata asset: {url}"))
}

fn http_client() -> Result<Client> {
    let client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .context("Cannot initialize HTTP client")?;
    Ok(client)
}

fn http_client_no_redirect() -> Result<Client> {
    let client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .context("Cannot initialize HTTP client")?;
    Ok(client)
}

fn validate_download_response(
    status: reqwest::StatusCode,
    content_length: Option<u64>,
) -> Result<Option<u64>> {
    if status == reqwest::StatusCode::FORBIDDEN {
        bail!("Release asset download was forbidden or rate-limited");
    }
    if status == reqwest::StatusCode::NOT_FOUND {
        bail!("Release asset was not found");
    }
    if !status.is_success() {
        bail!("Release asset download returned HTTP {status}");
    }
    if matches!(content_length, Some(0)) {
        bail!("Release asset download returned an empty body");
    }
    Ok(content_length)
}

pub(crate) fn validate_download_size(expected: Option<u64>, actual: u64) -> Result<()> {
    if actual == 0 {
        bail!("Downloaded release asset is empty");
    }
    if let Some(expected) = expected {
        if expected != actual {
            bail!(
                "Downloaded release asset size mismatch: expected {expected} bytes, got {actual}"
            );
        }
    }
    Ok(())
}

pub(crate) fn find_expected_checksum<'a>(checksums: &'a str, asset_name: &str) -> Result<&'a str> {
    for line in checksums.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(checksum) = parts.next() else {
            continue;
        };
        let Some(filename) = parts.next() else {
            continue;
        };
        let normalized_filename = filename.trim_start_matches('*');
        if normalized_filename == asset_name {
            validate_sha256_hex(checksum)?;
            return Ok(checksum);
        }
    }

    bail!("checksums.txt does not contain {asset_name}")
}

pub(crate) fn validate_sha256_hex(value: &str) -> Result<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("Invalid SHA256 checksum format");
    }
    Ok(())
}

fn verify_download_checksum(path: &Path, expected_checksum: &str) -> Result<()> {
    let bytes = fs::read(path).with_context(|| {
        format!(
            "Cannot read downloaded asset for checksum: {}",
            path.display()
        )
    })?;
    let actual_checksum = format!("{:x}", Sha256::digest(&bytes));

    if actual_checksum != expected_checksum {
        bail!(
            "Downloaded release asset checksum mismatch: expected {expected_checksum}, got {actual_checksum}"
        );
    }
    Ok(())
}

fn temp_download_path(current_exe: &Path) -> PathBuf {
    let extension = current_exe
        .extension()
        .map(|ext| format!("{}.download", ext.to_string_lossy()))
        .unwrap_or_else(|| "download".to_string());
    current_exe.with_extension(extension)
}

#[cfg(unix)]
fn replace_binary(current_exe: &Path, downloaded_path: &Path) -> Result<()> {
    let permissions = fs::metadata(current_exe)
        .with_context(|| {
            format!(
                "Cannot read current binary metadata: {}",
                current_exe.display()
            )
        })?
        .permissions();
    fs::set_permissions(downloaded_path, permissions).with_context(|| {
        format!(
            "Cannot apply executable permissions to {}",
            downloaded_path.display()
        )
    })?;
    fs::rename(downloaded_path, current_exe)
        .with_context(|| format!("Cannot replace current binary at {}", current_exe.display()))?;
    Ok(())
}

#[cfg(windows)]
fn replace_binary(current_exe: &Path, downloaded_path: &Path) -> Result<()> {
    let backup_path = current_exe.with_extension("old");
    cleanup_file_if_exists(&backup_path);

    fs::rename(current_exe, &backup_path).with_context(|| {
        format!(
            "Cannot replace the running Windows binary at {}. Try the PowerShell installer instead.",
            current_exe.display()
        )
    })?;

    if let Err(err) = fs::rename(downloaded_path, current_exe) {
        let _ = fs::rename(&backup_path, current_exe);
        bail!(
            "Cannot install the updated Windows binary at {}: {err}. Try the PowerShell installer instead.",
            current_exe.display()
        );
    }

    cleanup_file_if_exists(&backup_path);
    Ok(())
}

fn cleanup_file_if_exists(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

struct TempFileGuard {
    path: PathBuf,
    armed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.armed {
            cleanup_file_if_exists(&self.path);
        }
    }
}

#[cfg(test)]
pub(crate) use test_support::TestAsset;

#[cfg(test)]
mod test_support {
    use super::AsRefAsset;

    pub(crate) struct TestAsset<'a> {
        pub(crate) name: &'a str,
        pub(crate) download_url: &'a str,
    }

    impl AsRefAsset for TestAsset<'_> {
        fn name(&self) -> &str {
            self.name
        }

        fn download_url(&self) -> &str {
            self.download_url
        }
    }
}
