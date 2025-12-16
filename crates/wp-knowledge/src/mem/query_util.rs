use orion_error::ErrorOwe;
use rusqlite::Params;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wp_error::KnowledgeResult;
use wp_log::debug_kdb;
use wp_model_core::model::{self, DataField};

use lazy_static::lazy_static;

lazy_static! {
    /// Global column-name cache keyed by raw SQL text, shared by MemDB queries.
    pub static ref COLNAME_CACHE: RwLock<HashMap<String, Arc<Vec<String>>>> =
        RwLock::new(HashMap::new());
}

/// 将一行数据映射为 Vec<DataField>
fn map_row(row: &rusqlite::Row<'_>, col_names: &[String]) -> KnowledgeResult<Vec<DataField>> {
    let mut result = Vec::with_capacity(col_names.len());
    for (i, col_name) in col_names.iter().enumerate() {
        let value = row.get_ref(i).owe_rule()?;
        let field = match value {
            rusqlite::types::ValueRef::Null => {
                DataField::new(model::DataType::default(), col_name, model::Value::Null)
            }
            rusqlite::types::ValueRef::Integer(v) => DataField::from_digit(col_name, v),
            rusqlite::types::ValueRef::Real(v) => DataField::from_float(col_name, v),
            rusqlite::types::ValueRef::Text(v) => {
                DataField::from_chars(col_name, &String::from_utf8(v.to_vec()).owe_rule()?)
            }
            rusqlite::types::ValueRef::Blob(v) => {
                DataField::from_chars(col_name, &String::from_utf8_lossy(v).to_string())
            }
        };
        result.push(field);
    }
    Ok(result)
}

/// 从 statement 获取列名（普通版，带 debug 日志）
fn extract_col_names(stmt: &rusqlite::Statement<'_>) -> Vec<String> {
    let col_cnt = stmt.column_count();
    debug_kdb!("[memdb] col_cnt={}", col_cnt);
    let mut col_names = Vec::with_capacity(col_cnt);
    for i in 0..col_cnt {
        let name = stmt.column_name(i).unwrap_or("").to_string();
        debug_kdb!("[memdb] col[{}] name='{}'", i, name);
        col_names.push(name);
    }
    col_names
}

/// 从 statement 获取列名（cached 版，使用全局缓存）
fn extract_col_names_cached(
    stmt: &rusqlite::Statement<'_>,
    sql: &str,
) -> KnowledgeResult<Vec<String>> {
    if let Some(names) = COLNAME_CACHE.read().ok().and_then(|m| m.get(sql).cloned()) {
        return Ok((*names).clone());
    }
    let col_cnt = stmt.column_count();
    let mut names = Vec::with_capacity(col_cnt);
    for i in 0..col_cnt {
        names.push(stmt.column_name(i).owe_rule()?.to_string());
    }
    if let Ok(mut m) = COLNAME_CACHE.write() {
        m.insert(sql.to_string(), Arc::new(names.clone()));
    }
    Ok(names)
}

pub fn query<P: Params>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> KnowledgeResult<Vec<Vec<DataField>>> {
    let mut stmt = conn.prepare_cached(sql).owe_rule()?;
    let col_names = extract_col_names(&stmt);
    let mut rows = stmt.query(params).owe_rule()?;
    let mut all_result = Vec::new();
    while let Some(row) = rows.next().owe_rule()? {
        all_result.push(map_row(row, &col_names)?);
    }
    Ok(all_result)
}

/// Query first row and map columns into Vec<DataField> with column names preserved.
pub fn query_first_row<P: Params>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> KnowledgeResult<Vec<DataField>> {
    let mut stmt = conn.prepare_cached(sql).owe_rule()?;
    let col_names = extract_col_names(&stmt);
    let mut rows = stmt.query(params).owe_rule()?;
    if let Some(row) = rows.next().owe_rule()? {
        map_row(row, &col_names)
    } else {
        debug_kdb!("[memdb] no row for sql");
        Ok(Vec::new())
    }
}

pub fn query_cached<P: Params>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> KnowledgeResult<Vec<Vec<DataField>>> {
    let mut stmt = conn.prepare_cached(sql).owe_rule()?;
    // Column names cache (per SQL)
    let col_names = extract_col_names_cached(&stmt, sql)?;
    let mut rows = stmt.query(params).owe_rule()?;
    let mut all_result = Vec::new();
    while let Some(row) = rows.next().owe_rule()? {
        all_result.push(map_row(row, &col_names)?);
    }
    Ok(all_result)
}

/// Same as `query_first_row` but with a shared column-names cache to reduce metadata lookups.
pub fn query_first_row_cached<P: Params>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> KnowledgeResult<Vec<DataField>> {
    let mut stmt = conn.prepare_cached(sql).owe_rule()?;
    let col_names = extract_col_names_cached(&stmt, sql)?;
    let mut rows = stmt.query(params).owe_rule()?;
    if let Some(row) = rows.next().owe_rule()? {
        map_row(row, &col_names)
    } else {
        Ok(Vec::new())
    }
}
