//
// expr.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use std::{collections::{HashMap, LinkedList}, sync::{Mutex, Arc}};

use crate::{value::Value, State, VimError, VimScriptCtx};

#[derive(Debug, thiserror::Error)]
pub enum ValueError {
    #[error("String is not terminated")]
    UnterminatedString,
    #[error("Unexpected Symbol in value")]
    UnexpectedSymbol,
    #[error("Expression is not valid")]
    InvalidExpression,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprPeice<'a> {
    Op(&'a str),
    Var(&'a str),
    Value(Value),
    FnCall(&'a str),
    FnValueCall(String),
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
                    .find(|c| !matches!(c, '0'..='9' | '_' | '.' | 'a'..='z' | 'A'..='Z'))
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

    /// Checks if this is an operation. Note that although grouping symbols are counted as
    /// operations, this doesn't consider them as operations
    pub fn is_operation(&self) -> bool {
        matches!(self, Self::Op(op) if matches!(op.chars().next(), Some('+' | '.' | '*' | '-' | '/' | '%' | '=' | '!' | '<' | '>')))
    }

    pub fn fn_call(&self) -> Option<&str> {
        match self {
            Self::FnCall(s) => Some(s),
            Self::FnValueCall(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

pub fn parse<S: State + 'static>(
    mut expr: &str,
    ctx: &mut VimScriptCtx<S>,
    state: &mut S,
) -> Result<Value, VimError> {
    let mut parsed = vec![];
    while !expr.is_empty() {
        let (token, remaining) = ExprPeice::parse(expr)?;
        parsed.push(token);
        expr = remaining.trim();
    }
    function_call_extract(&mut parsed);
    let mut last = &ExprPeice::Op("");
    for token in parsed.iter_mut() {
        if let ExprPeice::Var(s) = token {
            let val = if matches!(last, ExprPeice::Op("&")) {
                state.get_option(s)?
            } else {
                ctx.lookup(s)?.clone()
            };
            *token = ExprPeice::Value(val);
        }
        last = token;
    }
    while parsed.len() > 1 {
        let mut changed = false;
        changed |= function_value_call_extract(&mut parsed, ctx)?;
        changed |= function_calls(&mut parsed, ctx, state)?;
        changed |= list(&mut parsed);
        changed |= list_index(&mut parsed, ctx)?;
        changed |= object(&mut parsed, ctx);
        changed |= parens(&mut parsed);
        changed |= unary_expr(
            &mut parsed,
            &[("-", &|rhs| rhs.neg(ctx)), ("!", &|rhs| rhs.not(ctx))],
        )?;
        changed |= binary_expr(
            &mut parsed,
            &[
                ("*", &|lhs, rhs| lhs.mul(rhs, ctx)),
                ("/", &|lhs, rhs| lhs.div(rhs, ctx)),
            ],
        )?;
        changed |= binary_expr(
            &mut parsed,
            &[
                ("+", &|lhs, rhs| lhs.add(rhs, ctx)),
                ("-", &|lhs, rhs| lhs.sub(rhs, ctx)),
            ],
        )?;
        changed |= binary_expr(&mut parsed, &[(".", &|lhs, rhs| lhs.concat(rhs, ctx))])?;
        changed |= binary_expr(
            &mut parsed,
            &[
                ("<", &|lhs, rhs| lhs.less(rhs, ctx)),
                (">", &|lhs, rhs| rhs.less(lhs, ctx)),
                ("<=", &|lhs, rhs| rhs.less(lhs, ctx)?.not(ctx)),
                (">=", &|lhs, rhs| lhs.less(rhs, ctx)?.not(ctx)),
                ("==", &|lhs, rhs| lhs.equal(rhs, ctx)),
                ("!=", &|lhs, rhs| lhs.equal(rhs, ctx)?.not(ctx)),
            ],
        )?;
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
                if lst.split(|c| c == &ExprPeice::Op(",")).all(|part| {
                    matches!(
                        part,
                        [ExprPeice::Value(_), ExprPeice::Op(":"), ExprPeice::Value(_)] | []
                    )
                }) {
                    let mut rem = lst.len() + 1;
                    let mut val = HashMap::new();
                    while rem > 0 {
                        rem -= 1;
                        if let ExprPeice::Value(key) = tokens.remove(i + 1) {
                            rem -= 2;
                            let token = tokens.remove(i + 1); // This can't be in the debug assert since it has side effects
                            debug_assert_eq!(token, ExprPeice::Op(":"));
                            if let ExprPeice::Value(v) = tokens.remove(i + 1) {
                                val.insert(key.to_string(ctx), v);
                            }
                        }
                    }
                    tokens[i] = ExprPeice::Value(Value::Object(Arc::new(Mutex::new(val))));
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
        if tokens[i] == ExprPeice::Op("[") && (i == 0 || tokens[i - 1].is_operation()) {
            if let Some(end) = tokens[i..].iter().position(|e| e == &ExprPeice::Op("]")) {
                let lst = &tokens[i + 1..][..end - 1];
                if lst
                    .split(|c| c == &ExprPeice::Op(","))
                    .all(|part| matches!(part, [ExprPeice::Value(_)] | []))
                {
                    let mut val = Vec::new();
                    for _ in 1..=end {
                        if let ExprPeice::Value(v) = tokens.remove(i + 1) {
                            val.push(v);
                        }
                    }
                    tokens[i] = ExprPeice::Value(Value::List(Arc::new(Mutex::new(val))));
                    changed = true;
                }
            }
        }
        i += 1;
    }
    changed
}

fn list_index<S>(tokens: &mut Vec<ExprPeice>, ctx: &mut VimScriptCtx<S>) -> Result<bool, VimError> {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(3) {
        if tokens[i + 1] == ExprPeice::Op("[")
            && tokens[i + 3] == ExprPeice::Op("]")
            && matches!(&tokens[i], ExprPeice::Value(_))
        {
            if let ExprPeice::Value(v) = &tokens[i + 2] {
                let index = v.clone();
                changed = true;
                if let ExprPeice::Value(v) = tokens.remove(i) {
                    tokens[i] = ExprPeice::Value(v.index(&index, ctx)?.clone());
                } else {
                    unreachable!("Prevous checked");
                }
                tokens.remove(i + 1);
                tokens.remove(i + 1);
            }
        }
        i += 1;
    }
    Ok(changed)
}

fn function_call_extract(tokens: &mut Vec<ExprPeice>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(1) {
        if let ExprPeice::Var(f) = tokens[i] {
            if tokens[i + 1] == ExprPeice::Op("(") {
                tokens.remove(i + 1);
                tokens[i] = ExprPeice::FnCall(f);
                changed = true;
            }
        }
        i += 1;
    }
    changed
}

fn function_value_call_extract<S: State + 'static>(
    tokens: &mut Vec<ExprPeice>,
    ctx: &mut VimScriptCtx<S>,
) -> Result<bool, VimError> {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(1) {
        if let ExprPeice::Value(Value::Function(None, name)) = &tokens[i] {
            if tokens[i + 1] == ExprPeice::Op("(") {
                tokens[i] = ExprPeice::FnValueCall(name.clone());
                tokens.remove(i + 1);
                changed = true;
            }
        }
        i += 1;
    }
    Ok(changed)
}

fn function_calls<S: State + 'static>(
    tokens: &mut Vec<ExprPeice>,
    ctx: &mut VimScriptCtx<S>,
    state: &mut S,
) -> Result<bool, VimError> {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].fn_call().is_some() {
            let mut t = i + 1;
            let mut end = i;
            while t < tokens.len() {
                if let ExprPeice::Value(_) = &tokens[t] {
                    if let ExprPeice::Op(",") = &tokens[t + 1] {
                        t += 2;
                    } else {
                        t += 1;
                    }
                } else if let ExprPeice::Op(")") = &tokens[t] {
                    end = t + 1;
                    break;
                } else {
                    break;
                }
            }
            if end > i {
                let mut args = vec![];
                for _ in i + 1..end {
                    if let ExprPeice::Value(v) = tokens.remove(i + 1) {
                        args.push(v)
                    }
                }
                if let Some(f) = tokens[i].fn_call() {
                    tokens[i] = ExprPeice::Value(ctx.run_function(f, args, state)?);
                    changed = true;
                } else {
                    unreachable!("Previously checked");
                }
            }
        }
        i += 1;
    }
    Ok(changed)
}

fn parens(tokens: &mut Vec<ExprPeice>) -> bool {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(2) {
        if tokens[i] == ExprPeice::Op("(")
            && tokens[i + 2] == ExprPeice::Op(")")
            && (i == 0 || tokens[i - 1].is_operation())
        {
            tokens.remove(i + 2);
            tokens.remove(i);
            changed = true;
        }
        i += 1;
    }
    changed
}

type OpDef<'a> = &'a dyn Fn(Value, Value) -> Result<Value, VimError>;
fn binary_expr(
    tokens: &mut Vec<ExprPeice>,
    ops: &[(&'static str, OpDef)],
) -> Result<bool, VimError> {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(2) {
        for (op, f) in ops {
            if tokens[i + 1] == ExprPeice::Op(op) {
                if let ExprPeice::Value(rhs) = tokens.remove(i + 2) {
                    if let ExprPeice::Value(lhs) = tokens.remove(i) {
                        tokens[i] = ExprPeice::Value(f(lhs, rhs)?);
                        changed = true;
                        break;
                    }
                }
            }
        }
        i += 1;
    }
    Ok(changed)
}

fn unary_expr(
    tokens: &mut Vec<ExprPeice>,
    ops: &[(&'static str, &dyn Fn(Value) -> Result<Value, VimError>)],
) -> Result<bool, VimError> {
    let mut changed = false;
    let mut i = 0;
    while i < tokens.len().saturating_sub(1) {
        for (op, f) in ops {
            if !matches!(
                tokens.get(i.saturating_sub(1)),
                Some(ExprPeice::Value(_) | ExprPeice::Var(_))
            ) && tokens[i] == ExprPeice::Op(op)
            {
                if let ExprPeice::Value(rhs) = tokens.remove(i + 1) {
                    tokens[i] = ExprPeice::Value(f(rhs)?);
                    changed = true;
                    break;
                }
            }
        }
        i += 1;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, LinkedList};

    use super::*;
    use crate::tests::{test_ctx, TestContext};

    #[track_caller]
    pub fn test_parse(s: &str) -> Value {
        parse(
            s,
            &mut test_ctx(), //.builtin("abs", nargs!(|ctx, v| v.abs(ctx))),
            &mut TestContext,
        )
        .expect("Expression failed to be parsed")
    }

    #[test]
    fn literal_bool() {
        assert_eq!(Value::Bool(false), test_parse("v:false"));
        assert_eq!(Value::Bool(true), test_parse("v:true"));
    }

    #[test]
    fn literal_string() {
        assert_eq!(Value::Str("".into()), test_parse("\"\""));
        assert_eq!(Value::Str("".into()), test_parse("''"));
    }

    // Used to allow prefix to verify the behaviour matches Rust.
    #[allow(clippy::zero_prefixed_literal)]
    #[test]
    fn literal_integer() {
        assert_eq!(Value::Integer(0), test_parse("0"));
        assert_eq!(Value::Integer(09), test_parse("09"));
        assert_eq!(Value::Integer(077), test_parse("077"));
        assert_eq!(Value::Integer(0xD), test_parse("0xD"));
        assert_eq!(Value::Integer(0o77), test_parse("0o77"));
    }

    #[test]
    fn literal_float() {
        assert_eq!(Value::Number(0.), test_parse("0.0"));
        assert_eq!(Value::Number(1.), test_parse("1.0"));
        assert_eq!(Value::Number(1.1), test_parse("1.1"));
    }

    #[test]
    fn literal_object() {
        assert_eq!(Value::Object(HashMap::new()), test_parse("{}"));
        assert_eq!(
            Value::Object(HashMap::from_iter([("a".into(), Value::Integer(0))])),
            test_parse("{'a': 0}")
        );
    }

    #[test]
    fn literal_list() {
        assert_eq!(Value::List(LinkedList::new()), test_parse("[]"));
        assert_eq!(
            Value::List([Value::Integer(0)].iter().cloned().collect()),
            test_parse("[0]")
        );
    }

    #[test]
    fn list_indexing() {
        let mut ctx = test_ctx();
        ctx.insert_var("g:a", Value::list([Value::Integer(1)]))
            .unwrap();
        assert_eq!(
            Value::Integer(1),
            parse("g:a[0]", &mut ctx, &mut TestContext).unwrap()
        );
        assert_eq!(
            Value::Integer(2),
            parse("g:a[0] + 1", &mut ctx, &mut TestContext).unwrap()
        );
    }
    //Function(String),

    #[test]
    fn integer_ops() {
        assert_eq!(Value::Integer(2), test_parse("1 + 1"));
        assert_eq!(Value::Integer(2), test_parse("1+1"));
        assert_eq!(Value::Integer(2), test_parse("1+ 1"));
        assert_eq!(Value::Integer(0), test_parse("1 - 1"));
        assert_eq!(Value::Integer(1), test_parse("1 * 1"));
        assert_eq!(Value::Integer(-1), test_parse("-1"));
    }

    #[test]
    fn number_ops() {
        assert_eq!(Value::Number(2.), test_parse("1.0 + 1"));
        assert_eq!(Value::Number(2.), test_parse("1+1.0"));
        assert_eq!(Value::Number(2.), test_parse("1+ 1.0"));
        assert_eq!(Value::Number(0.), test_parse("1.0 - 1"));
        assert_eq!(Value::Number(1.), test_parse("1.0 * 1"));
        assert_eq!(Value::Number(-1.), test_parse("-1.0"));
    }

    #[test]
    fn concat() {
        assert_eq!(Value::str("1.1"), test_parse("'' . 1.1"));
        assert_eq!(Value::str("ab"), test_parse("'a' . 'b'"));
        assert_eq!(Value::str("1"), test_parse("'' . 1"));
        assert_eq!(Value::str("1"), test_parse("1 . ''"));
    }

    #[test]
    fn comparison() {
        assert_eq!(Value::Bool(true), test_parse("1 == 1"));
        assert_eq!(Value::Bool(false), test_parse("1 == 2"));
        assert_eq!(Value::Bool(false), test_parse("1 != 1"));
        assert_eq!(Value::Bool(true), test_parse("1 != 2"));
        assert_eq!(Value::Bool(false), test_parse("1 < 1"));
        assert_eq!(Value::Bool(true), test_parse("1 < 2"));
        assert_eq!(Value::Bool(true), test_parse("1 <= 2"));
        assert_eq!(Value::Bool(true), test_parse("1 <= 1"));
        assert_eq!(Value::Bool(false), test_parse("1 > 1"));
        assert_eq!(Value::Bool(false), test_parse("1 > 2"));
        assert_eq!(Value::Bool(false), test_parse("1 >= 2"));
        assert_eq!(Value::Bool(true), test_parse("1 >= 1"));
    }

    #[test]
    fn function_call() {
        assert_eq!(Value::Number(1.), test_parse("abs(-1)"));
    }
}
