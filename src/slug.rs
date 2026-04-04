use deunicode::deunicode;

/// Convert a title string to a URL-safe slug.
///
/// Transliterates Unicode to ASCII, lowercases, replaces non-alphanumeric
/// character runs with a single hyphen, and trims leading/trailing hyphens.
///
/// # Returns
/// A URL-safe slug string. Returns an empty string if the input contains no
/// alphanumeric content after transliteration.
///
/// # Examples
/// ```
/// use cortx::slug::to_slug;
/// assert_eq!(to_slug("Buy groceries"), "buy-groceries");
/// assert_eq!(to_slug("Réunion café"), "reunion-cafe");
/// assert_eq!(to_slug("Meeting: John @ Acme"), "meeting-john-acme");
/// ```
pub fn to_slug(title: &str) -> String {
    let ascii = deunicode(title);
    let mut slug = String::new();
    let mut prev_hyphen = true; // suppress leading hyphens
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_hyphen = false;
        } else if !prev_hyphen {
            slug.push('-');
            prev_hyphen = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_lowercased_and_hyphenated() {
        assert_eq!(to_slug("Buy groceries"), "buy-groceries");
    }

    #[test]
    fn unicode_transliterated() {
        assert_eq!(to_slug("Réunion café"), "reunion-cafe");
    }

    #[test]
    fn special_chars_stripped() {
        assert_eq!(to_slug("Meeting: John @ Acme"), "meeting-john-acme");
    }

    #[test]
    fn multiple_spaces_collapsed() {
        assert_eq!(to_slug("Q2  Planning"), "q2-planning");
    }

    #[test]
    fn leading_trailing_hyphens_trimmed() {
        assert_eq!(to_slug("  hello world  "), "hello-world");
    }

    #[test]
    fn numbers_preserved() {
        assert_eq!(to_slug("Sprint 3 Goals"), "sprint-3-goals");
    }

    #[test]
    fn all_special_chars_returns_empty() {
        assert_eq!(to_slug("---"), "");
    }
}
