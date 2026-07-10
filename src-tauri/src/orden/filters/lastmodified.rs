use std::path::Path;

use chrono::{DateTime, Utc};

use crate::orden::filter::FilterResult;
use crate::orden::filters::timefilter::{TimeFilter, TimeMode};
use crate::orden::resource::Resource;
use crate::orden::Filter;

/// Matches files by last modified date.
///
/// Mirrors `organize.filters.lastmodified.LastModified`.
pub struct LastModified(TimeFilter);

impl LastModified {
    pub fn new(
        years: i64,
        months: i64,
        weeks: i64,
        days: i64,
        hours: i64,
        minutes: i64,
        seconds: i64,
        mode: TimeMode,
    ) -> Self {
        Self(TimeFilter::new(
            "lastmodified",
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
            mode,
        ))
    }
}

fn read_lastmodified(path: &Path) -> Result<DateTime<Utc>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let t = meta.modified().map_err(|e| e.to_string())?;
    Ok(DateTime::<Utc>::from(t))
}

impl Filter for LastModified {
    fn name(&self) -> &str {
        "lastmodified"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        self.0.run(res, read_lastmodified)
    }
}
