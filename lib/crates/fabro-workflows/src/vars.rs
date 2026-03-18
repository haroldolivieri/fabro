use std::collections::HashMap;

use anyhow::bail;

/// Expand `$name` placeholders in `source` using the given variable map.
///
/// Identifiers match `[a-zA-Z_][a-zA-Z0-9_]*`. A `$` not followed by an
/// identifier character is left as-is. Undefined variables produce an error.
pub fn expand_vars(source: &str, vars: &HashMap<String, String>) -> anyhow::Result<String> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' {
            let start = i + 1;
            if start < len && bytes[start] == b'$' {
                result.push('$');
                i = start + 1;
            } else if start < len && (bytes[start].is_ascii_alphabetic() || bytes[start] == b'_') {
                let mut end = start + 1;
                while end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                    end += 1;
                }
                let name = &source[start..end];
                match vars.get(name) {
                    Some(value) => result.push_str(value),
                    None => bail!("Undefined variable: ${name}"),
                }
                i = end;
            } else {
                result.push('$');
                i = start;
            }
        } else {
            result.push(source[i..].chars().next().unwrap());
            i += source[i..].chars().next().unwrap().len_utf8();
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::expand_vars;

    #[test]
    fn expand_single_var() {
        let vars = HashMap::from([("name".to_string(), "world".to_string())]);
        assert_eq!(expand_vars("Hello $name", &vars).unwrap(), "Hello world");
    }

    #[test]
    fn expand_multiple_vars() {
        let vars = HashMap::from([
            ("greeting".to_string(), "Hello".to_string()),
            ("name".to_string(), "world".to_string()),
        ]);
        assert_eq!(
            expand_vars("$greeting $name!", &vars).unwrap(),
            "Hello world!"
        );
    }

    #[test]
    fn expand_undefined_var_errors() {
        let vars = HashMap::new();
        let err = expand_vars("Hello $missing", &vars).unwrap_err();
        assert!(
            err.to_string().contains("Undefined variable: $missing"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn expand_escaped_dollar() {
        let vars = HashMap::from([("name".to_string(), "world".to_string())]);
        assert_eq!(
            expand_vars("literal $$name here", &vars).unwrap(),
            "literal $name here"
        );
    }
}
