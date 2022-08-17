//
// value.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::State;
use crate::expr::ValueError;
use crate::BuiltinFunction;
use crate::LineOwned;
use crate::VimError;
use crate::VimScriptCtx;
use std::collections::hash_map;
use std::collections::linked_list;
use std::collections::{HashMap, LinkedList};
use std::fmt::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct VimFunction {
    params: Vec<String>,
    pub(crate) inner: Vec<LineOwned>,
}

impl VimFunction {
    pub fn new(params: Vec<String>) -> Self {
        Self {
            params,
            inner: vec![],
        }
    }
}

pub enum Function<S> {
    VimScript(VimFunction),
    Builtin(Arc<dyn BuiltinFunction<S>>),
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
    Nil,
}

impl Value {
    pub fn str(s: impl Into<String>) -> Self {
        Self::Str(s.into())
    }

    pub fn list<S: Into<Value>>(l: impl IntoIterator<Item = S>) -> Self {
        Self::List(l.into_iter().map(|s| s.into()).collect())
    }

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
        } else if let Some(s) = s.strip_prefix("0x") {
            isize::from_str_radix(s, 16)
                .map_err(|_| VimError::ValError(ValueError::UnexpectedSymbol))
                .map(Self::Integer)
        } else if let Some(s) = s.strip_prefix("0o") {
            isize::from_str_radix(s, 8)
                .map_err(|_| VimError::ValError(ValueError::UnexpectedSymbol))
                .map(Self::Integer)
        } else {
            todo!("Invalid number")
        }
    }

    pub fn to_bool<S: State>(&self, ctx: &VimScriptCtx<S>) -> bool {
        match self {
            Value::Integer(i) => *i != 0,
            Value::Number(n) => *n != 0.,
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Object(o) => !o.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Function(f) => ctx.get_func(f).is_some(),
            Value::Nil => false,
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
            Value::Nil => format!("v:null"),
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
            Value::Nil => 0,
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
            Value::Nil => 0.,
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

    pub fn get_func<'a, S: State>(&self, ctx: &'a VimScriptCtx<S>) -> Option<&'a Function<S>> {
        match self {
            Value::Function(f) => ctx.get_func(f),
            _ => None,
        }
    }

    pub fn add<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Self {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l + r),
            (l, r) => Self::Number(l.to_num(ctx) + r.to_num(ctx)),
        }
    }

    pub fn sub<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Self {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l - r),
            (l, r) => Self::Number(l.to_num(ctx) - r.to_num(ctx)),
        }
    }

    pub fn neg<S>(self, ctx: &VimScriptCtx<S>) -> Self {
        match self {
            Self::Integer(r) => Self::Integer(-r),
            r => Self::Number(-r.to_num(ctx)),
        }
    }

    pub fn abs<S>(self, ctx: &VimScriptCtx<S>) -> Self {
        match self {
            Self::Integer(r) => Self::Integer(r.abs()),
            r => Self::Number(r.to_num(ctx).abs()),
        }
    }

    pub fn not<S: State>(self, ctx: &VimScriptCtx<S>) -> Self {
        Self::Bool(!self.to_bool(ctx))
    }

    pub fn mul<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Self {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l * r),
            (l, r) => Self::Number(l.to_num(ctx) * r.to_num(ctx)),
        }
    }

    pub fn div<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Self {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l / r),
            (l, r) => Self::Number(l.to_num(ctx) / r.to_num(ctx)),
        }
    }

    pub fn concat<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Self {
        Self::Str(format!("{}{}", self, rhs))
    }

    pub fn less<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Self {
        match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Bool(l < r),
            (Self::Number(l), Self::Number(r)) => Self::Bool(l < r),
            (Self::Str(l), Self::Str(r)) => Self::Bool(l < r),
            _ => Self::Bool(false),
        }
    }

    pub fn equal<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Self {
        Self::Bool(self == rhs)
    }

    pub fn index<S>(&self, idx: Self, ctx: &VimScriptCtx<S>) -> Self {
        match self {
            Self::List(l) => {
                let idx = idx.to_int(ctx);
                if idx < 0 {
                    l.iter()
                        .rev()
                        .nth((1 - idx) as usize)
                        .unwrap_or(&Self::Nil)
                        .clone()
                } else {
                    l.iter().nth(idx as usize).unwrap_or(&Self::Nil).clone()
                }
            }
            Self::Str(s) => {
                let idx = idx.to_int(ctx);
                if idx < 0 {
                    s.chars()
                        .rev()
                        .nth((1 - idx) as usize)
                        .map(|c| Self::Str(format!("{c}")))
                        .unwrap_or(Self::Nil)
                } else {
                    s.chars()
                        .nth(idx as usize)
                        .map(|c| Self::Str(format!("{c}")))
                        .unwrap_or(Self::Nil)
                }
            }
            Self::Object(m) => m.get(&idx.to_string(ctx)).unwrap_or(&Self::Nil).clone(),
            _ => todo!(),
        }
    }

    pub fn into_iter(self) -> ValueIter {
        match self {
            Self::List(l) => ValueIter::List(l.into_iter()),
            Self::Object(m) => ValueIter::Object(m.into_iter()),
            Self::Str(s) => ValueIter::Str(s, 0),
            _ => ValueIter::Empty,
        }
    }
}

pub enum ValueIter {
    Empty,
    List(linked_list::IntoIter<Value>),
    Object(hash_map::IntoIter<String, Value>),
    Str(String, usize),
}

impl Iterator for ValueIter {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::List(l) => l.next(),
            Self::Object(m) => m.next().map(|(k, v)| Value::list([Value::Str(k), v])),
            Self::Str(s, idx) => {
                if let Some(c) = s[*idx..].chars().next() {
                    *idx += c.len_utf8();
                    Some(Value::Str(format!("{c}")))
                } else {
                    None
                }
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Number(n) => write!(f, "{}", n),
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Object(_) => write!(f, "{{ -- }}"),
            Value::List(_) => write!(f, "[ -- ]"),
            Value::Function(name) => write!(f, "<Function@{}>", name),
            Value::Nil => write!(f, "v:null"),
        }
    }
}

pub enum Names<'a> {
    Single(&'a str),
    List(Vec<Names<'a>>),
    Object(Vec<(&'a str, Names<'a>)>),
}

impl<'a> Names<'a> {
    pub fn parse(s: &'a str) -> Result<(Self, &'a str), VimError> {
        if let Some(rem) = s.strip_prefix('[') {
            todo!("Array destructure")
        } else if let Some(rem) = s.strip_prefix('{') {
            todo!("Object destructure")
        } else if let Some(idx) = s.find(|c: char| !c.is_alphanumeric()) {
            Ok((Self::Single(&s[..idx]), &s[idx..]))
        } else {
            Err(VimError::Expected("in"))
        }
    }

    pub fn iter(&self, v: Value, mut f: impl FnMut(&'a str, Value) -> Result<(), VimError>) -> Result<(), VimError> {
        match self {
            Self::Single(name) => f(name, v),
            _ => todo!("Iterate over lists & objects"),
        }
    }
}
