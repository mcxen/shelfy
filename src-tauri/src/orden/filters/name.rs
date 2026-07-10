use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Match files and folders by name (startswith / contains / endswith / simplematch).
///
/// Mirrors `organize.filters.name.Name`. Uses globset-style wildcard matching for the
/// `match` parameter (equivalent to organize's `simplematch`).
pub struct Name {
    pub match_pattern: Option<String>,
    pub startswith: Vec<String>,
    pub contains: Vec<String>,
    pub endswith: Vec<String>,
    pub case_sensitive: bool,
}

impl Name {
    pub fn new(
        match_pattern: Option<String>,
        startswith: Vec<String>,
        contains: Vec<String>,
        endswith: Vec<String>,
        case_sensitive: bool,
    ) -> Self {
        Self {
            match_pattern,
            startswith,
            contains,
            endswith,
            case_sensitive,
        }
    }

    fn normalize(s: &str, case_sensitive: bool) -> String {
        if case_sensitive {
            s.to_string()
        } else {
            s.to_lowercase()
        }
    }

    fn matches(&self, name: &str) -> bool {
        let name = Self::normalize(name, self.case_sensitive);
        let match_ok = match &self.match_pattern {
            Some(pat) => {
                let pat = Self::normalize(pat, self.case_sensitive);
                glob_match(&pat, &name)
            }
            None => true,
        };
        let contains_ok = self.contains.is_empty()
            || self
                .contains
                .iter()
                .any(|x| name.contains(&Self::normalize(x, self.case_sensitive)));
        let startswith_ok = self.startswith.is_empty()
            || self
                .startswith
                .iter()
                .any(|x| name.starts_with(&Self::normalize(x, self.case_sensitive)));
        let endswith_ok = self.endswith.is_empty()
            || self
                .endswith
                .iter()
                .any(|x| name.ends_with(&Self::normalize(x, self.case_sensitive)));
        match_ok && contains_ok && startswith_ok && endswith_ok
    }
}

/// Simple glob matcher supporting `*` and `?`.
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    glob_match_inner(&p, &n)
}

fn glob_match_inner(p: &[char], n: &[char]) -> bool {
    if p.is_empty() {
        return n.is_empty();
    }
    match p[0] {
        '*' => {
            if n.is_empty() {
                return glob_match_inner(&p[1..], n);
            }
            glob_match_inner(&p[1..], n) || glob_match_inner(p, &n[1..])
        }
        '?' => {
            if n.is_empty() {
                false
            } else {
                glob_match_inner(&p[1..], &n[1..])
            }
        }
        c => {
            if n.is_empty() || n[0] != c {
                false
            } else {
                glob_match_inner(&p[1..], &n[1..])
            }
        }
    }
}

impl Filter for Name {
    fn name(&self) -> &str {
        "name"
    }
    fn supports_dirs(&self) -> bool {
        true
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("name: no path")?;
        let stem = if res.is_dir() {
            path.file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            if stem.is_empty() {
                path.extension()
                    .map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                stem
            }
        };
        let matched = self.matches(&stem);
        set_var(res, "name", Value::Str(stem));
        Ok(if matched {
            FilterResult::Match
        } else {
            FilterResult::NoMatch
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob() {
        assert!(glob_match("*.pdf", "invoice.pdf"));
        assert!(!glob_match("*.pdf", "invoice.txt"));
        assert!(glob_match("invoice*", "invoice_2025.pdf"));
        assert!(glob_match("*faktura*", "my_faktura_2025.pdf"));
    }
}
