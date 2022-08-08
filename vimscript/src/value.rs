//
// value.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::BuiltinFunction;
use crate::LineOwned;
use crate::VimError;
use crate::VimScriptCtx;
use std::collections::{HashMap, LinkedList};

pub struct VimFunction {
    params: Vec<String>,
    pub(crate) inner: Vec<LineOwned>,
}

pub enum Function<S> {
    VimScript(VimFunction),
    Builtin(Box<dyn BuiltinFunction<S>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(isize),
    Number(f64),
    Str(String),
    Bool(bool),
    Object(HashMap<String, Value>),
    List(LinkedList<Value>),
    Function(String),
}

impl Value {
    pub const TRUE: Self = Value::Bool(true);
    pub const FALSE: Self = Value::Bool(false);
    pub const NULL: Self = Value::Integer(0);

    fn string_decode(s: &str) -> Self {
        Self::Str(s.to_string())
    }

    pub fn parse_str(s: impl AsRef<str>) -> Result<Self, VimError> {
        let s = s.as_ref();
        if s.starts_with(|c| matches!(c, '\'' | '"')) {
            Ok(Self::string_decode(&s[1..]))
        } else {
            todo!("Invalid string `{}`", s)
        }
    }
    pub fn parse_num(s: impl AsRef<str>) -> Result<Self, VimError> {
        let s = s.as_ref();
        if let Ok(i) = s.parse() {
            Ok(Self::Integer(i))
        } else if let Ok(i) = s.parse() {
            Ok(Self::Number(i))
        } else {
            todo!("Invalid number")
        }
    }

    pub fn to_bool<S>(&self, ctx: &VimScriptCtx<S>) -> bool {
        match self {
            Value::Integer(i) => *i != 0,
            Value::Number(n) => *n != 0.,
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Object(o) => !o.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Function(f) => ctx.get_func(f).is_some(),
        }
    }

    pub fn to_string<S>(&self, _ctx: &VimScriptCtx<S>) -> String {
        match self {
            Value::Integer(i) => format!("{i}"),
            Value::Number(n) => format!("{n}"),
            Value::Str(s) => format!("{s}"),
            Value::Bool(b) => format!("{b}"),
            Value::Object(_o) => format!("{{}}"),
            Value::List(_l) => format!("[]"),
            Value::Function(f) => format!("{f}"),
        }
    }

    pub fn to_int<S>(&self, _ctx: &VimScriptCtx<S>) -> isize {
        match self {
            Value::Integer(i) => *i,
            Value::Number(n) => *n as isize,
            Value::Str(_s) => todo!(),
            Value::Bool(b) => {
                if *b {
                    1
                } else {
                    0
                }
            }
            Value::Object(_o) => todo!(),
            Value::List(_l) => todo!(),
            Value::Function(_f) => todo!(),
        }
    }

    pub fn to_num<S>(&self, _ctx: &VimScriptCtx<S>) -> f64 {
        match self {
            Value::Integer(i) => *i as f64,
            Value::Number(n) => *n,
            Value::Str(_s) => todo!(),
            Value::Bool(b) => {
                if *b {
                    1.
                } else {
                    0.
                }
            }
            Value::Object(_o) => todo!(),
            Value::List(_l) => todo!(),
            Value::Function(_f) => todo!(),
        }
    }

    pub fn get_int<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<isize> {
        match self {
            Value::Integer(o) => Some(*o),
            _ => None,
        }
    }

    pub fn get_float<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<f64> {
        match self {
            Value::Number(o) => Some(*o),
            Value::Integer(o) => Some(*o as f64),
            _ => None,
        }
    }

    pub fn get_obj<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get_list<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<&LinkedList<Value>> {
        match self {
            Value::List(o) => Some(o),
            _ => None,
        }
    }

    pub fn get_func<'a, S>(&self, ctx: &'a VimScriptCtx<S>) -> Option<&'a Function<S>> {
        match self {
            Value::Function(f) => ctx.get_func(f),
            _ => None,
        }
    }
}

impl std::ops::Add for Value {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l + r),
            _ => todo!("Addition"),
        }
    }
}

impl std::ops::Sub for Value {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l - r),
            _ => todo!("Subtraction"),
        }
    }
}

impl std::ops::Mul for Value {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l * r),
            _ => todo!("Multiplication"),
        }
    }
}

impl std::ops::Div for Value {
    type Output = Self;
    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l / r),
            _ => todo!("Division"),
        }
    }
}
