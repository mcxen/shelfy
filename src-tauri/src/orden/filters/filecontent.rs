use std::fs;
use std::io::Read;
use std::path::Path;

use regex::Regex;

use crate::orden::filter::FilterResult;
use crate::orden::resource::Resource;
use crate::orden::value::Value;
use crate::orden::Filter;

const MAX_TEXT_FILE_BYTES: u64 = 16 * 1024 * 1024;

const TEXT_EXTENSIONS: &[&str] = &[
    // Plain text / docs
    "txt",
    "text",
    "md",
    "mdx",
    "markdown",
    "rst",
    "adoc",
    "asciidoc",
    "log",
    "csv",
    "tsv",
    "jsonl",
    "ndjson",
    // Web / frontend
    "html",
    "htm",
    "xhtml",
    "css",
    "scss",
    "sass",
    "less",
    "js",
    "jsx",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "vue",
    "svelte",
    "astro",
    // Data / config
    "json",
    "json5",
    "yaml",
    "yml",
    "toml",
    "xml",
    "plist",
    "ini",
    "cfg",
    "conf",
    "config",
    "properties",
    "props",
    "env",
    "dotenv",
    "editorconfig",
    "gitignore",
    "gitattributes",
    "npmrc",
    "yarnrc",
    "lock",
    "gradle",
    "sql",
    "graphql",
    "gql",
    "proto",
    // Shell / scripts
    "sh",
    "bash",
    "zsh",
    "fish",
    "ps1",
    "psm1",
    "bat",
    "cmd",
    "awk",
    "sed",
    // General programming languages
    "rs",
    "go",
    "py",
    "pyw",
    "rb",
    "php",
    "java",
    "kt",
    "kts",
    "swift",
    "c",
    "h",
    "cpp",
    "cc",
    "cxx",
    "hpp",
    "hh",
    "hxx",
    "cs",
    "fs",
    "fsx",
    "vb",
    "scala",
    "sc",
    "clj",
    "cljs",
    "cljc",
    "ex",
    "exs",
    "erl",
    "hrl",
    "lua",
    "r",
    "jl",
    "nim",
    "zig",
    "dart",
    "m",
    "mm",
    "pl",
    "pm",
    "t",
    "hs",
    "lhs",
    "elm",
    "ml",
    "mli",
    "fsproj",
    "csproj",
    "vbproj",
    "sln",
    "cmake",
    // Templates / infra
    "tmpl",
    "tpl",
    "jinja",
    "j2",
    "hbs",
    "mustache",
    "liquid",
    "ejs",
    "erb",
    "tf",
    "tfvars",
    "hcl",
    "nomad",
    "dockerfile",
    "containerfile",
    "makefile",
    "mk",
    "bzl",
    "bazel",
    "BUILD",
    "workspace",
    // Misc text formats
    "tex",
    "bib",
    "org",
    "wiki",
    "patch",
    "diff",
    "pem",
    "crt",
    "cer",
    "key",
    "pub",
    "asc",
    "svg",
];

const TEXT_FILENAMES: &[&str] = &[
    "dockerfile",
    "containerfile",
    "makefile",
    "gnumakefile",
    "cmakelists.txt",
    "justfile",
    "rakefile",
    "gemfile",
    "podfile",
    "procfile",
    "vagrantfile",
    "brewfile",
    "license",
    "licence",
    "notice",
    "copying",
    "readme",
    "changelog",
    "authors",
    "contributors",
    "todo",
    ".env",
    ".env.local",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".npmrc",
    ".yarnrc",
    ".dockerignore",
    ".prettierrc",
    ".eslintrc",
    ".babelrc",
];

/// Matches file content with the given regular expression.
///
/// Mirrors `organize.filters.filecontent.FileContent`, with broader text support:
/// common code/config/document text extensions are read directly, extensionless
/// text files such as Dockerfile/Makefile are recognized, and unknown small files
/// are accepted when their bytes look like text. PDF still uses system `pdftotext`,
/// and DOCX is parsed from `word/document.xml`.
pub struct FileContent {
    pub expr: Regex,
}

impl FileContent {
    pub fn new(expr: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            expr: Regex::new(&format!("(?ms){}", expr))?,
        })
    }
}

fn extension_lower(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn filename_lower(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

fn is_text_mime(path: &Path) -> bool {
    mime_guess::from_path(path).iter().any(|mime| {
        let essence = mime.essence_str();
        essence.starts_with("text/")
            || matches!(
                essence,
                "application/json"
                    | "application/javascript"
                    | "application/ecmascript"
                    | "application/xml"
                    | "application/x-yaml"
                    | "application/toml"
                    | "application/sql"
                    | "application/graphql"
                    | "application/x-sh"
                    | "application/x-shellscript"
                    | "image/svg+xml"
            )
    })
}

fn is_known_text_path(path: &Path) -> bool {
    let ext = extension_lower(path);
    let filename = filename_lower(path);
    TEXT_EXTENSIONS.contains(&ext.as_str())
        || TEXT_FILENAMES.contains(&filename.as_str())
        || is_text_mime(path)
}

fn looks_like_text(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return true;
    }
    if bytes.contains(&0) {
        return false;
    }

    let suspicious_controls = bytes
        .iter()
        .filter(|b| matches!(**b, 0x01..=0x08 | 0x0b | 0x0e..=0x1f | 0x7f))
        .count();
    suspicious_controls <= (bytes.len() / 100).max(8)
}

fn extract_text_like(path: &Path) -> Result<String, String> {
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_TEXT_FILE_BYTES {
        return Err(format!(
            "filecontent: text file is too large ({} bytes, max {})",
            meta.len(),
            MAX_TEXT_FILE_BYTES
        ));
    }

    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if !is_known_text_path(path) && !looks_like_text(&bytes) {
        return Err("filecontent: file does not look like text".to_string());
    }
    if !looks_like_text(&bytes) {
        return Err("filecontent: text file contains binary control bytes".to_string());
    }
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn extract_pdf(path: &Path) -> Result<String, String> {
    let out = std::process::Command::new("pdftotext")
        .arg(path)
        .arg("-")
        .output()
        .map_err(|e| format!("pdftotext failed: {}", e))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn extract_docx(path: &Path) -> Result<String, String> {
    let f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut buf = String::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        if entry.name() == "word/document.xml" {
            entry.read_to_string(&mut buf).map_err(|e| e.to_string())?;
            break;
        }
    }
    if buf.is_empty() {
        return Ok(String::new());
    }
    // strip xml tags, keep text
    let mut text = String::new();
    let mut in_tag = false;
    for c in buf.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    Ok(text)
}

fn extract(path: &Path) -> Result<String, String> {
    match extension_lower(path).as_str() {
        "pdf" => extract_pdf(path),
        "docx" => extract_docx(path),
        _ => extract_text_like(path),
    }
}

impl Filter for FileContent {
    fn name(&self) -> &str {
        "filecontent"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("filecontent: no path")?;
        let content = match extract(path) {
            Ok(c) => c,
            Err(_) => return Ok(FilterResult::NoMatch),
        };
        if let Some(m) = self.expr.captures(&content) {
            let mut map = std::collections::BTreeMap::new();
            for name in self.expr.capture_names().flatten() {
                if let Some(val) = m.name(name) {
                    map.insert(name.to_string(), Value::Str(val.as_str().to_string()));
                }
            }
            res.vars.deep_merge_into("filecontent", Value::Map(map));
            Ok(FilterResult::Match)
        } else {
            Ok(FilterResult::NoMatch)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shelfy-filecontent-{}-{}", nonce, name))
    }

    fn resource_for(path: &Path) -> Resource {
        Resource::new(
            path.to_path_buf(),
            path.parent().unwrap().to_path_buf(),
            1,
            Some("test".to_string()),
        )
    }

    #[test]
    fn matches_code_files_and_named_groups() {
        let path = temp_path("main.rs");
        fs::write(&path, "fn main() { let invoice = \"INV-42\"; }\n").unwrap();

        let mut filter = FileContent::new(r#"invoice\s*=\s*\"(?P<number>INV-\d+)\""#).unwrap();
        let mut res = resource_for(&path);
        let matched = matches!(filter.pipeline(&mut res).unwrap(), FilterResult::Match);
        let _ = fs::remove_file(&path);

        assert!(matched);
        match res.vars.get("filecontent").unwrap() {
            Value::Map(map) => assert_eq!(map.get("number").unwrap().render(), "INV-42"),
            _ => panic!("filecontent vars should be a map"),
        }
    }

    #[test]
    fn matches_extensionless_text_files() {
        let path = temp_path("Dockerfile");
        fs::write(&path, "FROM rust:latest\nRUN cargo --version\n").unwrap();

        let mut filter = FileContent::new("cargo --version").unwrap();
        let mut res = resource_for(&path);
        let matched = matches!(filter.pipeline(&mut res).unwrap(), FilterResult::Match);
        let _ = fs::remove_file(&path);

        assert!(matched);
    }

    #[test]
    fn binary_files_do_not_match() {
        let path = temp_path("blob.bin");
        fs::write(&path, [0, 159, 146, 150, 0, 1, 2, 3]).unwrap();

        let mut filter = FileContent::new(".").unwrap();
        let mut res = resource_for(&path);
        let matched = matches!(filter.pipeline(&mut res).unwrap(), FilterResult::Match);
        let _ = fs::remove_file(&path);

        assert!(!matched);
    }
}
