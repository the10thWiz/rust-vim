//
// value.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::expr::ValueError;
use crate::BuiltinFunction;
use crate::LineOwned;
use crate::RunTy;
use crate::Section;
use crate::State;
use crate::Tokenizer;
use crate::VimError;
use crate::VimScriptCtx;
use std::borrow::Cow;
use std::collections::hash_map;
use std::collections::linked_list;
use std::collections::{HashMap, LinkedList};
use std::fmt::Display;
use std::str::pattern::Pattern;
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

    pub fn execute<S: State>(
        &self,
        args: Vec<Value>,
        ctx: &mut VimScriptCtx<S>,
        state: &mut S,
    ) -> Result<Value, VimError> {
        for (name, val) in self
            .params
            .iter()
            .zip(args.into_iter().chain(std::iter::repeat(Value::Nil)))
        {
            ctx.insert_var(name, val)?;
        }
        let mut script = Tokenizer::from_iter(self.inner.iter());
        ctx.run_inner(&mut script, Section::Function, RunTy::Now, state)
            .map(|v| v.unwrap_or(Value::Nil))
    }
}

pub enum Function<S> {
    VimScript(Arc<VimFunction>),
    Builtin(Arc<dyn BuiltinFunction<S>>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueRef<'a> {
    Integer(isize),
    Number(f64),
    Str(Cow<'a, str>),
    Bool(bool),
    Object(&'a HashMap<String, Value>),
    List(&'a LinkedList<Value>),
    Function(Cow<'a, str>),
    Nil,
}

impl<'a> From<isize> for ValueRef<'a> {
    fn from(v: isize) -> Self {
        Self::Integer(v)
    }
}
impl<'a> From<f64> for ValueRef<'a> {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}
impl<'a> From<bool> for ValueRef<'a> {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl<'a> From<&'a str> for ValueRef<'a> {
    fn from(v: &'a str) -> Self {
        Self::Str(Cow::Borrowed(v))
    }
}
impl<'a> From<&'a String> for ValueRef<'a> {
    fn from(v: &'a String) -> Self {
        Self::Str(Cow::Borrowed(v.as_str()))
    }
}
impl<'a, T: Into<ValueRef<'a>> + Copy> From<&T> for ValueRef<'a> {
    fn from(v: &T) -> Self {
        T::into(*v)
    }
}

impl Display for ValueRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(i) => write!(f, "{}", i),
            Self::Number(n) => write!(f, "{}", n),
            Self::Str(s) => write!(f, "{}", s),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Object(_) => write!(f, "{{ -- }}"),
            Self::List(_) => write!(f, "[ -- ]"),
            Self::Function(name) => write!(f, "<Function@{}>", name),
            Self::Nil => write!(f, "v:null"),
        }
    }
}

impl From<ValueRef<'_>> for Value {
    fn from(v: ValueRef<'_>) -> Self {
        match v {
            ValueRef::Integer(v) => Self::Integer(v),
            ValueRef::Number(v) => Self::Number(v),
            ValueRef::Str(v) => Self::Str(v.to_string()),
            ValueRef::Bool(v) => Self::Bool(v),
            ValueRef::Object(v) => Self::Object(v.clone()),
            ValueRef::List(v) => Self::List(v.clone()),
            ValueRef::Function(v) => Self::Function(v.to_string()),
            ValueRef::Nil => Self::Nil,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimType {
    Integer,
    Number,
    Str,
    Bool,
    Object,
    List,
    Function,
    Nil,
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

impl From<isize> for Value {
    fn from(v: isize) -> Self {
        Self::Integer(v)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}
impl From<String> for Value {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Str(v.to_string())
    }
}
impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        v.map_or(Self::Nil, |v| v.into())
    }
}
impl<T: Into<Value> + Clone> From<&T> for Value {
    fn from(v: &T) -> Self {
        v.clone().into()
    }
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

    pub fn to_bool<S: State + 'static>(&self, ctx: &VimScriptCtx<S>) -> Result<bool, VimError> {
        Ok(match self {
            Value::Integer(i) => *i != 0,
            Value::Number(n) => *n != 0.,
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Object(o) => !o.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Function(f) => ctx.get_func(f).is_some(),
            Value::Nil => false,
        })
    }

    pub fn to_string<S>(&self, ctx: &VimScriptCtx<S>) -> String {
        match self {
            Value::Integer(i) => format!("{i}"),
            Value::Number(n) => format!("{n}"),
            Value::Str(s) => s.to_string(),
            Value::Bool(b) => format!("{b}"),
            Value::Object(o) => std::iter::once("{".to_string())
                .chain(
                    o.iter()
                        .flat_map(|(n, v)| {
                            [n.clone(), ":".to_string(), v.to_string(ctx)].into_iter()
                        })
                        .intersperse(",".to_string()),
                )
                .chain(std::iter::once("}".to_string()))
                .collect(),
            Value::List(l) => std::iter::once("[".to_string())
                .chain(
                    l.iter()
                        .map(|v| v.to_string(ctx))
                        .intersperse(",".to_string()),
                )
                .chain(std::iter::once("]".to_string()))
                .collect(),
            Value::Function(f) => f.clone(),
            Value::Nil => "v:null".to_string(),
        }
    }

    pub fn to_int<S>(&self, _ctx: &VimScriptCtx<S>) -> Result<isize, VimError> {
        match self {
            Value::Integer(i) => Ok(*i),
            Value::Number(n) => Ok(*n as isize),
            Value::Str(_s) => todo!(),
            Value::Bool(b) => {
                if *b {
                    Ok(1)
                } else {
                    Ok(0)
                }
            }
            Value::Object(_o) => todo!(),
            Value::List(_l) => todo!(),
            Value::Function(_f) => todo!(),
            Value::Nil => Ok(0),
        }
    }

    pub fn to_num<S>(&self, _ctx: &VimScriptCtx<S>) -> Result<f64, VimError> {
        match self {
            Value::Integer(i) => Ok(*i as f64),
            Value::Number(n) => Ok(*n),
            Value::Str(_s) => todo!(),
            Value::Bool(b) => {
                if *b {
                    Ok(1.)
                } else {
                    Ok(0.)
                }
            }
            Value::Object(_o) => todo!(),
            Value::List(_l) => todo!(),
            Value::Function(_f) => todo!(),
            Value::Nil => Ok(0.),
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

    pub fn get_func<'a, S: State + 'static>(
        &self,
        ctx: &'a VimScriptCtx<S>,
    ) -> Option<&'a Function<S>> {
        match self {
            Value::Function(f) => ctx.get_func(f),
            _ => None,
        }
    }

    pub fn add<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l + r),
            (l, r) => Self::Number(l.to_num(ctx)? + r.to_num(ctx)?),
        })
    }

    pub fn sub<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l - r),
            (l, r) => Self::Number(l.to_num(ctx)? - r.to_num(ctx)?),
        })
    }

    pub fn neg<S>(self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match self {
            Self::Integer(r) => Self::Integer(-r),
            r => Self::Number(-r.to_num(ctx)?),
        })
    }

    pub fn abs<S>(self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match self {
            Self::Integer(r) => Self::Integer(r.abs()),
            r => Self::Number(r.to_num(ctx)?.abs()),
        })
    }

    pub fn not<S: State + 'static>(self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(Self::Bool(!self.to_bool(ctx)?))
    }

    pub fn mul<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l * r),
            (l, r) => Self::Number(l.to_num(ctx)? * r.to_num(ctx)?),
        })
    }

    pub fn div<S>(self, rhs: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Integer(l / r),
            (l, r) => Self::Number(l.to_num(ctx)? / r.to_num(ctx)?),
        })
    }

    pub fn concat<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(Self::Str(format!("{}{}", self, rhs)))
    }

    pub fn less<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match (self, rhs) {
            (Self::Integer(l), Self::Integer(r)) => Self::Bool(l < r),
            (Self::Number(l), Self::Number(r)) => Self::Bool(l < r),
            (Self::Str(l), Self::Str(r)) => Self::Bool(l < r),
            _ => Self::Bool(false),
        })
    }

    pub fn equal<S>(self, rhs: Self, _ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(Self::Bool(self == rhs))
    }

    pub fn index<S>(&self, idx: &Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        Ok(match self {
            Self::List(l) => {
                let idx = idx.to_int(ctx)?;
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
                let idx = idx.to_int(ctx)?;
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
        })
    }

    pub fn list_len(&self) -> Result<Self, VimError> {
        match self {
            Self::List(l) => Ok(Self::Integer(l.len() as isize)),
            _ => Err(VimError::ExpectedType(VimType::Object)),
        }
    }

    pub fn list_empty(&self) -> Result<Self, VimError> {
        match self {
            Self::List(l) => Ok(Self::Bool(l.is_empty())),
            _ => Err(VimError::ExpectedType(VimType::Object)),
        }
    }

    pub fn starts_with<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        match self {
            Self::Str(s) => s.starts_with(pat),
            _ => false,
        }
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        match self {
            Self::Str(s) => s == other,
            _ => false,
        }
    }
}

impl IntoIterator for Value {
    type Item = Self;
    type IntoIter = ValueIter;
    fn into_iter(self) -> ValueIter {
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
        if let Some(mut rem) = s.strip_prefix('[') {
            let mut ret = vec![];
            loop {
                if let Some(rem) = rem.trim().strip_prefix(']') {
                    return Ok((Self::List(ret), rem));
                } else if rem.trim() == "" {
                    return Err(VimError::Expected("]"));
                }
                let (name, new_rem) = Self::parse(rem)?;
                ret.push(name);
                rem = new_rem;
            }
        } else if let Some(mut rem) = s.strip_prefix('{') {
            let mut ret = vec![];
            loop {
                if let Some(rem) = rem.trim().strip_prefix('}') {
                    return Ok((Self::Object(ret), rem));
                } else if rem.trim() == "" {
                    return Err(VimError::Expected("}"));
                }
                if let Some((idx, new_rem)) = s.split_once(':') {
                    let (name, new_rem) = Self::parse(new_rem)?;
                    ret.push((idx, name));
                    rem = new_rem;
                } else if let (Self::Single(name), new_rem) = Self::parse(rem)? {
                    ret.push((name, Self::Single(name)));
                    rem = new_rem;
                } else {
                    return Err(VimError::Expected(":"));
                }
            }
        } else if let Some(idx) = s.find(|c: char| !c.is_alphanumeric()) {
            Ok((Self::Single(&s[..idx]), &s[idx..]))
        } else {
            Err(VimError::Expected("in"))
        }
    }

    pub fn iter(
        &self,
        v: Value,
        f: &mut impl FnMut(&'a str, Value) -> Result<(), VimError>,
    ) -> Result<(), VimError> {
        match self {
            Self::Single(name) => f(name, v),
            Self::List(names) => {
                if let Value::List(vals) = v {
                    for (name, val) in names.iter().zip(vals.into_iter()) {
                        name.iter(val, f)?;
                    }
                    Ok(())
                } else {
                    Err(VimError::Expected("List"))
                }
            }
            Self::Object(names) => {
                if let Value::Object(mut vals) = v {
                    for (idx, name) in names.iter() {
                        name.iter(vals.remove(*idx).unwrap_or(Value::Nil), f)?;
                    }
                    Ok(())
                } else {
                    Err(VimError::Expected("Object"))
                }
            }
        }
    }
}
