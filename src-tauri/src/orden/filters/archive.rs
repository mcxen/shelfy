use crate::orden::actions::archive::detect_archive_format;
use crate::orden::filter::{Filter, FilterResult};
use crate::orden::resource::Resource;

/// Match supported archives from their file signature instead of their suffix.
pub struct Archive;

impl Filter for Archive {
    fn name(&self) -> &str {
        "archivefile"
    }

    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("archivefile: no path")?;
        Ok(if detect_archive_format(path)?.is_some() {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
