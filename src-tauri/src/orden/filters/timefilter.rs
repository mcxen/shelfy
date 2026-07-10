use std::path::Path;

use chrono::{DateTime, Duration, Utc};

use crate::orden::filter::{set_var, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Shared time-based filter logic: compare a file's datetime against
/// `now - (years..seconds)` with mode older/newer.
///
/// Mirrors `organize.filters.common.timefilter.TimeFilter`.
pub struct TimeFilter {
    pub years: i64,
    pub months: i64,
    pub weeks: i64,
    pub days: i64,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub mode: TimeMode,
    pub name: &'static str,
    comparison_dt: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeMode {
    Older,
    Newer,
}

impl TimeFilter {
    pub fn new(
        name: &'static str,
        years: i64,
        months: i64,
        weeks: i64,
        days: i64,
        hours: i64,
        minutes: i64,
        seconds: i64,
        mode: TimeMode,
    ) -> Self {
        let has_comparison = years + months + weeks + days + hours + minutes + seconds != 0;
        let now = Utc::now();
        let comparison_dt = if has_comparison {
            // approximate month/year as 30/365 days (organize uses arrow.shift which
            // handles calendar arithmetic, but this approximation is close enough
            // for filtering; for exact behaviour we use Duration arithmetic)
            Some(
                now - Duration::days(days)
                    - Duration::weeks(weeks)
                    - Duration::hours(hours)
                    - Duration::minutes(minutes)
                    - Duration::seconds(seconds)
                    - Duration::days(months * 30)
                    - Duration::days(years * 365),
            )
        } else {
            None
        };
        Self {
            years,
            months,
            weeks,
            days,
            hours,
            minutes,
            seconds,
            mode,
            name,
            comparison_dt,
        }
    }

    fn matches(&self, dt: DateTime<Utc>) -> bool {
        match self.comparison_dt {
            None => true,
            Some(c) => match self.mode {
                TimeMode::Older => dt < c,
                TimeMode::Newer => dt > c,
            },
        }
    }

    pub fn run<F>(&self, res: &mut Resource, get_dt: F) -> Result<FilterResult, String>
    where
        F: FnOnce(&Path) -> Result<DateTime<Utc>, String>,
    {
        let path = res.path.as_ref().ok_or("timefilter: no path")?;
        let dt = match get_dt(path) {
            Ok(dt) => dt,
            Err(_) => return Ok(FilterResult::NoMatch),
        };
        set_var(res, self.name, Value::DateTime(dt));
        Ok(if self.matches(dt) {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
