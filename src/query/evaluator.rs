use crate::entity::Entity;
use crate::value::Value;
use super::ast::{CompareOp, Expr};

pub fn evaluate(expr: &Expr, entity: &Entity) -> bool {
    match expr {
        Expr::Compare { field, op, value } => {
            let entity_val = match entity.get(field) {
                Some(v) => v,
                None => return matches!(op, CompareOp::Ne),
            };
            compare_values(entity_val, op, value)
        }
        Expr::Between { field, start, end } => {
            let entity_val = match entity.get(field) {
                Some(v) => v,
                None => return false,
            };
            entity_val >= start && entity_val <= end
        }
        Expr::Contains { field, value } => {
            let entity_val = match entity.get(field) {
                Some(v) => v,
                None => return false,
            };
            entity_val.contains(value)
        }
        Expr::In { field, values } => {
            let entity_val = match entity.get(field) {
                Some(v) => v,
                None => return false,
            };
            values.iter().any(|v| entity_val == v)
        }
        Expr::TextSearch { pattern } => {
            let body_lower = entity.body.to_lowercase();
            let pattern_lower = pattern.to_lowercase();
            body_lower.contains(&pattern_lower)
        }
        Expr::And(left, right) => evaluate(left, entity) && evaluate(right, entity),
        Expr::Or(left, right) => evaluate(left, entity) || evaluate(right, entity),
        Expr::Not(inner) => !evaluate(inner, entity),
    }
}

fn compare_values(entity_val: &Value, op: &CompareOp, query_val: &Value) -> bool {
    match op {
        CompareOp::Eq => entity_val == query_val,
        CompareOp::Ne => entity_val != query_val,
        CompareOp::Lt => entity_val.partial_cmp(query_val) == Some(std::cmp::Ordering::Less),
        CompareOp::Le => matches!(
            entity_val.partial_cmp(query_val),
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        CompareOp::Gt => entity_val.partial_cmp(query_val) == Some(std::cmp::Ordering::Greater),
        CompareOp::Ge => matches!(
            entity_val.partial_cmp(query_val),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
    }
}
