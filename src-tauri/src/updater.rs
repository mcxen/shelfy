use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const LATEST_MANIFEST_URL: &str =
    "https://github.com/mcxen/shelfy/releases/latest/download/latest.json";
const RELEASE_TAG_BASE: &str = "https://github.com/mcxen/shelfy/releases/tag";
const STABLE_ASSET_NAME: &str = "Shelfy_universal-apple-darwin.app.zip";
const STABLE_ASSET_URL: &str =
    "https://github.com/mcxen/shelfy/releases/latest/download/Shelfy_universal-apple-darwin.app.zip";
const UPDATE_TARGET: &str = "universal-apple-darwin";
const HELPER_FLAG: &str = "--shelfy-update-helper";
const APP_BUNDLE_NAME: &str = "Shelfy.app";
const APP_BUNDLE_ID: &str = "cc.shelfy.app";
const APP_EXECUTABLE: &str = "shelfy";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub release_url: Option<String>,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub sha256: Option<String>,
    pub size: Option<u64>,
    pub can_install: bool,
    pub install_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateManifest {
    version: String,
    #[serde(default)]
    tag: String,
    #[serde(default)]
    platform: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    asset_name: String,
    #[serde(default)]
    asset_url: String,
    #[serde(default)]
    sha256: String,
    size: Option<u64>,
    #[serde(default)]
    notes: String,
}

pub fn check_update() -> Result<UpdateInfo, String> {
    let manifest: UpdateManifest = ureq::get(LATEST_MANIFEST_URL)
        .header("User-Agent", concat!("Shelfy/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| format!("Fetch update manifest failed: {error}"))?
        .body_mut()
        .read_json()
        .map_err(|error| format!("Invalid update manifest: {error}"))?;
    update_info_from_manifest(env!("CARGO_PKG_VERSION"), manifest)
}

fn update_info_from_manifest(
    current_version: &str,
    manifest: UpdateManifest,
) -> Result<UpdateInfo, String> {
    let latest_version = manifest.version.trim().trim_start_matches('v').to_string();
    if !is_release_version(&latest_version) {
        return Err("Update manifest version must use X.Y.Z numeric format".into());
    }
    let tag = if manifest.tag.trim().is_empty() {
        format!("v{latest_version}")
    } else {
        manifest.tag.trim().to_string()
    };
    if tag != format!("v{latest_version}") {
        return Err("Update manifest tag does not match its version".into());
    }

    let target_matches = manifest.target.trim() == UPDATE_TARGET;
    let platform_matches = manifest.platform.trim() == "macos";
    let (asset_name, asset_url, sha256, size) = if target_matches && platform_matches {
        let asset_name = manifest.asset_name.trim().to_string();
        let asset_url = manifest.asset_url.trim().to_string();
        validate_release_asset_url(&asset_url, &tag, &asset_name)?;
        (
            Some(asset_name),
            Some(asset_url),
            Some(normalize_sha256(&manifest.sha256)),
            manifest.size,
        )
    } else {
        (None, None, None, None)
    };

    let available = version_is_newer(&latest_version, current_version);
    let (can_install, install_reason) =
        install_state(available, asset_url.as_deref(), sha256.as_deref(), size);
    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest_version.clone(),
        available,
        release_name: format!("Shelfy v{latest_version}"),
        release_notes: manifest.notes,
        published_at: None,
        release_url: Some(format!("{RELEASE_TAG_BASE}/{tag}")),
        asset_name,
        asset_url,
        sha256,
        size,
        can_install,
        install_reason,
    })
}

pub fn download_and_install(app: &AppHandle) -> Result<(), String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        return Err("Automatic installation is currently supported on macOS only".into());
    }

    #[cfg(target_os = "macos")]
    {
        let info = check_update()?;
        if !info.available {
            return Err("Shelfy is already up to date".into());
        }
        if !info.can_install {
            return Err(info
                .install_reason
                .unwrap_or_else(|| "This update cannot be installed automatically".into()));
        }
        let asset_url = info
            .asset_url
            .as_deref()
            .ok_or("Update asset URL is missing")?;
        let expected_sha256 = info.sha256.as_deref().ok_or("Update SHA256 is missing")?;
        let cache_root = app
            .path()
            .app_cache_dir()
            .map_err(|error| error.to_string())?
            .join("updates");
        let target_app = current_app_bundle()?;
        ensure_writable_install(&target_app)?;
        let staged_app = download_and_stage(
            &cache_root,
            &info.latest_version,
            asset_url,
            expected_sha256,
            info.size,
        )?;
        validate_staged_app(&staged_app, &info.latest_version)?;
        spawn_update_helper(&cache_root, &staged_app, &target_app, &info.latest_version)?;
        Ok(())
    }
}

pub fn run_helper(args: &[String]) -> Result<bool, String> {
    if args.first().map(String::as_str) != Some(HELPER_FLAG) {
        return Ok(false);
    }

    #[cfg(not(target_os = "macos"))]
    return Err("Automatic installation is currently supported on macOS only".into());

    #[cfg(target_os = "macos")]
    {
        let target_app = PathBuf::from(parse_arg(args, "--target-app")?);
        let result = run_macos_helper(args);
        if result.is_err() {
            let _ = relaunch(&target_app);
        }
        result.map(|_| true)
    }
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    fn parts(value: &str) -> Vec<u64> {
        value
            .trim_start_matches('v')
            .split('.')
            .map(|part| part.split('-').next().unwrap_or("0").parse().unwrap_or(0))
            .collect()
    }
    let mut latest = parts(latest);
    let mut current = parts(current);
    let len = latest.len().max(current.len());
    latest.resize(len, 0);
    current.resize(len, 0);
    latest > current
}

fn is_release_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
}

fn normalize_sha256(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("sha256:")
        .to_ascii_lowercase()
}

fn validate_release_asset_url(url: &str, tag: &str, asset_name: &str) -> Result<(), String> {
    if tag.contains(['/', '\\']) || asset_name != STABLE_ASSET_NAME {
        return Err("Update manifest contains an invalid tag or asset name".into());
    }
    if url != STABLE_ASSET_URL {
        return Err("Update manifest contains an untrusted asset URL".into());
    }
    Ok(())
}

fn install_state(
    available: bool,
    asset_url: Option<&str>,
    sha256: Option<&str>,
    size: Option<u64>,
) -> (bool, Option<String>) {
    if !available {
        return (false, Some("No newer release is available".into()));
    }
    if cfg!(not(target_os = "macos")) {
        return (
            false,
            Some("Automatic installation is currently supported on macOS only".into()),
        );
    }
    if asset_url.is_none() {
        return (
            false,
            Some("No compatible macOS update asset was found".into()),
        );
    }
    if sha256.is_none_or(|value| value.len() != 64 || !value.chars().all(|c| c.is_ascii_hexdigit()))
    {
        return (
            false,
            Some("The update manifest has no valid SHA256 checksum".into()),
        );
    }
    if size.is_none_or(|value| value == 0) {
        return (
            false,
            Some("The update manifest has no valid package size".into()),
        );
    }
    #[cfg(target_os = "macos")]
    if current_app_bundle()
        .and_then(|app| ensure_writable_install(&app))
        .is_err()
    {
        return (
            false,
            Some("Automatic installation requires Shelfy to run from a writable Shelfy.app".into()),
        );
    }
    (true, None)
}

fn parse_arg(args: &[String], name: &str) -> Result<String, String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
        .ok_or_else(|| format!("Missing {name}"))
}

#[cfg(target_os = "macos")]
fn download_and_stage(
    cache_root: &Path,
    version: &str,
    asset_url: &str,
    expected_sha256: &str,
    expected_size: Option<u64>,
) -> Result<PathBuf, String> {
    let update_dir = cache_root.join(version);
    let download_path = update_dir.join(STABLE_ASSET_NAME);
    let staging_root = update_dir.join("staging");
    let _ = fs::remove_dir_all(&update_dir);
    fs::create_dir_all(&update_dir)
        .map_err(|error| format!("Create update cache failed: {error}"))?;

    let mut response = ureq::get(asset_url)
        .header("User-Agent", concat!("Shelfy/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| format!("Update download failed: {error}"))?;
    let mut reader = response.body_mut().as_reader();
    let mut file = fs::File::create(&download_path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
        hasher.update(&buffer[..count]);
        downloaded += count as u64;
    }
    file.sync_all().map_err(|error| error.to_string())?;
    if expected_size.is_some_and(|size| size != downloaded) {
        return Err(format!(
            "Update size mismatch: expected {expected_size:?}, got {downloaded}"
        ));
    }
    let actual_sha256 = hex::encode(hasher.finalize());
    if actual_sha256 != normalize_sha256(expected_sha256) {
        return Err("Update SHA256 checksum mismatch".into());
    }

    fs::create_dir_all(&staging_root).map_err(|error| error.to_string())?;
    let output = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(&download_path)
        .arg(&staging_root)
        .output()
        .map_err(|error| format!("Extract update failed: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Extract update failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let staged_app = staging_root.join(APP_BUNDLE_NAME);
    validate_staged_app(&staged_app, version)?;
    prepare_app_for_launch(&staged_app)?;
    Ok(staged_app)
}

#[cfg(target_os = "macos")]
fn current_app_bundle() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let executable_dir = executable.parent().ok_or("Invalid executable path")?;
    let app = executable_dir
        .parent()
        .and_then(Path::parent)
        .ok_or("Executable is not inside an application bundle")?;
    if app.file_name().and_then(|name| name.to_str()) != Some(APP_BUNDLE_NAME) {
        return Err("Automatic installation requires Shelfy.app".into());
    }
    Ok(app.to_path_buf())
}

#[cfg(target_os = "macos")]
fn ensure_writable_install(app: &Path) -> Result<(), String> {
    let parent = app.parent().ok_or("Shelfy.app has no parent directory")?;
    let status = Command::new("/usr/bin/test")
        .arg("-w")
        .arg(parent)
        .status()
        .map_err(|error| format!("Check Shelfy.app install permissions failed: {error}"))?;
    if !status.success() {
        return Err(
            "Shelfy.app cannot be replaced by the current user; use Homebrew or install the release manually"
                .into(),
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn validate_staged_app(app: &Path, version: &str) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    if app.file_name().and_then(|name| name.to_str()) != Some(APP_BUNDLE_NAME) {
        return Err("Update archive must contain Shelfy.app".into());
    }
    let info_plist = app.join("Contents/Info.plist");
    let executable = app.join("Contents/MacOS").join(APP_EXECUTABLE);
    if !info_plist.is_file() || !executable.is_file() {
        return Err("Update bundle is missing Info.plist or the Shelfy executable".into());
    }
    if fs::metadata(&executable)
        .map_err(|error| error.to_string())?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return Err("Update bundle executable is not executable".into());
    }
    if plist_value(&info_plist, "CFBundleIdentifier")? != APP_BUNDLE_ID {
        return Err("Update bundle has an unexpected identifier".into());
    }
    if plist_value(&info_plist, "CFBundleShortVersionString")? != version {
        return Err("Update bundle version does not match the manifest".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn validate_installed_app(app: &Path) -> Result<(), String> {
    if app.file_name().and_then(|name| name.to_str()) != Some(APP_BUNDLE_NAME) {
        return Err("Update target must be Shelfy.app".into());
    }
    let info_plist = app.join("Contents/Info.plist");
    let executable = app.join("Contents/MacOS").join(APP_EXECUTABLE);
    if !info_plist.is_file() || !executable.is_file() {
        return Err("Installed Shelfy.app is incomplete".into());
    }
    if plist_value(&info_plist, "CFBundleIdentifier")? != APP_BUNDLE_ID {
        return Err("Installed application has an unexpected identifier".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn validate_helper_paths(
    staged_app: &Path,
    target_app: &Path,
    version: &str,
) -> Result<(), String> {
    if !is_release_version(version)
        || staged_app.file_name().and_then(|name| name.to_str()) != Some(APP_BUNDLE_NAME)
        || staged_app
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some("staging")
        || staged_app
            .parent()
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            != Some(version)
        || target_app.file_name().and_then(|name| name.to_str()) != Some(APP_BUNDLE_NAME)
    {
        return Err("Update helper received an invalid application path".into());
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn plist_value(info_plist: &Path, key: &str) -> Result<String, String> {
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(info_plist)
        .output()
        .map_err(|error| format!("Read update Info.plist failed: {error}"))?;
    if !output.status.success() {
        return Err(format!("Update Info.plist is missing {key}"));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_string())
        .map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn spawn_update_helper(
    cache_root: &Path,
    staged_app: &Path,
    target_app: &Path,
    version: &str,
) -> Result<PathBuf, String> {
    fs::create_dir_all(cache_root).map_err(|error| error.to_string())?;
    let helper = cache_root.join(format!("shelfy-update-helper-{}", std::process::id()));
    let _ = fs::remove_file(&helper);
    fs::copy(
        std::env::current_exe().map_err(|error| error.to_string())?,
        &helper,
    )
    .map_err(|error| format!("Copy update helper failed: {error}"))?;
    prepare_detached_helper(&helper)?;

    let mut child = Command::new(&helper)
        .arg(HELPER_FLAG)
        .arg("--parent-pid")
        .arg(std::process::id().to_string())
        .arg("--version")
        .arg(version)
        .arg("--staged-app")
        .arg(staged_app)
        .arg("--target-app")
        .arg(target_app)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Start update helper failed: {error}"))?;
    std::thread::sleep(Duration::from_millis(120));
    if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
        let mut stderr = String::new();
        if let Some(mut stream) = child.stderr.take() {
            let _ = stream.read_to_string(&mut stderr);
        }
        return Err(format!(
            "Update helper exited immediately with {status}: {stderr}"
        ));
    }
    Ok(helper)
}

#[cfg(target_os = "macos")]
fn run_macos_helper(args: &[String]) -> Result<(), String> {
    let parent_pid = parse_arg(args, "--parent-pid")?
        .parse::<u32>()
        .map_err(|error| format!("Invalid parent PID: {error}"))?;
    let version = parse_arg(args, "--version")?;
    let staged_app = fs::canonicalize(parse_arg(args, "--staged-app")?)
        .map_err(|error| format!("Resolve staged application failed: {error}"))?;
    let target_app = fs::canonicalize(parse_arg(args, "--target-app")?)
        .map_err(|error| format!("Resolve installed application failed: {error}"))?;
    validate_helper_paths(&staged_app, &target_app, &version)?;
    validate_staged_app(&staged_app, &version)?;
    validate_installed_app(&target_app)?;
    ensure_writable_install(&target_app)?;
    wait_for_exit(parent_pid, Duration::from_secs(90))?;
    replace_app_bundle(&staged_app, &target_app)?;
    relaunch(&target_app)?;
    if let Some(version_dir) = staged_app.parent().and_then(Path::parent) {
        let _ = fs::remove_dir_all(version_dir);
    }
    if let Ok(helper) = std::env::current_exe() {
        let _ = fs::remove_file(helper);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn wait_for_exit(pid: u32, timeout: Duration) -> Result<(), String> {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        let running = Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !running {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err("Timed out waiting for Shelfy to exit".into())
}

#[cfg(target_os = "macos")]
fn replace_app_bundle(staged_app: &Path, target_app: &Path) -> Result<(), String> {
    let parent = target_app.parent().ok_or("Invalid Shelfy.app path")?;
    let backup = parent.join(".Shelfy.app.update-backup");
    let _ = fs::remove_dir_all(&backup);
    fs::rename(target_app, &backup)
        .map_err(|error| format!("Back up Shelfy.app failed: {error}"))?;
    let copied = Command::new("/usr/bin/ditto")
        .arg(staged_app)
        .arg(target_app)
        .status();
    if !matches!(copied, Ok(status) if status.success()) {
        let _ = fs::remove_dir_all(target_app);
        let _ = fs::rename(&backup, target_app);
        return Err("Install failed; the previous Shelfy.app was restored".into());
    }
    if let Err(error) = prepare_app_for_launch(target_app) {
        let _ = fs::remove_dir_all(target_app);
        let _ = fs::rename(&backup, target_app);
        return Err(format!(
            "Install validation failed; the previous Shelfy.app was restored: {error}"
        ));
    }
    let _ = fs::remove_dir_all(&backup);
    Ok(())
}

#[cfg(target_os = "macos")]
fn prepare_detached_helper(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let _ = Command::new("/usr/bin/xattr")
        .args(["-cr"])
        .arg(path)
        .status();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("Make update helper executable failed: {error}"))?;
    adhoc_codesign(path, false)
}

#[cfg(target_os = "macos")]
fn prepare_app_for_launch(path: &Path) -> Result<(), String> {
    let _ = Command::new("/usr/bin/xattr")
        .args(["-cr"])
        .arg(path)
        .status();
    adhoc_codesign(path, true)
}

#[cfg(target_os = "macos")]
fn adhoc_codesign(path: &Path, deep: bool) -> Result<(), String> {
    let mut command = Command::new("/usr/bin/codesign");
    command.args([
        "--force",
        "--sign",
        "-",
        "--identifier",
        "cc.shelfy.app",
        "--timestamp=none",
    ]);
    if deep {
        command.arg("--deep");
    }
    let output = command
        .arg(path)
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Ad-hoc signing failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(target_os = "macos")]
fn relaunch(app: &Path) -> Result<(), String> {
    Command::new("/usr/bin/open")
        .arg(app)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Restart Shelfy failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> UpdateManifest {
        let tag = format!("v{version}");
        UpdateManifest {
            version: version.into(),
            tag: tag.clone(),
            platform: "macos".into(),
            target: UPDATE_TARGET.into(),
            asset_url: STABLE_ASSET_URL.into(),
            asset_name: STABLE_ASSET_NAME.into(),
            sha256: "ab".repeat(32),
            size: Some(42),
            notes: "Notes".into(),
        }
    }

    #[test]
    fn compares_semantic_version_components() {
        assert!(version_is_newer("0.2.10", "0.2.9"));
        assert!(!version_is_newer("0.2.4", "0.2.4"));
        assert!(!version_is_newer("0.2.3", "0.2.4"));
    }

    #[test]
    fn accepts_matching_manifest_contract() {
        let info = update_info_from_manifest("0.2.4", manifest("0.3.0")).unwrap();
        assert!(info.available);
        assert_eq!(info.asset_name.as_deref(), Some(STABLE_ASSET_NAME));
        assert_eq!(
            info.sha256.as_deref(),
            Some("abababababababababababababababababababababababababababababababab")
        );
    }

    #[test]
    fn rejects_untrusted_manifest_asset_url() {
        let mut value = manifest("0.3.0");
        value.asset_url = "https://example.com/Shelfy.app.zip".into();
        assert!(update_info_from_manifest("0.2.4", value).is_err());
    }

    #[test]
    fn rejects_mismatched_manifest_tag() {
        let mut value = manifest("0.3.0");
        value.tag = "v9.9.9".into();
        assert!(update_info_from_manifest("0.2.4", value).is_err());
    }

    #[test]
    fn rejects_versioned_asset_in_stable_manifest() {
        let mut value = manifest("0.3.0");
        value.asset_name = "Shelfy_v0.3.0_universal-apple-darwin.app.zip".into();
        value.asset_url =
            "https://github.com/mcxen/shelfy/releases/download/v0.3.0/Shelfy_v0.3.0_universal-apple-darwin.app.zip"
                .into();
        assert!(update_info_from_manifest("0.2.4", value).is_err());
    }

    #[test]
    fn rejects_manifest_version_path_traversal() {
        let mut value = manifest("0.3.0");
        value.version = "../../Applications".into();
        value.tag = "v../../Applications".into();
        assert!(update_info_from_manifest("0.2.4", value).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn helper_rejects_paths_outside_versioned_staging_directory() {
        let staged = Path::new("/tmp/updates/0.3.0/not-staging/Shelfy.app");
        let target = Path::new("/Applications/Shelfy.app");
        assert!(validate_helper_paths(staged, target, "0.3.0").is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn staged_bundle_requires_main_executable() {
        let root = std::env::temp_dir().join(format!(
            "shelfy-updater-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("bundle")
        ));
        let app = root.join(APP_BUNDLE_NAME);
        fs::create_dir_all(app.join("Contents")).unwrap();
        fs::write(app.join("Contents/Info.plist"), b"not inspected").unwrap();
        assert!(validate_staged_app(&app, "0.3.0").is_err());
        let _ = fs::remove_dir_all(root);
    }
}
