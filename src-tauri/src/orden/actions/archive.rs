use std::fs;
use std::io::{Read, Write};
use std::path::Path;

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

/// Extract an archive. Currently supports ZIP, including encrypted ZIP entries
/// through a configured password list.
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
        let src_name = src
            .file_stem()
            .or_else(|| src.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .ok_or("extract: source has no filename")?;
        let dest_rendered = template::render(&self.dest, &res.dict())?;
        let dest =
            prepare_target_path(&src_name, &dest_rendered, self.autodetect_folder, simulate)?;
        let format = ArchiveFormat::detect_for_extract(&src, self.format)?;

        match format {
            ArchiveFormat::Zip => extract_zip(
                &src,
                &dest,
                &self.passwords,
                self.on_conflict,
                &self.rename_template,
                res,
                output,
                simulate,
            )?,
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
            output.msg(
                res,
                &format!("Deleting original archive {}", src.display()),
                "extract",
                Level::Info,
            );
            if !simulate {
                fs::remove_file(&src).map_err(|e| e.to_string())?;
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
        &format!("Running {} {}", cmd, args.join(" ")),
        sender,
        Level::Info,
    );
    let out = std::process::Command::new(cmd)
        .args(args)
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
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut candidates = if passwords.is_empty() {
        vec![String::new()]
    } else {
        passwords.to_vec()
    };
    if !candidates.iter().any(|p| p.is_empty()) {
        candidates.push(String::new());
    }

    let mut last_err = None;
    for password in candidates {
        let mut args = vec![
            "x".to_string(),
            "-y".to_string(),
            format!("-o{}", dest.display()),
        ];
        if !password.is_empty() {
            args.push(format!("-p{}", password));
        }
        args.push(src.to_string_lossy().to_string());
        match run_7z_command(&args, res, output, "extract") {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    Err(last_err.unwrap_or_else(|| "extract: no 7z password candidate worked".to_string()))
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
