use super::common::{
    DEFAULT_UNIT_SIZE, build_sink_instance, default_batch, per_pipeline_speed,
    split_total_among_parallel,
};
use crate::orchestrator::config::models::stat_reqs_from;
use crate::runtime::actor::TaskGroup;
use crate::runtime::actor::signal::ShutdownCmd;
use crate::runtime::generator::rule_source::RuleGenSource;
use crate::runtime::generator::types::GenGRA;
use crate::runtime::supervisor::monitor::ActorMonitor;
use crate::sinks::SinkBackendType;
use orion_error::{ErrorOwe, ErrorWith, UvsReason};
use orion_variate::EnvDict;
use tokio::task::JoinHandle;
use wp_conf::stat::StatConf;
use wp_conf::structure::SinkInstanceConf;
use wp_error::run_error::{RunError, RunErrorOwe, RunResult};
use wp_log::info_ctrl;
use wp_stat::StatRecorder;
use wp_stat::StatStage; // for record_task

/// 批量发送一个“单元”的规则生成结果（逐条生成+发送，但作为一个批次）。
async fn send_unit_rules(
    sink: &mut SinkBackendType,
    src: &std::sync::Arc<RuleGenSource>,
    cur_idx: &mut usize,
    unit_cnt: usize,
    collectors: &mut crate::stat::metric_collect::MetricCollectors,
) -> RunResult<usize> {
    use wp_stat::StatRecorder; // bring trait for record_task
    let rules_len = src.rule_len().max(1);
    let mut sent = 0usize;
    for _ in 0..unit_cnt {
        let ffv = src.gen_one(*cur_idx).map_err(|e| {
            RunError::from(wp_error::run_error::RunReason::Uvs(UvsReason::core_conf(
                e.to_string(),
            )))
        })?;
        *cur_idx = (*cur_idx + 1) % rules_len;
        // 将 FmtFieldVec 转换为字符串并调用 sink_str
        let raw_line = wpl::generator::RAWGenFmt(&ffv).to_string();
        wp_connector_api::AsyncRawDataSink::sink_str(sink, &raw_line)
            .await
            .owe_sink()?;
        collectors.record_task("gen_direct_rule", ());
        sent += 1;
    }
    Ok(sent)
}

pub async fn run_rule_direct(
    rule_root: &str,
    gar: &GenGRA,
    out_conf: &SinkInstanceConf,
    rate_limit_rps: usize,
    dict: &EnvDict,
) -> RunResult<()> {
    // 全局限速目标（构建期提示）：生成器直连路径在构建 sink 前设置；0 表示无限速。
    crate::sinks::set_global_rate_limit_rps(gar.gen_speed);
    // 全局 backoff gate 移除；由发送单元在构建期与实时水位自决。
    info_ctrl!(
        "run_rule_direct: rule_root='{}', parallel={}, total_line={:?}",
        rule_root,
        gar.parallel,
        gar.total_line
    );
    let units = crate::core::generator::rules::load_gen_confs(rule_root, dict)
        .owe_rule()
        .want("load rule")?;
    info_ctrl!("run_rule_direct: loaded {} rule units", units.len());
    let source = RuleGenSource::from_units(units).map_err(|e| {
        RunError::from(wp_error::run_error::RunReason::Uvs(UvsReason::core_conf(
            e.to_string(),
        )))
    })?;
    let source = std::sync::Arc::new(source);
    let parallel = std::cmp::max(1, gar.parallel);
    let batch = default_batch();
    info_ctrl!("run_rule_direct: batch={} (const)", batch);

    // 任务分配：平均 + 余数前置
    let per_counts = split_total_among_parallel(parallel, gar.total_line);

    // 监控
    let moni_group = TaskGroup::new("moni", ShutdownCmd::Timeout(200));
    let mut actor_mon =
        ActorMonitor::new(moni_group.subscribe(), None, gar.stat_print, gar.stat_sec);
    let mon_s = actor_mon.send_agent();
    let stat_reqs = stat_reqs_from(&StatConf::gen_default());
    let sink_reqs = stat_reqs.get_requ_items(StatStage::Sink);
    tokio::spawn(async move {
        let _ = actor_mon.stat_proc(Vec::new()).await;
    });

    // 速率配置：global gen_speed 平均到 pipeline
    let per_speed = per_pipeline_speed(gar.gen_speed, parallel);

    // 启动流水线
    let start_at = std::time::Instant::now();
    let mut tasks: Vec<JoinHandle<RunResult<usize>>> = Vec::with_capacity(parallel);
    for (i, cnt) in per_counts.iter().copied().enumerate().take(parallel) {
        let sink = build_sink_instance(out_conf, i, parallel, rate_limit_rps).await?;
        let src = source.clone();
        let mon = mon_s.clone();
        let reqs = sink_reqs.clone();
        info_ctrl!("run_rule_direct: spawn pipeline {} with count={:?}", i, cnt);
        let unit_size_cfg = DEFAULT_UNIT_SIZE;
        tasks.push(tokio::spawn(async move {
            run_rule_pipeline(sink, src, cnt, batch, per_speed, unit_size_cfg, mon, reqs).await
        }));
    }

    let mut total_produced: usize = 0;
    for t in tasks {
        let n = t.await.map_err(|e| {
            RunError::from(wp_error::run_error::RunReason::Uvs(UvsReason::core_conf(
                e.to_string(),
            )))
        })??;
        total_produced += n;
    }
    info_ctrl!("run_rule_direct: all pipelines finished");
    let elapsed = start_at.elapsed();
    let ms = elapsed.as_millis();
    info_ctrl!(
        "run_rule_direct: summary generated={} lines, elapsed={} ms, parallel={}, batch={}",
        total_produced,
        ms,
        parallel,
        batch
    );
    println!(
        "wpgen summary: generated={} lines, elapsed={} ms, mode=direct, parallel={}, batch={}",
        total_produced, ms, parallel, batch
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_rule_pipeline(
    mut sink: SinkBackendType,
    src: std::sync::Arc<RuleGenSource>,
    count: Option<usize>,
    _batch: usize,
    per_pipe_speed: Option<usize>,
    unit_size_cfg: usize,
    mon_s: crate::stat::MonSend,
    sink_reqs: Vec<wp_stat::StatReq>,
) -> RunResult<usize> {
    use crate::runtime::actor::limit::RateLimiter;
    use crate::stat::metric_collect::MetricCollectors;

    // 统计/速率器
    let mut collectors = MetricCollectors::new("gen_direct_rule".to_string(), sink_reqs);
    let unit_size = match per_pipe_speed {
        Some(per) => (per / 10).clamp(1, 1000),
        None => unit_size_cfg.max(1),
    };
    let mut limiter = per_pipe_speed
        .map(|per| RateLimiter::new(per, unit_size, "gen_direct_rule"))
        .unwrap_or_default();

    // 迭代状态
    let mut produced = 0usize;
    let mut cur_idx = 0usize; // 当前规则索引起点
    let total_limit = count; // 可选总量限制

    // 批量发送一个“单元”，然后统一进行限速；统计以 1W 行为粒度计数，保持表格速度单位与“W(万)”对齐。
    const REPORT_LINES_PER_TASK: usize = 10_000; // 1W 行 -> 1 个 task
    let mut acc_lines: usize = 0;
    loop {
        if let Some(limit) = total_limit
            && produced >= limit
        {
            break;
        }
        limiter.rec_beg();
        let left_global = total_limit
            .map(|l| l.saturating_sub(produced))
            .unwrap_or(usize::MAX);
        if left_global == 0 {
            break;
        }
        let take = unit_size.min(left_global);
        let sent = send_unit_rules(&mut sink, &src, &mut cur_idx, take, &mut collectors).await?;
        produced += sent;
        acc_lines += sent;
        let mut reported = 0usize;
        while acc_lines >= REPORT_LINES_PER_TASK {
            collectors.record_task("gen_direct_rule", ());
            acc_lines -= REPORT_LINES_PER_TASK;
            reported += 1;
        }
        if reported > 0 {
            let _ = collectors.send_stat(&mon_s).await;
        }
        let left = limiter.limit_speed_time();
        if left.as_nanos() > 0 {
            tokio::time::sleep(left).await;
        }
    }
    Ok(produced)
}
