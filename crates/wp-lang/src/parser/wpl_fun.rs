use std::net::IpAddr;

use winnow::{
    Parser,
    ascii::{digit1, multispace0},
    combinator::alt,
};
use wp_parser::{
    WResult,
    fun::{fun_trait::Fun0Builder, parser::call_fun_args0},
};
use wp_parser::{
    atom::take_path,
    fun::{
        fun_trait::{Fun1Builder, Fun2Builder, ParseNext},
        parser::{call_fun_args1, call_fun_args2, take_arr},
    },
};

use crate::ast::{
    WplFun,
    processor::{
        ExistsChars, FCharsHas, FCharsIn, FCharsNotHas, FDigitHas, FDigitIn, FIpAddrIn, FdHas,
        SelectLast, TakeField,
    },
};

use super::utils::take_key;

pub fn wpl_fun(input: &mut &str) -> WResult<WplFun> {
    multispace0.parse_next(input)?;
    let fun = alt((
        call_fun_args1::<TakeField>.map(WplFun::Take),
        call_fun_args0::<SelectLast>.map(WplFun::Last),
        call_fun_args2::<FCharsHas>.map(WplFun::FCharsExists),
        call_fun_args1::<CharsHasAlias>.map(|alias| {
            WplFun::FCharsExists(FCharsHas {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args2::<FCharsNotHas>.map(WplFun::FCharsNotExists),
        call_fun_args1::<CharsNotHasAlias>.map(|alias| {
            WplFun::FCharsNotExists(FCharsNotHas {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args2::<FCharsIn>.map(WplFun::FCharsIn),
        call_fun_args1::<CharsInAlias>.map(|alias| {
            WplFun::FCharsIn(FCharsIn {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args2::<FDigitHas>.map(WplFun::FDigitExists),
        call_fun_args1::<DigitHasAlias>.map(|alias| {
            WplFun::FDigitExists(FDigitHas {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args2::<FDigitIn>.map(WplFun::FDigitIn),
        call_fun_args1::<DigitInAlias>.map(|alias| {
            WplFun::FDigitIn(FDigitIn {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args2::<FIpAddrIn>.map(WplFun::FIpAddrIn),
        call_fun_args1::<IpInAlias>.map(|alias| {
            WplFun::FIpAddrIn(FIpAddrIn {
                target: None,
                value: alias.value,
            })
        }),
        call_fun_args1::<FdHas>.map(WplFun::FExists),
        call_fun_args0::<HasAlias>.map(|_| WplFun::FExists(FdHas { found: None })),
        call_fun_args0::<JsonUnescape>.map(WplFun::CUnescape),
        call_fun_args0::<Base64Decode>.map(WplFun::CBase64Decode),
    ))
    .parse_next(input)?;
    Ok(fun)
}

impl Fun2Builder for FDigitHas {
    type ARG1 = String;
    type ARG2 = i64;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }
    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        multispace0.parse_next(data)?;
        let val = digit1.parse_next(data)?;
        Ok(val.parse::<i64>().unwrap_or(0))
    }

    fn fun_name() -> &'static str {
        "f_digit_has"
    }

    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        Self {
            target: normalize_target(args.0),
            value: args.1,
        }
    }
}

impl Fun1Builder for CharsHasAlias {
    type ARG1 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_path.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "chars_has"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { value: args }
    }
}

impl Fun1Builder for CharsNotHasAlias {
    type ARG1 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_path.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "chars_not_has"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { value: args }
    }
}

impl Fun1Builder for CharsInAlias {
    type ARG1 = Vec<ExistsChars>;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        take_arr::<ExistsChars>(data)
    }

    fn fun_name() -> &'static str {
        "chars_in"
    }

    fn build(args: Self::ARG1) -> Self {
        let value = args.iter().map(|i| i.0.clone()).collect();
        Self { value }
    }
}

impl Fun1Builder for DigitHasAlias {
    type ARG1 = i64;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = digit1.parse_next(data)?;
        Ok(val.parse::<i64>().unwrap_or(0))
    }

    fn fun_name() -> &'static str {
        "digit_has"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { value: args }
    }
}

impl Fun1Builder for DigitInAlias {
    type ARG1 = Vec<i64>;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        take_arr::<i64>(data)
    }

    fn fun_name() -> &'static str {
        "digit_in"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { value: args }
    }
}

impl Fun1Builder for IpInAlias {
    type ARG1 = Vec<IpAddr>;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        take_arr::<IpAddr>(data)
    }

    fn fun_name() -> &'static str {
        "ip_in"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { value: args }
    }
}

impl Fun0Builder for HasAlias {
    fn fun_name() -> &'static str {
        "has"
    }

    fn build() -> Self {
        HasAlias
    }
}
impl Fun2Builder for FCharsHas {
    type ARG1 = String;
    type ARG2 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }
    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        multispace0.parse_next(data)?;
        let val = take_path.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "f_chars_has"
    }
    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        Self {
            target: normalize_target(args.0),
            value: args.1,
        }
    }
}

impl Fun2Builder for FCharsNotHas {
    type ARG1 = String;
    type ARG2 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }
    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        multispace0.parse_next(data)?;
        let val = take_path.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "f_chars_not_has"
    }
    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        Self {
            target: normalize_target(args.0),
            value: args.1,
        }
    }
}

impl ParseNext<ExistsChars> for ExistsChars {
    fn parse_next(input: &mut &str) -> WResult<ExistsChars> {
        let val = take_path.parse_next(input)?;
        Ok(ExistsChars(val.to_string()))
    }
}
impl Fun2Builder for FCharsIn {
    type ARG1 = String;
    type ARG2 = Vec<ExistsChars>;
    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }

    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        take_arr::<ExistsChars>(data)
    }

    fn fun_name() -> &'static str {
        "f_chars_in"
    }

    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        let value: Vec<String> = args.1.iter().map(|i| i.0.clone()).collect();
        Self {
            target: normalize_target(args.0),
            value,
        }
    }
}

impl Fun2Builder for FDigitIn {
    type ARG1 = String;
    type ARG2 = Vec<i64>;

    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        take_arr::<i64>(data)
    }
    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "f_digit_in"
    }
    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        Self {
            target: normalize_target(args.0),
            value: args.1,
        }
    }
}
impl Fun1Builder for FdHas {
    type ARG1 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "f_has"
    }

    fn build(args: Self::ARG1) -> Self {
        Self {
            found: normalize_target(args),
        }
    }
}

impl Fun2Builder for FIpAddrIn {
    type ARG1 = String;
    type ARG2 = Vec<IpAddr>;

    fn args2(data: &mut &str) -> WResult<Self::ARG2> {
        take_arr::<IpAddr>(data)
    }
    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "f_ip_in"
    }
    fn build(args: (Self::ARG1, Self::ARG2)) -> Self {
        Self {
            target: normalize_target(args.0),
            value: args.1,
        }
    }
}

// ---------------- String Mode ----------------
use crate::ast::processor::JsonUnescape;

impl Fun0Builder for JsonUnescape {
    fn fun_name() -> &'static str {
        "json_unescape"
    }

    fn build() -> Self {
        JsonUnescape {}
    }
}

use crate::ast::processor::Base64Decode;
impl Fun0Builder for Base64Decode {
    fn fun_name() -> &'static str {
        "base64_decode"
    }

    fn build() -> Self {
        Base64Decode {}
    }
}

#[derive(Clone, Debug, PartialEq)]
struct CharsHasAlias {
    value: String,
}

#[derive(Clone, Debug, PartialEq)]
struct CharsNotHasAlias {
    value: String,
}

#[derive(Clone, Debug, PartialEq)]
struct CharsInAlias {
    value: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct DigitHasAlias {
    value: i64,
}

#[derive(Clone, Debug, PartialEq)]
struct DigitInAlias {
    value: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
struct IpInAlias {
    value: Vec<IpAddr>,
}

#[derive(Clone, Debug, PartialEq)]
struct HasAlias;

fn normalize_target(target: String) -> Option<String> {
    if target == "_" { None } else { Some(target) }
}

impl Fun1Builder for TakeField {
    type ARG1 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        multispace0.parse_next(data)?;
        let val = take_key.parse_next(data)?;
        Ok(val.to_string())
    }

    fn fun_name() -> &'static str {
        "take"
    }

    fn build(args: Self::ARG1) -> Self {
        Self { target: args }
    }
}

impl Fun0Builder for SelectLast {
    fn fun_name() -> &'static str {
        "last"
    }

    fn build() -> Self {
        SelectLast {}
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use orion_error::TestAssert;

    use crate::ast::processor::{FdHas, JsonUnescape, SelectLast, TakeField};

    use super::*;

    #[test]
    fn test_parse_fun() {
        let fun = wpl_fun.parse(r#"f_has(src)"#).assert();
        assert_eq!(
            fun,
            WplFun::FExists(FdHas {
                found: Some("src".to_string())
            })
        );

        let fun = wpl_fun.parse("has()").assert();
        assert_eq!(fun, WplFun::FExists(FdHas { found: None }));

        let fun = wpl_fun.parse(r#"f_digit_in(src, [1,2,3])"#).assert();
        assert_eq!(
            fun,
            WplFun::FDigitIn(FDigitIn {
                target: Some("src".to_string()),
                value: vec![1, 2, 3]
            })
        );

        let fun = wpl_fun.parse("digit_has(42)").assert();
        assert_eq!(
            fun,
            WplFun::FDigitExists(FDigitHas {
                target: None,
                value: 42,
            })
        );

        let fun = wpl_fun.parse("digit_in([4,5])").assert();
        assert_eq!(
            fun,
            WplFun::FDigitIn(FDigitIn {
                target: None,
                value: vec![4, 5],
            })
        );

        let fun = wpl_fun
            .parse(r#"f_ip_in(src, [127.0.0.1, 127.0.0.2])"#)
            .assert();
        assert_eq!(
            fun,
            WplFun::FIpAddrIn(FIpAddrIn {
                target: Some("src".to_string()),
                value: vec![
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))
                ]
            })
        );

        let fun = wpl_fun.parse(r#"ip_in([127.0.0.1,::1])"#).assert();
        assert_eq!(
            fun,
            WplFun::FIpAddrIn(FIpAddrIn {
                target: None,
                value: vec![
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    IpAddr::V6(Ipv6Addr::LOCALHOST),
                ],
            })
        );

        // IPv6 裸字面量与混合示例
        let fun = wpl_fun
            .parse(r#"f_ip_in(src, [::1, 2001:db8::1])"#)
            .assert();
        assert_eq!(
            fun,
            WplFun::FIpAddrIn(FIpAddrIn {
                target: Some("src".to_string()),
                value: vec![
                    IpAddr::V6(Ipv6Addr::LOCALHOST),
                    IpAddr::V6("2001:db8::1".parse().unwrap()),
                ]
            })
        );

        let fun = wpl_fun.parse("json_unescape()").assert();
        assert_eq!(fun, WplFun::CUnescape(JsonUnescape {}));

        assert!(wpl_fun.parse("json_unescape(decoded)").is_err());

        let fun = wpl_fun.parse("take(src)").assert();
        assert_eq!(
            fun,
            WplFun::Take(TakeField {
                target: "src".to_string(),
            })
        );

        let fun = wpl_fun.parse("last()").assert();
        assert_eq!(fun, WplFun::Last(SelectLast {}));

        let fun = wpl_fun.parse("f_chars_has(_, foo)").assert();
        assert_eq!(
            fun,
            WplFun::FCharsExists(FCharsHas {
                target: None,
                value: "foo".to_string(),
            })
        );

        let fun = wpl_fun.parse("chars_has(bar)").assert();
        assert_eq!(
            fun,
            WplFun::FCharsExists(FCharsHas {
                target: None,
                value: "bar".to_string(),
            })
        );

        let fun = wpl_fun.parse("chars_not_has(baz)").assert();
        assert_eq!(
            fun,
            WplFun::FCharsNotExists(FCharsNotHas {
                target: None,
                value: "baz".to_string(),
            })
        );

        let fun = wpl_fun.parse("chars_in([foo,bar])").assert();
        assert_eq!(
            fun,
            WplFun::FCharsIn(FCharsIn {
                target: None,
                value: vec!["foo".to_string(), "bar".to_string()],
            })
        );

        let fun = wpl_fun.parse("base64_decode()").assert();
        assert_eq!(fun, WplFun::CBase64Decode(Base64Decode {}));
        assert!(wpl_fun.parse("base64_decode(decoded)").is_err());
    }
}
