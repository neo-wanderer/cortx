use cortx::query::ast::{CompareOp, Expr};
use cortx::query::parser::parse_query;
use cortx::value::Value;

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
    let expr = parse_query(r#"type = "task" and (status = "open" or status = "in_progress")"#).unwrap();
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
