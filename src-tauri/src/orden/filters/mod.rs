use std::collections::BTreeMap;

use crate::orden::filter::FilterMode;

pub mod created;
pub mod duplicate;
pub mod empty;
pub mod exif;
pub mod extension;
pub mod filecontent;
pub mod hash;
pub mod lastmodified;
pub mod mimetype;
pub mod name;
pub mod regex;
pub mod size;
pub mod timefilter;

pub use created::Created;
pub use duplicate::{DetectMethod, Duplicate};
pub use empty::Empty;
pub use exif::Exif;
pub use extension::Extension;
pub use filecontent::FileContent;
pub use hash::Hash;
pub use lastmodified::LastModified;
pub use mimetype::MimeType;
pub use name::Name;
pub use regex::RegexFilter;
pub use size::Size;

use crate::orden::Filter;

/// A single filter definition as parsed from YAML (before instantiation).
#[derive(Debug, Clone)]
pub struct FilterDef {
    pub name: String,
    pub inverted: bool,
    pub value: serde_yaml::Value,
}

/// Parse a YAML filter entry into a FilterDef.
///
/// Supports the organize shorthand forms:
///   `- extension`          → { extension: null }
///   `- extension: pdf`     → { extension: "pdf" }
///   `- extension: [pdf, docx]`
///   `- not created: { days: 30 }
pub fn filter_def_from_yaml(v: &serde_yaml::Value) -> Result<FilterDef, String> {
    // string shorthand: "- extension"
    if let serde_yaml::Value::String(s) = v {
        return parse_key_value(s, serde_yaml::Value::Null);
    }
    // dict form: one key
    let mapping = v
        .as_mapping()
        .ok_or("Filter must be a single-key mapping")?;
    if mapping.len() != 1 {
        return Err("Filter definition must have only one key".into());
    }
    let (k, val) = mapping.iter().next().unwrap();
    let key = k.as_str().ok_or("Filter key must be a string")?;
    parse_key_value(key, val.clone())
}

fn parse_key_value(raw_key: &str, value: serde_yaml::Value) -> Result<FilterDef, String> {
    let (name, inverted) = if let Some(rest) = raw_key.strip_prefix("not ") {
        (rest.to_string(), true)
    } else {
        (raw_key.to_string(), false)
    };
    Ok(FilterDef {
        name,
        inverted,
        value,
    })
}

/// Instantiate a filter from its definition.
pub fn build_filter(def: &FilterDef) -> Result<Box<dyn Filter>, String> {
    let inner: Box<dyn Filter> = match def.name.as_str() {
        "extension" => {
            let exts = as_string_list(&def.value, &def.name)?;
            Box::new(Extension::new(exts))
        }
        "name" => {
            let (match_pattern, startswith, contains, endswith, case_sensitive) =
                parse_name(&def.value)?;
            Box::new(Name::new(
                match_pattern,
                startswith,
                contains,
                endswith,
                case_sensitive,
            ))
        }
        "regex" => {
            let expr = as_string(&def.value, "regex")?;
            Box::new(RegexFilter::new(&expr).map_err(|e| e.to_string())?)
        }
        "size" => {
            let conditions = as_string_list(&def.value, "size")?;
            Box::new(Size::new(conditions)?)
        }
        "empty" => Box::new(Empty),
        "mimetype" => {
            let mts = as_string_list(&def.value, "mimetype")?;
            Box::new(MimeType::new(mts))
        }
        "hash" => {
            let algo = as_string_optional(&def.value, "md5");
            Box::new(Hash::new(algo))
        }
        "duplicate" => {
            let (method, algo) = parse_duplicate(&def.value)?;
            Box::new(Duplicate::new(method, algo))
        }
        "created" => {
            let t = parse_time(&def.value, "created")?;
            Box::new(Created::from_time(t))
        }
        "lastmodified" => {
            let t = parse_time(&def.value, "lastmodified")?;
            Box::new(LastModified::from_time(t))
        }
        "filecontent" => {
            let expr = as_string_optional(&def.value, r"(?P<all>.*)");
            Box::new(FileContent::new(&expr).map_err(|e| e.to_string())?)
        }
        "exif" => {
            let (tags, lowercase) = parse_exif(&def.value)?;
            Box::new(Exif::new(tags, lowercase))
        }
        other => return Err(format!("Unknown filter: \"{}\"", other)),
    };
    if def.inverted {
        Ok(Box::new(crate::orden::filter::Not(inner)))
    } else {
        Ok(inner)
    }
}

// ---------- helpers to coerce YAML values ----------

fn as_string(v: &serde_yaml::Value, name: &str) -> Result<String, String> {
    match v {
        serde_yaml::Value::String(s) => Ok(s.clone()),
        serde_yaml::Value::Bool(b) => Ok(b.to_string()),
        serde_yaml::Value::Number(n) => Ok(n.to_string()),
        serde_yaml::Value::Null => Err(format!("Filter \"{}\" requires a value", name)),
        _ => Err(format!("Filter \"{}\" expects a string", name)),
    }
}

fn as_string_optional(v: &serde_yaml::Value, default: &str) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        _ => default.to_string(),
    }
}

fn as_string_list(v: &serde_yaml::Value, _name: &str) -> Result<Vec<String>, String> {
    match v {
        serde_yaml::Value::String(s) => Ok(s.split_whitespace().map(|x| x.to_string()).collect()),
        serde_yaml::Value::Null | serde_yaml::Value::Bool(_) | serde_yaml::Value::Number(_) => {
            Ok(vec![as_string_optional(v, "")])
        }
        serde_yaml::Value::Sequence(seq) => {
            let mut out = Vec::new();
            for item in seq {
                match item {
                    serde_yaml::Value::String(s) => out.push(s.clone()),
                    serde_yaml::Value::Number(n) => out.push(n.to_string()),
                    serde_yaml::Value::Bool(b) => out.push(b.to_string()),
                    _ => {}
                }
            }
            Ok(out)
        }
        _ => Ok(Vec::new()),
    }
}

/// Parse `name` filter config: {match, startswith, contains, endswith, case_sensitive}.
fn parse_name(
    v: &serde_yaml::Value,
) -> Result<(Option<String>, Vec<String>, Vec<String>, Vec<String>, bool), String> {
    let default = (
        Some("*".to_string()),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        true,
    );
    let m = match v {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::String(s) => {
            let mut d = default;
            d.0 = Some(s.clone());
            return Ok(d);
        }
        serde_yaml::Value::Null => return Ok(default),
        _ => return Err("name filter expects a mapping or string".into()),
    };
    let match_pattern = m
        .get(&serde_yaml::Value::String("match".into()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or(default.0);
    let startswith = str_or_list(m, "startswith");
    let contains = str_or_list(m, "contains");
    let endswith = str_or_list(m, "endswith");
    let case_sensitive = m
        .get(&serde_yaml::Value::String("case_sensitive".into()))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    Ok((
        match_pattern,
        startswith,
        contains,
        endswith,
        case_sensitive,
    ))
}

fn str_or_list(m: &serde_yaml::Mapping, key: &str) -> Vec<String> {
    match m.get(&serde_yaml::Value::String(key.into())) {
        Some(serde_yaml::Value::String(s)) => vec![s.clone()],
        Some(serde_yaml::Value::Sequence(seq)) => seq
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Parse `duplicate` filter: {detect_original_by: ..., hash_algorithm: ...}.
fn parse_duplicate(v: &serde_yaml::Value) -> Result<(DetectMethod, String), String> {
    let m = match v {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::Null => {
            return Ok((DetectMethod::FirstSeen, "sha1".to_string()));
        }
        _ => return Err("duplicate filter expects a mapping".into()),
    };
    let method_str = m
        .get(&serde_yaml::Value::String("detect_original_by".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("first_seen");
    let (method, _reverse) = DetectMethod::parse(method_str)?;
    let algo = m
        .get(&serde_yaml::Value::String("hash_algorithm".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("sha1")
        .to_string();
    Ok((method, algo))
}

/// Shared time-filter config.
pub struct TimeConfig {
    years: i64,
    months: i64,
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
    mode: timefilter::TimeMode,
}

fn parse_time(v: &serde_yaml::Value, _name: &str) -> Result<TimeConfig, String> {
    let m = match v {
        serde_yaml::Value::Mapping(m) => m,
        serde_yaml::Value::Null => {
            return Ok(TimeConfig {
                years: 0,
                months: 0,
                weeks: 0,
                days: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
                mode: timefilter::TimeMode::Older,
            });
        }
        _ => return Err("time filter expects a mapping".into()),
    };
    let num = |key: &str| -> i64 {
        m.get(&serde_yaml::Value::String(key.into()))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    };
    let mode = m
        .get(&serde_yaml::Value::String("mode".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("older");
    let mode = match mode {
        "older" => timefilter::TimeMode::Older,
        "newer" => timefilter::TimeMode::Newer,
        other => return Err(format!("Unknown time mode: {}", other)),
    };
    Ok(TimeConfig {
        years: num("years"),
        months: num("months"),
        weeks: num("weeks"),
        days: num("days"),
        hours: num("hours"),
        minutes: num("minutes"),
        seconds: num("seconds"),
        mode,
    })
}

// adapters to construct Created/LastModified from shared TimeConfig
impl Created {
    pub fn from_time(t: TimeConfig) -> Self {
        Created::new(
            t.years, t.months, t.weeks, t.days, t.hours, t.minutes, t.seconds, t.mode,
        )
    }
}
impl LastModified {
    pub fn from_time(t: TimeConfig) -> Self {
        LastModified::new(
            t.years, t.months, t.weeks, t.days, t.hours, t.minutes, t.seconds, t.mode,
        )
    }
}

/// Parse exif filter config. Tags are passed as kwargs or filter_tags dict.
fn parse_exif(v: &serde_yaml::Value) -> Result<(BTreeMap<String, Option<String>>, bool), String> {
    let mut tags = BTreeMap::new();
    let lowercase = true;
    match v {
        serde_yaml::Value::Mapping(m) => {
            for (k, v) in m {
                let key = k.as_str().ok_or("exif keys must be strings")?.to_string();
                let val = v.as_str().map(|s| s.to_string());
                tags.insert(key, val);
            }
        }
        serde_yaml::Value::Null => {}
        _ => return Err("exif filter expects a mapping".into()),
    }
    Ok((tags, lowercase))
}

/// Parse filter_mode from YAML string.
pub fn parse_filter_mode(s: &str) -> Result<FilterMode, String> {
    match s {
        "all" => Ok(FilterMode::All),
        "any" => Ok(FilterMode::Any),
        "none" => Ok(FilterMode::None),
        other => Err(format!("Unknown filter_mode: {}", other)),
    }
}
