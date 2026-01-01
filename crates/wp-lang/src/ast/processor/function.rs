use std::net::IpAddr;

#[derive(Clone, Debug, PartialEq)]
pub struct ExistsChars(pub(crate) String);
#[derive(Clone, Debug, PartialEq)]
pub struct FdHas {
    pub(crate) found: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsHas {
    pub(crate) target: Option<String>,
    pub(crate) value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsNotHas {
    pub(crate) target: Option<String>,
    pub(crate) value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FDigitHas {
    pub(crate) target: Option<String>,
    pub(crate) value: i64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FDigitIn {
    pub(crate) target: Option<String>,
    pub(crate) value: Vec<i64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsIn {
    pub(crate) target: Option<String>,
    pub(crate) value: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FIpAddrIn {
    pub(crate) target: Option<String>,
    pub(crate) value: Vec<IpAddr>,
}

impl FdHas {
    pub fn new<S: Into<String>>(found: S) -> Self {
        Self {
            found: Some(found.into()),
        }
    }
}

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct StubFun {}

#[derive(Clone, Debug, PartialEq)]
pub struct JsonUnescape {}

#[derive(Clone, Debug, PartialEq)]
pub struct Base64Decode {}

#[derive(Clone, Debug, PartialEq)]
pub struct TakeField {
    pub(crate) target: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectLast {}
