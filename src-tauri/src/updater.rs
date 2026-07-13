use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tauri::{AppHandle, Manager};

const RELEASE_API: &str = "https://api.github.com/repos/mcxen/shelfy/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub release_name: String,
    pub release_notes: String,
    pub published_at: Option<String>,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

pub fn check_update() -> Result<UpdateInfo, String> {
    let release: GithubRelease = ureq::get(RELEASE_API)
        .header("User-Agent", "Shelfy-Updater")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("GitHub release request failed: {e}"))?
        .body_mut()
        .read_json()
        .map_err(|e| format!("Invalid GitHub release response: {e}"))?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let asset = select_asset(&release.assets);
    Ok(UpdateInfo {
        available: version_is_newer(&latest, &current),
        current_version: current,
        latest_version: latest,
        release_name: release.name.unwrap_or(release.tag_name),
        release_notes: release.body.unwrap_or_default(),
        published_at: release.published_at,
        asset_name: asset.map(|a| a.name.clone()),
        asset_url: asset.map(|a| a.browser_download_url.clone()),
    })
}

pub fn download_and_install(app: &AppHandle, info: &UpdateInfo) -> Result<(), String> {
    if !info.available {
        return Err("No update is available".into());
    }
    let asset_name = info
        .asset_name
        .as_deref()
        .ok_or("No compatible release asset")?;
    let asset_url = info
        .asset_url
        .as_deref()
        .ok_or("No compatible release download URL")?;
    validate_download(asset_name, asset_url)?;
    let update_dir = std::env::temp_dir().join(format!("shelfy-update-{}", std::process::id()));
    std::fs::create_dir_all(&update_dir).map_err(|e| e.to_string())?;
    let package = update_dir.join(safe_file_name(asset_name));
    download(asset_url, &package)?;
    launch_helper(app, &package, &update_dir)?;
    app.exit(0);
    Ok(())
}

pub fn run_helper(args: &[String]) -> Result<bool, String> {
    if args.first().map(String::as_str) != Some("--update-helper") {
        return Ok(false);
    }
    let parent_pid = parse_arg(args, "--parent-pid")?
        .parse::<u32>()
        .map_err(|e| e.to_string())?;
    let package = PathBuf::from(parse_arg(args, "--package")?);
    let target = PathBuf::from(parse_arg(args, "--target")?);
    wait_for_exit(parent_pid);
    install_package(&package, &target)?;
    relaunch(&target)?;
    Ok(true)
}

fn select_asset(assets: &[GithubAsset]) -> Option<&GithubAsset> {
    assets.iter().find(|asset| {
        let name = asset.name.to_lowercase();
        #[cfg(target_os = "macos")]
        let matches =
            (name.ends_with(".dmg") || name.ends_with(".app.tar.gz") || name.ends_with(".zip"))
                && (name.contains("universal")
                    || if cfg!(target_arch = "aarch64") {
                        name.contains("aarch64") || name.contains("arm64")
                    } else {
                        name.contains("x64") || name.contains("x86_64")
                    });
        #[cfg(target_os = "windows")]
        let matches =
            name.ends_with(".exe") && (name.contains("setup") || name.contains("installer"));
        #[cfg(target_os = "linux")]
        let matches = name.ends_with(".appimage");
        matches
    })
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    fn parts(value: &str) -> Vec<u64> {
        value
            .split('.')
            .map(|p| p.split('-').next().unwrap_or("0").parse().unwrap_or(0))
            .collect()
    }
    let mut latest = parts(latest);
    let mut current = parts(current);
    let len = latest.len().max(current.len());
    latest.resize(len, 0);
    current.resize(len, 0);
    latest > current
}

fn validate_download(name: &str, url: &str) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Unsafe release asset name".into());
    }
    let allowed = url.starts_with("https://github.com/mcxen/shelfy/releases/download/")
        || url.starts_with("https://objects.githubusercontent.com/");
    if !allowed {
        return Err("Release asset URL is not trusted".into());
    }
    Ok(())
}

fn safe_file_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric() || ".-_".contains(*c))
        .collect()
}

fn download(url: &str, destination: &Path) -> Result<(), String> {
    let mut response = ureq::get(url)
        .header("User-Agent", "Shelfy-Updater")
        .call()
        .map_err(|e| format!("Update download failed: {e}"))?;
    let mut reader = response.body_mut().as_reader();
    let mut file = std::fs::File::create(destination).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        file.write_all(&buffer[..count])
            .map_err(|e| e.to_string())?;
    }
    file.sync_all().map_err(|e| e.to_string())
}

fn launch_helper(app: &AppHandle, package: &Path, update_dir: &Path) -> Result<(), String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = app.path().executable_dir().map_err(|e| e.to_string())?;
    let helper = update_dir.join(if cfg!(windows) {
        "shelfy-update-helper.exe"
    } else {
        "shelfy-update-helper"
    });
    std::fs::copy(&executable, &helper).map_err(|e| e.to_string())?;
    Command::new(&helper)
        .arg("--update-helper")
        .arg("--parent-pid")
        .arg(std::process::id().to_string())
        .arg("--package")
        .arg(package)
        .arg("--target")
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to start update helper: {e}"))?;
    Ok(())
}

fn parse_arg(args: &[String], name: &str) -> Result<String, String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
        .ok_or_else(|| format!("Missing {name}"))
}

fn wait_for_exit(pid: u32) {
    for _ in 0..120 {
        #[cfg(unix)]
        let running = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        #[cfg(windows)]
        let running = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false);
        if !running {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

#[cfg(target_os = "macos")]
fn install_package(package: &Path, executable_dir: &Path) -> Result<(), String> {
    let app_target = app_bundle_from_executable_dir(executable_dir)?;
    let staging = package
        .parent()
        .unwrap_or(Path::new("/tmp"))
        .join("unpacked");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging).map_err(|e| e.to_string())?;

    let is_dmg = package
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dmg"));
    if is_dmg {
        let mount = staging.join("mount");
        std::fs::create_dir_all(&mount).map_err(|e| e.to_string())?;
        let status = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mount)
            .arg(package)
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("Failed to mount update image".into());
        }
        let result = find_app(&mount)
            .ok_or_else(|| "Update does not contain Shelfy.app".to_string())
            .and_then(|source| replace_app_bundle(&source, &app_target));
        let _ = Command::new("hdiutil").arg("detach").arg(&mount).status();
        return result;
    }

    let status = if package.to_string_lossy().ends_with(".tar.gz") {
        Command::new("tar")
            .args(["-xzf"])
            .arg(package)
            .arg("-C")
            .arg(&staging)
            .status()
    } else {
        Command::new("ditto")
            .args(["-x", "-k"])
            .arg(package)
            .arg(&staging)
            .status()
    }
    .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Failed to unpack update".into());
    }
    let source = find_app(&staging).ok_or("Update does not contain Shelfy.app")?;
    replace_app_bundle(&source, &app_target)
}

#[cfg(target_os = "macos")]
fn replace_app_bundle(source: &Path, app_target: &Path) -> Result<(), String> {
    let backup = app_target.with_extension("app.old");
    let _ = std::fs::remove_dir_all(&backup);
    std::fs::rename(app_target, &backup).map_err(|e| format!("Cannot replace application: {e}"))?;
    let copied = Command::new("ditto").arg(source).arg(app_target).status();
    if !matches!(copied, Ok(status) if status.success()) {
        let _ = std::fs::remove_dir_all(app_target);
        let _ = std::fs::rename(&backup, app_target);
        return Err("Cannot install update; restored previous application".into());
    }
    let _ = std::fs::remove_dir_all(backup);
    Ok(())
}

#[cfg(target_os = "macos")]
fn app_bundle_from_executable_dir(executable_dir: &Path) -> Result<PathBuf, String> {
    let app = executable_dir
        .parent()
        .and_then(Path::parent)
        .ok_or("Invalid app path")?;
    if app.extension().and_then(|extension| extension.to_str()) != Some("app") {
        return Err("Executable is not inside an application bundle".into());
    }
    Ok(app.to_path_buf())
}

#[cfg(target_os = "macos")]
fn find_app(root: &Path) -> Option<PathBuf> {
    std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("app"))
}

#[cfg(target_os = "windows")]
fn install_package(package: &Path, _target: &Path) -> Result<(), String> {
    let status = Command::new(package)
        .args(["/S"])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Installer failed".into())
    }
}

#[cfg(target_os = "linux")]
fn install_package(package: &Path, target: &Path) -> Result<(), String> {
    std::fs::copy(package, target.join("shelfy"))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn relaunch(executable_dir: &Path) -> Result<(), String> {
    let app = app_bundle_from_executable_dir(executable_dir)?;
    Command::new("open")
        .arg(app)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(not(target_os = "macos"))]
fn relaunch(target: &Path) -> Result<(), String> {
    Command::new(target.join(if cfg!(windows) {
        "shelfy.exe"
    } else {
        "shelfy"
    }))
    .spawn()
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_semantic_version_components() {
        assert!(version_is_newer("0.2.2", "0.2.1"));
        assert!(!version_is_newer("0.2.1", "0.2.1"));
        assert!(!version_is_newer("0.2.0", "0.2.1"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn selects_universal_macos_disk_image() {
        let assets = vec![GithubAsset {
            name: "Shelfy_0.2.2_universal.dmg".into(),
            browser_download_url: "https://example.invalid/Shelfy.dmg".into(),
        }];
        assert_eq!(
            select_asset(&assets).map(|asset| asset.name.as_str()),
            Some("Shelfy_0.2.2_universal.dmg")
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn resolves_app_bundle_without_climbing_to_applications() {
        let executable_dir = Path::new("/Applications/Shelfy.app/Contents/MacOS");
        assert_eq!(
            app_bundle_from_executable_dir(executable_dir).unwrap(),
            PathBuf::from("/Applications/Shelfy.app")
        );
        assert!(app_bundle_from_executable_dir(Path::new("/usr/local/bin")).is_err());
    }
}
