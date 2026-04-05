//! Wikilink wrapping for link-typed frontmatter fields.
//!
//! Link-typed fields (`FieldType::Link`, `FieldType::ArrayLink`) are stored
//! in YAML frontmatter as wrapped wikilink strings (`"[[Title]]"`) so they
//! render as clickable links in Obsidian. This module is the single seam
//! where wrap and unwrap happen; all downstream code operates on bare titles.

use crate::error::{CortxError, Result};
use crate::schema::types::{FieldType, TypeDefinition};
use crate::value::Value;
use std::collections::HashMap;

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

/// Wrap every link-typed field value in `[[...]]` form.
///
/// Operates in place. Consults the type definition to decide which fields
/// are `Link`/`ArrayLink`. Non-link fields are untouched.
///
/// Assumes link field values are bare titles (strings or arrays of strings).
/// Already-wrapped values are wrapped again — callers should only pass bare
/// titles here.
pub fn wrap_frontmatter(fm: &mut HashMap<String, Value>, type_def: &TypeDefinition) {
    for (field_name, field_def) in &type_def.fields {
        let is_link = matches!(
            field_def.field_type,
            FieldType::Link(_) | FieldType::ArrayLink(_)
        );
        if !is_link {
            continue;
        }
        let Some(value) = fm.get_mut(field_name) else {
            continue;
        };
        match value {
            Value::String(s) if !s.is_empty() => {
                *s = wrap(s);
            }
            Value::Array(items) => {
                for item in items.iter_mut() {
                    if let Value::String(s) = item
                        && !s.is_empty()
                    {
                        *s = wrap(s);
                    }
                }
            }
            _ => {}
        }
    }
}

/// Unwrap every link-typed field value from `[[...]]` to bare titles.
///
/// Operates in place. Returns an error if any link-typed field contains a
/// malformed value (not `[[Title]]` form, piped, or empty).
///
/// Empty strings, nulls, and empty arrays are tolerated.
pub fn unwrap_frontmatter(
    fm: &mut HashMap<String, Value>,
    type_def: &TypeDefinition,
) -> Result<()> {
    for (field_name, field_def) in &type_def.fields {
        let is_link = matches!(
            field_def.field_type,
            FieldType::Link(_) | FieldType::ArrayLink(_)
        );
        if !is_link {
            continue;
        }
        let Some(value) = fm.get_mut(field_name) else {
            continue;
        };
        match value {
            Value::String(s) if !s.is_empty() => {
                let bare = unwrap(s)
                    .map_err(|e| CortxError::Validation(format!("field '{field_name}': {e}")))?;
                *s = bare;
            }
            Value::Array(items) => {
                for (idx, item) in items.iter_mut().enumerate() {
                    if let Value::String(s) = item {
                        if s.is_empty() {
                            continue;
                        }
                        let bare = unwrap(s).map_err(|e| {
                            CortxError::Validation(format!("field '{field_name}[{idx}]': {e}"))
                        })?;
                        *s = bare;
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
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

    use crate::schema::types::{FieldDefinition, LinkDef, LinkTargets};

    fn mk_type_def_with_link_fields() -> TypeDefinition {
        let mut fields = HashMap::new();

        let single_link = FieldDefinition {
            field_type: FieldType::Link(LinkDef {
                targets: LinkTargets::Single {
                    ref_type: "project".into(),
                    inverse: None,
                },
                bidirectional: false,
                inverse_one: false,
            }),
            required: false,
            default: None,
        };
        let array_link = FieldDefinition {
            field_type: FieldType::ArrayLink(LinkDef {
                targets: LinkTargets::Single {
                    ref_type: "note".into(),
                    inverse: None,
                },
                bidirectional: false,
                inverse_one: false,
            }),
            required: false,
            default: None,
        };
        let string_field = FieldDefinition {
            field_type: FieldType::String,
            required: false,
            default: None,
        };

        fields.insert("project".into(), single_link);
        fields.insert("related".into(), array_link);
        fields.insert("title".into(), string_field);

        TypeDefinition {
            name: "task".into(),
            folder: "tasks".into(),
            required: vec![],
            fields,
        }
    }

    #[test]
    fn wrap_frontmatter_wraps_link_fields() {
        let mut fm = HashMap::new();
        fm.insert("project".into(), Value::String("Website Redesign".into()));
        fm.insert(
            "related".into(),
            Value::Array(vec![
                Value::String("Weekly Review".into()),
                Value::String("Meal Planning".into()),
            ]),
        );
        fm.insert("title".into(), Value::String("Buy Groceries".into()));

        let td = mk_type_def_with_link_fields();
        wrap_frontmatter(&mut fm, &td);

        assert_eq!(
            fm["project"],
            Value::String("[[Website Redesign]]".into())
        );
        assert_eq!(
            fm["related"],
            Value::Array(vec![
                Value::String("[[Weekly Review]]".into()),
                Value::String("[[Meal Planning]]".into()),
            ])
        );
        assert_eq!(fm["title"], Value::String("Buy Groceries".into()));
    }

    #[test]
    fn unwrap_frontmatter_unwraps_link_fields() {
        let mut fm = HashMap::new();
        fm.insert(
            "project".into(),
            Value::String("[[Website Redesign]]".into()),
        );
        fm.insert(
            "related".into(),
            Value::Array(vec![Value::String("[[Weekly Review]]".into())]),
        );
        fm.insert("title".into(), Value::String("[[Buy Groceries]]".into()));

        let td = mk_type_def_with_link_fields();
        unwrap_frontmatter(&mut fm, &td).unwrap();

        assert_eq!(fm["project"], Value::String("Website Redesign".into()));
        assert_eq!(
            fm["related"],
            Value::Array(vec![Value::String("Weekly Review".into())])
        );
        // Non-link string field untouched — literal `[[...]]` preserved
        assert_eq!(fm["title"], Value::String("[[Buy Groceries]]".into()));
    }

    #[test]
    fn unwrap_frontmatter_errors_on_malformed_link_field() {
        let mut fm = HashMap::new();
        fm.insert(
            "project".into(),
            Value::String("bare-string-not-wrapped".into()),
        );
        let td = mk_type_def_with_link_fields();
        assert!(unwrap_frontmatter(&mut fm, &td).is_err());
    }

    #[test]
    fn unwrap_frontmatter_tolerates_empty_arrays_and_null() {
        let mut fm = HashMap::new();
        fm.insert("related".into(), Value::Array(vec![]));
        fm.insert("project".into(), Value::Null);
        let td = mk_type_def_with_link_fields();
        unwrap_frontmatter(&mut fm, &td).unwrap();
        assert_eq!(fm["related"], Value::Array(vec![]));
        assert_eq!(fm["project"], Value::Null);
    }
}
