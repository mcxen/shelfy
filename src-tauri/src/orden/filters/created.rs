use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use crate::orden::filter::FilterResult;
use crate::orden::filters::timefilter::{TimeFilter, TimeMode};
use crate::orden::resource::Resource;
use crate::orden::Filter;

/// Matches files / folders by created date.
///
/// Mirrors `organize.filters.created.Created`. On Windows `std::fs::Metadata::created()`
/// returns the creation time; on Unix it is birthtime when available. Falls back to
/// `stat` then to `mtime` (with a warning) when birthtime is unavailable.
pub struct Created(TimeFilter);

impl Created {
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
            "created", years, months, weeks, days, hours, minutes, seconds, mode,
        ))
    }
}

fn read_created(path: &Path) -> Result<DateTime<Utc>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if let Ok(t) = meta.created() {
        return systemtime_to_utc(t);
    }
    // fall back to mtime
    let t = meta.modified().map_err(|e| e.to_string())?;
    systemtime_to_utc(t)
}

fn systemtime_to_utc(t: SystemTime) -> Result<DateTime<Utc>, String> {
    let dt: DateTime<Utc> = DateTime::from(t);
    Ok(dt)
}

impl Filter for Created {
    fn name(&self) -> &str {
        "created"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        self.0.run(res, read_created)
    }
}
