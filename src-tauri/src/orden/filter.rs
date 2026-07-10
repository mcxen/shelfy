use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Outcome of running a filter: matched, with optionally updated variables.
pub enum FilterResult {
    Match,
    NoMatch,
}

/// A filter decides whether a resource passes through the pipeline.
pub trait Filter: Send + Sync {
    /// The name used to store extracted variables (e.g. "extension", "size").
    fn name(&self) -> &str;

    /// Whether this filter supports files.
    fn supports_files(&self) -> bool {
        true
    }

    /// Whether this filter supports directories.
    fn supports_dirs(&self) -> bool {
        false
    }

    /// Run the filter against a resource. Returns match result; may mutate
    /// `res.vars` to expose data to later filters / actions.
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String>;
}

/// Inverts a filter (the `not ` prefix in organize config).
pub struct Not(pub Box<dyn Filter>);

impl Filter for Not {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn supports_files(&self) -> bool {
        self.0.supports_files()
    }
    fn supports_dirs(&self) -> bool {
        self.0.supports_dirs()
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        match self.0.pipeline(res)? {
            FilterResult::Match => Ok(FilterResult::NoMatch),
            FilterResult::NoMatch => Ok(FilterResult::Match),
        }
    }
}

/// All filters must match (filter_mode: all).
pub struct All<'a> {
    pub filters: Vec<&'a mut dyn Filter>,
}

impl<'a> Filter for All<'a> {
    fn name(&self) -> &str {
        "all"
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        for f in &mut self.filters {
            match f.pipeline(res)? {
                FilterResult::Match => {}
                FilterResult::NoMatch => return Ok(FilterResult::NoMatch),
            }
        }
        Ok(FilterResult::Match)
    }
}

/// Any filter matches (filter_mode: any).
pub struct Any<'a> {
    pub filters: Vec<&'a mut dyn Filter>,
}

impl<'a> Filter for Any<'a> {
    fn name(&self) -> &str {
        "any"
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        for f in &mut self.filters {
            if let FilterResult::Match = f.pipeline(res)? {
                return Ok(FilterResult::Match);
            }
        }
        Ok(FilterResult::NoMatch)
    }
}

/// Run a filter pipeline according to a filter_mode ("all" / "any" / "none").
pub fn run_pipeline(
    filters: &mut [Box<dyn Filter>],
    mode: FilterMode,
    res: &mut Resource,
) -> Result<bool, String> {
    match mode {
        FilterMode::All => {
            for f in filters {
                if let FilterResult::NoMatch = f.pipeline(res)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        FilterMode::Any => {
            for f in filters {
                if let FilterResult::Match = f.pipeline(res)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        FilterMode::None => {
            for f in filters {
                if let FilterResult::Match = f.pipeline(res)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    All,
    Any,
    None,
}

/// Helper to set a string variable on the resource.
pub fn set_var(res: &mut Resource, key: &str, value: Value) {
    if let Value::Map(ref mut m) = res.vars {
        m.insert(key.to_string(), value);
    }
}
