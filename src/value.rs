use chrono::NaiveDate;
use std::cmp::Ordering;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Date(NaiveDate),
    Bool(bool),
    Number(f64),
    Array(Vec<Value>),
    Null,
}

impl Value {
    /// Convert a `serde_yaml::Value` into a typed `Value`.
    ///
    /// Strings that look like ISO dates (`YYYY-MM-DD`) are automatically
    /// parsed as `Value::Date`.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortx::value::Value;
    ///
    /// let yaml: serde_yaml::Value = serde_yaml::from_str("\"hello\"").unwrap();
    /// assert_eq!(Value::from_yaml(&yaml), Value::String("hello".into()));
    ///
    /// let yaml: serde_yaml::Value = serde_yaml::from_str("\"2026-04-02\"").unwrap();
    /// assert!(matches!(Value::from_yaml(&yaml), Value::Date(_)));
    ///
    /// let yaml: serde_yaml::Value = serde_yaml::from_str("[a, b]").unwrap();
    /// assert!(matches!(Value::from_yaml(&yaml), Value::Array(_)));
    /// ```
    pub fn from_yaml(yaml: &serde_yaml::Value) -> Self {
        match yaml {
            serde_yaml::Value::String(s) => {
                // Try parsing as date first
                if let Some(date_val) = Self::parse_as_date(s) {
                    return date_val;
                }
                Value::String(s.clone())
            }
            serde_yaml::Value::Bool(b) => Value::Bool(*b),
            serde_yaml::Value::Number(n) => {
                Value::Number(n.as_f64().unwrap_or(0.0))
            }
            serde_yaml::Value::Sequence(seq) => {
                Value::Array(seq.iter().map(Value::from_yaml).collect())
            }
            serde_yaml::Value::Null => Value::Null,
            serde_yaml::Value::Tagged(tagged) => Value::from_yaml(&tagged.value),
            serde_yaml::Value::Mapping(_) => Value::Null,
        }
    }

    /// Try to parse a string as an ISO date (`YYYY-MM-DD`).
    ///
    /// Returns `Some(Value::Date(...))` on success, `None` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortx::value::Value;
    ///
    /// assert!(Value::parse_as_date("2026-04-02").is_some());
    /// assert!(Value::parse_as_date("not-a-date").is_none());
    /// ```
    pub fn parse_as_date(s: &str) -> Option<Value> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .ok()
            .map(Value::Date)
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Value::Date(d) => Some(*d),
            _ => None,
        }
    }

    /// Check if an array value contains the given item.
    ///
    /// Returns `false` for non-array values.
    ///
    /// # Examples
    ///
    /// ```
    /// use cortx::value::Value;
    ///
    /// let arr = Value::Array(vec![
    ///     Value::String("home".into()),
    ///     Value::String("work".into()),
    /// ]);
    /// assert!(arr.contains(&Value::String("home".into())));
    /// assert!(!arr.contains(&Value::String("urgent".into())));
    /// assert!(!Value::String("not array".into()).contains(&Value::String("x".into())));
    /// ```
    pub fn contains(&self, item: &Value) -> bool {
        match self {
            Value::Array(arr) => arr.contains(item),
            _ => false,
        }
    }

    pub fn to_yaml(&self) -> serde_yaml::Value {
        match self {
            Value::String(s) => serde_yaml::Value::String(s.clone()),
            Value::Date(d) => serde_yaml::Value::String(d.format("%Y-%m-%d").to_string()),
            Value::Bool(b) => serde_yaml::Value::Bool(*b),
            Value::Number(n) => {
                serde_yaml::Value::Number(serde_yaml::Number::from(*n))
            }
            Value::Array(arr) => {
                serde_yaml::Value::Sequence(arr.iter().map(|v| v.to_yaml()).collect())
            }
            Value::Null => serde_yaml::Value::Null,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Value::Date(a), Value::Date(b)) => a.partial_cmp(b),
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{s}"),
            Value::Date(d) => write!(f, "{d}"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
            Value::Null => write!(f, "null"),
        }
    }
}
