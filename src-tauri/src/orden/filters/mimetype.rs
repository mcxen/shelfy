use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Filter by MIME type associated with the file extension.
///
/// Mirrors `organize.filters.mimetype.MimeType`. Uses `mime_guess`. A bare type
/// like "audio" matches anything from "audio/midi" to "audio/quicktime".
pub struct MimeType {
    pub mimetypes: Vec<String>,
}

impl MimeType {
    pub fn new(mimetypes: Vec<String>) -> Self {
        Self { mimetypes }
    }

    fn guess(path: &std::path::Path) -> Option<String> {
        mime_guess::from_path(path).first().map(|m| m.to_string())
    }

    fn matches(&self, mt: Option<&str>) -> bool {
        match mt {
            None => false,
            Some(m) => {
                if self.mimetypes.is_empty() {
                    return true;
                }
                self.mimetypes.iter().any(|x| m.starts_with(x))
            }
        }
    }
}

impl Filter for MimeType {
    fn name(&self) -> &str {
        "mimetype"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("mimetype: no path")?;
        let mt = Self::guess(path);
        set_var(res, "mimetype", Value::Str(mt.clone().unwrap_or_default()));
        Ok(if self.matches(mt.as_deref()) {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}
