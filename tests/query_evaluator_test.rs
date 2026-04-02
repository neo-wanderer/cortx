use cortx::entity::Entity;
use cortx::query::evaluator::evaluate;
use cortx::query::parser::parse_query;
use cortx::value::Value;
use chrono::NaiveDate;
use std::collections::HashMap;

fn make_task(status: &str, due: &str, tags: Vec<&str>) -> Entity {
    let mut fm = HashMap::new();
    fm.insert("id".into(), Value::String("task-001".into()));
    fm.insert("type".into(), Value::String("task".into()));
    fm.insert("title".into(), Value::String("Test task".into()));
    fm.insert("status".into(), Value::String(status.into()));
    if !due.is_empty() {
        fm.insert(
            "due".into(),
            Value::Date(NaiveDate::parse_from_str(due, "%Y-%m-%d").unwrap()),
        );
    }
    fm.insert(
        "tags".into(),
        Value::Array(tags.iter().map(|t| Value::String(t.to_string())).collect()),
    );
    Entity::new(fm, "Some body text about protein shakes.".into())
}

#[test]
fn test_eval_simple_eq() {
    let entity = make_task("open", "2026-04-05", vec!["home"]);
    let expr = parse_query(r#"status = "open""#).unwrap();
    assert!(evaluate(&expr, &entity));
}

#[test]
fn test_eval_simple_ne() {
    let entity = make_task("open", "2026-04-05", vec!["home"]);
    let expr = parse_query(r#"status != "done""#).unwrap();
    assert!(evaluate(&expr, &entity));
}

#[test]
fn test_eval_date_lt() {
    let entity = make_task("open", "2026-03-01", vec![]);
    let expr = parse_query(r#"due < "2026-04-01""#).unwrap();
    assert!(evaluate(&expr, &entity));
}

#[test]
fn test_eval_contains() {
    let entity = make_task("open", "", vec!["home", "urgent"]);
    let expr = parse_query(r#"tags contains "home""#).unwrap();
    assert!(evaluate(&expr, &entity));

    let expr2 = parse_query(r#"tags contains "work""#).unwrap();
    assert!(!evaluate(&expr2, &entity));
}

#[test]
fn test_eval_in() {
    let entity = make_task("open", "", vec![]);
    let expr = parse_query(r#"status in ["open", "in_progress"]"#).unwrap();
    assert!(evaluate(&expr, &entity));

    let entity2 = make_task("done", "", vec![]);
    assert!(!evaluate(&expr, &entity2));
}

#[test]
fn test_eval_text_search() {
    let entity = make_task("open", "", vec![]);
    let expr = parse_query(r#"text ~ "protein""#).unwrap();
    assert!(evaluate(&expr, &entity));

    let expr2 = parse_query(r#"text ~ "banana""#).unwrap();
    assert!(!evaluate(&expr2, &entity));
}

#[test]
fn test_eval_and() {
    let entity = make_task("open", "2026-03-01", vec!["home"]);
    let expr = parse_query(r#"status = "open" and tags contains "home""#).unwrap();
    assert!(evaluate(&expr, &entity));

    let expr2 = parse_query(r#"status = "done" and tags contains "home""#).unwrap();
    assert!(!evaluate(&expr2, &entity));
}

#[test]
fn test_eval_or() {
    let entity = make_task("done", "", vec![]);
    let expr = parse_query(r#"status = "open" or status = "done""#).unwrap();
    assert!(evaluate(&expr, &entity));
}

#[test]
fn test_eval_not() {
    let entity = make_task("open", "", vec![]);
    let expr = parse_query(r#"not status = "done""#).unwrap();
    assert!(evaluate(&expr, &entity));
}

#[test]
fn test_eval_between() {
    let entity = make_task("open", "2026-04-15", vec![]);
    let expr = parse_query(r#"due between ["2026-04-01", "2026-04-30"]"#).unwrap();
    assert!(evaluate(&expr, &entity));

    let entity2 = make_task("open", "2026-05-01", vec![]);
    assert!(!evaluate(&expr, &entity2));
}

#[test]
fn test_eval_complex() {
    let entity = make_task("open", "2026-03-15", vec!["home", "urgent"]);
    let expr = parse_query(
        r#"type = "task" and status = "open" and due < "2026-04-01" and tags contains "home""#,
    )
    .unwrap();
    assert!(evaluate(&expr, &entity));
}
