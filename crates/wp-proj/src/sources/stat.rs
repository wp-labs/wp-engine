use orion_conf::{ToStructError, UvsConfFrom};
use orion_variate::EnvDict;
use wp_engine::facade::config;
use wp_error::run_error::{RunReason, RunResult};

/// Result structure for file source statistics
///
/// This struct contains the results of analyzing file-based data sources,
/// including the work root directory path and a detailed line count report.
pub struct SourceStatResult {
    /// The resolved work root directory path
    pub work_root: String,
    /// Optional report containing line count statistics for each file source
    pub report: Option<wpcnt_lib::SrcLineReport>,
}

/// Statistics module for file-based sources
///
/// This module provides functionality to analyze and gather statistics
/// from file-based data sources configured in the project.
///
pub fn stat_file_sources(work_root: &str, dict: &EnvDict) -> RunResult<SourceStatResult> {
    // Load engine configuration to get source settings
    let (cm, main) = config::load_warp_engine_confs(work_root, dict).map_err(|e| {
        RunReason::from_conf(format!("Failed to load engine config: {}", e)).to_err()
    })?;

    // Resolve the actual work root path
    let resolved = cm.work_root_path();

    // Gather statistics from file sources
    let report = wp_cli_core::obs::stat::stat_src_file(&resolved, &main, dict).map_err(|e| {
        RunReason::from_conf(format!(
            "Failed to collect source statistics from '{}': {}",
            resolved, e
        ))
        .to_err()
    })?;

    // Return the statistics result
    Ok(SourceStatResult {
        work_root: resolved,
        report,
    })
}
