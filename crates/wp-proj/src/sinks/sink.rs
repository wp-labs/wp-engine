use orion_error::ErrorConv;
use std::path::{Path, PathBuf};
use wp_cli_core::connectors::sinks as sinks_core;
use wp_error::run_error::RunResult;

use crate::utils::config_path::{ConfigPathResolver, SpecConfPath};

#[derive(Clone)]
pub struct Sinks;

impl Sinks {
    pub fn new() -> Self {
        Sinks
    }

    // 校验路由（严格）
    pub fn check<P: AsRef<Path>>(&self, work_root: P) -> RunResult<()> {
        sinks_core::validate_routes(work_root.as_ref().to_string_lossy().as_ref()).err_conv()
        //.map_err(|e| RunReason::from_conf(e.to_string()).to_err())
    }

    // 展平成路由表（biz+infra），带过滤
    pub fn route_rows<P: AsRef<Path>>(
        &self,
        work_root: P,
        group_names: &[String],
        sink_names: &[String],
    ) -> RunResult<Vec<sinks_core::RouteRow>> {
        sinks_core::route_table(
            work_root.as_ref().to_string_lossy().as_ref(),
            group_names,
            sink_names,
        )
        .err_conv()
        //.map_err(|e| RunReason::from_conf(e.to_string()).to_err())
    }

    // 初始化 sinks 骨架（写入配置指定的sink目录，如果配置不存在则使用默认路径）
    pub fn init<P: AsRef<Path>>(&self, work_root: P) -> RunResult<()> {
        // 使用统一的路径解析器
        let sink_root = SpecConfPath::topology(PathBuf::from(work_root.as_ref()), "sinks")?;

        Self::ensure_defaults_file(&sink_root)?;
        Self::ensure_business_demo(&sink_root)?;
        Self::ensure_infra_defaults(&sink_root.join("infra.d"))?;
        Ok(())
    }

    fn ensure_defaults_file(sink_root: &std::path::Path) -> RunResult<()> {
        let p = sink_root.join("defaults.toml");
        let should_write = if p.exists() {
            match std::fs::read_to_string(&p) {
                Ok(body) => body.trim().is_empty(),
                Err(_) => true,
            }
        } else {
            true
        };
        if should_write {
            let body = include_str!("../example/topology/sinks/defaults.toml");
            ConfigPathResolver::write_file_with_dir(&p, body)?;
        }
        Ok(())
    }

    fn ensure_business_demo(sink_root: &std::path::Path) -> RunResult<()> {
        let biz = sink_root.join("business.d");
        ConfigPathResolver::ensure_dir_exists(&biz)?;
        let demo = biz.join("demo.toml");
        if !demo.exists() {
            let demo_content = include_str!("../example/topology/sinks/business.d/demo.toml");
            ConfigPathResolver::write_file_with_dir(&demo, demo_content)?;
        }
        Ok(())
    }

    fn ensure_infra_defaults(dir: &std::path::Path) -> RunResult<()> {
        ConfigPathResolver::ensure_dir_exists(dir)?;

        for (name, body) in [
            (
                "default.toml",
                include_str!("../example/topology/sinks/infra.d/default.toml"),
            ),
            (
                "miss.toml",
                include_str!("../example/topology/sinks/infra.d/miss.toml"),
            ),
            (
                "residue.toml",
                include_str!("../example/topology/sinks/infra.d/residue.toml"),
            ),
            (
                "error.toml",
                include_str!("../example/topology/sinks/infra.d/error.toml"),
            ),
            (
                "monitor.toml",
                include_str!("../example/topology/sinks/infra.d/monitor.toml"),
            ),
        ] {
            let path = dir.join(name);
            if !path.exists() {
                ConfigPathResolver::write_file_with_dir(&path, body)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{temp_workdir, write_basic_wparse_config};

    #[test]
    fn init_populates_sink_templates() {
        let temp = temp_workdir();
        write_basic_wparse_config(temp.path());
        let sinks = Sinks::new();
        sinks
            .init(temp.path().to_str().unwrap())
            .expect("init sinks");

        let sink_root = temp.path().join("topology/sinks");
        assert!(sink_root.join("defaults.toml").exists());
        assert!(sink_root.join("business.d/demo.toml").exists());
        assert!(sink_root.join("infra.d/default.toml").exists());
    }
}
