use orion_variate::EnvDict;
use std::path::Path;
use wp_engine::facade::config::load_warp_engine_confs;
use wp_error::run_error::RunResult;
use wp_log::conf::LogConf;

/// 通用日志处理器 - 基于WpEngine的LogConf对象进行日志处理
pub struct LogHandler;

impl LogHandler {
    /// 清理日志目录（基于WpEngine的LogConf对象）
    pub fn clean_logs<P: AsRef<Path>>(log_conf: &LogConf, work_root: P) -> RunResult<bool> {
        let work_root = work_root.as_ref();
        if let Some(log_path) = Self::log_path_from_conf(log_conf) {
            Self::clean_log_dir(work_root, &log_path)
        } else {
            Ok(false)
        }
    }

    /// 从工作目录加载配置并清理日志
    pub fn clean_logs_via_config<P: AsRef<Path>>(work_root: P, dict: &EnvDict) -> RunResult<bool> {
        let work_root = work_root.as_ref();
        match load_warp_engine_confs(work_root.to_string_lossy().as_ref(), dict) {
            Ok((_, main_conf)) => {
                let log_conf = main_conf.log_conf();
                Self::clean_logs(log_conf, work_root)
            }
            Err(e) => {
                eprintln!("Warning: Failed to load main config: {}", e);
                Ok(false)
            }
        }
    }

    /// 从WpEngine的LogConf提取日志路径
    fn log_path_from_conf(log_conf: &LogConf) -> Option<String> {
        log_conf.file.as_ref().and_then(|f| {
            let path = f.path.trim();
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
    }

    /// 清理日志目录
    fn clean_log_dir<P: AsRef<Path>>(work_root: P, log_path: &str) -> RunResult<bool> {
        let work_root = work_root.as_ref();
        let log_dir = Path::new(log_path);

        // 如果是相对路径，则与工作目录组合
        let full_log_dir = if log_dir.is_absolute() {
            log_dir.to_path_buf()
        } else {
            work_root.join(log_dir)
        };

        if full_log_dir.exists() {
            match std::fs::remove_dir_all(&full_log_dir) {
                Ok(_) => {
                    println!("✓ Cleaned log directory: {}", full_log_dir.display());
                    Ok(true)
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to remove log directory {}: {}",
                        full_log_dir.display(),
                        e
                    );
                    Ok(false)
                }
            }
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::temp_workdir;

    #[test]
    fn clean_logs_removes_relative_directory() {
        use wp_log::conf::{FileLogConf, LogConf, Output};

        let temp = temp_workdir();
        let log_dir = temp.path().join("data/logs");
        std::fs::create_dir_all(&log_dir).expect("log dir");
        std::fs::write(log_dir.join("test.log"), "line").expect("log file");

        let mut cfg = LogConf::default();
        cfg.output = Output::File;
        cfg.file = Some(FileLogConf {
            path: "./data/logs".to_string(),
        });

        let cleaned = LogHandler::clean_logs(&cfg, temp.path().to_str().unwrap()).unwrap();
        assert!(cleaned);
        assert!(!log_dir.exists());
    }
}
