use crate::error::{CortxError, Result};
use crate::value::Value;
use super::ast::{CompareOp, Expr};

pub fn parse_query(input: &str) -> Result<Expr> {
    let tokens = tokenize(input)?;
    let mut pos = 0;
    let expr = parse_or_expr(&tokens, &mut pos)?;
    if pos < tokens.len() {
        return Err(CortxError::QueryParse(format!(
            "unexpected token '{}' at position {pos}",
            tokens[pos]
        )));
    }
    Ok(expr)
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    StringLit(String),
    Op(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "{s}"),
            Token::StringLit(s) => write!(f, "\"{s}\""),
            Token::Op(s) => write!(f, "{s}"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Comma => write!(f, ","),
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => { i += 1; }
            '"' => {
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '"' { i += 1; }
                if i >= chars.len() {
                    return Err(CortxError::QueryParse("unclosed string literal".into()));
                }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::StringLit(s));
                i += 1;
            }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '[' => { tokens.push(Token::LBracket); i += 1; }
            ']' => { tokens.push(Token::RBracket); i += 1; }
            ',' => { tokens.push(Token::Comma); i += 1; }
            '~' => { tokens.push(Token::Op("~".into())); i += 1; }
            '!' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Op("!=".into())); i += 2;
            }
            '<' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Op("<=".into())); i += 2;
            }
            '>' if i + 1 < chars.len() && chars[i + 1] == '=' => {
                tokens.push(Token::Op(">=".into())); i += 2;
            }
            '=' => { tokens.push(Token::Op("=".into())); i += 1; }
            '<' => { tokens.push(Token::Op("<".into())); i += 1; }
            '>' => { tokens.push(Token::Op(">".into())); i += 1; }
            c if c.is_alphanumeric() || c == '_' || c == '-' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(word));
            }
            other => {
                return Err(CortxError::QueryParse(format!("unexpected character '{other}'")));
            }
        }
    }
    Ok(tokens)
}

fn parse_or_expr(tokens: &[Token], pos: &mut usize) -> Result<Expr> {
    let mut left = parse_and_expr(tokens, pos)?;
    while *pos < tokens.len() && matches!(&tokens[*pos], Token::Ident(s) if s == "or") {
        *pos += 1;
        let right = parse_and_expr(tokens, pos)?;
        left = Expr::Or(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_and_expr(tokens: &[Token], pos: &mut usize) -> Result<Expr> {
    let mut left = parse_unary_expr(tokens, pos)?;
    while *pos < tokens.len() && matches!(&tokens[*pos], Token::Ident(s) if s == "and") {
        *pos += 1;
        let right = parse_unary_expr(tokens, pos)?;
        left = Expr::And(Box::new(left), Box::new(right));
    }
    Ok(left)
}

fn parse_unary_expr(tokens: &[Token], pos: &mut usize) -> Result<Expr> {
    if *pos < tokens.len() && matches!(&tokens[*pos], Token::Ident(s) if s == "not") {
        *pos += 1;
        let inner = parse_unary_expr(tokens, pos)?;
        return Ok(Expr::Not(Box::new(inner)));
    }
    parse_primary(tokens, pos)
}

fn parse_primary(tokens: &[Token], pos: &mut usize) -> Result<Expr> {
    if *pos >= tokens.len() {
        return Err(CortxError::QueryParse("unexpected end of query".into()));
    }

    if tokens[*pos] == Token::LParen {
        *pos += 1;
        let expr = parse_or_expr(tokens, pos)?;
        if *pos >= tokens.len() || tokens[*pos] != Token::RParen {
            return Err(CortxError::QueryParse("missing closing parenthesis".into()));
        }
        *pos += 1;
        return Ok(expr);
    }

    let field = match &tokens[*pos] {
        Token::Ident(s) => s.clone(),
        other => {
            return Err(CortxError::QueryParse(format!("expected field name, got '{other}'")));
        }
    };
    *pos += 1;

    if *pos >= tokens.len() {
        return Err(CortxError::QueryParse(format!("unexpected end after field '{field}'")));
    }

    // text ~ "pattern"
    if field == "text"
        && let Token::Op(op) = &tokens[*pos]
            && op == "~" {
                *pos += 1;
                let pattern = parse_value(tokens, pos)?;
                if let Value::String(s) = pattern {
                    return Ok(Expr::TextSearch { pattern: s });
                }
                return Err(CortxError::QueryParse("text search pattern must be a string".into()));
            }

    if matches!(&tokens[*pos], Token::Ident(s) if s == "contains") {
        *pos += 1;
        let value = parse_value(tokens, pos)?;
        return Ok(Expr::Contains { field, value });
    }

    if matches!(&tokens[*pos], Token::Ident(s) if s == "between") {
        *pos += 1;
        if *pos >= tokens.len() || tokens[*pos] != Token::LBracket {
            return Err(CortxError::QueryParse("expected '[' after 'between'".into()));
        }
        *pos += 1;
        let start = parse_value(tokens, pos)?;
        if *pos >= tokens.len() || tokens[*pos] != Token::Comma {
            return Err(CortxError::QueryParse("expected ',' in between range".into()));
        }
        *pos += 1;
        let end = parse_value(tokens, pos)?;
        if *pos >= tokens.len() || tokens[*pos] != Token::RBracket {
            return Err(CortxError::QueryParse("expected ']' after between range".into()));
        }
        *pos += 1;
        return Ok(Expr::Between { field, start, end });
    }

    if matches!(&tokens[*pos], Token::Ident(s) if s == "in") {
        *pos += 1;
        let values = parse_value_list(tokens, pos)?;
        return Ok(Expr::In { field, values });
    }

    if let Token::Op(op_str) = &tokens[*pos] {
        let op = match op_str.as_str() {
            "=" => CompareOp::Eq,
            "!=" => CompareOp::Ne,
            "<" => CompareOp::Lt,
            "<=" => CompareOp::Le,
            ">" => CompareOp::Gt,
            ">=" => CompareOp::Ge,
            other => {
                return Err(CortxError::QueryParse(format!("unexpected operator '{other}'")));
            }
        };
        *pos += 1;
        let value = parse_value(tokens, pos)?;
        return Ok(Expr::Compare { field, op, value });
    }

    Err(CortxError::QueryParse(format!(
        "expected operator after field '{field}', got '{}'",
        tokens[*pos]
    )))
}

fn parse_value(tokens: &[Token], pos: &mut usize) -> Result<Value> {
    if *pos >= tokens.len() {
        return Err(CortxError::QueryParse("expected value".into()));
    }
    match &tokens[*pos] {
        Token::StringLit(s) => {
            *pos += 1;
            if let Some(date_val) = Value::parse_as_date(s) {
                return Ok(date_val);
            }
            Ok(Value::String(s.clone()))
        }
        Token::Ident(s) => {
            *pos += 1;
            match s.as_str() {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                "null" => Ok(Value::Null),
                "today" => {
                    let today = chrono::Local::now().date_naive();
                    Ok(Value::Date(today))
                }
                "yesterday" => {
                    let d = chrono::Local::now().date_naive() - chrono::Duration::days(1);
                    Ok(Value::Date(d))
                }
                "tomorrow" => {
                    let d = chrono::Local::now().date_naive() + chrono::Duration::days(1);
                    Ok(Value::Date(d))
                }
                other => Ok(Value::String(other.to_string())),
            }
        }
        other => Err(CortxError::QueryParse(format!("expected value, got '{other}'"))),
    }
}

fn parse_value_list(tokens: &[Token], pos: &mut usize) -> Result<Vec<Value>> {
    if *pos >= tokens.len() || tokens[*pos] != Token::LBracket {
        return Err(CortxError::QueryParse("expected '['".into()));
    }
    *pos += 1;

    let mut values = Vec::new();
    while *pos < tokens.len() && tokens[*pos] != Token::RBracket {
        let val = parse_value(tokens, pos)?;
        values.push(val);
        if *pos < tokens.len() && tokens[*pos] == Token::Comma {
            *pos += 1;
        }
    }

    if *pos >= tokens.len() || tokens[*pos] != Token::RBracket {
        return Err(CortxError::QueryParse("expected ']'".into()));
    }
    *pos += 1;

    Ok(values)
}
