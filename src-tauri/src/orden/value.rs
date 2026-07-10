use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use std::collections::BTreeMap;

/// A dynamically-typed value used by filters to expose data to actions via templates.
///
/// Mirrors organize-tool's `resource.vars` dict, which can hold strings, numbers,
/// datetimes, dates and nested dictionaries.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    DateTime(DateTime<Utc>),
    Date(NaiveDate),
    List(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn str(s: impl Into<String>) -> Self {
        Value::Str(s.into())
    }

    pub fn map() -> Self {
        Value::Map(BTreeMap::new())
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Render this value as a string for template substitution.
    pub fn render(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Str(s) => s.clone(),
            Value::DateTime(dt) => dt.format("%Y-%m-%d %H:%M:%S").to_string(),
            Value::Date(d) => d.format("%Y-%m-%d").to_string(),
            Value::List(items) => items
                .iter()
                .map(|v| v.render())
                .collect::<Vec<_>>()
                .join(", "),
            Value::Map(m) => serde_json::to_string(m).unwrap_or_default(),
        }
    }

    /// Look up a dotted path like `size.bytes` or `regex.groupname`.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Map(m) => m.get(key),
            _ => None,
        }
    }

    /// Merge another value's map into this one (deep merge) at the given key.
    pub fn deep_merge_into(&mut self, key: &str, data: Value) {
        match self {
            Value::Map(m) => {
                let existing = m.remove(key).unwrap_or(Value::Map(BTreeMap::new()));
                let merged = deep_merge(existing, data);
                m.insert(key.to_string(), merged);
            }
            _ => {}
        }
    }

    /// Set a key in a map value.
    pub fn set(&mut self, key: &str, value: Value) {
        if let Value::Map(m) = self {
            m.insert(key.to_string(), value);
        }
    }
}

/// Deep-merge two map values. Non-map values are overwritten by `b`.
pub fn deep_merge(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Map(mut ma), Value::Map(mb)) => {
            for (k, v) in mb {
                if let Some(existing) = ma.remove(&k) {
                    ma.insert(k, deep_merge(existing, v));
                } else {
                    ma.insert(k, v);
                }
            }
            Value::Map(ma)
        }
        (_, b) => b,
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}
