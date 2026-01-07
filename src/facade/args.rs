use clap::{Args, Parser, Subcommand};
use orion_conf::ToStructError;
use orion_conf::UvsConfFrom;
use std::{env, path::Path};
use wpl::check_level_or_stop;

//use crate::build::CLAP_LONG_VERSION;
use wp_error::run_error::{RunReason, RunResult};

use orion_overload::conv::val_or;
use wp_conf::RunArgs;
use wp_error::error_handling::RobustnessMode;
use wp_error::error_handling::switch_sys_robust_mode;

use wp_conf::engine::EngineConfig;

#[derive(Parser)]
// `-V/--version` prints version; keep name as wparse to match release package
// `-V/--version` 打印版本号；名称固定为 wparse 以匹配发行包名
#[command(
    name = "wparse",
    version,
    about = "Dayu-Veyron ETL Engine/Dayu-Veyron ETL 引擎"
)]
pub enum WParseCLI {
    /// Run engine in daemon mode (alias of `work --run-mode=daemon`)/以守护进程模式运行引擎（等价于 `work --run-mode=daemon`）
    #[command(name = "daemon", visible_alias = "deamon")]
    Daemon(ParseArgs),

    /// Run engine in batch mode (alias of `work --batch`)/以批处理模式运行引擎（等价于 `work --batch`）
    #[command(name = "batch")]
    Batch(ParseArgs),
}

#[derive(Parser)]
#[command(name = "wpchk", about = "Diagnostic checker/诊断检查器")]
pub enum DvChk {
    #[command(name = "engine")]
    Engine(ParseArgs),
}

#[derive(Args, Debug)]
pub struct DataArgs {
    #[clap(long, default_value = "true")]
    pub local: bool,
    /// Work root directory (absolute); leave empty to use current dir/工作根目录（需使用绝对路径）；不传时默认当前目录
    #[clap(short, long, default_value = "", hide_default_value = true)]
    pub work_root: String,
}

#[derive(Subcommand, Debug)]
#[command(name = "conf")]
pub enum DataCmd {
    /// Check data sources/检查数据源
    Check(DataArgs),
    /// Clean generated data/清理已生成数据
    Clean(DataArgs),
}

#[derive(Parser, Debug, Default)]
#[command(name = "parse")]
pub struct ParseArgs {
    /// Work root directory (absolute path); omit to use current dir/工作根目录（绝对路径）；省略则使用当前目录
    #[clap(long, default_value = "", hide_default_value = true)]
    pub work_root: String,
    /// Execution mode: p=precise, else=automated/执行模式：p=精确，否则=自动
    #[clap(short, long, default_value = "p")]
    pub mode: String,
    /// Max lines to process/最大处理行数
    #[clap(short = 'n', long, default_value = None)]
    pub max_line: Option<usize>,
    /// Parse worker count/并发解析 worker 数
    #[clap(short = 'w', long = "parse-workers")]
    pub parse_workers: Option<usize>,
    /// Stop threshold/停止阈值
    #[clap(short = 'S', long)]
    pub check_stop: Option<usize>,
    /// Continue threshold/继续阈值
    #[clap(short = 's', long)]
    pub check_continue: Option<usize>,
    /// Stats window seconds; fallback to conf [stat].window_sec (default 60)/统计窗口秒数；不传沿用配置 [stat].window_sec（默认 60）
    #[clap(long = "stat")]
    pub stat_sec: Option<usize>,
    /// Robust mode: develop, alpha, beta, online, crucial/鲁棒模式：develop、alpha、beta、online、crucial
    /// e.g. --robust develop/例如：--robust develop
    #[clap(long = "robust")]
    pub robust: Option<RobustnessMode>,
    /// Print stats periodically/周期性打印统计信息
    #[clap(short = 'p', long = "print_stat", default_value = "false")]
    pub stat_print: bool,
    /// Log profile: dev/int/prod (override log_conf.level)/日志预设：dev/int/prod（覆盖配置文件中的 log_conf.level）
    #[clap(long = "log-profile")]
    pub log_profile: Option<String>,
    /// Override WPL models directory; takes precedence over wparse.toml [models].wpl
    /// 覆盖 WPL 模型目录；优先于 wparse.toml 内 [models].wpl 配置
    #[clap(long = "wpl")]
    pub wpl_dir: Option<String>,
}

impl ParseArgs {
    pub fn ensure_work_root_absolute(&mut self) -> RunResult<()> {
        self.work_root = resolve_cli_work_root(&self.work_root)?;
        Ok(())
    }

    pub fn completion_from(&self, conf: &EngineConfig) -> RunResult<RunArgs> {
        let (lev, stop) = check_level_or_stop(self.check_continue, self.check_stop);
        let robust = self.robust.clone().unwrap_or(conf.robust().clone());
        switch_sys_robust_mode(robust);

        Ok(RunArgs {
            line_max: self.max_line,
            parallel: val_or(self.parse_workers, conf.parallel()),
            speed_limit: conf.speed_limit(),
            check: lev,
            check_fail_stop: stop,
            need_complete: true,
            stat_print: self.stat_print,
            // 优先使用配置中的统计窗口；若 CLI 显式覆盖，则以 CLI 为准
            stat_sec: self
                .stat_sec
                .unwrap_or(conf.stat_conf().window_sec.unwrap_or(60) as usize),
            ldm_root: conf.rule_root().to_string(),
            // 阶段开关来自 EngineConfig（也可后续考虑 CLI 覆盖）
            skip_parse: conf.skip_parse(),
            skip_sink: conf.skip_sink(),
            ..Default::default()
        })
    }
}

#[derive(Subcommand, Debug)]
#[command(name = "conf")]
pub enum ConfCmd {
    /// Check config file/检查配置
    #[command(name = "check")]
    Check(ConfCmdArgs),
    /// Initialize config/初始化配置
    Init(ConfCmdArgs),
    /// Clean config/清理配置
    Clean(ConfCmdArgs),
}

impl Default for ConfCmd {
    fn default() -> Self {
        Self::Check(Default::default())
    }
}

#[derive(Args, Debug, Default)]
pub struct ConfCmdArgs {
    /// Work root directory (absolute path); omit to use current dir/工作根目录（绝对路径）；省略则使用当前目录
    #[clap(short, long, default_value = "", hide_default_value = true)]
    pub work_root: String,
    /// Output logs to console (default false)/输出日志到控制台（默认否；false 时仅写文件）
    #[clap(long, default_value_t = false)]
    pub console: bool,
    /// Mode: base(A)=conf+data | env(B)=base+connectors | full(C)=env+models/模式：base(A)=conf+data | env(B)=base+connectors | full(C)=env+models
    #[clap(long = "mode", default_value = "base")]
    pub mode: String,
    /// wpgen: preset conf/wpgen.toml output.connect (generator-only)/wpgen：预置 conf/wpgen.toml 的 output.connect（仅生成器 CLI 使用）
    #[clap(long = "gen-connect")]
    pub gen_connect: Option<String>,
}

impl DataArgs {
    pub fn ensure_work_root_absolute(&mut self) -> RunResult<()> {
        self.work_root = resolve_cli_work_root(&self.work_root)?;
        Ok(())
    }
}

impl ConfCmdArgs {
    pub fn ensure_work_root_absolute(&mut self) -> RunResult<()> {
        self.work_root = resolve_cli_work_root(&self.work_root)?;
        Ok(())
    }
}

fn resolve_cli_work_root(raw: &str) -> RunResult<String> {
    if raw.trim().is_empty() {
        let cwd = env::current_dir().map_err(|err| {
            RunReason::from_conf(format!("获取当前工作目录失败: {}", err)).to_err()
        })?;
        return Ok(cwd.to_string_lossy().to_string());
    }
    let path = Path::new(raw);
    if !path.is_absolute() {
        return RunReason::from_conf(format!(
            "--work_root 必须为绝对路径（传入: '{}'）。可去掉该参数以默认当前目录",
            raw
        ))
        .err_result();
    }
    Ok(path.to_string_lossy().to_string())
}
