use anyhow::Result;
use orion_variate::EnvDict;
use std::path::Path;
use wp_conf::{
    engine::EngineConfig,
    sinks::{load_business_route_confs, load_infra_route_confs},
};
use wpcnt_lib::{SrcLineReport, list_file_sources_with_lines};

/// Sources (file) only
pub fn stat_src_file(
    work_root: &str,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> Result<Option<SrcLineReport>> {
    let ctx = wpcnt_lib::types::Ctx::new(work_root.to_string());
    Ok(wpcnt_lib::list_file_sources_with_lines(
        Path::new(work_root),
        eng_conf,
        &ctx,
        dict,
    ))
}

/// Sinks (file-like) only; caller must pass resolved sink_root
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Vec<wpcnt_lib::types::Row>, u64)> {
    use wpcnt_lib as wlib;
    if !(sink_root.join("business.d").exists() || sink_root.join("infra.d").exists()) {
        anyhow::bail!(
            "缺少 sinks 配置目录：在 '{}' 下未发现 business.d/ 或 infra.d/",
            sink_root.display()
        );
    }
    let mut rows = Vec::new();
    let mut total = 0u64;
    let env_dict = EnvDict::new();
    for conf in load_business_route_confs(sink_root.to_string_lossy().as_ref(), &env_dict)? {
        let g = conf.sink_group;
        if !wlib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = wlib::process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            false,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    for conf in load_infra_route_confs(sink_root.to_string_lossy().as_ref(), &env_dict)? {
        let g = conf.sink_group;
        if !wlib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = wlib::process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            true,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    Ok((rows, total))
}

/// Combined: src-file + sink-file; requires work_root and sink_root
pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &wpcnt_lib::types::Ctx,
    dict: &EnvDict,
) -> Result<(Option<SrcLineReport>, Vec<wpcnt_lib::types::Row>, u64)> {
    let src_rep = list_file_sources_with_lines(Path::new(work_root), eng_conf, ctx, dict);
    let sink_root = Path::new(work_root).join(eng_conf.sink_root());
    let (rows, total) = stat_sink_file(&sink_root, ctx)?;
    Ok((src_rep, rows, total))
}
