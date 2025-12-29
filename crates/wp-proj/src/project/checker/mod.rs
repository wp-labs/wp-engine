mod options;
mod report;
pub use options::{CheckComponent, CheckComponents, CheckOptions};
use report::{build_detail_table, component_cells};
use std::path::PathBuf;

use super::warp::WarpProject;
use super::{Cell, ConnectorCounts, Row, SourceBreakdown};
use crate::types::CheckStatus;
use orion_conf::UvsConfFrom;
use orion_error::ToStructError;
use wp_cli_core::connectors::{sinks as sink_connectors, sources as source_connectors};
use wp_engine::facade::config::{self as cfg_face, ENGINE_CONF_FILE};
use wp_error::run_error::RunResult;

/// 检查工程（与 `wproj prj check` 语义一致）。
/// 执行全面的项目检查，包括所有组件。
pub fn check_with(
    project: &WarpProject,
    opts: &CheckOptions,
    comps: &CheckComponents,
) -> RunResult<()> {
    let (targets, default_root) = resolve_targets(project, opts);
    let rows = collect_rows(project, &targets, &default_root, opts, comps);
    let stats = summarize_components(&rows, comps);

    render_output(&rows, &stats, opts, comps);

    if has_failures(&rows, comps) {
        return Err(
            wp_error::run_error::RunReason::from_conf("some project checks failed").to_err(),
        );
    }
    Ok(())
}

fn component_stat_value(enabled: bool, count: &ComponentCount) -> serde_json::Value {
    use serde_json::json;
    if enabled {
        json!({ "passed": count.ok, "total": count.total })
    } else {
        serde_json::Value::Null
    }
}

fn resolve_targets(project: &WarpProject, opts: &CheckOptions) -> (Vec<PathBuf>, String) {
    let default_root = if opts.work_root.trim().is_empty() {
        project.work_root().to_string()
    } else {
        opts.work_root.clone()
    };

    let targets = if opts.work_root.trim().is_empty() {
        vec![project.paths().root.clone()]
    } else {
        vec![PathBuf::from(&opts.work_root)]
    };

    (targets, default_root)
}

fn collect_rows(
    project: &WarpProject,
    targets: &[PathBuf],
    default_root: &str,
    opts: &CheckOptions,
    comps: &CheckComponents,
) -> Vec<Row> {
    let mut rows = Vec::new();
    for work in targets.iter() {
        let wrs = if work.as_os_str().is_empty() {
            default_root.to_string()
        } else {
            work.to_string_lossy().to_string()
        };
        let row = evaluate_target(project, &wrs, opts, comps);
        rows.push(row);
    }
    rows
}

fn evaluate_target(
    project: &WarpProject,
    wrs: &str,
    opts: &CheckOptions,
    comps: &CheckComponents,
) -> Row {
    let mut row = Row::new(wrs.to_string());

    if comps.engine {
        row.conf = match cfg_face::load_warp_engine_confs(&wrs) {
            Ok((cm, _)) => {
                row.conf_detail = Some(cm.config_path_string(ENGINE_CONF_FILE));
                Cell::success()
            }
            Err(e) => Cell::failure(e.to_string()),
        };
        if !row.conf.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.conf = Cell::skipped();
    }

    if comps.sources {
        let sources_parse = project.sources_c().check_sources_config().map(|_| ());
        let sources_runtime = project
            .sources_c()
            .check()
            .map_err(|e| e.reason().to_string())
            .map(|_| ());
        let parse_cell = Cell::from_result(sources_parse);
        let runtime_cell = Cell::from_result(sources_runtime);
        row.source_checks = Some(SourceBreakdown {
            syntax: parse_cell.clone(),
            runtime: runtime_cell.clone(),
        });
        row.sources = if parse_cell.ok && runtime_cell.ok {
            Cell::success()
        } else if !parse_cell.ok {
            parse_cell.clone()
        } else {
            runtime_cell.clone()
        };
        if !row.sources.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.sources = Cell::skipped();
        row.source_checks = None;
    }

    if comps.connectors {
        row.connectors = Cell::from_result(project.connectors().check(&wrs).map(|_| ()));
        match collect_connector_counts(&wrs) {
            Ok(stats) => row.connector_counts = Some(stats),
            Err(_e) => {
                row.connector_counts = None;
            }
        }
        if !row.connectors.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.connectors = Cell::skipped();
        row.connector_counts = None;
    }

    if comps.sinks {
        row.sinks = Cell::from_result(
            project
                .sinks_c()
                .check(&wrs)
                .map_err(|e| e.reason().to_string())
                .map(|_| ()),
        );
        if !row.sinks.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.sinks = Cell::skipped();
    }

    if comps.wpl {
        row.wpl = Cell::from_result(
            project
                .wpl()
                .check(&wrs)
                .map_err(|e| e.reason().to_string())
                .map(|_| ()),
        );
        if !row.wpl.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.wpl = Cell::skipped();
    }

    if comps.oml {
        row.oml = match project.oml().check(&wrs) {
            Ok(check_status) => match check_status {
                CheckStatus::Suc => Cell::success(),
                CheckStatus::Miss => Cell::success_with_message("OML 文件缺失".to_string()),
                CheckStatus::Error => Cell::failure("OML 检查错误".to_string()),
            },
            Err(e) => Cell::failure(e.reason().to_string()),
        };
        if !row.oml.ok && opts.fail_fast {
            return row;
        }
    } else {
        row.oml = Cell::skipped();
    }

    row
}

#[derive(Default, Clone, Copy)]
struct ComponentCount {
    ok: usize,
    total: usize,
}

impl ComponentCount {
    fn record(&mut self, passed: bool) {
        self.total += 1;
        if passed {
            self.ok += 1;
        }
    }
}

#[derive(Default)]
struct SummaryCounts {
    conf: ComponentCount,
    connectors: ComponentCount,
    sources: ComponentCount,
    sinks: ComponentCount,
    wpl: ComponentCount,
    oml: ComponentCount,
}

fn summarize_components(rows: &[Row], comps: &CheckComponents) -> SummaryCounts {
    let mut stats = SummaryCounts::default();
    for r in rows {
        if comps.engine {
            stats.conf.record(r.conf.ok);
        }
        if comps.connectors {
            stats.connectors.record(r.connectors.ok);
        }
        if comps.sources {
            stats.sources.record(r.sources.ok);
        }
        if comps.sinks {
            stats.sinks.record(r.sinks.ok);
        }
        if comps.wpl {
            stats.wpl.record(r.wpl.ok);
        }
        if comps.oml {
            stats.oml.record(r.oml.ok);
        }
    }
    stats
}

fn render_output(
    rows: &[Row],
    stats: &SummaryCounts,
    opts: &CheckOptions,
    comps: &CheckComponents,
) {
    if opts.json {
        use serde_json::{Map, Value, json};
        let mut stat = Map::new();
        stat.insert("total".into(), Value::from(rows.len()));
        stat.insert(
            "conf".into(),
            component_stat_value(comps.engine, &stats.conf),
        );
        stat.insert(
            "connectors".into(),
            component_stat_value(comps.connectors, &stats.connectors),
        );
        stat.insert(
            "sources".into(),
            component_stat_value(comps.sources, &stats.sources),
        );
        stat.insert(
            "sinks".into(),
            component_stat_value(comps.sinks, &stats.sinks),
        );
        stat.insert("wpl".into(), component_stat_value(comps.wpl, &stats.wpl));
        stat.insert("oml".into(), component_stat_value(comps.oml, &stats.oml));

        let output = json!({
            "stat": Value::Object(stat),
            "detail": rows
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else if opts.console {
        println!();
        let table = build_detail_table(rows, comps);
        println!("{}", table);
    } else {
        print_text_summary(rows.len(), stats, comps);
        println!("\n{}", build_detail_table(rows, comps));
        output_failure_details(rows, comps);
    }
}

fn print_text_summary(total: usize, stats: &SummaryCounts, comps: &CheckComponents) {
    println!(
        "Project check completed ({} project{})",
        total,
        if total == 1 { "" } else { "s" }
    );
    if comps.engine {
        println!("Config: {}/{} passed", stats.conf.ok, stats.conf.total);
    } else {
        println!("Config: skipped");
    }
    if comps.connectors {
        println!(
            "Connectors: {}/{} passed",
            stats.connectors.ok, stats.connectors.total
        );
    } else {
        println!("Connectors: skipped");
    }
    if comps.sources {
        println!(
            "Sources: {}/{} passed",
            stats.sources.ok, stats.sources.total
        );
    } else {
        println!("Sources: skipped");
    }
    if comps.sinks {
        println!("Sinks: {}/{} passed", stats.sinks.ok, stats.sinks.total);
    } else {
        println!("Sinks: skipped");
    }
    if comps.wpl {
        println!("WPL models: {}/{} passed", stats.wpl.ok, stats.wpl.total);
    } else {
        println!("WPL models: skipped");
    }
    if comps.oml {
        println!("OML models: {}/{} passed", stats.oml.ok, stats.oml.total);
    } else {
        println!("OML models: skipped");
    }
}

fn output_failure_details(rows: &[Row], comps: &CheckComponents) {
    let failed_rows: Vec<_> = rows
        .iter()
        .filter(|r| {
            (comps.engine && !r.conf.ok)
                || (comps.connectors && !r.connectors.ok)
                || (comps.sources && !r.sources.ok)
                || (comps.sinks && !r.sinks.ok)
                || (comps.wpl && !r.wpl.ok)
                || (comps.oml && !r.oml.ok)
        })
        .collect();

    if failed_rows.is_empty() {
        return;
    }

    println!("Failure details:");
    for r in failed_rows {
        for (label, cell) in component_cells(r, comps) {
            if !cell.ok {
                let detail = cell.msg.as_deref().unwrap_or("no error message");
                println!("  - {} -> {}: {}", r.path, label, detail);
            }
        }
    }
}

fn has_failures(rows: &[Row], comps: &CheckComponents) -> bool {
    rows.iter().any(|r| {
        (comps.engine && !r.conf.ok)
            || (comps.connectors && !r.connectors.ok)
            || (comps.sources && !r.sources.ok)
            || (comps.sinks && !r.sinks.ok)
            || (comps.wpl && !r.wpl.ok)
            || (comps.oml && !r.oml.ok)
    })
}

/// 默认检查配置的便捷函数
#[allow(dead_code)]
pub fn check_with_default(project: &WarpProject, opts: &CheckOptions) -> RunResult<()> {
    check_with(project, opts, &CheckComponents::default())
}

fn collect_connector_counts(work_root: &str) -> Result<ConnectorCounts, String> {
    let (_cm, main) = cfg_face::load_warp_engine_confs(work_root).map_err(|e| e.to_string())?;
    let src_rows =
        source_connectors::list_connectors(work_root, &main).map_err(|e| e.to_string())?;
    let src_defs = src_rows.len();
    let src_refs: usize = src_rows.iter().map(|row| row.refs).sum();

    let (sink_map, sink_usage) =
        sink_connectors::list_connectors_usage(work_root).map_err(|e| e.to_string())?;
    let sink_defs = sink_map.len();
    let sink_routes = sink_usage.len();

    Ok(ConnectorCounts {
        source_defs: src_defs,
        source_refs: src_refs,
        sink_defs,
        sink_routes,
    })
}
