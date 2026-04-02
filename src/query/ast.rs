use crate::value::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// field = value, field != value, field < value, etc.
    Compare {
        field: String,
        op: CompareOp,
        value: Value,
    },
    /// field between [start, end]
    Between {
        field: String,
        start: Value,
        end: Value,
    },
    /// field contains value (for arrays)
    Contains { field: String, value: Value },
    /// field in [val1, val2, ...]
    In { field: String, values: Vec<Value> },
    /// text ~ "pattern" (body text search)
    TextSearch { pattern: String },
    /// expr AND expr
    And(Box<Expr>, Box<Expr>),
    /// expr OR expr
    Or(Box<Expr>, Box<Expr>),
    /// NOT expr
    Not(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
