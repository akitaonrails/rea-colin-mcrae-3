use std::ffi::OsStr;

/// Quotes one argument for the Windows command-line parser used by the C
/// runtime, including backslashes before quotes and at the end.
pub fn quote_windows(arg: &OsStr) -> String {
    let mut out = String::from("\"");
    let mut slashes = 0;
    for ch in arg.to_string_lossy().chars() {
        match ch {
            '\\' => {
                slashes += 1;
                out.push(ch);
            }
            '"' => {
                out.push_str(&"\\".repeat(slashes + 1));
                out.push(ch);
                slashes = 0;
            }
            _ => {
                out.push(ch);
                slashes = 0;
            }
        }
    }
    out.push_str(&"\\".repeat(slashes));
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_spaces_empty_strings_quotes_and_trailing_backslashes() {
        assert_eq!(quote_windows(OsStr::new("plain")), "\"plain\"");
        assert_eq!(quote_windows(OsStr::new("two words")), "\"two words\"");
        assert_eq!(quote_windows(OsStr::new("")), "\"\"");
        assert_eq!(quote_windows(OsStr::new("a\"b")), "\"a\\\"b\"");
        assert_eq!(
            quote_windows(OsStr::new(r"C:\Games\Colin McRae Rally 3\")),
            r#""C:\Games\Colin McRae Rally 3\\""#
        );
    }
}
