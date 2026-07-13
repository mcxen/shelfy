use std::collections::BTreeMap;

use chrono::{Datelike, Local, Timelike, Utc};

use crate::orden::value::Value;

/// Render a jinja2-like template string (`{variable}`, `{variable.method()}`) against
/// the given variable context.
///
/// Mirrors `organize.template.render` which uses a jinja2 environment with `{` / `}`
/// delimiters, `StrictUndefined`, `expanduser` and `expandvars`.
pub fn render(template: &str, args: &Value) -> Result<String, String> {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut chars = template.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '{' {
            // find the matching close brace, respecting quotes inside the expression
            if let Some((expr, end)) = read_expression(bytes, i) {
                let evaluated = eval_expression(expr.trim(), args)?;
                out.push_str(&evaluated);
                // advance the char iterator past the closing brace (byte index `end`)
                while let Some(&(pos, _)) = chars.peek() {
                    if pos <= end {
                        chars.next();
                    } else {
                        break;
                    }
                }
                continue;
            }
        }
        out.push(c);
    }

    // expand ~ and environment variables (like organize's render)
    let expanded = expand(&out);
    Ok(expanded)
}

/// Read a `{...}` expression starting at `start` (pointing at `{`).
/// Returns the inner expression text and the index of the closing `}`.
fn read_expression(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    // start points at '{'
    let mut i = start + 1;
    let mut depth = 1u32;
    let mut quote: Option<u8> = None;
    while i < bytes.len() {
        let c = bytes[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match c {
            b'\'' | b'"' => quote = Some(c),
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let inner = std::str::from_utf8(&bytes[start + 1..i])
                        .unwrap_or("")
                        .to_string();
                    return Some((inner, i));
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// Evaluate a single expression like `name`, `size.bytes`, `created.strftime('%Y-%m')`.
fn eval_expression(expr: &str, args: &Value) -> Result<String, String> {
    if expr.is_empty() {
        return Ok(String::new());
    }

    // split off an optional method-call suffix: identifier.chain.method(args)
    // We look for the first '(' that opens a call at the top level.
    let (target_part, call_part) = split_call(expr);
    let value = resolve_chain(&target_part, args)?;

    if let Some((method, raw_args)) = call_part {
        apply_method(&value, &method, &raw_args, args)
    } else {
        Ok(value.render())
    }
}

/// Split "a.b.method(arg)" into ("a.b", Some(("method", "arg"))).
fn split_call(expr: &str) -> (String, Option<(String, String)>) {
    // find the first '(' that is not inside quotes, scanning from the end of the
    // identifier chain.
    let mut quote: Option<u8> = None;
    for (idx, c) in expr.char_indices() {
        if let Some(q) = quote {
            if c as u8 == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c as u8),
            '(' => {
                let target = expr[..idx].to_string();
                // strip trailing dots from target
                let target = target.trim_end_matches('.').to_string();
                // extract method name (last segment after '.')
                let method = target.rsplit('.').next().unwrap_or("").to_string();
                let target_base = target[..target.len().saturating_sub(method.len())]
                    .trim_end_matches('.')
                    .to_string();
                // find matching close paren
                let after = &expr[idx + 1..];
                let close = find_close_paren(after);
                let raw_args = close.map(|p| after[..p].to_string()).unwrap_or_default();
                return (target_base, Some((method, raw_args)));
            }
            _ => {}
        }
    }
    (expr.to_string(), None)
}

fn find_close_paren(s: &str) -> Option<usize> {
    let mut depth = 1i32;
    let mut quote: Option<u8> = None;
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if c as u8 == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' => quote = Some(c as u8),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve a dotted identifier chain against the context.
fn resolve_chain(chain: &str, args: &Value) -> Result<Value, String> {
    let chain = chain.trim();
    if chain.is_empty() {
        return Ok(Value::Null);
    }

    let segments: Vec<&str> = chain.split('.').collect();
    let first = segments[0];

    // built-in functions
    match first {
        "now" => {
            return Ok(Value::DateTime(Utc::now()));
        }
        "utcnow" => {
            return Ok(Value::DateTime(Utc::now()));
        }
        "today" => {
            return Ok(Value::Date(Local::now().date_naive()));
        }
        "env" => {
            // env.VAR_NAME
            if segments.len() > 1 {
                let var = std::env::var(segments[1]).unwrap_or_default();
                return Ok(Value::Str(var));
            }
            return Ok(Value::Null);
        }
        _ => {}
    }

    let mut current = args.get(first).cloned();
    if current.is_none() {
        // try nested: the chain might be a single filter name whose value is a map
        // and a sub-key is requested.
        if let Some(v) = args.get(first) {
            current = Some(v.clone());
        } else {
            return Err(format!("Missing value for template: {{{}}}", chain));
        }
    }

    let mut value = current.unwrap();
    for seg in segments.iter().skip(1) {
        value = match value.get(seg).cloned() {
            Some(v) => v,
            None => return Err(format!("Missing value for template: {{{}}}", chain)),
        };
    }
    Ok(value)
}

/// Apply a method call to a value.
fn apply_method(
    value: &Value,
    method: &str,
    raw_args: &str,
    _args: &Value,
) -> Result<String, String> {
    let parsed_args = parse_args(raw_args);
    match method {
        "upper" => Ok(Value::Str(value.render().to_uppercase()).render()),
        "lower" => Ok(Value::Str(value.render().to_lowercase()).render()),
        "strftime" => {
            let fmt = parsed_args.first().cloned().unwrap_or_default();
            match value {
                Value::DateTime(dt) => Ok(dt.format(&fmt).to_string()),
                Value::Date(d) => Ok(d.format(&fmt).to_string()),
                _ => Err(format!(
                    "strftime called on non-datetime value: {}",
                    value.render()
                )),
            }
        }
        "replace" => {
            let s = value.render();
            let from = parsed_args.first().cloned().unwrap_or_default();
            let to = parsed_args.get(1).cloned().unwrap_or_default();
            Ok(s.replace(&from, &to))
        }
        "format" => {
            // simple: replace {} placeholders
            let s = value.render();
            let mut result = s;
            for a in parsed_args {
                result = result.replacen("{}", &a, 1);
            }
            Ok(result)
        }
        "year" => match value {
            Value::DateTime(dt) => Ok(dt.year().to_string()),
            Value::Date(d) => Ok(d.year().to_string()),
            _ => Err("year on non-date".into()),
        },
        "month" => match value {
            Value::DateTime(dt) => Ok(format!("{:02}", dt.month())),
            Value::Date(d) => Ok(format!("{:02}", d.month())),
            _ => Err("month on non-date".into()),
        },
        "day" => match value {
            Value::DateTime(dt) => Ok(format!("{:02}", dt.day())),
            Value::Date(d) => Ok(format!("{:02}", d.day())),
            _ => Err("day on non-date".into()),
        },
        "hour" => match value {
            Value::DateTime(dt) => Ok(format!("{:02}", dt.hour())),
            _ => Err("hour on non-datetime".into()),
        },
        "minute" => match value {
            Value::DateTime(dt) => Ok(format!("{:02}", dt.minute())),
            _ => Err("minute on non-datetime".into()),
        },
        _ => Err(format!("Unknown method in template: {}", method)),
    }
}

/// Parse comma-separated string literal args: `'a', "b"` -> ["a", "b"]
fn parse_args(raw: &str) -> Vec<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    let mut chars = raw.chars().peekable();
    while let Some(&c) = chars.peek() {
        if c == ' ' || c == ',' {
            chars.next();
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            chars.next();
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == quote {
                    chars.next();
                    break;
                }
                s.push(c);
                chars.next();
            }
            args.push(s);
        } else {
            // unquoted token until comma
            let mut s = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' {
                    break;
                }
                s.push(c);
                chars.next();
            }
            args.push(s.trim().to_string());
        }
    }
    args
}

/// Expand `~` to home and `$VAR` / `${VAR}` environment variables.
fn expand(s: &str) -> String {
    let mut s = expand_env(s);
    if s.starts_with('~') {
        if let Some(d) = directories::UserDirs::new() {
            let home = d.home_dir();
            s = format!("{}{}", home.to_string_lossy(), &s[1..]);
        }
    }
    s
}

fn expand_env(s: &str) -> String {
    // replace ${VAR} and $VAR
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() {
            if bytes[i + 1] == b'{' {
                if let Some(close) = s[i + 2..].find('}') {
                    let var = &s[i + 2..i + 2 + close];
                    out.push_str(&std::env::var(var).unwrap_or_default());
                    i = i + 2 + close + 1;
                    continue;
                }
            } else if bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_' {
                let mut j = i + 1;
                while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                    j += 1;
                }
                let var = &s[i + 1..j];
                out.push_str(&std::env::var(var).unwrap_or_default());
                i = j;
                continue;
            }
        }
        let c = s[i..]
            .chars()
            .next()
            .expect("valid UTF-8 character boundary");
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Convenience: build a Value::Map from a slice of (key, value) pairs.
pub fn map_from(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = BTreeMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::Map(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Value {
        let mut m = BTreeMap::new();
        m.insert("name".to_string(), Value::Str("hello".into()));
        m.insert("extension".to_string(), Value::Str("PDF".into()));
        let mut size = BTreeMap::new();
        size.insert("bytes".to_string(), Value::Int(1024));
        m.insert("size".to_string(), Value::Map(size));
        m.insert(
            "created".to_string(),
            Value::DateTime(
                chrono::DateTime::parse_from_rfc3339("2026-05-03T10:00:00+00:00")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
        );
        m.insert("counter".to_string(), Value::Int(2));
        Value::Map(m)
    }

    #[test]
    fn test_simple() {
        let v = render("hello {name}", &ctx()).unwrap();
        assert_eq!(v, "hello hello");
    }

    #[test]
    fn test_dotted() {
        let v = render("{size.bytes} bytes", &ctx()).unwrap();
        assert_eq!(v, "1024 bytes");
    }

    #[test]
    fn test_method_upper() {
        let v = render("{extension.lower()}", &ctx()).unwrap();
        assert_eq!(v, "pdf");
    }

    #[test]
    fn test_strftime() {
        let v = render("{created.strftime('%Y-%m')}", &ctx()).unwrap();
        assert_eq!(v, "2026-05");
    }

    #[test]
    fn test_env_expand() {
        std::env::set_var("SHELFY_TEST_ENV_ASCII", "world");
        let v = render("hi $SHELFY_TEST_ENV_ASCII!", &ctx()).unwrap();
        assert_eq!(v, "hi world!");
    }

    #[test]
    fn test_multibyte_literal_is_preserved() {
        // Regression: an earlier byte-wise implementation cast each UTF-8 byte
        // as char, mangling multi-byte characters into mojibake. Static
        // literals (e.g. a Chinese destination folder in an orden template)
        // must round-trip unchanged.
        let v = render("移动到 ~/下载/{extension.lower()}/", &ctx()).unwrap();
        assert_eq!(v, "移动到 ~/下载/pdf/");
    }

    #[test]
    fn test_multibyte_env_value_preserved() {
        std::env::set_var("SHELFY_TEST_ENV_UTF8", "下载");
        let v = render("folder/$SHELFY_TEST_ENV_UTF8/file", &ctx()).unwrap();
        assert_eq!(v, "folder/下载/file");
    }
}
