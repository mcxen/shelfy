use regex::Regex;

use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Matches files and folders by size.
///
/// Mirrors `organize.filters.size.Size`. Accepts size conditions like
/// `">= 500 MB"`, `"< 20k"`, `">20k, < 1 TB"`. Returns `{size.bytes}` etc.
pub struct Size {
    pub conditions: Vec<String>,
    constraints: Vec<Constraint>,
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
}

#[derive(Debug, Clone, Copy)]
struct Constraint {
    op: Op,
    bytes: u64,
}

impl Size {
    pub fn new(conditions: Vec<String>) -> Result<Self, String> {
        let mut constraints = Vec::new();
        for c in &conditions {
            for constraint in create_constraints(c)? {
                constraints.push(constraint);
            }
        }
        Ok(Self {
            conditions,
            constraints,
        })
    }
}

use once_cell::sync::Lazy;

static SIZE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<op>[<>=]*)(?P<num>(\d*\.)?\d+)(?P<unit>[kmgtpezy]?i?)b?$").unwrap()
});

fn create_constraints(inp: &str) -> Result<Vec<Constraint>, String> {
    let lower = inp.replace(' ', "").to_lowercase();
    let parts: Vec<&str> = lower.split(',').collect();
    let mut out = Vec::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let m = SIZE_RE
            .captures(part)
            .ok_or(format!("Invalid size format: {}", part))?;
        let op = match &m["op"] {
            "<" => Op::Lt,
            "<=" => Op::Le,
            "==" | "=" | "" => Op::Eq,
            ">=" => Op::Ge,
            ">" => Op::Gt,
            _ => return Err(format!("Invalid operator in: {}", part)),
        };
        let num_str = &m["num"];
        let num: f64 = num_str
            .parse()
            .map_err(|_| format!("Invalid number in: {}", part))?;
        let unit = &m["unit"];
        let base = if unit.ends_with('i') { 1024 } else { 1000 };
        let exp = if !unit.is_empty() {
            let prefix = unit.trim_end_matches('i');
            "kmgtpezy".find(prefix).map(|i| i + 1).unwrap_or(0) as u32
        } else {
            0
        };
        let bytes = (num * (base as f64).powi(exp as i32)) as u64;
        out.push(Constraint { op, bytes });
    }
    Ok(out)
}

fn read_file_size(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn read_dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += read_file_size(&p);
            } else if p.is_dir() {
                total += read_dir_size(&p);
            }
        }
    }
    total
}

fn satisfies(size: u64, constraints: &[Constraint]) -> bool {
    constraints.iter().all(|c| {
        let ok = match c.op {
            Op::Lt => size < c.bytes,
            Op::Le => size <= c.bytes,
            Op::Eq => size == c.bytes,
            Op::Ge => size >= c.bytes,
            Op::Gt => size > c.bytes,
        };
        ok
    })
}

fn number_with_unit(size: u64, suffixes: &[&str], base: u64) -> String {
    if size < base {
        return format!("{} bytes", size);
    }
    let mut value = size as f64;
    let mut unit = "bytes";
    let mut idx = 0;
    while idx < suffixes.len() {
        if value < base as f64 {
            unit = suffixes[idx];
            break;
        }
        value /= base as f64;
        idx += 1;
    }
    if idx >= suffixes.len() {
        unit = suffixes[suffixes.len() - 1];
    }
    format!("{:.1} {}", value, unit)
}

fn traditional(size: u64) -> String {
    number_with_unit(
        size,
        &["KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"],
        1024,
    )
}

fn binary(size: u64) -> String {
    number_with_unit(
        size,
        &["KiB", "MiB", "GiB", "TiB", "PiB", "EiB", "ZiB", "YiB"],
        1024,
    )
}

fn decimal(size: u64) -> String {
    number_with_unit(
        size,
        &["kB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"],
        1000,
    )
}

impl Filter for Size {
    fn name(&self) -> &str {
        "size"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("size: no path")?;
        let bytes = if res.is_file() {
            read_file_size(path)
        } else if res.is_dir() {
            read_dir_size(path)
        } else {
            return Err("size: unknown file type".into());
        };
        let mut size_map = std::collections::BTreeMap::new();
        size_map.insert("bytes".to_string(), Value::Int(bytes as i64));
        size_map.insert("traditional".to_string(), Value::Str(traditional(bytes)));
        size_map.insert("binary".to_string(), Value::Str(binary(bytes)));
        size_map.insert("decimal".to_string(), Value::Str(decimal(bytes)));
        set_var(res, "size", Value::Map(size_map));
        if self.constraints.is_empty() {
            return Ok(FilterResult::Match);
        }
        Ok(if satisfies(bytes, &self.constraints) {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
