use chrono::NaiveDate;
use cortx::value::Value;

#[test]
fn test_value_from_yaml_string() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("\"hello\"").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(val, Value::String("hello".to_string()));
}

#[test]
fn test_value_from_yaml_date_string() {
    let val = Value::parse_as_date("2026-04-02");
    assert_eq!(
        val,
        Some(Value::Date(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap()))
    );
}

#[test]
fn test_value_from_yaml_array() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("[home, urgent]").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(
        val,
        Value::Array(vec![
            Value::String("home".to_string()),
            Value::String("urgent".to_string()),
        ])
    );
}

#[test]
fn test_value_from_yaml_bool() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("true").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn test_value_from_yaml_null() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("null").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(val, Value::Null);
}

#[test]
fn test_value_compare_dates() {
    let d1 = Value::Date(NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
    let d2 = Value::Date(NaiveDate::from_ymd_opt(2026, 4, 5).unwrap());
    assert!(d1 < d2);
}

#[test]
fn test_value_array_contains() {
    let arr = Value::Array(vec![
        Value::String("home".to_string()),
        Value::String("urgent".to_string()),
    ]);
    assert!(arr.contains(&Value::String("home".to_string())));
    assert!(!arr.contains(&Value::String("work".to_string())));
}
