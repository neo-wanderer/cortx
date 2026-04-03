use chrono::NaiveDate;
use cortx::value::Value;

// -- Display tests --

#[test]
fn test_value_display_bool() {
    assert_eq!(format!("{}", Value::Bool(true)), "true");
    assert_eq!(format!("{}", Value::Bool(false)), "false");
}

#[test]
fn test_value_display_number() {
    assert_eq!(format!("{}", Value::Number(42.0)), "42");
    assert_eq!(format!("{}", Value::Number(3.14)), "3.14");
}

#[test]
fn test_value_display_null() {
    assert_eq!(format!("{}", Value::Null), "null");
}

#[test]
fn test_value_display_string() {
    assert_eq!(format!("{}", Value::String("hello".into())), "hello");
}

#[test]
fn test_value_display_date() {
    let d = Value::Date(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap());
    assert_eq!(format!("{d}"), "2026-04-02");
}

#[test]
fn test_value_display_array() {
    let arr = Value::Array(vec![Value::String("a".into()), Value::String("b".into())]);
    assert_eq!(format!("{arr}"), "[a, b]");
}

// -- PartialOrd tests --

#[test]
fn test_value_compare_numbers() {
    let a = Value::Number(1.0);
    let b = Value::Number(5.0);
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn test_value_compare_strings() {
    let a = Value::String("alpha".into());
    let b = Value::String("beta".into());
    assert!(a < b);
}

#[test]
fn test_value_compare_bools() {
    assert!(Value::Bool(false) < Value::Bool(true));
    assert!(Value::Bool(true) > Value::Bool(false));
    assert_eq!(Value::Bool(false), Value::Bool(false));
}

#[test]
fn test_value_compare_arrays() {
    let a = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
    let b = Value::Array(vec![Value::Number(1.0), Value::Number(3.0)]);
    assert!(a < b);
}

#[test]
fn test_value_compare_mixed_types_consistent_ordering() {
    // Mixed types should have a consistent ordering for stable sorting
    // Order: Null < Bool < Number < String < Date < Array
    assert!(Value::Null < Value::Bool(false));
    assert!(Value::Bool(false) < Value::Number(1.0));
    assert!(Value::Number(1.0) < Value::String("hello".into()));
    let d = Value::Date(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
    assert!(Value::String("hello".into()) < d);
    let arr = Value::Array(vec![]);
    assert!(d < arr);
}

// -- from_yaml edge cases --

#[test]
fn test_value_from_yaml_number() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("42").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(val, Value::Number(42.0));
}

#[test]
fn test_value_from_yaml_mapping_becomes_null() {
    let yaml: serde_yaml::Value = serde_yaml::from_str("foo: bar").unwrap();
    let val = Value::from_yaml(&yaml);
    assert_eq!(val, Value::Null);
}

// -- to_yaml roundtrip tests --

#[test]
fn test_value_to_yaml_roundtrip() {
    let values = vec![
        Value::String("hello".into()),
        Value::Bool(true),
        Value::Number(3.14),
        Value::Null,
        Value::Date(NaiveDate::from_ymd_opt(2026, 4, 2).unwrap()),
        Value::Array(vec![Value::String("a".into())]),
    ];
    for val in values {
        let yaml = val.to_yaml();
        let back = Value::from_yaml(&yaml);
        assert_eq!(val, back, "roundtrip failed for {val:?}");
    }
}

// -- as_str and as_date --

#[test]
fn test_value_as_str_non_string() {
    assert_eq!(Value::Bool(true).as_str(), None);
    assert_eq!(Value::Null.as_str(), None);
}

#[test]
fn test_value_as_date_non_date() {
    assert_eq!(Value::String("foo".into()).as_date(), None);
    assert_eq!(Value::Null.as_date(), None);
}

#[test]
fn test_value_contains_non_array() {
    assert!(!Value::String("hello".into()).contains(&Value::String("h".into())));
    assert!(!Value::Null.contains(&Value::String("x".into())));
}

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
