use regex::Regex;

use crate::orden::filter::FilterResult;
use crate::orden::resource::Resource;
use crate::orden::value::Value;
use crate::orden::Filter;

/// Matches filenames with a regular expression.
///
/// Mirrors `organize.filters.regex.Regex`. Named groups become `{regex.groupname}`.
pub struct RegexFilter {
    pub expr: Regex,
}

impl RegexFilter {
    pub fn new(expr: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            expr: Regex::new(expr)?,
        })
    }
}

impl Filter for RegexFilter {
    fn name(&self) -> &str {
        "regex"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("regex: no path")?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(m) = self.expr.captures(&name) {
            // collect named groups into a map
            let mut map = std::collections::BTreeMap::new();
            for name in self.expr.capture_names().flatten() {
                if let Some(val) = m.name(name) {
                    map.insert(name.to_string(), Value::Str(val.as_str().to_string()));
                }
            }
            res.vars.deep_merge_into("regex", Value::Map(map));
            Ok(FilterResult::Match)
        } else {
            Ok(FilterResult::NoMatch)
        }
    }
}
