use crate::ast::WplFun;
use crate::ast::processor::{
    Base64Decode, FCharsHas, FCharsIn, FCharsNotHas, FDigitHas, FDigitIn, FIpAddrIn, FdHas, StubFun,
};
use crate::eval::runtime::field_pipe::{FieldIndex, FiledSetProcessor};
use base64::Engine;
use base64::engine::general_purpose;
use winnow::combinator::fail;
use wp_model_core::model::{DataField, Value};
use wp_parser::Parser;
use wp_parser::WResult;
use wp_parser::symbol::ctx_desc;

impl FiledSetProcessor for FCharsHas {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
        {
            if let Some(v) = value.get(i)
                && let Value::Chars(s) = v.get_value()
                && *s == self.value
            {
                return Ok(());
            }
            return fail
                .context(ctx_desc("<pipe> | not exists"))
                .parse_next(&mut "");
        }
        for v in value.iter() {
            if v.get_name() == target
                && let Value::Chars(v) = v.get_value()
                && *v == self.value
            {
                return Ok(());
            }
        }
        fail.context(ctx_desc("<pipe> | not exists"))
            .parse_next(&mut "")
    }
}

impl FiledSetProcessor for FdHas {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.found.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
            && value.get(i).is_some()
        {
            return Ok(());
        }
        for v in value.iter() {
            if v.get_name() == target {
                return Ok(());
            }
        }
        fail.context(ctx_desc("json not exists sub item"))
            .parse_next(&mut "")
    }
}
impl FiledSetProcessor for FCharsNotHas {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        let mut is_exists = false;
        if let Some(ix) = index {
            if let Some(i) = ix.get(target)
                && let Some(v) = value.get(i)
            {
                is_exists = true;
                if let Value::Chars(s) = v.get_value()
                    && *s != self.value
                {
                    return Ok(());
                }
            }
        } else {
            for v in value.iter() {
                if v.get_name() == target {
                    is_exists = true;
                    if let Value::Chars(vs) = v.get_value()
                        && *vs != self.value
                    {
                        return Ok(());
                    }
                }
            }
        }
        if !is_exists {
            return Ok(());
        }
        fail.context(ctx_desc("<pipe> | not exists"))
            .parse_next(&mut "")
    }
}

impl FiledSetProcessor for FDigitHas {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
        {
            if let Some(v) = value.get(i)
                && let Value::Digit(d) = v.get_value()
                && *d == self.value
            {
                return Ok(());
            }
            return fail
                .context(ctx_desc("<pipe> | not exists"))
                .parse_next(&mut "");
        }
        for v in value.iter() {
            if v.get_name() == target
                && let Value::Digit(d) = v.get_value()
                && *d == self.value
            {
                return Ok(());
            }
        }
        fail.context(ctx_desc("<pipe> | not exists"))
            .parse_next(&mut "")
    }
}

impl FiledSetProcessor for FDigitIn {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
        {
            if let Some(v) = value.get(i)
                && let Value::Digit(d) = v.get_value()
                && self.value.contains(d)
            {
                return Ok(());
            }
            return fail
                .context(ctx_desc("<pipe> | not in"))
                .parse_next(&mut "");
        }
        for v in value.iter() {
            if v.get_name() == target
                && let Value::Digit(d) = v.get_value()
                && self.value.contains(d)
            {
                return Ok(());
            }
        }
        fail.context(ctx_desc("<pipe> | not in"))
            .parse_next(&mut "")
    }
}

impl FiledSetProcessor for FCharsIn {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
        {
            if let Some(v) = value.get(i)
                && let Value::Chars(s) = v.get_value()
                && self.value.contains(s)
            {
                return Ok(());
            }
            return fail
                .context(ctx_desc("<pipe> | not in"))
                .parse_next(&mut "");
        }
        for v in value.iter() {
            if v.get_name() == target
                && let Value::Chars(s) = v.get_value()
                && self.value.contains(s)
            {
                return Ok(());
            }
        }
        fail.context(ctx_desc("<pipe> | not in"))
            .parse_next(&mut "")
    }
}

impl FiledSetProcessor for FIpAddrIn {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        let target = self.target.as_str();
        if let Some(ix) = index
            && let Some(i) = ix.get(target)
        {
            if let Some(v) = value.get(i)
                && let Value::IpAddr(a) = v.get_value()
                && self.value.contains(a)
            {
                return Ok(());
            }
            return fail
                .context(ctx_desc("<pipe> | not in"))
                .parse_next(&mut "");
        }
        for v in value.iter() {
            if v.get_name() == target
                && let Value::IpAddr(a) = v.get_value()
                && self.value.contains(a)
            {
                return Ok(());
            }
        }
        fail.context(ctx_desc("<pipe> | not in"))
            .parse_next(&mut "")
    }
}
impl FiledSetProcessor for StubFun {
    #[inline]
    fn process(&self, _value: &mut Vec<DataField>, _index: Option<&FieldIndex>) -> WResult<()> {
        Ok(())
    }
}
impl FiledSetProcessor for WplFun {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, index: Option<&FieldIndex>) -> WResult<()> {
        match self {
            WplFun::FCharsExists(f) => f.process(value, index),
            WplFun::FCharsNotExists(f) => f.process(value, index),
            WplFun::FCharsIn(f) => f.process(value, index),
            WplFun::FDigitExists(f) => f.process(value, index),
            WplFun::FDigitIn(f) => f.process(value, index),
            WplFun::FIpAddrIn(f) => f.process(value, index),
            WplFun::FExists(f) => f.process(value, index),
            WplFun::CUnescape(f) => f.process(value, index),
            WplFun::CBase64Decode(f) => f.process(value, index),
        }
    }
}

// ---------------- String Mode ----------------
use crate::ast::processor::JsonUnescape;

#[inline]
fn decode_json_escapes(raw: &str) -> Option<String> {
    // 通过 serde_json 反转义 JSON 字符串：重新包裹引号再解析
    let quoted = format!("\"{}\"", raw);
    serde_json::from_str::<String>(&quoted).ok()
}

#[inline]
fn value_json_unescape(v: &mut Value) -> bool {
    if let Value::Chars(s) = v {
        // fast path: 没有反斜杠则无需反转义
        if !s.as_bytes().contains(&b'\\') {
            return true;
        }
        if let Some(decoded) = decode_json_escapes(s) {
            *s = decoded;
            return true;
        }
    }
    false
}

impl FiledSetProcessor for JsonUnescape {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, _index: Option<&FieldIndex>) -> WResult<()> {
        for f in value.iter_mut() {
            let v = f.get_value_mut();
            if !value_json_unescape(v) {
                return fail.context(ctx_desc("json_unescape")).parse_next(&mut "");
            }
        }
        Ok(())
    }
}

impl FiledSetProcessor for Base64Decode {
    #[inline]
    fn process(&self, value: &mut Vec<DataField>, _index: Option<&FieldIndex>) -> WResult<()> {
        for f in value.iter_mut() {
            let v = f.get_value_mut();
            if !value_base64_decode(v) {
                return fail.context(ctx_desc("base64_decode")).parse_next(&mut "");
            }
        }
        Ok(())
    }
}

#[inline]
fn value_base64_decode(v: &mut Value) -> bool {
    match v {
        Value::Chars(s) => {
            if let Ok(decoded) = general_purpose::STANDARD.decode(s.as_bytes())
                && let Ok(vstring) = String::from_utf8(decoded)
            {
                *s = vstring;
                return true;
            }
            false
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decode_successfully_rewrites_chars_field() {
        let encoded = general_purpose::STANDARD.encode("hello world");
        let mut fields = vec![DataField::from_chars("payload".to_string(), encoded)];
        Base64Decode {}
            .process(&mut fields, None)
            .expect("decode ok");
        if let Value::Chars(s) = fields[0].get_value() {
            assert_eq!(s, "hello world");
        } else {
            panic!("payload should remain chars");
        }
    }

    #[test]
    fn base64_decode_returns_err_on_invalid_payload() {
        let mut fields = vec![DataField::from_chars(
            "payload".to_string(),
            "***".to_string(),
        )];
        assert!(Base64Decode {}.process(&mut fields, None).is_err());
    }

    #[test]
    fn json_unescape_successfully_decodes_chars_field() {
        let mut fields = vec![DataField::from_chars(
            "txt".to_string(),
            r"line1\nline2".to_string(),
        )];
        JsonUnescape {}
            .process(&mut fields, None)
            .expect("decode ok");
        if let Value::Chars(s) = fields[0].get_value() {
            assert!(s.contains('\n'));
        } else {
            panic!("txt should stay chars");
        }
    }

    #[test]
    fn json_unescape_returns_err_on_invalid_escape() {
        let mut fields = vec![DataField::from_chars(
            "txt".to_string(),
            r"line1\qline2".to_string(),
        )];
        assert!(JsonUnescape {}.process(&mut fields, None).is_err());
    }
}
