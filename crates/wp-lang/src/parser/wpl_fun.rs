use std::net::IpAddr;

use winnow::{
    Parser,
    ascii::{digit1, multispace0},
    combinator::alt,
};
use wp_parser::WResult;
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
    },
};

use super::utils::take_key;

pub fn wpl_fun(input: &mut &str) -> WResult<WplFun> {
    multispace0.parse_next(input)?;
    let fun = alt((
        call_fun_args2::<FCharsHas>.map(WplFun::FCharsExists),
        call_fun_args2::<FCharsNotHas>.map(WplFun::FCharsNotExists),
        call_fun_args2::<FCharsIn>.map(WplFun::FCharsIn),
        call_fun_args2::<FDigitHas>.map(WplFun::FDigitExists),
        call_fun_args2::<FDigitIn>.map(WplFun::FDigitIn),
        call_fun_args2::<FIpAddrIn>.map(WplFun::FIpAddrIn),
        call_fun_args1::<FdHas>.map(WplFun::FExists),
        call_fun_args1::<CharsDecode>.map(WplFun::CDecode),
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
            target: args.0,
            value: args.1,
        }
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
            target: args.0,
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
            target: args.0,
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
            target: args.0,
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
            target: args.0,
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
        Self { found: args }
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
            target: args.0,
            value: args.1,
        }
    }
}

// ---------------- String Mode ----------------
use crate::ast::processor::CharsDecode;

impl Fun1Builder for CharsDecode {
    type ARG1 = String;

    fn args1(data: &mut &str) -> WResult<Self::ARG1> {
        use winnow::token::take_till;
        multispace0.parse_next(data)?;
        let val = take_till(1.., |c: char| c == ')' || c == ',').parse_next(data)?;
        Ok(val.trim().to_string())
    }

    fn fun_name() -> &'static str {
        "chars_unescape"
    }

    fn build(args: Self::ARG1) -> Self {
        CharsDecode { mode: args }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use orion_error::TestAssert;

    use crate::ast::processor::FdHas;

    use super::*;

    #[test]
    fn test_parse_fun() {
        let fun = wpl_fun.parse(r#"f_has(src)"#).assert();
        assert_eq!(
            fun,
            WplFun::FExists(FdHas {
                found: "src".to_string()
            })
        );

        let fun = wpl_fun.parse(r#"f_digit_in(src, [1,2,3])"#).assert();
        assert_eq!(
            fun,
            WplFun::FDigitIn(FDigitIn {
                target: "src".to_string(),
                value: vec![1, 2, 3]
            })
        );

        let fun = wpl_fun
            .parse(r#"f_ip_in(src, [127.0.0.1, 127.0.0.2])"#)
            .assert();
        assert_eq!(
            fun,
            WplFun::FIpAddrIn(FIpAddrIn {
                target: "src".to_string(),
                value: vec![
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                    IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))
                ]
            })
        );

        // IPv6 裸字面量与混合示例
        let fun = wpl_fun
            .parse(r#"f_ip_in(src, [::1, 2001:db8::1])"#)
            .assert();
        assert_eq!(
            fun,
            WplFun::FIpAddrIn(FIpAddrIn {
                target: "src".to_string(),
                value: vec![
                    IpAddr::V6(Ipv6Addr::LOCALHOST),
                    IpAddr::V6("2001:db8::1".parse().unwrap()),
                ]
            })
        );
    }
}
