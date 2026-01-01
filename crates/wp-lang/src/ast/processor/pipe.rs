use derive_getters::Getters;

use super::function::{
    FCharsHas, FCharsIn, FCharsNotHas, FDigitHas, FDigitIn, FIpAddrIn, FdHas, JsonUnescape,
    SelectLast, TakeField,
};
use crate::ast::{group::WplGroup, processor::Base64Decode};

#[derive(Debug, Clone, PartialEq)]
pub enum WplFun {
    Take(TakeField),
    Last(SelectLast),
    FCharsExists(FCharsHas),
    FCharsNotExists(FCharsNotHas),
    FCharsIn(FCharsIn),
    FDigitExists(FDigitHas),
    FDigitIn(FDigitIn),
    FIpAddrIn(FIpAddrIn),
    FExists(FdHas),
    CUnescape(JsonUnescape),
    CBase64Decode(Base64Decode),
}

#[derive(Debug, Clone, PartialEq, Getters)]
#[allow(dead_code)]
pub struct FunArg0 {
    name: String,
}
impl<S> From<S> for FunArg0
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        Self { name: value.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Getters)]
#[allow(dead_code)]
pub struct FunArg1 {
    name: String,
    arg1: String,
}

impl<S> From<(S, S)> for FunArg1
where
    S: Into<String>,
{
    fn from(value: (S, S)) -> Self {
        Self {
            name: value.0.into(),
            arg1: value.1.into(),
        }
    }
}

impl<S> From<(S, Option<S>)> for FunArg1
where
    S: Into<String>,
{
    fn from(value: (S, Option<S>)) -> Self {
        Self {
            name: value.0.into(),
            arg1: value.1.map(|f| f.into()).unwrap_or("_".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Getters)]
#[allow(dead_code)]
pub struct FunArg2 {
    name: String,
    arg1: String,
    arg2: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WplPipe {
    Fun(WplFun),
    Group(WplGroup),
}

impl<S> From<(S, S, S)> for FunArg2
where
    S: Into<String>,
{
    fn from(value: (S, S, S)) -> Self {
        Self {
            name: value.0.into(),
            arg1: value.1.into(),
            arg2: value.2.into(),
        }
    }
}
