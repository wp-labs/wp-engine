use orion_error::{ToStructError, UvsConfFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wp_conf::{engine::EngineConfig, utils::find_conf_files};
use wp_engine::facade::config::WPARSE_OML_FILE;
use wp_engine::facade::generator::fetch_oml_data;
use wp_error::run_error::{RunReason, RunResult};

use crate::types::CheckStatus;
use crate::utils::{error_handler::ErrorHandler, PathResolvable, TemplateInitializer};

#[derive(Clone)]
pub struct Oml {
    work_root: PathBuf,
    eng_conf: Arc<EngineConfig>,
}

impl PathResolvable for Oml {
    fn work_root(&self) -> &Path {
        &self.work_root
    }
}

impl Oml {
    pub fn new<P: AsRef<Path>>(work_root: P, eng_conf: Arc<EngineConfig>) -> Self {
        Self {
            work_root: work_root.as_ref().to_path_buf(),
            eng_conf,
        }
    }

    pub fn update_engine_conf(&mut self, eng_conf: Arc<EngineConfig>) {
        self.eng_conf = eng_conf;
    }

    fn oml_root(&self) -> PathBuf {
        self.resolve_path(self.eng_conf.oml_root())
    }

    /// Initialize OML with example content for the specified project directory
    pub fn init_with_examples(&self) -> RunResult<()> {
        let work_root = &self.work_root;
        let example_oml_content = include_str!("../example/oml/nginx.oml");
        if !example_oml_content.contains("name") || !example_oml_content.contains("rule") {
            return ErrorHandler::config_error("example OML content is missing essential fields");
        }

        self.create_example_files(work_root)?;

        println!("OML initialized successfully with example content");
        Ok(())
    }

    /// Create example OML files in the specified project directory
    fn create_example_files(&self, _work_root: &Path) -> RunResult<()> {
        let oml_dir = self.oml_root();
        let initializer = TemplateInitializer::new(oml_dir.clone());

        // Prepare file contents
        let example_oml_content = include_str!("../example/oml/nginx.oml");
        let knowdb_content = r#"# OML Knowledge Database Configuration
# This file defines the OML models available for use

[[models]]
name = "example_oml"
file = "example.oml"
description = "Example OML model for demonstration purposes"
rule = "/example/*"
"#;

        // Write all files using the initializer
        initializer.write_files(&[("example.oml", example_oml_content), ("knowdb.toml", knowdb_content)])?;

        println!("Created example OML files:");
        println!("  - {:?}", oml_dir.join("example.oml"));
        println!("  - {:?}", oml_dir.join("knowdb.toml"));

        Ok(())
    }

    pub fn check(&self) -> RunResult<CheckStatus> {
        let oml_root = self.oml_root();
        if !oml_root.exists() {
            return Ok(CheckStatus::Miss);
        }
        let root_str = oml_root
            .to_str()
            .ok_or_else(|| RunReason::from_conf("OML文件路径无效").to_err())?;
        let oml_files = find_conf_files(root_str, WPARSE_OML_FILE)
            .map_err(|e| RunReason::from_conf(format!("OML 查找失败: {}", e)).to_err())?;
        if oml_files.is_empty() {
            return Err(RunReason::from_conf(format!(
                "OML 文件不存在: {}/{}",
                root_str, WPARSE_OML_FILE
            ))
            .to_err());
        }
        for f in &oml_files {
            ErrorHandler::check_file_not_empty(f, "OML")?;
        }

        fetch_oml_data(root_str, WPARSE_OML_FILE)
            .map_err(|e| RunReason::from_conf(format!("parse oml failed: {}", e)).to_err())?;
        Ok(CheckStatus::Suc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::temp_workdir;
    use std::sync::Arc;
    use wp_conf::engine::EngineConfig;

    #[test]
    fn initialize_examples_creates_valid_files() {
        let temp = temp_workdir();
        let root = temp.path().to_str().unwrap();
        let eng = Arc::new(EngineConfig::init(root));
        let oml = Oml::new(root, eng);
        oml.init_with_examples().expect("init examples");

        let example_file = temp.path().join("models/oml/example.oml");
        assert!(example_file.exists());
        assert!(!temp.path().join("models/oml/*.oml").exists());
    }
}
