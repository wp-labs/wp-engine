use crate::language::ArgsTakeAble;
use crate::language::PreciseEvaluator;
use crate::language::SqlQuery;
use crate::parser::keyword::{kw_sql_select, kw_sql_where};
#[cfg(test)]
use std::cell::Cell;
use std::env;
use std::sync::atomic::{AtomicI8, Ordering};
#[cfg(test)]
thread_local! { static STRICT_TL: Cell<i8> = const { Cell::new(0) }; }
use winnow::combinator::fail;
use winnow::error::StrContext;
use winnow::error::StrContextValue;
use winnow::token::take_until;
use wp_parser::Parser;
use wp_parser::WResult;

use super::cond::SCondParser;

// 0: no override; 1: force strict on; -1: force strict off
static STRICT_OVERRIDE: AtomicI8 = AtomicI8::new(0);

fn is_sql_strict() -> bool {
    // test-thread override takes highest priority
    #[cfg(test)]
    {
        let v = STRICT_TL.with(|c| c.get());
        if v == 1 {
            return true;
        }
        if v == -1 {
            return false;
        }
    }
    // global override (rarely used)
    let ov = STRICT_OVERRIDE.load(Ordering::Relaxed);
    if ov == 1 {
        return true;
    } else if ov == -1 {
        return false;
    }
    env::var("OML_SQL_STRICT")
        .ok()
        .map(|v| v != "0")
        .unwrap_or(true)
}

#[cfg(test)]
pub fn set_sql_strict_for_test(val: Option<bool>) {
    // None: clear override; Some(true): on; Some(false): off
    let v = match val {
        Some(true) => 1,
        Some(false) => -1,
        None => 0,
    };
    // set thread-local to avoid cross-test races
    STRICT_TL.with(|c| c.set(v));
}

pub fn oml_sql(data: &mut &str) -> WResult<SqlQuery> {
    // Parse `select <body> where <cond>;`
    // We sanitize `<body>` to avoid unsafe identifiers: only [A-Za-z0-9_.] and '*' are allowed
    // and we split `cols from table`. If sanitize fails, we fall back to original body to keep
    // backward compatibility (recommended to provide whitelisted identifiers at source).
    kw_sql_select.parse_next(data)?;
    let sql_body = take_until(0.., "where").parse_next(data)?;
    kw_sql_where.parse_next(data)?;
    let sql_cond_raw = take_until(0.., ";").parse_next(data)?;
    // debug: sql where raw
    // 兼容写法：允许在 WHERE 中写 `fn(...) = <literal>`，按对称性改写为 `<literal> = fn(...)`
    fn rewrite_lhs_fn_eq_literal(s: &str) -> Option<String> {
        let t = s.trim();
        let bytes = t.as_bytes();
        // quick check: starts with ident and '('
        let mut i = 0usize;
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
            return None;
        }
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'.')
        {
            i += 1;
        }
        while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'(' {
            return None;
        }
        // find matching ')'
        let mut depth = 0i32;
        let mut j = i;
        while j < bytes.len() {
            let c = bytes[j];
            if c == b'(' {
                depth += 1;
            } else if c == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        if j >= bytes.len() || depth != 0 {
            return None;
        }
        // remaining: ") ..."
        let mut k = j + 1;
        while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        if k >= bytes.len() || bytes[k] != b'=' {
            return None;
        }
        k += 1; // skip '='
        while k < bytes.len() && (bytes[k] == b' ' || bytes[k] == b'\t') {
            k += 1;
        }
        let rhs = t[k..].trim();
        if rhs.is_empty() {
            return None;
        }
        let lhs = t[..=j].trim();
        Some(format!("{} = {}", rhs, lhs))
    }
    let sql_cond_buf: String =
        rewrite_lhs_fn_eq_literal(sql_cond_raw).unwrap_or_else(|| sql_cond_raw.to_string());
    // debug: sql where rewritten
    // Fast path: support `1 = ip4_between(read(x), a, b)` without going through the generic cond parser
    use crate::language::CondAccessor;
    use std::collections::HashMap;
    fn fast_path_ip4_between_eq_one(s: &str) -> Option<(String, HashMap<String, CondAccessor>)> {
        let t = s.trim();
        let t = if let Some(rest) = t.strip_prefix("1=") {
            rest
        } else if let Some(rest) = t.strip_prefix("1 =") {
            rest
        } else {
            return None;
        };
        let t = t.trim_start();
        if !t.starts_with("ip4_between(") {
            return None;
        }
        let inside = t.strip_prefix("ip4_between(")?;
        let inside = inside.strip_suffix(")")?;
        let parts: Vec<&str> = inside.split(',').map(|x| x.trim()).collect();
        if parts.len() != 3 {
            return None;
        }
        let mut params: HashMap<String, CondAccessor> = HashMap::new();
        // Helper to map read(arg) -> ":arg" and collect params
        fn to_sql_piece(s: &str, params: &mut HashMap<String, CondAccessor>) -> String {
            let st = s.trim();
            if let Some(rest) = st.strip_prefix("read(")
                && let Some(rest2) = rest.strip_suffix(")")
            {
                let var = rest2.trim();
                params.insert(var.to_string(), CondAccessor::from_read(var.to_string()));
                return format!(":{}", var);
            }
            st.to_string()
        }
        let p1 = to_sql_piece(parts[0], &mut params);
        let p2 = to_sql_piece(parts[1], &mut params);
        let p3 = to_sql_piece(parts[2], &mut params);
        // Prefer using range compare to avoid dependency on ip4_between UDF semantics
        let where_sql = format!("{} <= ip4_int({}) and {} >= ip4_int({})", p2, p1, p3, p1);
        Some((where_sql, params))
    }
    if let Some((w_sql, vars)) = fast_path_ip4_between_eq_one(&sql_cond_buf) {
        let sql = format!("select {} where {}", sql_body, w_sql);
        return Ok(SqlQuery::new(sql, vars));
    }
    // Generic path
    let mut sql_cond = sql_cond_buf.as_str();
    let cond = SCondParser::end_exp(&mut sql_cond, ";")?;
    //{
    //println!("{}", cond);
    let (w_sql, vars) = cond.args_take();
    // 基础校验：仅允许形如 "col_a, col_b from table_name" 的主体（列/表标识符限定在 [A-Za-z0-9_\.] 与 '*'）
    fn is_ident(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    }
    fn sanitize_sql_body(body: &str) -> Option<String> {
        let body_trim = body.trim();
        let lower = body_trim.to_lowercase();
        let from_pos = lower.rfind(" from ")?;
        let (cols_part, table_part) = body_trim.split_at(from_pos);
        let table_name = table_part[" from ".len()..].trim();
        if table_name.is_empty() || !is_ident(table_name) {
            return None;
        }
        let cols: Vec<&str> = cols_part.split(',').map(|s| s.trim()).collect();
        if cols.is_empty() {
            return None;
        }
        for c in &cols {
            if *c != "*" && !is_ident(c) {
                return None;
            }
        }
        Some(format!("{} from {}", cols.join(","), table_name))
    }
    // 严格模式：非法主体直接报错；兼容模式：回退原文
    let strict = is_sql_strict();
    let safe_body = match sanitize_sql_body(sql_body) {
        Some(b) => b,
        None if strict => {
            return fail
                .context(StrContext::Label("sql body"))
                .context(StrContext::Expected(StrContextValue::Description(
                    "expected `<cols from table>`",
                )))
                .context(StrContext::Expected(StrContextValue::Description(
                    "cols in [A-Za-z0-9_.] or '*'",
                )))
                .context(StrContext::Expected(StrContextValue::Description(
                    "table in [A-Za-z0-9_.]",
                )))
                .parse_next(data);
        }
        None => sql_body.to_string(),
    };
    let sql = format!("select {} where {}", safe_body, w_sql);
    Ok(SqlQuery::new(sql, vars))
    // 旧版在解析失败时构造 SQLPrimitive；已废弃，现统一走知识库门面 Provider
}

pub fn oml_aga_sql(data: &mut &str) -> WResult<PreciseEvaluator> {
    Ok(PreciseEvaluator::Sql(oml_sql.parse_next(data)?))
}
#[cfg(test)]
mod tests {
    use wp_data_model::cache::FieldQueryCache;
    use wp_parser::WResult as ModalResult;

    use crate::parser::sql_prm::oml_sql;
    use crate::parser::utils::for_test::assert_oml_parse;
    use winnow::Parser;

    #[test]
    fn test_oml_sql() -> ModalResult<()> {
        super::set_sql_strict_for_test(Some(true));
        let mut code = r#" select a, b from table_1 where x = read (src);"#;
        assert_oml_parse(&mut code, oml_sql);

        let mut code = r#" select a, b from table_1 where x = take (src);"#;
        assert_oml_parse(&mut code, oml_sql);

        let mut code = r#" select a, b from table_1 where x = Now::time() ;"#;
        assert_oml_parse(&mut code, oml_sql);

        let mut code = r#" select a, b from table_1 where x = 1 ;"#;
        assert_oml_parse(&mut code, oml_sql);

        let mut code = r#" select a, b from table_1 where x = 'china' ;"#;
        assert_oml_parse(&mut code, oml_sql);

        let mut code = r#"select name,pinying from example where pinying = 'xiaolongnu' ;"#;
        assert_oml_parse(&mut code, oml_sql);
        Ok(())
    }

    #[test]
    fn test_oml_sql2() -> ModalResult<()> {
        super::set_sql_strict_for_test(Some(true));
        let mut code = r#" select a, b from table_1 where x = Now::time() and y = read(src) ;"#;
        assert_oml_parse(&mut code, oml_sql);

        Ok(())
    }

    #[test]
    fn test_oml_sql_strict_err() {
        super::set_sql_strict_for_test(Some(true));
        let code = r#" select a, b from table-1 where x = read(src) ;"#;
        let err = oml_sql.parse(code).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("sql body"));
        assert!(msg.contains("expected `"));
        super::set_sql_strict_for_test(None);
    }

    #[test]
    fn test_oml_sql_compat_ok() -> ModalResult<()> {
        // 双保险：覆盖为兼容模式，且写入 env 变量
        super::set_sql_strict_for_test(Some(false));
        unsafe {
            std::env::set_var("OML_SQL_STRICT", "0");
        }
        let mut code = r#" select a, b from table-1 where x = read(src) ;"#;
        assert_oml_parse(&mut code, oml_sql);
        super::set_sql_strict_for_test(None);
        unsafe {
            std::env::remove_var("OML_SQL_STRICT");
        }
        Ok(())
    }

    use crate::core::DataTransformer;
    use crate::parser::oml_parse;
    use orion_error::TestAssert;
    use wp_know::mem::memdb::MemDB;
    use wp_knowledge::facade as kdb;
    use wp_model_core::model::{DataField, DataRecord};

    #[test]
    fn test_sql_udf_ip4_between_exec() -> ModalResult<()> {
        // 1) init in-memory provider and prepare table with an IPv4 range
        let db = MemDB::global();
        db.table_create(
            "CREATE TABLE IF NOT EXISTS zone (zone TEXT, ip_start_int INTEGER, ip_end_int INTEGER)",
        )
        .assert();
        // 10.0.0.0 - 10.255.255.255 => [167772160, 184549375]
        db.execute(
            "INSERT INTO zone (zone, ip_start_int, ip_end_int) VALUES ('A', 167772160, 184549375)",
        )
        .assert();
        let _ = kdb::init_mem_provider(db);

        // 2) build OML with UDF in WHERE
        let mut conf = r#"
name : test
---
zone : chars = select zone from zone where ip_start_int <= ip4_int(read(src_ip)) and ip_end_int >= ip4_int(read(src_ip)) ;
        "#;
        let model = oml_parse(&mut conf).assert();

        // 3) transform with src_ip within range
        let src = DataRecord {
            items: vec![DataField::from_chars("src_ip", "10.1.2.3")],
        };
        let cache = &mut FieldQueryCache::default();
        let out = model.transform(src, cache);
        use wp_model_core::model::Value;
        let zone = out.get2("zone").and_then(|f| match f.get_value() {
            Value::Chars(s) => Some(s.as_str()),
            _ => None,
        });
        assert_eq!(zone, Some("A"));
        Ok(())
    }
}
