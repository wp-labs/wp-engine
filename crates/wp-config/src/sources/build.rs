use super::types::WpSourcesConfig;
use crate::sources::types::SourceConnector;
use crate::structure::SourceInstanceConf;
use orion_conf::error::{ConfIOReason, OrionConfResult};
use orion_error::{ErrorOwe, ErrorWith, ToStructError, UvsValidationFrom};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use wp_connector_api::ParamMap;

/// 仅解析并执行最小校验（不进行实际构建，不触发 I/O）
pub fn parse_and_validate_only(config_str: &str) -> OrionConfResult<Vec<wp_specs::CoreSourceSpec>> {
    let wrapper: WpSourcesConfig = toml::from_str(config_str)
        .owe_conf()
        .want("parse sources v2")?;
    let mut out: Vec<wp_specs::CoreSourceSpec> = Vec::new();
    for s in wrapper.sources.into_iter() {
        if !s.enable.unwrap_or(true) {
            continue;
        }
        out.push(wp_specs::CoreSourceSpec {
            name: s.key,
            kind: String::new(),
            params: ParamMap::new(),
            tags: s.tags,
        });
    }
    Ok(out)
}

/// 加载文件并解析（最小校验）
pub fn parse_and_validate_only_from_file(
    path: &std::path::Path,
) -> OrionConfResult<Vec<wp_specs::CoreSourceSpec>> {
    let content = std::fs::read_to_string(path)
        .owe_conf()
        .want("load sources config")
        .with(path)?;
    parse_and_validate_only(&content)
}

/// 从起点向上查找 connectors/source.d 并加载连接器
pub fn connectors_from_start(start: &Path) -> OrionConfResult<BTreeMap<String, SourceConnector>> {
    super::io::load_connectors_for(start)
}

/// whitelist + 合并参数，返回 a merged table
fn is_nested_field_blacklisted(k: &str) -> bool {
    matches!(k, "params" | "params_override")
}

fn merge_params(
    base: &ParamMap,
    override_tbl: &ParamMap,
    allow: &[String],
) -> OrionConfResult<ParamMap> {
    let mut out = base.clone();
    for (k, v) in override_tbl.iter() {
        if is_nested_field_blacklisted(k) {
            return ConfIOReason::from_validation(format!(
                "invalid nested table '{}' in params override; please flatten and set keys [{}] directly under 'params'/'params_override'",
                k,
                allow.join(", ")
            ))
            .err_result();
        }
        if !allow.iter().any(|x| x == k) {
            return ConfIOReason::from_validation("override not allowed")
                .err_result()
                .with(allow.join(","));
        }
        out.insert(k.clone(), v.clone());
    }
    Ok(out)
}

/// 解析字符串并结合 connectors（通过 `connect` 字段）构建 CoreSourceSpec + connector_id 列表
pub fn build_specs_with_ids_from_str(
    config_str: &str,
    start: &Path,
) -> OrionConfResult<Vec<SourceInstanceConf>> {
    let wrapper: WpSourcesConfig = toml::from_str(config_str)
        .owe_conf()
        .want("parse sources v2")?;
    let cmap = connectors_from_start(start)?;
    specs_from_wrapper(wrapper, &cmap)
}

/// 解析文件并结合 connectors 构建 CoreSourceSpec + connector_id 列表
pub fn build_specs_with_ids_from_file(
    path: &std::path::Path,
) -> OrionConfResult<Vec<SourceInstanceConf>> {
    let content = std::fs::read_to_string(path)
        .owe_conf()
        .want("load sources config")
        .with(path)?;
    let start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
    };
    build_specs_with_ids_from_str(&content, &start)
}

/// 从 WarpSources + 连接器字典 构建 SourceInstanceConf（包含 Core + connector_id）列表
pub fn specs_from_wrapper(
    wrapper: WpSourcesConfig,
    cmap: &BTreeMap<String, SourceConnector>,
) -> OrionConfResult<Vec<SourceInstanceConf>> {
    let mut specs: Vec<SourceInstanceConf> = Vec::new();
    for s in wrapper.sources.into_iter() {
        if !s.enable.unwrap_or(true) {
            continue;
        }
        let conn = cmap.get(&s.connect).ok_or_else(|| {
            ConfIOReason::from_validation(format!(
                "connector not found: '{}' (looked up under connectors/source.d)",
                s.connect
            ))
            .to_err()
        })?;
        let merged = merge_params(&conn.default_params, &s.params, &conn.allow_override)?;
        let mut inst = SourceInstanceConf::new_type(s.key, conn.kind.clone(), merged, s.tags);
        inst.connector_id = Some(conn.id.clone());
        specs.push(inst);
    }
    Ok(specs)
}

/// 使用插件 Factory 执行“类型特有校验”（不触发 I/O）。
pub trait SourceFactoryRegistry {
    fn get_factory(&self, kind: &str)
    -> Option<Arc<dyn wp_connector_api::SourceFactory + 'static>>;
}

pub fn validate_specs_with_factory(
    specs: &[SourceInstanceConf],
    reg: &dyn SourceFactoryRegistry,
) -> OrionConfResult<()> {
    for item in specs.iter() {
        let core: wp_specs::CoreSourceSpec = item.into();
        if let Some(factory) = reg.get_factory(&core.kind) {
            let resolved = crate::sources::resolved::core_to_resolved_with(
                &core,
                item.connector_id.clone().unwrap_or_default(),
            );
            factory.validate_spec(&resolved).map_err(|e| {
                ConfIOReason::from_validation(format!(
                    "plugin validate failed for source '{}' of kind '{}': {}",
                    core.name, core.kind, e
                ))
            })?;
        }
    }
    Ok(())
}

/// Deprecated: kept for compatibility; does nothing when API registry is removed.
pub fn validate_specs_with_factory_and_registry(
    _specs: &[SourceInstanceConf],
) -> OrionConfResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{io, types};
    use orion_conf::UvsConfFrom;
    use serde_json::json;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wp_connector_api::{ConnectorScope, SourceReason, SourceResult, SourceSvcIns};

    fn tmp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("{}_{}", prefix, nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_minimal_ok() {
        let raw = r#"[[sources]]
key = "s1"
connect = "conn1"
[connectors]
"#;
        // 最小解析：不校验 connectors（仅返回 name/tags）
        let _ = parse_and_validate_only(raw).expect("parse");
    }

    #[test]
    fn merge_params_whitelist_ok_and_err() {
        let mut base = ParamMap::new();
        base.insert("endpoint".into(), json!("127.0.0.1"));
        let allow = vec!["path".to_string(), "fmt".to_string()];

        // ok: allowed key
        let mut over = ParamMap::new();
        over.insert("path".into(), json!("/a"));
        let ok = merge_params(&base, &over, &allow).expect("ok");
        assert_eq!(ok.get("path").and_then(|v| v.as_str()), Some("/a"));

        // err: disallowed key
        let mut bad = ParamMap::new();
        bad.insert("badkey".into(), json!("v"));
        let e = merge_params(&base, &bad, &allow)
            .expect_err("err")
            .to_string();
        assert!(e.contains("override not allowed"));

        // err: nested blacklisted field
        let mut nested = ParamMap::new();
        nested.insert("params".into(), json!("x"));
        let e2 = merge_params(&base, &nested, &allow)
            .expect_err("err")
            .to_string();
        assert!(e2.contains("invalid nested table"));
    }

    #[test]
    fn specs_from_wrapper_filters_disabled() {
        let cmap = {
            let mut m = BTreeMap::new();
            m.insert(
                "c1".to_string(),
                SourceConnector {
                    id: "c1".into(),
                    kind: "dummy".into(),
                    scope: ConnectorScope::Source,
                    allow_override: vec!["a".into()],
                    default_params: ParamMap::new(),
                    origin: None,
                },
            );
            m
        };
        let w = WpSourcesConfig {
            sources: vec![
                types::WpSource {
                    key: "s1".into(),
                    enable: Some(false),
                    connect: "c1".into(),
                    tags: vec![],
                    params: ParamMap::new(),
                },
                types::WpSource {
                    key: "s2".into(),
                    enable: Some(true),
                    connect: "c1".into(),
                    tags: vec![],
                    params: ParamMap::new(),
                },
            ],
        };
        let specs = specs_from_wrapper(w, &cmap).expect("specs");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].name(), &"s2".to_string());
    }

    #[test]
    fn connectors_dedup_detected() {
        let base = tmp_dir("src_conn");
        let cdir = base.join("connectors").join("source.d");
        fs::create_dir_all(&cdir).unwrap();
        // write two files with same id
        fs::write(
            cdir.join("a.toml"),
            r#"[[connectors]]
id = "c1"
type = "dummy"
[connectors.params]
"#,
        )
        .unwrap();
        fs::write(
            cdir.join("b.toml"),
            r#"[[connectors]]
id = "c1"
type = "dummy"
[connectors.params]
"#,
        )
        .unwrap();
        let e = io::load_connectors_for(&base)
            .expect_err("dup err")
            .to_string();
        assert!(e.contains("duplicate connector id"));
    }

    use crate::connectors::ConnectorDef;
    use crate::connectors::ParamMap;
    use wp_connector_api::SourceFactory;

    struct DummyFactory;
    #[allow(clippy::needless_lifetimes)]
    #[async_trait::async_trait]
    impl wp_connector_api::SourceFactory for DummyFactory {
        fn kind(&self) -> &'static str {
            "dummy"
        }
        fn validate_spec(&self, spec: &wp_connector_api::SourceSpec) -> SourceResult<()> {
            // require key 'a' in params
            if !spec.params.contains_key("a") {
                return Err(SourceReason::from_conf("missing required param 'a'").to_err());
            }
            Ok(())
        }
        async fn build(
            &self,
            _spec: &wp_connector_api::SourceSpec,
            _ctx: &wp_connector_api::SourceBuildCtx,
        ) -> SourceResult<SourceSvcIns> {
            Err(SourceReason::from_conf("not used in validate test").to_err())
        }
    }

    impl wp_connector_api::SourceDefProvider for DummyFactory {
        fn source_def(&self) -> ConnectorDef {
            ConnectorDef {
                id: "dummy".into(),
                kind: self.kind().into(),
                scope: ConnectorScope::Source,
                allow_override: vec!["a".into()],
                default_params: ParamMap::new(),
                origin: Some("test:dummy".into()),
            }
        }
    }

    struct DummyReg;
    impl SourceFactoryRegistry for DummyReg {
        fn get_factory(
            &self,
            kind: &str,
        ) -> Option<Arc<dyn wp_connector_api::SourceFactory + 'static>> {
            if kind == "dummy" {
                Some(Arc::new(DummyFactory))
            } else {
                None
            }
        }
    }

    #[test]
    fn plugin_validate_fails_without_param() {
        // prepare one spec without 'a'
        let mut inst =
            SourceInstanceConf::new_type("s1".into(), "dummy".into(), ParamMap::new(), vec![]);
        inst.connector_id = Some("c1".into());
        let reg = DummyReg;
        let err = validate_specs_with_factory(&[inst], &reg)
            .expect_err("error")
            .to_string();
        assert!(err.contains("plugin validate failed"));
    }
}
