//
// value.rs
// Copyright (C) 2022 matthew <matthew@matthew-VirtualBox>
// Distributed under terms of the MIT license.
//

use crate::expr::ValueError;
use crate::namespace::IdDisplay;
use crate::BuiltinFunction;
use crate::Id;
use crate::LineOwned;
use crate::RunTy;
use crate::Section;
use crate::State;
use crate::Tokenizer;
use crate::VimError;
use crate::VimScriptCtx;
use crate::namespace::NameSpaced;
use std::borrow::Cow;
use std::collections::hash_map;
use std::collections::linked_list;
use std::collections::{HashMap, LinkedList};
use std::fmt::Display;
use std::ops::Deref;
use std::str::pattern::Pattern;
use std::sync::Arc;
use std::sync::Mutex;
use std::vec;

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

#[derive(Debug, Clone)]
pub enum ValueRef<'a> {
    Integer(isize),
    Number(f64),
    Str(Cow<'a, str>),
    Bool(bool),
    Object(Arc<Mutex<HashMap<String, Value>>>),
    List(Arc<Mutex<Vec<Value>>>),
    Function(Option<Id>, Cow<'a, str>),
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
            Self::Function(id, name) => write!(f, "<Function@{}{}>", IdDisplay(*id), name),
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
            ValueRef::Function(id, v) => Self::Function(id, v.to_string()),
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
    Blob,
}

impl VimType {
    pub fn as_int(&self) -> isize {
        match self {
            VimType::Integer => 0,
            VimType::Str => 1,
            VimType::Function => 2,
            VimType::List => 3,
            VimType::Object => 4,
            VimType::Number => 5,
            VimType::Bool => 6,
            VimType::Nil => 7,
            VimType::Blob => 10,
        }
    }

    pub fn ty_names(ctx: &mut NameSpaced<Value>) {
        ctx.insert_builtin("v:t_number", Value::Integer(Self::Integer.as_int()));
        ctx.insert_builtin("v:t_string", Value::Integer(Self::Str.as_int()));
        ctx.insert_builtin("v:t_func", Value::Integer(Self::Function.as_int()));
        ctx.insert_builtin("v:t_list", Value::Integer(Self::List.as_int()));
        ctx.insert_builtin("v:t_dict", Value::Integer(Self::Object.as_int()));
        ctx.insert_builtin("v:t_float", Value::Integer(Self::Number.as_int()));
        ctx.insert_builtin("v:t_bool", Value::Integer(Self::Bool.as_int()));
        ctx.insert_builtin("v:t_null", Value::Integer(Self::Nil.as_int()));
        ctx.insert_builtin("v:t_blob", Value::Integer(Self::Blob.as_int()));
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(isize),
    Number(f64),
    Str(String),
    Bool(bool),
    Object(Arc<Mutex<HashMap<String, Value>>>),
    List(Arc<Mutex<Vec<Value>>>),
    Function(Option<Id>, String),
    Nil,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Number(l), Value::Number(r)) => l == r,
            (Value::Str(l), Value::Str(r)) => l == r,
            (Value::Bool(l), Value::Bool(r)) => l == r,
            (Value::Object(l), Value::Object(r)) => {
                l.lock().unwrap().deref() == r.lock().unwrap().deref()
            }
            (Value::List(l), Value::List(r)) => {
                l.lock().unwrap().deref() == r.lock().unwrap().deref()
            }
            (Value::Function(li, l), Value::Function(ri, r)) => l == r && li == ri,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
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
        Self::List(Arc::new(Mutex::new(
            l.into_iter().map(|s| s.into()).collect(),
        )))
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

    pub fn nil_or<R: Into<Result<Self, VimError>>>(self, f: impl Fn() -> R) -> Result<Self, VimError> {
        match self {
            Self::Nil => f().into(),
            v => Ok(v),
        }
    }

    pub fn is_nil(&self) -> bool {
        match self {
            Self::Nil => true,
            _ => false,
        }
    }

    pub fn ty(&self) -> VimType {
        match self {
            Value::Integer(_) => VimType::Integer,
            Value::Number(_) => VimType::Number,
            Value::Str(_) => VimType::Str,
            Value::Bool(_) => VimType::Bool,
            Value::Object(_) => VimType::Object,
            Value::List(_) => VimType::List,
            Value::Function(_, _) => VimType::Function,
            Value::Nil => VimType::Nil,
        }
    }

    pub fn to_bool<S: State + 'static>(&self, ctx: &VimScriptCtx<S>) -> Result<bool, VimError> {
        Ok(match self {
            Value::Integer(i) => *i != 0,
            Value::Number(n) => *n != 0.,
            Value::Str(s) => !s.is_empty(),
            Value::Bool(b) => *b,
            Value::Object(o) => !o.lock().unwrap().is_empty(),
            Value::List(l) => !l.lock().unwrap().is_empty(),
            Value::Function(id, f) => ctx.get_func(*id, f).is_some(),
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
                    o.lock()
                        .unwrap()
                        .iter()
                        .flat_map(|(n, v)| {
                            [n.clone(), ":".to_string(), v.to_string(ctx)].into_iter()
                        })
                        .intersperse(",".to_string()),
                )
                .chain(std::iter::once("}".to_string()))
                .collect(),
            Value::List(l) => std::iter::once("[".to_string())
                .chain(
                    l.lock()
                        .unwrap()
                        .iter()
                        .map(|v| v.to_string(ctx))
                        .intersperse(",".to_string()),
                )
                .chain(std::iter::once("]".to_string()))
                .collect(),
            Value::Function(_id, f) => f.clone(),
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
            Value::Function(_id, _f) => todo!(),
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
            Value::Function(_id, _f) => todo!(),
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

    // pub fn get_obj<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<&HashMap<String, Value>> {
    //     match self {
    //         Value::Object(o) => Some(o),
    //         _ => None,
    //     }
    // }
    //
    // pub fn get_list<S>(&self, _ctx: &VimScriptCtx<S>) -> Option<&LinkedList<Value>> {
    //     match self {
    //         Value::List(o) => Some(o),
    //         _ => None,
    //     }
    // }

    pub fn get_func<'a, S: State + 'static>(
        &self,
        ctx: &'a VimScriptCtx<S>,
    ) -> Option<&'a Function<S>> {
        match self {
            Value::Function(id, f) => ctx.get_func(*id, f),
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
                    l.lock()
                        .unwrap()
                        .iter()
                        .rev()
                        .nth((1 - idx) as usize)
                        .unwrap_or(&Self::Nil)
                        .clone()
                } else {
                    l.lock()
                        .unwrap()
                        .iter()
                        .nth(idx as usize)
                        .unwrap_or(&Self::Nil)
                        .clone()
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
            Self::Object(m) => m
                .lock()
                .unwrap()
                .get(&idx.to_string(ctx))
                .unwrap_or(&Self::Nil)
                .clone(),
            _ => todo!(),
        })
    }

    pub fn len(&self) -> Result<Self, VimError> {
        match self {
            Self::List(l) => Ok(Self::Integer(l.lock().unwrap().len() as isize)),
            Self::Object(l) => Ok(Self::Integer(l.lock().unwrap().len() as isize)),
            Self::Str(l) => Ok(Self::Integer(l.len() as isize)),
            Self::Integer(_) => Ok(Self::Integer(std::mem::size_of::<isize>() as isize)),
            Self::Number(_) => Ok(Self::Integer(std::mem::size_of::<f64>() as isize)),
            _ => Err(VimError::ExpectedType(VimType::Object)),
        }
    }

    pub fn empty(&self) -> Result<Self, VimError> {
        match self {
            Self::List(l) => Ok(Self::Bool(l.lock().unwrap().is_empty())),
            Self::Object(l) => Ok(Self::Bool(l.lock().unwrap().is_empty())),
            Self::Str(l) => Ok(Self::Bool(l.is_empty())),
            Self::Integer(l) => Ok(Self::Bool(*l == 0)),
            Self::Number(l) => Ok(Self::Bool(*l == 0.)),
            Self::Bool(l) => Ok(Self::Bool(*l == false)),
            Self::Nil => Ok(Self::Bool(true)),
            _ => Err(VimError::ExpectedType(VimType::Object)),
        }
    }

    pub fn insert<S>(&self, index: Self, element: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        match self {
            Self::List(l) => {
                let index = index.to_int(ctx)?;
                if element.contains(l) {
                    Err(VimError::IllegalArgument("List cannot contain itself"))
                } else {
                    let mut l = l.lock().unwrap();
                    if index >= 0 {
                        l.insert(index as usize, element);
                    } else {
                        let max = l.len();
                        l.insert(max + ((-index) as usize), element);
                    }
                    Ok(Self::Nil)
                }
            }
            _ => Err(VimError::ExpectedType(VimType::Object)),
        }
    }

    /// Checks if this Value already contains the provided Arc. This is only possible if the Arc is
    /// a HashMap or Vec, but the inner logic doesn't actually care. Equality checking is done by
    /// comparing the underlying Arc pointers. This is safe, since we do not derefence the
    /// pointers, we just inspect their bits.
    fn contains<T>(&self, list: &Arc<T>) -> bool {
        match self {
            Self::Object(v) => {
                if Arc::as_ptr(v).to_bits() == Arc::as_ptr(list).to_bits() {
                    true
                } else {
                    for (_, inner) in v.lock().unwrap().iter() {
                        if inner.contains(list) {
                            return true;
                        }
                    }
                    false
                }
            }
            Self::List(v) => {
                if Arc::as_ptr(v).to_bits() == Arc::as_ptr(list).to_bits() {
                    true
                } else {
                    for inner in v.lock().unwrap().iter() {
                        if inner.contains(list) {
                            return true;
                        }
                    }
                    false
                }
            }
            _ => false,
        }
    }

    pub fn extend<S>(&self, list: Self, index: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        match (self, list) {
            (Self::List(l), Self::List(r)) => {
                let index = index.nil_or(|| Self::Integer(r.lock().unwrap().len() as isize))?.to_int(ctx);
                if self.contains(&r) {
                    Err(VimError::IllegalArgument("List cannot contain itself"))
                } else {
                    todo!("Append list")
                }
            }
            (Self::Object(l), Self::Object(r)) => {
                if index.contains_str("keep") {
                    todo!("Append objects")
                } else if index.contains_str("force") || index.is_nil() {
                    todo!("Append objects")
                } else if index.contains_str("error") {
                    todo!("Append objects")
                } else {
                    Err(VimError::IllegalArgument("Extend arg must be `keep`, `force`, or `error`"))
                }
            }
            _ => Err(VimError::ExpectedType(VimType::List)),
        }
    }

    pub fn remove<S>(&self, index: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!()
    }

    pub fn deep_copy(&self) -> Self {
        match self {
            Self::List(l) => Self::List(Arc::new(Mutex::new(
                l.lock().unwrap().iter().map(|v| v.deep_copy()).collect(),
            ))),
            Self::Object(l) => Self::Object(Arc::new(Mutex::new(
                l.lock()
                    .unwrap()
                    .iter()
                    .map(|(n, v)| (n.clone(), v.deep_copy()))
                    .collect(),
            ))),
            other => other.clone(),
        }
    }

    pub fn starts_with<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        match self {
            Self::Str(s) => s.starts_with(pat),
            _ => false,
        }
    }

    pub fn contains_str<'a, P: Pattern<'a>>(&'a self, pat: P) -> bool {
        match self {
            Self::Str(s) => s.strip_prefix(pat) == Some(""),
            _ => false,
        }
    }

    pub fn items<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("Items")
    }

    pub fn values<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("Values")
    }

    pub fn keys<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("Keys")
    }

    pub fn has_key<S>(&self, key: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("has_key")
    }

    pub fn flatten<S>(&self, max_depth: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("flatten")
    }

    pub fn repeat<S>(&self, times: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("repeat")
    }

    pub fn count<S>(&self, val: Self, c: Self, d: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("count")
    }

    pub fn min<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("min")
    }

    pub fn max<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("max")
    }

    pub fn call<S>(&self, args: Self, dict: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("call")
    }

    pub fn join<S>(&self, seperator: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("join")
    }

    pub fn range<S>(&self, end: Self, stride: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("range")
    }

    pub fn split<S>(&self, pattern: Self, stride: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("split")
    }

    pub fn unique<S>(&self, b: Self, c: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("unique")
    }

    pub fn reverse<S>(&self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("reverse")
    }

    pub fn sort<S>(&self, b: Self, c: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("sort")
    }

    pub fn map<S>(&self, b: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("map")
    }

    pub fn filter<S>(&self, b: Self, ctx: &VimScriptCtx<S>) -> Result<Self, VimError> {
        todo!("filter")
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
            Self::List(l) => ValueIter::List(l.lock().unwrap().clone().into_iter()),
            Self::Object(m) => ValueIter::Object(m.lock().unwrap().clone().into_iter()),
            Self::Str(s) => ValueIter::Str(s, 0),
            _ => ValueIter::Empty,
        }
    }
}

pub enum ValueIter {
    Empty,
    List(vec::IntoIter<Value>),
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
            Value::List(l) => {
                write!(f, "[")?;
                for el in l.lock().unwrap().iter() {
                    write!(f, "{el},")?;
                }
                write!(f, "]")
            }
            Value::Function(id, name) => write!(f, "<Function@{}{}>", IdDisplay(*id), name),
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
                    let mut vals = vals.lock().unwrap();
                    for (name, val) in names.iter().zip(vals.clone().into_iter()) {
                        name.iter(val, f)?;
                    }
                    Ok(())
                } else {
                    Err(VimError::Expected("List"))
                }
            }
            Self::Object(names) => {
                if let Value::Object(mut vals) = v {
                    let mut vals = vals.lock().unwrap();
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
