use orion_conf::{ToStructError, UvsConfFrom};
use wp_engine::facade::config;
use wp_error::run_error::{RunReason, RunResult};

/// 文件型源统计结果。
pub struct SourceStatResult {
    pub work_root: String,
    pub report: Option<wpcnt_lib::SrcLineReport>,
}

/// 统计文件型源的输入行数。
pub fn stat_file_sources(work_root: &str) -> RunResult<SourceStatResult> {
    let (cm, main) = config::load_warp_engine_confs(work_root)?;
    let resolved = cm.work_root_path();
    let report = wp_cli_core::obs::stat::stat_src_file(&resolved, &main)
        .map_err(|e| RunReason::from_conf(e.to_string()).to_err())?;
    Ok(SourceStatResult {
        work_root: resolved,
        report,
    })
}
