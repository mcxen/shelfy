use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::orden::action::{DefaultOutput, Level, Output};
use crate::orden::conflict::{resolve_conflict, ConflictMode};
use crate::orden::resource::Resource;
use crate::orden::target_path::prepare_target_path;
use crate::orden::template;
use crate::orden::Action;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Auto,
    Zip,
    SevenZip,
    Rar,
}

impl ArchiveFormat {
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "auto" | "detect" => Ok(Self::Auto),
            "zip" => Ok(Self::Zip),
            "7z" | "sevenzip" | "seven_zip" => Ok(Self::SevenZip),
            "rar" => Ok(Self::Rar),
            other => Err(format!("Unsupported archive format: {}", other)),
        }
    }

    fn detect_for_extract(path: &Path, configured: Self) -> Result<Self, String> {
        if configured != Self::Auto {
            return Ok(configured);
        }
        if seven_zip_volume(path).is_some() {
            return Ok(Self::SevenZip);
        }
        match path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "zip" => Ok(Self::Zip),
            "7z" => Ok(Self::SevenZip),
            "rar" => Ok(Self::Rar),
            ext => Err(format!(
                "extract: cannot auto-detect archive format from .{}",
                ext
            )),
        }
    }

    fn default_for_compress(configured: Self) -> Self {
        match configured {
            Self::Auto => Self::Zip,
            other => other,
        }
    }
}

/// Extract an archive. ZIP is handled in process; 7z and RAR use an installed
/// 7z-compatible command. Archives are tested before extraction.
pub struct ExtractArchive {
    pub dest: String,
    pub format: ArchiveFormat,
    pub passwords: Vec<String>,
    pub delete_original: bool,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
    pub autodetect_folder: bool,
}

impl ExtractArchive {
    pub fn new(
        dest: String,
        format: ArchiveFormat,
        passwords: Vec<String>,
        delete_original: bool,
        on_conflict: ConflictMode,
        rename_template: String,
        autodetect_folder: bool,
    ) -> Self {
        Self {
            dest,
            format,
            passwords,
            delete_original,
            on_conflict,
            rename_template,
            autodetect_folder,
        }
    }
}

/// Create a ZIP archive from a file or folder. Password encryption uses ZIP AES-256.
pub struct CompressArchive {
    pub dest: String,
    pub format: ArchiveFormat,
    pub password: Option<String>,
    pub delete_original: bool,
    pub on_conflict: ConflictMode,
    pub rename_template: String,
    pub autodetect_folder: bool,
}

impl CompressArchive {
    pub fn new(
        dest: String,
        format: ArchiveFormat,
        password: Option<String>,
        delete_original: bool,
        on_conflict: ConflictMode,
        rename_template: String,
        autodetect_folder: bool,
    ) -> Self {
        Self {
            dest,
            format,
            password,
            delete_original,
            on_conflict,
            rename_template,
            autodetect_folder,
        }
    }
}

impl Action for ExtractArchive {
    fn name(&self) -> &str {
        "extract"
    }

    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        self.pipeline_with_output(res, simulate, &DefaultOutput)
    }

    fn pipeline_with_output(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let src = res.path.clone().ok_or("extract: no path")?;
        if !src.is_file() {
            return Err("extract: source must be a file".into());
        }
        let src_name = archive_stem(&src).ok_or("extract: source has no filename")?;
        let dest_rendered = template::render(&self.dest, &res.dict())?;
        let dest =
            prepare_target_path(&src_name, &dest_rendered, self.autodetect_folder, simulate)?;
        let format = ArchiveFormat::detect_for_extract(&src, self.format)?;
        let source_files = archive_source_files(&src, format)?;

        match format {
            ArchiveFormat::Zip => {
                if !simulate {
                    test_zip(&src, &self.passwords)?;
                }
                extract_zip(
                    &src,
                    &dest,
                    &self.passwords,
                    self.on_conflict,
                    &self.rename_template,
                    res,
                    output,
                    simulate,
                )?
            }
            ArchiveFormat::SevenZip | ArchiveFormat::Rar => {
                extract_with_7z(&src, &dest, &self.passwords, res, output, simulate)?
            }
            ArchiveFormat::Auto => unreachable!(),
        }

        output.msg(
            res,
            &format!("Extracted {} to {}", src.display(), dest.display()),
            "extract",
            Level::Info,
        );

        if self.delete_original {
            for source_file in source_files {
                output.msg(
                    res,
                    &format!("Deleting original archive {}", source_file.display()),
                    "extract",
                    Level::Info,
                );
                if !simulate {
                    fs::remove_file(&source_file).map_err(|e| e.to_string())?;
                }
            }
            res.path = Some(dest);
        }
        Ok(())
    }
}

impl Action for CompressArchive {
    fn name(&self) -> &str {
        "compress"
    }

    fn supports_dirs(&self) -> bool {
        true
    }

    fn pipeline(&mut self, res: &mut Resource, simulate: bool) -> Result<(), String> {
        self.pipeline_with_output(res, simulate, &DefaultOutput)
    }

    fn pipeline_with_output(
        &mut self,
        res: &mut Resource,
        simulate: bool,
        output: &dyn Output,
    ) -> Result<(), String> {
        let src = res.path.clone().ok_or("compress: no path")?;
        let src_name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .ok_or("compress: source has no filename")?;
        let format = ArchiveFormat::default_for_compress(self.format);
        let ext = match format {
            ArchiveFormat::Zip => "zip",
            ArchiveFormat::SevenZip => "7z",
            ArchiveFormat::Rar => "7z",
            ArchiveFormat::Auto => unreachable!(),
        };
        let dest_rendered = template::render(&self.dest, &res.dict())?;
        let mut dest = prepare_target_path(
            &format!("{}.{}", src_name, ext),
            &dest_rendered,
            self.autodetect_folder,
            simulate,
        )?;
        if dest.extension().and_then(|s| s.to_str()).is_none() {
            dest.set_extension(ext);
        }
        let conflict = resolve_conflict(
            &dest,
            res,
            self.on_conflict,
            &self.rename_template,
            simulate,
        )?;
        if conflict.skip_action {
            output.msg(
                res,
                &format!("Skipping existing archive {}", dest.display()),
                "compress",
                Level::Warn,
            );
            return Ok(());
        }
        let dest = conflict.use_dst;

        output.msg(
            res,
            &format!(
                "Compressing {} to {}{}",
                src.display(),
                dest.display(),
                if self.password.is_some() {
                    " with password"
                } else {
                    ""
                }
            ),
            "compress",
            Level::Info,
        );

        if !simulate {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            match format {
                ArchiveFormat::Zip => create_zip(&src, &dest, self.password.as_deref())?,
                ArchiveFormat::SevenZip | ArchiveFormat::Rar => {
                    compress_with_7z(&src, &dest, self.password.as_deref(), res, output)?
                }
                ArchiveFormat::Auto => unreachable!(),
            }
            if self.delete_original {
                if src.is_dir() {
                    fs::remove_dir_all(&src).map_err(|e| e.to_string())?;
                } else {
                    fs::remove_file(&src).map_err(|e| e.to_string())?;
                }
                res.path = Some(dest);
            }
        }
        Ok(())
    }
}

fn extract_zip(
    src: &Path,
    dest: &Path,
    passwords: &[String],
    on_conflict: ConflictMode,
    rename_template: &str,
    res: &Resource,
    output: &dyn Output,
    simulate: bool,
) -> Result<(), String> {
    if !simulate {
        fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    }

    let f = fs::File::open(src).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let encrypted = archive
            .by_index_raw(i)
            .map(|file| file.encrypted())
            .map_err(|e| e.to_string())?;
        let password = if encrypted {
            Some(choose_zip_password(src, i, passwords)?)
        } else {
            None
        };
        let mut entry = open_zip_entry(&mut archive, i, password.as_deref())?;
        let Some(enclosed) = entry.enclosed_name() else {
            output.msg(
                res,
                &format!("Skipping unsafe archive entry {}", entry.name()),
                "extract",
                Level::Warn,
            );
            continue;
        };
        let outpath = dest.join(enclosed);
        if entry.is_dir() {
            output.msg(
                res,
                &format!("Create directory {}", outpath.display()),
                "extract",
                Level::Info,
            );
            if !simulate {
                fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
            }
            continue;
        }

        let final_path = if outpath.exists() {
            let mut entry_res = res.clone();
            entry_res.path = Some(src.to_path_buf());
            let conflict =
                resolve_conflict(&outpath, &entry_res, on_conflict, rename_template, simulate)?;
            if conflict.skip_action {
                output.msg(
                    res,
                    &format!("Skipping existing file {}", outpath.display()),
                    "extract",
                    Level::Warn,
                );
                continue;
            }
            conflict.use_dst
        } else {
            outpath
        };

        output.msg(
            res,
            &format!("Extract {}", final_path.display()),
            "extract",
            Level::Info,
        );
        if !simulate {
            if let Some(parent) = final_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = fs::File::create(&final_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn open_zip_entry<'a, R: Read + std::io::Seek>(
    archive: &'a mut zip::ZipArchive<R>,
    index: usize,
    password: Option<&str>,
) -> Result<zip::read::ZipFile<'a>, String> {
    if let Some(password) = password {
        archive
            .by_index_decrypt(index, password.as_bytes())
            .map_err(|e| e.to_string())
    } else {
        archive.by_index(index).map_err(|e| e.to_string())
    }
}

fn test_zip(src: &Path, passwords: &[String]) -> Result<(), String> {
    let f = fs::File::open(src).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    for index in 0..archive.len() {
        let encrypted = archive
            .by_index_raw(index)
            .map(|file| file.encrypted())
            .map_err(|e| e.to_string())?;
        let password = if encrypted {
            Some(choose_zip_password(src, index, passwords)?)
        } else {
            None
        };
        let mut entry = open_zip_entry(&mut archive, index, password.as_deref())?;
        std::io::copy(&mut entry, &mut std::io::sink()).map_err(|e| {
            format!(
                "extract: archive integrity check failed for entry {}: {}",
                entry.name(),
                e
            )
        })?;
    }
    Ok(())
}

fn choose_zip_password(src: &Path, index: usize, passwords: &[String]) -> Result<String, String> {
    if passwords.is_empty() {
        return Err(format!("extract: entry {} requires a password", index));
    }
    for password in passwords {
        let f = fs::File::open(src).map_err(|e| e.to_string())?;
        let mut archive = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
        let Ok(mut entry) = archive.by_index_decrypt(index, password.as_bytes()) else {
            continue;
        };
        let mut probe = [0u8; 1];
        if entry.read(&mut probe).is_ok() {
            return Ok(password.clone());
        }
    }
    Err(format!(
        "extract: no configured password worked for entry {}",
        index
    ))
}

fn find_7z_command() -> Option<&'static str> {
    ["7z", "7zz", "7za"].into_iter().find(|cmd| {
        std::process::Command::new(cmd)
            .arg("--help")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    })
}

fn run_7z_command(
    args: &[String],
    res: &Resource,
    output: &dyn Output,
    sender: &str,
) -> Result<(), String> {
    let cmd = find_7z_command().ok_or_else(|| {
        format!(
            "{}: 7z/7zz/7za command not found. Install 7-Zip to handle 7z/rar archives.",
            sender
        )
    })?;
    output.msg(
        res,
        &format!("Running {} {}", cmd, display_7z_args(args)),
        sender,
        Level::Info,
    );
    let out = std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| e.to_string())?;
    for line in String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
    {
        output.msg(res, line, sender, Level::Info);
    }
    for line in String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter(|l| !l.trim().is_empty())
    {
        output.msg(res, line, sender, Level::Warn);
    }
    if out.status.success() {
        Ok(())
    } else {
        Err(format!("{}: 7z exited with status {}", sender, out.status))
    }
}

fn display_7z_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.starts_with("-p") && arg.len() > 2 {
                "-p********".to_string()
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_with_7z(
    src: &Path,
    dest: &Path,
    passwords: &[String],
    res: &Resource,
    output: &dyn Output,
    simulate: bool,
) -> Result<(), String> {
    output.msg(
        res,
        &format!("Extracting {} to {} via 7z", src.display(), dest.display()),
        "extract",
        Level::Info,
    );
    if simulate {
        return Ok(());
    }
    let candidates = if passwords.is_empty() {
        vec![String::new()]
    } else {
        let mut candidates = passwords.to_vec();
        if !candidates.iter().any(|password| password.is_empty()) {
            candidates.push(String::new());
        }
        candidates
    };

    let mut last_err = None;
    let mut verified_password = None;
    for password in &candidates {
        let mut args = vec!["t".to_string(), "-y".to_string()];
        if !password.is_empty() {
            args.push(format!("-p{}", password));
        }
        args.push(src.to_string_lossy().to_string());
        match run_7z_command(&args, res, output, "archive-test") {
            Ok(()) => {
                verified_password = Some(password.clone());
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let password = verified_password.ok_or_else(|| {
        last_err
            .unwrap_or_else(|| "extract: archive is incomplete or no password worked".to_string())
    })?;

    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut args = vec![
        "x".to_string(),
        "-y".to_string(),
        format!("-o{}", dest.display()),
    ];
    if !password.is_empty() {
        args.push(format!("-p{}", password));
    }
    args.push(src.to_string_lossy().to_string());
    run_7z_command(&args, res, output, "extract")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SevenZipVolume {
    prefix: String,
    part: u64,
    width: usize,
}

fn seven_zip_volume(path: &Path) -> Option<SevenZipVolume> {
    let name = path.file_name()?.to_str()?;
    let (prefix, suffix) = name.rsplit_once('.')?;
    if !prefix.to_ascii_lowercase().ends_with(".7z")
        || suffix.is_empty()
        || !suffix.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    Some(SevenZipVolume {
        prefix: prefix.to_string(),
        part: suffix.parse().ok()?,
        width: suffix.len(),
    })
}

fn archive_stem(path: &Path) -> Option<String> {
    if let Some(volume) = seven_zip_volume(path) {
        return Path::new(&volume.prefix)
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string());
    }
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|name| name.to_string_lossy().to_string())
}

fn archive_source_files(src: &Path, format: ArchiveFormat) -> Result<Vec<PathBuf>, String> {
    if format != ArchiveFormat::SevenZip {
        return Ok(vec![src.to_path_buf()]);
    }
    let Some(volume) = seven_zip_volume(src) else {
        return Ok(vec![src.to_path_buf()]);
    };
    if volume.part != 1 {
        return Err(format!(
            "extract: split 7z archives must start from the first volume, not {}",
            src.display()
        ));
    }

    let parent = src.parent().unwrap_or_else(|| Path::new("."));
    let mut parts = fs::read_dir(parent)
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let candidate = seven_zip_volume(&entry.path())?;
            (candidate.prefix.eq_ignore_ascii_case(&volume.prefix)
                && candidate.width == volume.width)
                .then_some((candidate.part, entry.path()))
        })
        .collect::<Vec<_>>();
    parts.sort_by_key(|(part, _)| *part);

    for (index, (part, _)) in parts.iter().enumerate() {
        let expected = index as u64 + 1;
        if *part != expected {
            return Err(format!(
                "extract: split archive is incomplete; expected volume {:0width$}",
                expected,
                width = volume.width
            ));
        }
    }
    Ok(parts.into_iter().map(|(_, path)| path).collect())
}

fn compress_with_7z(
    src: &Path,
    dest: &Path,
    password: Option<&str>,
    res: &Resource,
    output: &dyn Output,
) -> Result<(), String> {
    let mut args = vec!["a".to_string(), "-y".to_string()];
    if let Some(password) = password.filter(|p| !p.is_empty()) {
        args.push(format!("-p{}", password));
        args.push("-mhe=on".to_string());
    }
    args.push(dest.to_string_lossy().to_string());
    args.push(src.to_string_lossy().to_string());
    run_7z_command(&args, res, output, "compress")
}

fn create_zip(src: &Path, dest: &Path, password: Option<&str>) -> Result<(), String> {
    let f = fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut writer = zip::ZipWriter::new(f);
    let root = src.parent().unwrap_or_else(|| Path::new(""));
    add_path_to_zip(&mut writer, src, root, password)?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(())
}

fn zip_options<'a>(password: Option<&'a str>) -> zip::write::FileOptions<'a, ()> {
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    if let Some(password) = password.filter(|p| !p.is_empty()) {
        options.with_aes_encryption(zip::AesMode::Aes256, password)
    } else {
        options
    }
}

fn add_path_to_zip(
    writer: &mut zip::ZipWriter<fs::File>,
    path: &Path,
    root: &Path,
    password: Option<&str>,
) -> Result<(), String> {
    let name = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    if path.is_dir() {
        if !name.is_empty() {
            writer
                .add_directory(
                    format!("{}/", name.trim_end_matches('/')),
                    zip_options(None),
                )
                .map_err(|e| e.to_string())?;
        }
        let mut entries = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            add_path_to_zip(writer, &entry.path(), root, password)?;
        }
    } else {
        writer
            .start_file(name, zip_options(password))
            .map_err(|e| e.to_string())?;
        let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            writer.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "shelfy-archive-{name}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn detects_two_and_three_digit_7z_volumes() {
        let two = Path::new("bundle.7z.01");
        let three = Path::new("bundle.7Z.001");
        assert_eq!(
            ArchiveFormat::detect_for_extract(two, ArchiveFormat::Auto),
            Ok(ArchiveFormat::SevenZip)
        );
        assert_eq!(
            ArchiveFormat::detect_for_extract(three, ArchiveFormat::Auto),
            Ok(ArchiveFormat::SevenZip)
        );
        assert_eq!(archive_stem(two).as_deref(), Some("bundle"));
        assert_eq!(seven_zip_volume(two).unwrap().part, 1);
        assert_eq!(seven_zip_volume(three).unwrap().width, 3);
    }

    #[test]
    fn split_archive_requires_first_and_contiguous_volumes() {
        let root = temp_dir("volumes");
        fs::create_dir_all(&root).unwrap();
        for name in ["bundle.7z.01", "bundle.7z.02", "bundle.7z.03"] {
            fs::write(root.join(name), b"part").unwrap();
        }

        let parts =
            archive_source_files(&root.join("bundle.7z.01"), ArchiveFormat::SevenZip).unwrap();
        assert_eq!(parts.len(), 3);
        assert!(archive_source_files(&root.join("bundle.7z.02"), ArchiveFormat::SevenZip).is_err());

        fs::remove_file(root.join("bundle.7z.02")).unwrap();
        assert!(archive_source_files(&root.join("bundle.7z.01"), ArchiveFormat::SevenZip).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn hides_7z_passwords_in_logs() {
        assert_eq!(
            display_7z_args(&["t".into(), "-psecret".into(), "file.7z".into()]),
            "t -p******** file.7z"
        );
    }
}
