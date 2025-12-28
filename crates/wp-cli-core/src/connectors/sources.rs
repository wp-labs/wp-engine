use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use orion_conf::TomlIO;
use orion_conf::error::{ConfIOReason, OrionConfResult};
use orion_error::{ToStructError, UvsValidationFrom};
use wp_conf::connectors::{
    ConnectorScope, ParamMap, load_connector_defs_from_dir, param_map_to_table,
};
use wp_conf::sources::{SourceConnector, WpSourcesConfig, find_connectors_dir};

/// A flattened row for listing source connectors and their usages.
#[derive(Debug, Clone)]
pub struct ConnectorListRow {
    pub id: String,
    pub kind: String,
    pub allow_override: Vec<String>,
    pub detail: String,
    pub refs: usize,
}

/// A flattened row for showing resolved source routes after applying overrides.
#[derive(Debug, Clone)]
pub struct RouteRow {
    pub key: String,
    pub connect: String,
    pub kind: String,
    pub enabled: bool,
    pub detail: String,
}

/// Locate `wpsrc.toml` starting from a work root (supports `topology/sources/` and legacy `source/`).
fn resolve_wpsrc_path(work_root: &str) -> OrionConfResult<PathBuf> {
    let wr = PathBuf::from(work_root);
    let candidate1 = wr.join("topology").join("sources").join("wpsrc.toml");
    if candidate1.exists() {
        return Ok(candidate1);
    }
    let candidate2 = wr.join("source").join("wpsrc.toml");
    if candidate2.exists() {
        return Ok(candidate2);
    }
    ConfIOReason::from_validation(format!(
        "missing wpsrc.toml under '{}/topology/sources' or '{}/source'",
        work_root, work_root
    ))
    .err_result()
}

/// Load connectors map from `connectors/source.d` (dedup and validate ids).
fn load_connectors_map(base_dir: &Path) -> OrionConfResult<BTreeMap<String, SourceConnector>> {
    let defs = load_connector_defs_from_dir(base_dir, ConnectorScope::Source)?;
    Ok(defs.into_iter().map(|def| (def.id.clone(), def)).collect())
}

/// Merge per-source overrides onto connector defaults, honoring the connector whitelist.
fn merge_params(base: &ParamMap, ov: &ParamMap, allow: &[String]) -> OrionConfResult<ParamMap> {
    let mut out = base.clone();
    for (k, v) in ov.iter() {
        if !allow.iter().any(|x| x == k) {
            return ConfIOReason::from_validation(format!(
                "override '{}' not allowed; whitelist: [{}]",
                k,
                allow.join(", ")
            ))
            .err_result();
        }
        out.insert(k.clone(), v.clone());
    }
    Ok(out)
}

/// Best-effort visualization of params without guessing semantics.
/// Render the whole params table as a single-line TOML snippet for display.
fn detail_of(_kind: &str, params: &ParamMap) -> String {
    let table = param_map_to_table(params);
    match toml::to_string(&table) {
        Ok(s) => s.replace(['\n', '\t'], " ").trim().to_string(),
        Err(_) => format!("{:?}", params),
    }
}

/// List source connectors and reference count from wpsrc.toml
pub fn list_connectors(work_root: &str) -> OrionConfResult<Vec<ConnectorListRow>> {
    let wpsrc_path = resolve_wpsrc_path(work_root)?;
    let conn_base = find_connectors_dir(&wpsrc_path).ok_or_else(|| {
        ConfIOReason::from_validation(format!(
            "connectors/source.d not found (start from: {})",
            wpsrc_path.display()
        ))
    })?;
    let conn_map = load_connectors_map(&conn_base)?;
    let wp_sources = WpSourcesConfig::load_toml(&wpsrc_path)?;

    // Count how many times each connector id is referenced.
    let mut refs: BTreeMap<String, usize> = BTreeMap::new();
    for s in wp_sources.sources {
        *refs.entry(s.connect.clone()).or_insert(0) += 1;
    }

    let mut rows: Vec<ConnectorListRow> = conn_map
        .iter()
        .map(|(id, c)| ConnectorListRow {
            id: id.clone(),
            kind: c.kind.clone(),
            allow_override: c.allow_override.clone(),
            detail: detail_of(&c.kind, &c.default_params),
            refs: *refs.get(id).unwrap_or(&0),
        })
        .collect();
    rows.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.id.cmp(&b.id)));
    Ok(rows)
}

/// Build a resolved view of `[[sources]]`, applying connector param whitelists and simple filtering.
pub fn route_table(work_root: &str, path_like: Option<&str>) -> OrionConfResult<Vec<RouteRow>> {
    let wpsrc_path = resolve_wpsrc_path(work_root)?;
    let conn_base = find_connectors_dir(&wpsrc_path).ok_or_else(|| {
        ConfIOReason::from_validation(format!(
            "connectors/source.d not found (start from: {})",
            wpsrc_path.display()
        ))
    })?;
    let conn_map = load_connectors_map(&conn_base)?;
    let wrapper = WpSourcesConfig::load_toml(&wpsrc_path)?;
    let mut rows: Vec<RouteRow> = Vec::new();
    for src in wrapper.sources.into_iter() {
        let conn = conn_map.get(&src.connect).ok_or_else(|| {
            ConfIOReason::from_validation(format!("connector not found: {}", src.connect))
        })?;
        let merged = merge_params(&conn.default_params, &src.params, &conn.allow_override)?;
        let detail = detail_of(&conn.kind, &merged);
        let row = RouteRow {
            key: src.key,
            connect: conn.id.clone(),
            kind: conn.kind.clone(),
            enabled: src.enable.unwrap_or(true),
            detail,
        };
        if let Some(substr) = path_like {
            if !row.detail.contains(substr) {
                continue;
            }
        }
        rows.push(row);
    }
    rows.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.key.cmp(&b.key)));
    Ok(rows)
}

// Note: previously we parsed raw TOML items into a local struct; we now use `WarpSources` directly.
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn list_connectors_counts_refs() {
        let root = tmp_dir("wpcore_src_refs");
        // connectors under root/connectors/source.d
        let cdir = root.join("connectors").join("source.d");
        fs::create_dir_all(&cdir).unwrap();
        fs::write(
            cdir.join("c1.toml"),
            r#"[[connectors]]
id = "c1"
type = "file"
allow_override = ["path","fmt"]
"#,
        )
        .unwrap();
        // wpsrc under root/topology/sources/wpsrc.toml
        let wpsrc = root.join("topology").join("sources");
        fs::create_dir_all(&wpsrc).unwrap();
        fs::write(
            wpsrc.join("wpsrc.toml"),
            r#"[[sources]]
key = "s1"
connect = "c1"

[[sources]]
key = "s2"
connect = "c1"
params_override = { path = "/x" }
"#,
        )
        .unwrap();

        let rows = list_connectors(root.to_string_lossy().as_ref()).expect("list");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "c1");
        assert_eq!(rows[0].refs, 2);
    }

    #[test]
    fn route_table_merges_overrides_and_renders_params() {
        let root = tmp_dir("wpcore_src_route");
        // connectors
        let cdir = root.join("connectors").join("source.d");
        fs::create_dir_all(&cdir).unwrap();
        fs::write(
            cdir.join("c1.toml"),
            r#"[[connectors]]
id = "c1"
type = "file"
allow_override = ["path","fmt"]
"#,
        )
        .unwrap();
        // wpsrc
        let wpsrc = root.join("topology").join("sources");
        fs::create_dir_all(&wpsrc).unwrap();
        fs::write(
            wpsrc.join("wpsrc.toml"),
            r#"[[sources]]
key = "s1"
connect = "c1"
params_override = { path = "/data/x.dat" }
"#,
        )
        .unwrap();

        let rows = route_table(root.to_string_lossy().as_ref(), None).expect("routes");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, "s1");
        assert_eq!(rows[0].kind, "file");
        // detail is single-line TOML; should contain path value
        assert!(rows[0].detail.contains("/data/x.dat"));
    }

    #[test]
    fn route_table_path_like_filters() {
        let root = tmp_dir("wpcore_src_filter");
        let cdir = root.join("connectors").join("source.d");
        fs::create_dir_all(&cdir).unwrap();
        fs::write(
            cdir.join("c1.toml"),
            r#"[[connectors]]
id = "c1"
type = "file"
allow_override = ["path"]
"#,
        )
        .unwrap();
        let wpsrc = root.join("topology").join("sources");
        fs::create_dir_all(&wpsrc).unwrap();
        fs::write(
            wpsrc.join("wpsrc.toml"),
            r#"[[sources]]
key = "s1"
connect = "c1"
params_override = { path = "/data/a.dat" }
"#,
        )
        .unwrap();

        let rows_all = route_table(root.to_string_lossy().as_ref(), None).expect("all");
        assert_eq!(rows_all.len(), 1);
        let rows_none =
            route_table(root.to_string_lossy().as_ref(), Some("b.dat")).expect("filtered");
        assert_eq!(rows_none.len(), 0);
    }
}
