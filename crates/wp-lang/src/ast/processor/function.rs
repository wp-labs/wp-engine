use std::net::IpAddr;

#[derive(Clone, Debug, PartialEq)]
pub struct ExistsChars(pub(crate) String);
#[derive(Clone, Debug, PartialEq)]
pub struct FdHas {
    pub(crate) found: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsHas {
    pub(crate) target: String,
    pub(crate) value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsNotHas {
    pub(crate) target: String,
    pub(crate) value: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FDigitHas {
    pub(crate) target: String,
    pub(crate) value: i64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FDigitIn {
    pub(crate) target: String,
    pub(crate) value: Vec<i64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FCharsIn {
    pub(crate) target: String,
    pub(crate) value: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FIpAddrIn {
    pub(crate) target: String,
    pub(crate) value: Vec<IpAddr>,
}

impl FdHas {
    pub fn new<S: Into<String>>(found: S) -> Self {
        Self {
            found: found.into(),
        }
    }
}

#[derive(Clone, Default)]
#[allow(dead_code)]
pub struct StubFun {}

#[derive(Clone, Debug, PartialEq)]
pub struct CharsDecode {
    pub(crate) mode: String,
}
