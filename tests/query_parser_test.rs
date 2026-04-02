use cortx::query::ast::{CompareOp, Expr};
use cortx::query::parser::parse_query;
use cortx::value::Value;

// -- Error cases --

#[test]
fn test_parse_unclosed_string() {
    let err = parse_query(r#"status = "open"#).unwrap_err();
    assert!(err.to_string().contains("unclosed string"));
}

#[test]
fn test_parse_unexpected_character() {
    let err = parse_query(r#"status = @"#).unwrap_err();
    assert!(err.to_string().contains("unexpected character"));
}

#[test]
fn test_parse_unexpected_end_of_query() {
    let err = parse_query("").unwrap_err();
    assert!(err.to_string().contains("unexpected end"));
}

#[test]
fn test_parse_unexpected_end_after_field() {
    let err = parse_query("status").unwrap_err();
    assert!(err.to_string().contains("unexpected end after field"));
}

#[test]
fn test_parse_unexpected_token_after_expression() {
    let err = parse_query(r#"status = "open" "extra""#).unwrap_err();
    assert!(err.to_string().contains("unexpected token"));
}

#[test]
fn test_parse_missing_closing_paren() {
    let err = parse_query(r#"(status = "open""#).unwrap_err();
    assert!(err.to_string().contains("missing closing parenthesis"));
}

#[test]
fn test_parse_expected_field_name_got_other() {
    let err = parse_query(r#""value" = "open""#).unwrap_err();
    assert!(err.to_string().contains("expected field name"));
}

#[test]
fn test_parse_unexpected_operator() {
    let err = parse_query("status ~ 5").unwrap_err();
    assert!(err.to_string().contains("unexpected operator"));
}

#[test]
fn test_parse_between_missing_bracket() {
    let err = parse_query(r#"due between "2026-01-01", "2026-12-31""#).unwrap_err();
    assert!(err.to_string().contains("expected '['"));
}

#[test]
fn test_parse_between_missing_comma() {
    let err = parse_query(r#"due between ["2026-01-01" "2026-12-31"]"#).unwrap_err();
    assert!(err.to_string().contains("expected ','"));
}

#[test]
fn test_parse_between_missing_close_bracket() {
    let err = parse_query(r#"due between ["2026-01-01", "2026-12-31""#).unwrap_err();
    assert!(err.to_string().contains("expected ']'"));
}

#[test]
fn test_parse_in_missing_bracket() {
    let err = parse_query(r#"status in "open""#).unwrap_err();
    assert!(err.to_string().contains("expected '['"));
}

#[test]
fn test_parse_expected_value_got_other() {
    let err = parse_query("status = (").unwrap_err();
    assert!(err.to_string().contains("expected value"));
}

#[test]
fn test_parse_expected_value_at_end() {
    let err = parse_query("status =").unwrap_err();
    assert!(err.to_string().contains("expected value"));
}

// -- Comparison operators --

#[test]
fn test_parse_le() {
    let expr = parse_query(r#"due <= "2026-04-30""#).unwrap();
    match &expr {
        Expr::Compare { op, .. } => assert_eq!(*op, CompareOp::Le),
        _ => panic!("expected Compare"),
    }
}

#[test]
fn test_parse_gt() {
    let expr = parse_query(r#"due > "2026-01-01""#).unwrap();
    match &expr {
        Expr::Compare { op, .. } => assert_eq!(*op, CompareOp::Gt),
        _ => panic!("expected Compare"),
    }
}

#[test]
fn test_parse_ge() {
    let expr = parse_query(r#"due >= "2026-01-01""#).unwrap();
    match &expr {
        Expr::Compare { op, .. } => assert_eq!(*op, CompareOp::Ge),
        _ => panic!("expected Compare"),
    }
}

#[test]
fn test_parse_lt() {
    let expr = parse_query(r#"due < "2026-12-31""#).unwrap();
    match &expr {
        Expr::Compare { op, .. } => assert_eq!(*op, CompareOp::Lt),
        _ => panic!("expected Compare"),
    }
}

// -- Value parsing --

#[test]
fn test_parse_value_true_false_null() {
    let expr = parse_query("active = true").unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "active".into(),
            op: CompareOp::Eq,
            value: Value::Bool(true),
        }
    );
    let expr = parse_query("active = false").unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "active".into(),
            op: CompareOp::Eq,
            value: Value::Bool(false),
        }
    );
    let expr = parse_query("active = null").unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "active".into(),
            op: CompareOp::Eq,
            value: Value::Null,
        }
    );
}

#[test]
fn test_parse_date_keywords_yesterday_tomorrow() {
    let expr = parse_query("due > yesterday").unwrap();
    match &expr {
        Expr::Compare { value, .. } => assert!(matches!(value, Value::Date(_))),
        _ => panic!("expected Compare"),
    }
    let expr = parse_query("due < tomorrow").unwrap();
    match &expr {
        Expr::Compare { value, .. } => assert!(matches!(value, Value::Date(_))),
        _ => panic!("expected Compare"),
    }
}

#[test]
fn test_parse_ident_as_string_value() {
    // Bare identifiers that aren't keywords become strings
    let expr = parse_query("status = active").unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "status".into(),
            op: CompareOp::Eq,
            value: Value::String("active".into()),
        }
    );
}

// -- Text search non-string pattern --

#[test]
fn test_parse_text_search_non_string_pattern() {
    let err = parse_query("text ~ true").unwrap_err();
    assert!(
        err.to_string()
            .contains("text search pattern must be a string")
    );
}

// -- Token Display --

#[test]
fn test_parse_field_with_unexpected_keyword() {
    // When field is followed by something that isn't an operator or keyword
    let err = parse_query(r#"status "value""#).unwrap_err();
    assert!(err.to_string().contains("expected operator"));
}

#[test]
fn test_parse_simple_eq() {
    let expr = parse_query(r#"type = "task""#).unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "type".into(),
            op: CompareOp::Eq,
            value: Value::String("task".into()),
        }
    );
}

#[test]
fn test_parse_date_comparison() {
    let expr = parse_query(r#"due < today"#).unwrap();
    match &expr {
        Expr::Compare { field, op, value } => {
            assert_eq!(field, "due");
            assert_eq!(*op, CompareOp::Lt);
            assert!(matches!(value, Value::Date(_)));
        }
        _ => panic!("expected Compare, got {expr:?}"),
    }
}

#[test]
fn test_parse_and() {
    let expr = parse_query(r#"type = "task" and status = "open""#).unwrap();
    match &expr {
        Expr::And(left, right) => {
            assert!(matches!(left.as_ref(), Expr::Compare { .. }));
            assert!(matches!(right.as_ref(), Expr::Compare { .. }));
        }
        _ => panic!("expected And, got {expr:?}"),
    }
}

#[test]
fn test_parse_contains() {
    let expr = parse_query(r#"tags contains "home""#).unwrap();
    assert_eq!(
        expr,
        Expr::Contains {
            field: "tags".into(),
            value: Value::String("home".into()),
        }
    );
}

#[test]
fn test_parse_in_list() {
    let expr = parse_query(r#"status in ["open", "in_progress"]"#).unwrap();
    assert_eq!(
        expr,
        Expr::In {
            field: "status".into(),
            values: vec![
                Value::String("open".into()),
                Value::String("in_progress".into()),
            ],
        }
    );
}

#[test]
fn test_parse_text_search() {
    let expr = parse_query(r#"text ~ "protein""#).unwrap();
    assert_eq!(
        expr,
        Expr::TextSearch {
            pattern: "protein".into(),
        }
    );
}

#[test]
fn test_parse_between() {
    let expr = parse_query(r#"due between ["2026-04-01", "2026-04-30"]"#).unwrap();
    match &expr {
        Expr::Between { field, start, end } => {
            assert_eq!(field, "due");
            assert!(matches!(start, Value::Date(_)));
            assert!(matches!(end, Value::Date(_)));
        }
        _ => panic!("expected Between, got {expr:?}"),
    }
}

#[test]
fn test_parse_complex_query() {
    let expr = parse_query(
        r#"type = "task" and status = "open" and due < today and tags contains "home""#,
    )
    .unwrap();
    match &expr {
        Expr::And(_, _) => {}
        _ => panic!("expected And, got {expr:?}"),
    }
}

#[test]
fn test_parse_or() {
    let expr = parse_query(r#"status = "open" or status = "in_progress""#).unwrap();
    assert!(matches!(expr, Expr::Or(_, _)));
}

#[test]
fn test_parse_not() {
    let expr = parse_query(r#"not status = "done""#).unwrap();
    assert!(matches!(expr, Expr::Not(_)));
}

#[test]
fn test_parse_parentheses() {
    let expr =
        parse_query(r#"type = "task" and (status = "open" or status = "in_progress")"#).unwrap();
    match &expr {
        Expr::And(_, right) => {
            assert!(matches!(right.as_ref(), Expr::Or(_, _)));
        }
        _ => panic!("expected And with Or inside, got {expr:?}"),
    }
}

#[test]
fn test_parse_ne() {
    let expr = parse_query(r#"status != "done""#).unwrap();
    assert_eq!(
        expr,
        Expr::Compare {
            field: "status".into(),
            op: CompareOp::Ne,
            value: Value::String("done".into()),
        }
    );
}
