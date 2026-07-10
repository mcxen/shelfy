use crate::orden::filter::{Filter, FilterResult};
use crate::orden::resource::Resource;

/// Finds empty dirs and files.
///
/// Mirrors `organize.filters.empty.Empty`.
pub struct Empty;

impl Filter for Empty {
    fn name(&self) -> &str {
        "empty"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        Ok(if res.is_empty() {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
