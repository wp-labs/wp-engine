use crate::fsutils::{count_lines_file, resolve_path};
use crate::types::Ctx;
use orion_variate::EnvDict;
use serde::Serialize;
use std::collections::BTreeMap;
use wp_conf::connectors::{ParamMap, param_value_from_toml};
use wp_conf::engine::EngineConfig;

// Minimal local model to parse file sources from wpsrc.toml
#[derive(Debug, serde::Deserialize, Clone)]
pub struct FileSourceConfLite {
    pub key: String,
    pub path: String,
    pub enable: bool,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct UnifiedSourcesLite {
    #[serde(default)]
    pub sources: Vec<toml::Value>,
}

fn read_wpsrc_toml(work_root: &std::path::Path, engine_conf: &EngineConfig) -> Option<String> {
    let modern = work_root.join(engine_conf.src_root()).join("wpsrc.toml");
    if modern.exists() {
        return std::fs::read_to_string(&modern).ok();
    }
    None
}

type SrcConnectorRec = wp_conf::sources::SourceConnector;

// 删除本地封装，调用点直接使用配置层公共定位

fn load_connectors_map(base_dir: &std::path::Path) -> Option<BTreeMap<String, SrcConnectorRec>> {
    wp_conf::sources::load_connectors_for(base_dir, &EnvDict::default()).ok()
}

fn merge_params(base: &ParamMap, override_tbl: &toml::value::Table, allow: &[String]) -> ParamMap {
    let mut out = base.clone();
    for (k, v) in override_tbl.iter() {
        if allow.iter().any(|x| x == k) {
            out.insert(k.clone(), param_value_from_toml(v));
        }
    }
    out
}

/// 从 wpsrc 配置推导总输入条数（仅统计启用的文件源）
pub fn total_input_from_wpsrc(
    work_root: &std::path::Path,
    engine_conf: &EngineConfig,
    ctx: &Ctx,
) -> Option<u64> {
    let content = read_wpsrc_toml(work_root, engine_conf)?;
    let toml_val: toml::Value = toml::from_str(&content).ok()?;
    let mut sum = 0u64;
    if let Some(arr) = toml_val.get("sources").and_then(|v| v.as_array()) {
        // load connectors once
        let conn_dir =
            wp_conf::find_connectors_base_dir(&ctx.work_root.join("sources"), "source.d");
        let conn_map = conn_dir
            .as_ref()
            .and_then(|p| load_connectors_map(p.as_path()))
            .unwrap_or_default();
        for item in arr {
            // v2: prefer connect flow
            if let Some(conn_id) = item.get("connect").and_then(|v| v.as_str()) {
                let enabled = item.get("enable").and_then(|v| v.as_bool()).unwrap_or(true);
                if !enabled {
                    continue;
                }
                if let Some(conn) = conn_map.get(conn_id) {
                    if conn.kind.eq_ignore_ascii_case("file") {
                        // 支持 params_override 与 params 两种写法
                        let ov = item
                            .get("params_override")
                            .or_else(|| item.get("params"))
                            .and_then(|v| v.as_table())
                            .cloned()
                            .unwrap_or_default();
                        let merged = merge_params(&conn.default_params, &ov, &conn.allow_override);
                        // 支持 path 或 base+file 两种写法
                        let maybe_path = merged
                            .get("path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                let b = merged.get("base").and_then(|v| v.as_str());
                                let f = merged.get("file").and_then(|v| v.as_str());
                                match (b, f) {
                                    (Some(b), Some(f)) => {
                                        Some(std::path::Path::new(b).join(f).display().to_string())
                                    }
                                    _ => None,
                                }
                            });
                        if let Some(path) = maybe_path {
                            let pathbuf = resolve_path(&path, &ctx.work_root);
                            if let Ok(n) = count_lines_file(&pathbuf) {
                                sum += n;
                            }
                        }
                    }
                }
                continue;
            }
        }
        return Some(sum);
    }
    None
}

#[derive(Serialize)]
pub struct SrcLineItem {
    pub key: String,
    pub path: String,
    pub enabled: bool,
    pub lines: Option<u64>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SrcLineReport {
    pub total_enabled_lines: u64,
    pub items: Vec<SrcLineItem>,
}

/// 返回所有文件源（包含未启用）的行数信息；total 仅统计启用项
pub fn list_file_sources_with_lines(
    work_root: &std::path::Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
) -> Option<SrcLineReport> {
    let content = read_wpsrc_toml(work_root, eng_conf)?;
    let toml_val: toml::Value = toml::from_str(&content).ok()?;
    let mut items = Vec::new();
    let mut total = 0u64;
    if let Some(arr) = toml_val.get("sources").and_then(|v| v.as_array()) {
        // load connectors once
        let conn_dir =
            wp_conf::find_connectors_base_dir(&ctx.work_root.join("sources"), "source.d");
        let conn_map = conn_dir
            .as_ref()
            .and_then(|p| load_connectors_map(p.as_path()))
            .unwrap_or_default();
        for it in arr {
            let conn_id = match it.get("connect").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => continue, // 不兼容旧写法
            };
            let key = it
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let enabled = it.get("enable").and_then(|v| v.as_bool()).unwrap_or(true);
            // 支持 params_override 与 params 两种写法
            let ov = it
                .get("params_override")
                .or_else(|| it.get("params"))
                .and_then(|v| v.as_table())
                .cloned()
                .unwrap_or_default();
            if let Some(conn) = conn_map.get(conn_id) {
                if !conn.kind.eq_ignore_ascii_case("file") {
                    continue;
                }
                let merged = merge_params(&conn.default_params, &ov, &conn.allow_override);
                // 支持 path 或 base+file 两种写法
                let path_str = merged
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        let b = merged.get("base").and_then(|v| v.as_str());
                        let f = merged.get("file").and_then(|v| v.as_str());
                        match (b, f) {
                            (Some(b), Some(f)) => {
                                Some(std::path::Path::new(b).join(f).display().to_string())
                            }
                            _ => None,
                        }
                    })
                    .unwrap_or_default();
                let pbuf = resolve_path(&path_str, &ctx.work_root);
                if enabled {
                    match count_lines_file(&pbuf) {
                        Ok(n) => {
                            total += n;
                            items.push(SrcLineItem {
                                key,
                                path: pbuf.display().to_string(),
                                enabled,
                                lines: Some(n),
                                error: None,
                            });
                        }
                        Err(e) => {
                            items.push(SrcLineItem {
                                key,
                                path: pbuf.display().to_string(),
                                enabled,
                                lines: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    items.push(SrcLineItem {
                        key,
                        path: pbuf.display().to_string(),
                        enabled,
                        lines: None,
                        error: None,
                    });
                }
            }
        }
        return Some(SrcLineReport {
            total_enabled_lines: total,
            items,
        });
    }
    None
}
