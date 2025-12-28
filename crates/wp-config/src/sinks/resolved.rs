use super::build::merge_params; // reuse whitelist+flatten logic from build.rs
use super::io::{load_connectors_for, load_route_files_from, load_sink_defaults};
// no local type imports needed
use crate::structure::GroupExpectSpec;
use orion_conf::error::OrionConfResult;
use orion_error::{ToStructError, UvsValidationFrom};
use std::collections::BTreeMap;
use std::path::Path;
use wp_connector_api::SinkSpec as ResolvedSinkSpec;
use wp_specs::CoreSinkSpec;

/// Bridge CoreSinkSpec to ResolvedSinkSpec (flattened params, empty group/connector)
pub fn core_to_resolved(core: &CoreSinkSpec) -> wp_connector_api::SinkSpec {
    wp_connector_api::SinkSpec {
        group: String::new(),
        name: core.name.clone(),
        kind: core.kind.clone(),
        connector_id: String::new(),
        params: wp_connector_api::parammap_from_toml_table(core.params.clone()),
        filter: core.filter.clone(),
    }
}

/// Bridge CoreSinkSpec to ResolvedSinkSpec with given group and connector id
pub fn core_to_resolved_with(
    core: &CoreSinkSpec,
    group: impl Into<String>,
    connector_id: impl Into<String>,
) -> wp_connector_api::SinkSpec {
    let g = group.into();
    let cid = connector_id.into();
    debug_assert!(
        !cid.is_empty(),
        "connector_id should be non-empty when resolving with connectors (group='{}', name='{}')",
        g,
        core.name
    );
    wp_connector_api::SinkSpec {
        group: g,
        name: core.name.clone(),
        kind: core.kind.clone(),
        connector_id: cid,
        params: core.params.clone(),
        filter: core.filter.clone(),
    }
}

// merge_params 复用 build.rs 的实现，避免重复维护

#[derive(Debug, Clone)]
pub struct ResolvedGroup {
    pub name: String,
    pub expect: Option<GroupExpectSpec>,
    pub sinks: Vec<ResolvedSinkSpec>,
}

pub fn load_business_resolved<P: AsRef<Path>>(sink_root: P) -> OrionConfResult<Vec<ResolvedGroup>> {
    let sink_root_str = sink_root.as_ref().display().to_string();
    let conn_map = load_connectors_for(&sink_root_str)?;
    let routes = load_route_files_from(&super::io::business_dir(&sink_root_str))?;
    let defaults = load_sink_defaults(&sink_root_str)?;
    let mut out = Vec::new();
    for rf in routes {
        let mut sinks = Vec::new();
        for (idx, s) in rf.sink_group.sinks.iter().enumerate() {
            let conn = conn_map.get(s.use_id()).ok_or_else(|| {
                orion_conf::error::ConfIOReason::from_validation(format!(
                    "connector '{}' not found (group '{}')",
                    s.use_id(),
                    rf.sink_group.name
                ))
                .to_err()
            })?;
            let merged = merge_params(&rf.sink_group.name, idx, rf.origin.as_deref(), conn, s)?;
            let name = s
                .inner_name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("[{}]", idx));
            // Bridge via CoreSinkSpec to keep field mapping in one place
            let core = CoreSinkSpec {
                name: name.clone(),
                kind: conn.kind.clone(),
                params: merged,
                filter: s.filter_path().map(|x| x.to_string()),
                tags: Vec::new(),
            };
            sinks.push(core_to_resolved_with(
                &core,
                rf.sink_group.name.clone(),
                conn.id.clone(),
            ));
        }
        // 组级 expect：优先 route，次之 defaults
        let mut expect = rf.sink_group.expect.clone();
        if expect.is_none()
            && let Some(def) = &defaults
        {
            expect = Some(def.expect.clone());
        }
        out.push(ResolvedGroup {
            name: rf.sink_group.name,
            expect,
            sinks,
        });
    }
    Ok(out)
}

pub struct ResolvedInfra {
    pub fixed: BTreeMap<String, (Option<GroupExpectSpec>, Vec<ResolvedSinkSpec>)>,
    pub flexi_monitor: Option<ResolvedGroup>,
}

pub fn load_infra_resolved<P: AsRef<Path>>(sink_root: P) -> OrionConfResult<ResolvedInfra> {
    let sink_root_str = sink_root.as_ref().display().to_string();
    let conn_map = load_connectors_for(&sink_root_str)?;
    let routes = load_route_files_from(&super::io::infra_dir(&sink_root_str))?;
    let defaults = load_sink_defaults(&sink_root_str)?;
    let mut fixed: BTreeMap<String, (Option<GroupExpectSpec>, Vec<ResolvedSinkSpec>)> =
        BTreeMap::new();
    let mut flexi_monitor: Option<ResolvedGroup> = None;
    for rf in routes {
        let name = rf.sink_group.name.clone();
        let mut sinks = Vec::new();
        for (idx, s) in rf.sink_group.sinks.iter().enumerate() {
            let conn = conn_map.get(s.use_id()).ok_or_else(|| {
                orion_conf::error::ConfIOReason::from_validation(format!(
                    "infra group '{}' references unknown connector '{}'",
                    name,
                    s.use_id()
                ))
                .to_err()
            })?;
            let merged = merge_params(&name, idx, rf.origin.as_deref(), conn, s)?;
            let nm = s
                .inner_name()
                .map(|n| n.to_string())
                .unwrap_or_else(|| format!("[{}]", idx));
            let core = CoreSinkSpec {
                name: nm.clone(),
                kind: conn.kind.clone(),
                params: merged,
                filter: s.filter_path().map(|x| x.to_string()),
                tags: Vec::new(),
            };
            sinks.push(core_to_resolved_with(&core, name.clone(), conn.id.clone()));
        }
        let mut expect = rf.sink_group.expect.clone();
        if expect.is_none()
            && let Some(def) = &defaults
        {
            expect = Some(def.expect.clone());
        }
        use super::constants::{GROUP_MONITOR, is_infra_group_name};
        if GROUP_MONITOR == name {
            flexi_monitor = Some(ResolvedGroup {
                name,
                expect,
                sinks,
            });
        } else if is_infra_group_name(&name) {
            fixed.insert(name, (expect, sinks));
        }
    }
    Ok(ResolvedInfra {
        fixed,
        flexi_monitor,
    })
}
