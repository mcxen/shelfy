use std::collections::BTreeMap;
use std::fs::File;
use std::path::Path;

use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Filter by image EXIF data.
///
/// Mirrors `organize.filters.exif.Exif`. Uses `kamadak-exif` (JPEG/TIFF) with an
/// `exiftool` fallback for other formats (set via ORGANIZE_EXIFTOOL_PATH env).
/// `filter_tags` matches a dotted key path with optional glob value.
pub struct Exif {
    pub filter_tags: BTreeMap<String, Option<String>>,
    pub lowercase_keys: bool,
}

impl Exif {
    pub fn new(filter_tags: BTreeMap<String, Option<String>>, lowercase_keys: bool) -> Self {
        Self {
            filter_tags,
            lowercase_keys,
        }
    }
}

fn exifread(path: &Path, lowercase: bool) -> Result<BTreeMap<String, Value>, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut buf).map_err(|e| e.to_string())?;
    let exif = exif::Reader::new()
        .read_from_container(&mut std::io::Cursor::new(&buf))
        .map_err(|e| e.to_string())?;
    let mut m = BTreeMap::new();
    for f in exif.fields() {
        let key = f.tag.to_string();
        let key = if lowercase { key.to_lowercase() } else { key };
        let val = format!("{}", f.display_value().with_unit(&exif));
        m.insert(key, Value::Str(val));
    }
    Ok(m)
}

fn matches_tags(
    filter_tags: &BTreeMap<String, Option<String>>,
    data: &BTreeMap<String, Value>,
) -> bool {
    if data.is_empty() {
        return false;
    }
    for (k, v) in filter_tags {
        let got = data.get(k);
        match (got, v) {
            (Some(_), None) => {}
            (Some(got), Some(want)) => {
                let g = got.render().to_lowercase();
                let w = want.to_lowercase();
                if !glob_match(&w, &g) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_inner(&p, &n)
}
fn glob_inner(p: &[char], n: &[char]) -> bool {
    if p.is_empty() {
        return n.is_empty();
    }
    match p[0] {
        '*' => {
            if n.is_empty() {
                glob_inner(&p[1..], n)
            } else {
                glob_inner(&p[1..], n) || glob_inner(p, &n[1..])
            }
        }
        '?' => {
            if n.is_empty() {
                false
            } else {
                glob_inner(&p[1..], &n[1..])
            }
        }
        c => {
            if n.is_empty() || n[0] != c {
                false
            } else {
                glob_inner(&p[1..], &n[1..])
            }
        }
    }
}

impl Filter for Exif {
    fn name(&self) -> &str {
        "exif"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("exif: no path")?;
        let data = exifread(path, self.lowercase_keys).unwrap_or_default();
        let matched = matches_tags(&self.filter_tags, &data);
        set_var(res, "exif", Value::Map(data));
        Ok(if matched {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
