//! Wikilink wrapping for link-typed frontmatter fields.
//!
//! Link-typed fields (`FieldType::Link`, `FieldType::ArrayLink`) are stored
//! in YAML frontmatter as wrapped wikilink strings (`"[[Title]]"`) so they
//! render as clickable links in Obsidian. This module is the single seam
//! where wrap and unwrap happen; all downstream code operates on bare titles.

use crate::error::{CortxError, Result};

/// Wrap a bare title in wikilink syntax.
///
/// # Examples
/// ```
/// use cortx::wikilink::wrap;
/// assert_eq!(wrap("Buy Groceries"), "[[Buy Groceries]]");
/// ```
pub fn wrap(title: &str) -> String {
    format!("[[{title}]]")
}

/// Unwrap a wikilink string to a bare title.
///
/// Returns an error for:
/// - missing open/close brackets
/// - empty or whitespace-only content
/// - piped forms (`[[slug|Display]]`)
///
/// Whitespace inside brackets is trimmed.
///
/// # Examples
/// ```
/// use cortx::wikilink::unwrap;
/// assert_eq!(unwrap("[[Buy Groceries]]").unwrap(), "Buy Groceries");
/// assert!(unwrap("[[slug|Display]]").is_err());
/// ```
pub fn unwrap(wrapped: &str) -> Result<String> {
    let s = wrapped.trim();
    if !s.starts_with("[[") || !s.ends_with("]]") {
        return Err(CortxError::Validation(format!(
            "not a wikilink: {wrapped:?} (expected [[Title]])"
        )));
    }
    let inner = &s[2..s.len() - 2];
    if inner.contains('|') {
        return Err(CortxError::Validation(format!(
            "piped wikilinks are not supported: {wrapped:?}"
        )));
    }
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return Err(CortxError::Validation(format!(
            "empty wikilink: {wrapped:?}"
        )));
    }
    Ok(trimmed.to_string())
}

/// Check if a string is a well-formed `[[...]]` wikilink.
pub fn is_wrapped(s: &str) -> bool {
    let t = s.trim();
    t.starts_with("[[") && t.ends_with("]]") && t.len() > 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_bare_title() {
        assert_eq!(wrap("Buy Groceries"), "[[Buy Groceries]]");
    }

    #[test]
    fn unwrap_wrapped_title() {
        assert_eq!(unwrap("[[Buy Groceries]]").unwrap(), "Buy Groceries");
    }

    #[test]
    fn unwrap_trims_whitespace_inside() {
        assert_eq!(unwrap("[[  Buy Groceries  ]]").unwrap(), "Buy Groceries");
    }

    #[test]
    fn unwrap_rejects_missing_close() {
        assert!(unwrap("[[Buy Groceries").is_err());
    }

    #[test]
    fn unwrap_rejects_missing_open() {
        assert!(unwrap("Buy Groceries]]").is_err());
    }

    #[test]
    fn unwrap_rejects_empty() {
        assert!(unwrap("[[]]").is_err());
        assert!(unwrap("[[   ]]").is_err());
    }

    #[test]
    fn unwrap_rejects_piped_form() {
        assert!(unwrap("[[slug|Display]]").is_err());
    }

    #[test]
    fn unwrap_rejects_bare_string() {
        assert!(unwrap("bare-string").is_err());
    }

    #[test]
    fn is_wrapped_predicate() {
        assert!(is_wrapped("[[foo]]"));
        assert!(!is_wrapped("foo"));
        assert!(!is_wrapped("[[foo"));
        assert!(!is_wrapped("foo]]"));
    }

    #[test]
    fn round_trip() {
        let title = "Meeting Q2 Review";
        assert_eq!(unwrap(&wrap(title)).unwrap(), title);
    }
}
