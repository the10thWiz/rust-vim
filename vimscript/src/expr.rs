//
// expr.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::collections::{HashMap, LinkedList};

use crate::{value::Value, VimError, VimScriptCtx};

#[derive(Debug)]
pub enum ValueError {
    UnterminatedString,
    UnexpectedSymbol,
    InvalidExpression,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprPeice<'a> {
    Op(&'a str),
    Var(&'a str),
    Value(Value),
}

impl<'a> ExprPeice<'a> {
    fn parse(expr: &'a str) -> Result<(Self, &'a str), VimError> {
        let mut chars = expr.chars();
        let first_char = chars.next().expect("Non-empty string");
        match first_char {
            '\'' | '"' => {
                let mut last = '\\';
                let i = expr
                    .find(|c| {
                        let res = last != '\\' && c == first_char;
                        last = c;
                        res
                    })
                    .ok_or(ValueError::UnterminatedString)?;
                Ok((Self::Value(Value::parse_str(&expr[..i])?), &expr[i + 1..]))
            }
            '0'..='9' => {
                let i = expr
                    .find(|c| !matches!(c, '0'..='9' | '_' | '.'))
                    .unwrap_or(expr.len());
                Ok((Self::Value(Value::parse_num(&expr[..i])?), &expr[i..]))
            }
            'a'..='z' | 'A'..='Z' => {
                let i = expr
                    .find(|c| !matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | ':'))
                    .unwrap_or(expr.len());
                Ok((Self::Var(&expr[..i]), &expr[i..]))
            }
            '+' | '.' | '*' | '-' | '/' | '%' | '=' | '!' | '<' | '>' | ',' | '[' | ']' | '{'
            | '}' | '(' | ')' | ':' => {
                if matches!(chars.next(), Some('=')) {
                    Ok((Self::Op(&expr[..2]), &expr[2..]))
                } else {
                    Ok((Self::Op(&expr[..1]), &expr[1..]))
                }
            }
            _ => Err(ValueError::UnexpectedSymbol.into()),
        }
    }
}

pub fn parse<S>(mut expr: &str, ctx: &mut VimScriptCtx<S>) -> Result<Value, VimError> {
    let mut parsed = vec![];
    while !expr.is_empty() {
        let (token, remaining) = ExprPeice::parse(expr)?;
        parsed.push(token);
        expr = remaining.trim();
    }
    for token in parsed.iter_mut() {
        if let ExprPeice::Var(s) = token {
            *token = ExprPeice::Value(ctx.lookup(s)?.clone());
        }
    }
    while parsed.len() > 1 {
        let mut changed = false;
        changed |= list(&mut parsed);
        changed |= object(&mut parsed, ctx);
        changed |= parens(&mut parsed);
        changed |= binary_expr(
            &mut parsed,
            &[("*", &|lhs, rhs| lhs * rhs), ("/", &|lhs, rhs| lhs / rhs)],
        );
        changed |= binary_expr(
            &mut parsed,
            &[("+", &|lhs, rhs| lhs + rhs), ("-", &|lhs, rhs| lhs - rhs)],
        );
        if !changed {
            todo!("parse {parsed:?}");
        }
    }
    if let ExprPeice::Value(v) = parsed.remove(0) {
        Ok(v)
    } else {
        Err(ValueError::InvalidExpression.into())
    }
}

fn object<S>(tokens: &mut Vec<ExprPeice>, ctx: &mut VimScriptCtx<S>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(1) {
        if tokens[i] == ExprPeice::Op("{") {
            if let Some(end) = tokens[i..].iter().position(|e| e == &ExprPeice::Op("}")) {
                let lst = &tokens[i + 1..][..end - 1];
                let mut valid = true;
                for part in lst.split(|c| c == &ExprPeice::Op(",")) {
                    if !matches!(
                        part,
                        [ExprPeice::Value(_), ExprPeice::Op(":"), ExprPeice::Value(_)] | []
                    ) {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    let mut val = HashMap::new();
                    for _ in 1..=end {
                        if let ExprPeice::Value(key) = tokens.remove(i + 1) {
                            debug_assert_eq!(tokens.remove(i + 1), ExprPeice::Op(":"));
                            if let ExprPeice::Value(v) = tokens.remove(i + 1) {
                                val.insert(key.to_string(ctx), v);
                            }
                        }
                    }
                    tokens[i] = ExprPeice::Value(Value::Object(val));
                    changed = true;
                }
            }
        }
        i += 1;
    }
    changed
}

fn list(tokens: &mut Vec<ExprPeice>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(1) {
        if tokens[i] == ExprPeice::Op("[") {
            if let Some(end) = tokens[i..].iter().position(|e| e == &ExprPeice::Op("]")) {
                let lst = &tokens[i + 1..][..end - 1];
                let mut valid = true;
                for part in lst.split(|c| c == &ExprPeice::Op(",")) {
                    if !matches!(part, [ExprPeice::Value(_)] | []) {
                        valid = false;
                        break;
                    }
                }
                if valid {
                    let mut val = LinkedList::new();
                    for _ in 1..=end {
                        if let ExprPeice::Value(v) = tokens.remove(i + 1) {
                            val.push_back(v);
                        }
                    }
                    tokens[i] = ExprPeice::Value(Value::List(val));
                    changed = true;
                }
            }
        }
        i += 1;
    }
    changed
}

fn parens(tokens: &mut Vec<ExprPeice>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(2) {
        if tokens[i] == ExprPeice::Op("(") && tokens[i + 2] == ExprPeice::Op(")") {
            tokens.remove(i + 2);
            tokens.remove(i);
            changed = true;
        }
        i += 1;
    }
    changed
}

fn binary_expr(
    tokens: &mut Vec<ExprPeice>,
    ops: &[(&'static str, &dyn Fn(Value, Value) -> Value)],
) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(2) {
        for (op, f) in ops {
            if tokens[i + 1] == ExprPeice::Op(op) {
                if let ExprPeice::Value(rhs) = tokens.remove(i + 2) {
                    if let ExprPeice::Value(lhs) = tokens.remove(i) {
                        tokens[i] = ExprPeice::Value(f(lhs, rhs));
                        changed = true;
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    changed
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, LinkedList};

    use super::*;
    use crate::tests::test_ctx;

    #[test]
    fn literal_bool() {
        assert_eq!(
            Value::Bool(false),
            parse("v:false", &mut test_ctx()).unwrap()
        );
        assert_eq!(Value::Bool(true), parse("v:true", &mut test_ctx()).unwrap());
    }

    #[test]
    fn literal_string() {
        assert_eq!(
            Value::Str("".into()),
            parse("\"\"", &mut test_ctx()).unwrap()
        );
        assert_eq!(Value::Str("".into()), parse("''", &mut test_ctx()).unwrap());
    }

    #[test]
    fn literal_integer() {
        assert_eq!(Value::Integer(0), parse("0", &mut test_ctx()).unwrap());
        assert_eq!(Value::Integer(09), parse("09", &mut test_ctx()).unwrap());
        //assert_eq!(Value::Integer(0xD), parse("0xD", &mut test_ctx()).unwrap());
    }

    #[test]
    fn literal_float() {
        assert_eq!(Value::Number(0.), parse("0.0", &mut test_ctx()).unwrap());
        assert_eq!(Value::Number(1.), parse("1.0", &mut test_ctx()).unwrap());
        assert_eq!(Value::Number(1.1), parse("1.1", &mut test_ctx()).unwrap());
    }

    #[test]
    fn literal_object() {
        assert_eq!(
            Value::Object(HashMap::new()),
            parse("{}", &mut test_ctx()).unwrap()
        );
    }

    #[test]
    fn literal_list() {
        assert_eq!(
            Value::List(LinkedList::new()),
            parse("[]", &mut test_ctx()).unwrap()
        );
        assert_eq!(
            Value::List([Value::Integer(0)].iter().cloned().collect()),
            parse("[0]", &mut test_ctx()).unwrap()
        );
    }
    //Function(String),

    #[test]
    fn add_integer() {
        assert_eq!(Value::Integer(2), parse("1 + 1", &mut test_ctx()).unwrap());
        assert_eq!(Value::Integer(2), parse("1+1", &mut test_ctx()).unwrap());
        assert_eq!(Value::Integer(2), parse("1+ 1", &mut test_ctx()).unwrap());
        assert_eq!(Value::Integer(0), parse("1 - 1", &mut test_ctx()).unwrap());
        assert_eq!(Value::Integer(1), parse("1 * 1", &mut test_ctx()).unwrap());
    }
}
