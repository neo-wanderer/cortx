use unicode_normalization::UnicodeNormalization;

/// Derive a filesystem-safe `id` from a human title.
///
/// Replaces each of `/ \ : * ? " < > |` and ASCII control chars with a
/// space, strips trailing dots (Windows), collapses runs of whitespace
/// to single spaces, trims edges, and NFC-normalizes the result.
///
/// Preserves uppercase, unicode letters, and filesystem-safe punctuation.
/// This function is idempotent: `sanitize(sanitize(x)) == sanitize(x)`.
///
/// # Examples
///
/// ```
/// use cortx::slug::sanitize_title;
/// assert_eq!(sanitize_title("Buy Groceries"), "Buy Groceries");
/// assert_eq!(sanitize_title("Meeting: Q2/Q3 Review"), "Meeting Q2 Q3 Review");
/// assert_eq!(sanitize_title("  multiple   spaces  "), "multiple spaces");
/// ```
pub fn sanitize_title(title: &str) -> String {
    // NFC-normalize first so composed/decomposed unicode compares equal
    let normalized: String = title.nfc().collect();

    // Replace illegal filesystem chars and control chars with a space
    let mut replaced = String::with_capacity(normalized.len());
    for c in normalized.chars() {
        let illegal =
            matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control();
        if illegal {
            replaced.push(' ');
        } else {
            replaced.push(c);
        }
    }

    // Collapse whitespace runs, trim leading
    let mut result = String::with_capacity(replaced.len());
    let mut prev_space = true;
    for c in replaced.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }
    // Trim trailing whitespace
    while result.ends_with(' ') {
        result.pop();
    }

    // Strip trailing dots (Windows constraint)
    loop {
        if result.ends_with('.') {
            result.pop();
            while result.ends_with(' ') {
                result.pop();
            }
        } else {
            break;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- sanitize_title tests ---

    #[test]
    fn sanitize_preserves_simple_title() {
        assert_eq!(sanitize_title("Buy Groceries"), "Buy Groceries");
    }

    #[test]
    fn sanitize_replaces_illegal_chars_with_space() {
        assert_eq!(
            sanitize_title("Meeting: Q2/Q3 Review"),
            "Meeting Q2 Q3 Review"
        );
    }

    #[test]
    fn sanitize_collapses_whitespace() {
        assert_eq!(sanitize_title("Q2    Planning"), "Q2 Planning");
        assert_eq!(sanitize_title("A\tB"), "A B");
    }

    #[test]
    fn sanitize_trims_edges() {
        assert_eq!(sanitize_title("  hello world  "), "hello world");
    }

    #[test]
    fn sanitize_strips_all_illegal() {
        assert_eq!(sanitize_title(r#"\/:*?"<>|"#), "");
    }

    #[test]
    fn sanitize_trailing_dot_removed() {
        assert_eq!(sanitize_title("Note..."), "Note");
        assert_eq!(sanitize_title("Foo."), "Foo");
    }

    #[test]
    fn sanitize_preserves_unicode() {
        assert_eq!(sanitize_title("Café Réunion"), "Café Réunion");
    }

    #[test]
    fn sanitize_preserves_caps_and_punct() {
        assert_eq!(
            sanitize_title("Don't Forget (Urgent)!"),
            "Don't Forget (Urgent)!"
        );
    }

    #[test]
    fn sanitize_idempotent() {
        let s = "Meeting: Q2/Q3 Review";
        assert_eq!(sanitize_title(s), sanitize_title(&sanitize_title(s)));
    }

    #[test]
    fn sanitize_nfc_normalizes() {
        // "é" as NFD (e + combining acute) vs NFC (single code point)
        let nfd = "Cafe\u{0301}";
        let nfc = "Café";
        assert_eq!(sanitize_title(nfd), sanitize_title(nfc));
    }

    #[test]
    fn sanitize_control_chars_stripped() {
        assert_eq!(sanitize_title("foo\x00bar\x1fbaz"), "foo bar baz");
    }

    #[test]
    fn sanitize_empty_input() {
        assert_eq!(sanitize_title(""), "");
        assert_eq!(sanitize_title("   "), "");
    }
}
