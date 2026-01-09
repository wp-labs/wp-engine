//! New wpgen configuration structure (generalized)
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::structure::ConfStdOperation;
use crate::structure::SinkInstanceConf;
use crate::utils::{backup_clean, save_conf};
use orion_conf::error::OrionConfResult;
use orion_variate::EnvDict;
use serde_derive::{Deserialize, Serialize};
use toml;
// no external IO traits for resolved wpgen; handled in loader

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WpGenConfig {
    pub version: String,
    pub generator: GeneratorConfig,
    pub output: OutputConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub presets: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct GeneratorConfig {
    pub mode: GenMode,
    pub count: Option<usize>,
    pub duration_secs: Option<u64>,
    pub speed: usize,
    pub parallel: usize,
    pub rule_root: Option<String>,
    pub sample_pattern: Option<String>,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            mode: GenMode::Rule,
            count: Some(1000),
            duration_secs: None,
            speed: 1000,
            parallel: 1,
            rule_root: None,
            sample_pattern: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum GenMode {
    #[serde(rename = "rule")]
    #[default]
    Rule,
    #[serde(rename = "sample")]
    Sample,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputConfig {
    // 统一走 connectors：connect + params；
    // 仍保留 file/kafka/syslog/stdout 以兼容迁移（旧式到新式），但不鼓励直接使用。
    pub connect: Option<String>,
    #[serde(default)]
    pub params: toml::value::Table,
    pub name: Option<String>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            connect: Some("file_json_sink".to_string()),
            params: toml::value::Table::new(),
            name: None,
        }
    }
}

// Removed OutputType/DataFormat/ErrorHandling: 由 connectors 决定输出类型与格式

// 兼容类型移除：File/Kafka/Syslog/Stdout 等旧式输出定义已废弃

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct LoggingConfig {
    pub level: String,
    pub output: String,
    pub file_path: Option<String>,
    pub format: Option<String>,
    pub rotation: Option<String>,
}

// MonitoringConfig 已移除：旧版 [monitoring] 顶层段将触发未知字段错误（deny_unknown_fields）。

impl WpGenConfig {
    pub fn validate(&self) -> OrionConfResult<()> {
        Ok(())
    }
}

/// 运行期解析后的 wpgen 配置：保留原始新格式，同时给出已解析的输出目标
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WpGenResolved {
    pub conf: WpGenConfig,
    pub out_sink: SinkInstanceConf,
}

// WpGenResolved is assembled by loader; no direct disk IO here

impl LoggingConfig {
    /// 将新格式 logging 映射为运行期使用的 wp_log::conf::LogConf
    pub fn to_log_conf(&self) -> wp_log::conf::LogConf {
        use wp_log::conf::{FileLogConf, LogConf, Output};
        let output = match self.output.as_str() {
            "stdout" | "console" => Output::Console,
            "both" => Output::Both,
            _ => Output::File,
        };
        let file = match &self.file_path {
            Some(p) => Some(FileLogConf { path: p.clone() }),
            None => Some(FileLogConf {
                path: "./logs".to_string(),
            }),
        };
        let mut lc = LogConf::default();
        lc.level = self.level.clone();
        lc.levels = None; // 统一用合成后的 level 字符串解析
        lc.output = output;
        lc.file = file;
        lc
    }
}

use orion_conf::EnvTomlLoad;
impl WpGenConfig {
    /// Load WpGenConfig from a path with generic path parameter support
    pub fn load_from_path<P: AsRef<Path>>(path: P, dict: &EnvDict) -> OrionConfResult<Self> {
        Self::env_load_toml(path.as_ref(), dict)
    }

    /// Initialize WpGenConfig to a path with generic path parameter support
    pub fn init_to_path<P: AsRef<Path>>(path: P) -> OrionConfResult<Self> {
        let mut conf = Self::default();
        conf.output.connect = Some("file_json_sink".to_string());
        conf.output
            .params
            .insert("base".into(), "data/in_dat".into());
        conf.output.params.insert("file".into(), "gen.dat".into());
        conf.logging.file_path = Some("./data/logs".to_string());
        save_conf(Some(conf.clone()), path, true)?;
        Ok(conf)
    }

    /// Safe clean WpGenConfig at path with generic path parameter support
    pub fn safe_clean_at_path<P: AsRef<Path>>(path: P) -> OrionConfResult<()> {
        backup_clean(path)
    }
}

impl ConfStdOperation for WpGenConfig {
    fn load(path: &str, dict: &EnvDict) -> OrionConfResult<Self>
    where
        Self: Sized,
    {
        WpGenConfig::env_load_toml(&PathBuf::from(path), dict)
    }

    fn init(path: &str) -> OrionConfResult<Self>
    where
        Self: Sized,
    {
        let mut conf = WpGenConfig::default();
        conf.output.connect = Some("file_json_sink".to_string());
        conf.output
            .params
            .insert("base".into(), "data/in_dat".into());
        conf.output.params.insert("file".into(), "gen.dat".into());
        conf.logging.file_path = Some("./data/logs".to_string());
        save_conf(Some(conf.clone()), path, true)?;
        Ok(conf)
    }

    fn safe_clean(path: &str) -> OrionConfResult<()> {
        backup_clean(path)
    }
}

#[cfg(test)]
mod tests {
    use super::LoggingConfig;
    #[test]
    fn to_log_conf_uses_plain_level() {
        let lg = LoggingConfig {
            level: "warn".into(),
            output: "file".into(),
            file_path: Some("./data/logs".into()),
            format: None,
            rotation: None,
        };
        let lc = lg.to_log_conf();
        assert_eq!(lc.level, "warn");
        assert_eq!(lc.file.as_ref().unwrap().path, "./data/logs");
    }
}
