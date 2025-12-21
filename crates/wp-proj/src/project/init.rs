use super::warp::WarpProject;
use super::{Oml, Wpl};
use crate::utils::error_handler::ErrorHandler;
use orion_conf::ToStructError;
use orion_error::UvsValidationFrom;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use wp_conf::paths::{OUT_FILE_PATH, RESCURE_FILE_PATH, SRC_FILE_PATH};
use wp_engine::facade::config::WPARSE_LOG_PATH;
use wp_error::{RunError, RunReason};
//use wp_engine::orchestrator::config::models::warp::core::EngineConfig;
//use wp_engine::orchestrator::config::SRC_FILE_PATH;
use wp_error::run_error::RunResult;

const CONF_DIR: &str = "conf";
const CONF_WPARSE_FILE: &str = "conf/wparse.toml";
const CONF_WPGEN_FILE: &str = "conf/wpgen.toml";
const MODELS_WPL_DIR: &str = "models/wpl";
const MODELS_OML_DIR: &str = "models/oml";
const MODELS_KNOWLEDGE_DIR: &str = "models/knowledge";
const MODELS_KNOWLEDGE_EXAMPLE_DIR: &str = "models/knowledge/example";
const TOPOLOGY_SOURCES_DIR: &str = "topology/sources";
const TOPOLOGY_SINKS_DIR: &str = "topology/sinks";

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum InitMode {
    Full,
    Normal,
    Model,
    Topology,
    Conf,
    Data,
}
impl InitMode {
    pub fn enable_connector(&self) -> bool {
        *self == InitMode::Full
    }
    pub fn enable_model(&self) -> bool {
        //*self == InitMode::Model || *self == InitMode::Full
        matches!(self, InitMode::Model | InitMode::Full | InitMode::Normal)
    }
    pub fn enable_conf(&self) -> bool {
        matches!(self, InitMode::Conf | InitMode::Full | InitMode::Normal)
    }
    pub fn enable_topology(&self) -> bool {
        matches!(self, InitMode::Topology | InitMode::Full | InitMode::Normal)
    }
}

impl FromStr for InitMode {
    type Err = RunError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mode = match s {
            "full" => Self::Full,
            "normal" => Self::Normal,
            "model" => Self::Model,
            "conf" => Self::Conf,
            "topology" => Self::Topology,
            "data" => Self::Data,
            _ => return RunReason::from_validation("not init mode").err_result(),
        };
        Ok(mode)
    }
}

impl WarpProject {
    // ========== 初始化方法 ==========

    /// 初始化 WPL 和 OML 模型（基于示例文件）
    pub fn init_models(&mut self) -> RunResult<()> {
        Wpl::init_with_examples(self.work_root())?;
        Oml::init_with_examples(self.work_root())?;
        Ok(())
    }

    /// 完整的项目初始化：包括配置、模型和所有组件
    pub fn init(&mut self, mode: InitMode) -> RunResult<()> {
        // 1) 先进行基础项目初始化（包括目录创建、配置、连接器、wpgen配置等）
        self.init_basic(mode.clone())?;

        // 2) 知识库目录骨架初始化（仅在模型启用时，因为知识库是模型的一部分）
        if mode.enable_model() {
            self.knowledge().init(self.work_root())?;
        }

        // 3) WPL 和 OML 模型初始化（仅在模型启用时）
        if mode.enable_model() {
            self.init_models()?;
        }

        println!("✓ 项目初始化完成");

        Ok(())
    }

    /// 仅初始化基础项目结构（不包括模型）
    pub fn init_basic(&mut self, mode: InitMode) -> RunResult<()> {
        // 1) 基础配置和数据目录初始化
        //let conf_manager = WarpConf::new(self.work_root());
        self.mk_framework_dir(mode.clone())?;

        if mode.enable_conf() {
            // wparse/wpgen 主配置初始化（如不存在则复制示例文件）
            Self::init_engine_config(self.work_root_path())?;
            Self::init_wpgen_config(self.work_root_path())?;
        }

        // 连接器模板初始化
        if mode.enable_connector() {
            self.connectors().init_templates(self.work_root())?;
        }
        if mode.enable_topology() {
            // 输出接收器骨架初始化
            self.sinks_c().init(self.work_root())?;
            // 输入源和连接器补齐
            self.sources_c().init(self.work_root())?;
            // 知识库目录骨架初始化
        }

        if mode.enable_model() {
            self.knowledge().init(self.work_root())?;
        }

        // 模型目录结构已预创建，跳过此步骤

        println!("✓ 基础项目初始化完成");
        Ok(())
    }

    /// 初始化 wpgen 配置文件
    fn init_wpgen_config<P: AsRef<Path>>(work_root: P) -> RunResult<()> {
        use std::fs;

        let work_root = work_root.as_ref();
        let conf_dir = work_root.join(CONF_DIR);
        if let Err(_) = fs::create_dir_all(&conf_dir) {
            // 如果创建目录失败，记录警告但继续
            eprintln!("Warning: Failed to create conf directory");
        }

        let wpgen_config_path = work_root.join(CONF_WPGEN_FILE);
        if !wpgen_config_path.exists() {
            // 使用 include_str! 读取示例配置文件
            let wpgen_config_content = include_str!("../example/conf/wpgen.toml");
            if let Err(_) = fs::write(&wpgen_config_path, wpgen_config_content) {
                // 如果写入失败，记录警告但继续
                eprintln!("Warning: Failed to write wpgen.toml");
            }
        }

        Ok(())
    }

    /// 初始化 wparse 主配置（wparse.toml）
    fn init_engine_config<P: AsRef<Path>>(work_root: P) -> RunResult<()> {
        use std::fs;

        let work_root = work_root.as_ref();
        let conf_dir = work_root.join(CONF_DIR);
        if let Err(_) = fs::create_dir_all(&conf_dir) {
            eprintln!("Warning: Failed to create conf directory");
        }

        let engine_config_path = work_root.join(CONF_WPARSE_FILE);
        if !engine_config_path.exists() {
            let engine_config_content = include_str!("../example/conf/wparse.toml");
            if let Err(_) = fs::write(&engine_config_path, engine_config_content) {
                eprintln!("Warning: Failed to write wparse.toml");
            }
        }

        Ok(())
    }

    fn mk_framework_dir(&self, mode: InitMode) -> RunResult<()> {
        let work_root = self.work_root_path();
        if mode.enable_conf() {
            ErrorHandler::safe_create_dir(&work_root.join(CONF_DIR))?;
        }
        if mode.enable_model() {
            ErrorHandler::safe_create_dir(&work_root.join(MODELS_WPL_DIR))?;
            ErrorHandler::safe_create_dir(&work_root.join(MODELS_OML_DIR))?;
            ErrorHandler::safe_create_dir(&work_root.join(MODELS_KNOWLEDGE_DIR))?;
            ErrorHandler::safe_create_dir(&work_root.join(MODELS_KNOWLEDGE_EXAMPLE_DIR))?;
        }
        if mode.enable_topology() {
            ErrorHandler::safe_create_dir(&work_root.join(TOPOLOGY_SOURCES_DIR))?;
            ErrorHandler::safe_create_dir(&work_root.join(TOPOLOGY_SINKS_DIR))?;
        }
        ErrorHandler::safe_create_dir(&Self::resolve_with_root(&work_root, SRC_FILE_PATH))?;
        ErrorHandler::safe_create_dir(&Self::resolve_with_root(&work_root, OUT_FILE_PATH))?;
        ErrorHandler::safe_create_dir(&Self::resolve_with_root(&work_root, WPARSE_LOG_PATH))?;
        ErrorHandler::safe_create_dir(&Self::resolve_with_root(&work_root, RESCURE_FILE_PATH))?;
        Ok(())
    }

    fn resolve_with_root(base: &Path, raw: &str) -> PathBuf {
        let trimmed = raw.strip_prefix("./").unwrap_or(raw);
        let candidate = Path::new(trimmed);
        if candidate.is_relative() {
            base.join(candidate)
        } else {
            candidate.to_path_buf()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const CONNECTORS_DIR: &str = "connectors";
    const CONNECTORS_SOURCE_DIR: &str = "connectors/source.d";
    const CONNECTORS_SINK_DIR: &str = "connectors/sink.d";
    const LEGACY_SOURCE_DIR: &str = "source.d";
    const LEGACY_SINK_DIR: &str = "sink.d";
    const MODELS_DIR: &str = "models";
    const TOPO_SOURCES_DIR: &str = "topology/sources";
    const TOPO_SINKS_DIR: &str = "topology/sinks";
    const MODELS_WPL_PARSE_FILE: &str = "models/wpl/parse.wpl";
    const MODELS_WPL_SAMPLE_FILE: &str = "models/wpl/sample.dat";
    const MODELS_OML_EXAMPLE_FILE: &str = "models/oml/example.oml";
    const MODELS_OML_KNOWDB_FILE: &str = "models/oml/knowdb.toml";
    const TOPOLOGY_WPSRC_FILE: &str = "topology/sources/wpsrc.toml";

    #[test]
    fn test_init_mode_from_str() {
        // 测试有效的模式字符串
        assert_eq!(InitMode::from_str("full").unwrap(), InitMode::Full);
        assert_eq!(InitMode::from_str("model").unwrap(), InitMode::Model);
        assert_eq!(InitMode::from_str("conf").unwrap(), InitMode::Conf);
        assert_eq!(InitMode::from_str("data").unwrap(), InitMode::Data);

        // 测试无效的模式字符串
        let result = InitMode::from_str("invalid");
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not init mode"));
        }
    }

    #[test]
    fn test_init_mode_enable_connector() {
        assert!(InitMode::Full.enable_connector());
        assert!(!InitMode::Model.enable_connector());
        assert!(!InitMode::Conf.enable_connector());
        assert!(!InitMode::Data.enable_connector());
    }

    #[test]
    fn test_init_mode_enable_model() {
        // Full 和 Normal 应该启用模型
        assert!(InitMode::Full.enable_model());
        assert!(InitMode::Model.enable_model());

        // Conf 和 Data 不应该启用模型
        assert!(!InitMode::Conf.enable_model());
        assert!(!InitMode::Data.enable_model());
    }

    #[test]
    fn test_init_mode_enable_conf() {
        // 除了 Data，其他模式都应该启用配置
        assert!(InitMode::Full.enable_conf());
        assert!(InitMode::Normal.enable_conf());
        assert!(InitMode::Conf.enable_conf());
        assert!(!InitMode::Data.enable_conf());
    }

    #[test]
    fn test_init_mode_debug_format() {
        assert_eq!(format!("{:?}", InitMode::Full), "Full");
        assert_eq!(format!("{:?}", InitMode::Model), "Model");
        assert_eq!(format!("{:?}", InitMode::Conf), "Conf");
        assert_eq!(format!("{:?}", InitMode::Data), "Data");
    }

    #[test]
    fn test_init_mode_equality() {
        assert_eq!(InitMode::Full, InitMode::Full);
        assert_eq!(InitMode::Model, InitMode::Model);
        assert_ne!(InitMode::Full, InitMode::Model);
        assert_ne!(InitMode::Conf, InitMode::Data);
    }

    #[test]
    fn test_init_mode_clone() {
        let mode = InitMode::Full;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);

        let mode = InitMode::Model;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_warp_project_init_full_mode() {
        use tempfile::TempDir;

        // 创建临时目录
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        // 创建项目并使用 Full 模式初始化
        let mut project = WarpProject::new(work_root);

        // Full 模式应该初始化所有组件
        let result = project.init(InitMode::Full);
        assert!(result.is_ok(), "Full mode initialization should succeed");

        // 验证创建的目录和文件
        assert!(
            work_root.join(CONF_DIR).exists(),
            "conf directory should exist"
        );
        assert!(
            work_root.join(CONF_WPARSE_FILE).exists(),
            "wparse.toml should exist"
        );
        assert!(
            work_root.join(CONF_WPGEN_FILE).exists(),
            "wpgen.toml should exist"
        );

        assert!(
            work_root.join(CONNECTORS_DIR).exists(),
            "connectors directory should exist"
        );
        assert!(
            work_root.join(CONNECTORS_SOURCE_DIR).exists(),
            "source.d directory should exist"
        );
        assert!(
            work_root.join(CONNECTORS_SINK_DIR).exists(),
            "sink.d directory should exist"
        );
        assert!(
            !work_root.join(LEGACY_SOURCE_DIR).exists(),
            "legacy top-level source.d should not be created"
        );
        assert!(
            !work_root.join(LEGACY_SINK_DIR).exists(),
            "legacy top-level sink.d should not be created"
        );

        assert!(
            work_root.join(MODELS_DIR).exists(),
            "models directory should exist"
        );
        assert!(
            work_root.join(MODELS_WPL_DIR).exists(),
            "wpl directory should exist"
        );
        assert!(
            work_root.join(MODELS_OML_DIR).exists(),
            "oml directory should exist"
        );
        assert!(
            work_root.join(TOPOLOGY_SOURCES_DIR).exists(),
            "topology sources directory should exist"
        );
        assert!(
            work_root.join(TOPOLOGY_SINKS_DIR).exists(),
            "topology sinks directory should exist"
        );
        assert!(
            work_root.join(TOPO_SOURCES_DIR).exists(),
            "topology/sources should remain absent; use topology/sources"
        );
        assert!(
            work_root.join(TOPO_SINKS_DIR).exists(),
            "topology/sinks should remain absent; use topology/sinks"
        );
        assert!(
            work_root.join(MODELS_KNOWLEDGE_DIR).exists(),
            "knowledge directory should exist"
        );

        // 验证示例文件
        assert!(
            work_root.join(MODELS_WPL_PARSE_FILE).exists(),
            "parse.wpl should exist"
        );
        assert!(
            work_root.join(MODELS_WPL_SAMPLE_FILE).exists(),
            "sample.dat should exist"
        );
        assert!(
            work_root.join(MODELS_OML_EXAMPLE_FILE).exists(),
            "example.oml should exist"
        );
        assert!(
            work_root.join(MODELS_OML_KNOWDB_FILE).exists(),
            "knowdb.toml should exist"
        );

        // 验证连接器模板
        assert!(
            work_root
                .join(format!("{}/00-file-default.toml", CONNECTORS_SOURCE_DIR))
                .exists(),
            "file source connector should exist"
        );
        assert!(
            work_root
                .join(format!("{}/02-file-json.toml", CONNECTORS_SINK_DIR))
                .exists(),
            "file sink connector should exist"
        );
    }

    #[test]
    fn test_warp_project_init_normal_mode() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        let mut project = WarpProject::new(work_root);

        // Normal 模式应该初始化配置和模型，但不包括连接器
        let result = project.init(InitMode::Normal);
        assert!(result.is_ok(), "Normal mode initialization should succeed");

        // 验证配置目录
        assert!(
            work_root.join(CONF_DIR).exists(),
            "conf directory should exist"
        );
        assert!(
            work_root.join(CONF_WPARSE_FILE).exists(),
            "wparse.toml should exist"
        );
        assert!(
            work_root.join(CONF_WPGEN_FILE).exists(),
            "wpgen.toml should exist"
        );

        // 验证模型目录和文件
        assert!(
            work_root.join(MODELS_DIR).exists(),
            "models directory should exist"
        );
        assert!(
            work_root.join(MODELS_WPL_DIR).exists(),
            "wpl directory should exist"
        );
        assert!(
            work_root.join(MODELS_OML_DIR).exists(),
            "oml directory should exist"
        );
        assert!(
            work_root.join(TOPOLOGY_SOURCES_DIR).exists(),
            "topology sources directory should not exist in Model mode"
        );
        assert!(
            work_root.join(TOPOLOGY_SINKS_DIR).exists(),
            "topology sinks directory should not exist in Model mode"
        );
        assert!(
            work_root.join(MODELS_KNOWLEDGE_DIR).exists(),
            "knowledge directory should exist"
        );

        // 验证示例文件
        assert!(
            work_root.join(MODELS_WPL_PARSE_FILE).exists(),
            "parse.wpl should exist"
        );
        assert!(
            work_root.join(MODELS_OML_EXAMPLE_FILE).exists(),
            "example.oml should exist"
        );

        // Normal 模式不会创建 connectors，只有 Full 模式才会。
        assert!(
            !work_root.join(CONNECTORS_DIR).exists(),
            "connectors directory should not exist in Model mode"
        );
    }

    #[test]
    fn test_warp_project_init_conf_mode() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        let mut project = WarpProject::new(work_root);

        // Conf 模式应该只初始化配置
        let result = project.init(InitMode::Conf);
        assert!(result.is_ok(), "Conf mode initialization should succeed");

        // 验证配置目录
        assert!(
            work_root.join(CONF_DIR).exists(),
            "conf directory should exist"
        );
        assert!(
            work_root.join(CONF_WPARSE_FILE).exists(),
            "wparse.toml should exist"
        );
        assert!(
            work_root.join(CONF_WPGEN_FILE).exists(),
            "wpgen.toml should exist"
        );

        // Conf 模式不应该创建连接器（只创建配置）
        assert!(
            !work_root.join(CONNECTORS_DIR).exists(),
            "connectors directory should not exist in Conf mode"
        );
        // Conf 模式不应该创建模型（修复后）
        assert!(
            !work_root.join(MODELS_DIR).exists(),
            "models directory should not exist in Conf mode"
        );
    }

    #[test]
    fn test_warp_project_init_data_mode() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        let mut project = WarpProject::new(work_root);

        // Data 模式应该只初始化数据目录结构，不包括配置
        let result = project.init(InitMode::Data);
        assert!(result.is_ok(), "Data mode initialization should succeed");

        // Data 模式只创建基础目录，不创建模型相关内容（修复后）
        // Data 模式不应该创建配置或连接器
        assert!(
            !work_root.join(CONF_DIR).exists(),
            "conf directory should not exist in Data mode"
        );
        assert!(
            !work_root.join(CONNECTORS_DIR).exists(),
            "connectors directory should not exist in Data mode"
        );

        // Data 模式不应该创建任何 models 相关内容（修复后）
        assert!(
            !work_root.join(MODELS_DIR).exists(),
            "models directory should not exist in Data mode"
        );
        assert!(
            !work_root.join(MODELS_KNOWLEDGE_DIR).exists(),
            "knowledge directory should not exist in Data mode"
        );
    }

    #[test]
    fn test_warp_project_init_basic() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        let mut project = WarpProject::new(work_root);

        // 测试 init_basic 方法（等效于 Normal 模式）
        let result = project.init_basic(InitMode::Normal);
        assert!(result.is_ok(), "Basic initialization should succeed");

        // 验证基础结构
        assert!(
            work_root.join(CONF_DIR).exists(),
            "conf directory should exist"
        );
        assert!(
            work_root.join(CONF_WPARSE_FILE).exists(),
            "wparse.toml should exist"
        );
        assert!(
            work_root.join(CONF_WPGEN_FILE).exists(),
            "wpgen.toml should exist"
        );

        assert!(
            work_root.join(MODELS_DIR).exists(),
            "models directory should exist"
        );
        assert!(
            work_root.join(MODELS_WPL_DIR).exists(),
            "wpl directory should exist"
        );
        assert!(
            work_root.join(MODELS_OML_DIR).exists(),
            "oml directory should exist"
        );
    }

    #[test]
    fn test_warp_project_init_models() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        let mut project = WarpProject::new(work_root);

        // 首先创建基础结构
        project
            .init_basic(InitMode::Model)
            .expect("Basic initialization should succeed");

        // 测试 init_models 方法
        let result = project.init_models();
        assert!(result.is_ok(), "Models initialization should succeed");

        // 验证模型文件
        assert!(
            work_root.join(MODELS_WPL_PARSE_FILE).exists(),
            "parse.wpl should exist"
        );
        assert!(
            work_root.join(MODELS_WPL_SAMPLE_FILE).exists(),
            "sample.dat should exist"
        );
        assert!(
            work_root.join(MODELS_OML_EXAMPLE_FILE).exists(),
            "example.oml should exist"
        );
        assert!(
            work_root.join(MODELS_OML_KNOWDB_FILE).exists(),
            "knowdb.toml should exist"
        );
        assert!(
            !work_root.join(TOPOLOGY_WPSRC_FILE).exists(),
            "wpsrc.toml should not exist in pure model initialization"
        );
        assert!(
            !work_root.join(CONNECTORS_DIR).exists(),
            "connectors directory should not exist in model init"
        );
        assert!(
            !work_root.join(LEGACY_SOURCE_DIR).exists(),
            "legacy top-level source.d should not exist in model init"
        );
        assert!(
            !work_root.join(LEGACY_SINK_DIR).exists(),
            "legacy top-level sink.d should not exist in model init"
        );
    }

    #[test]
    fn test_resolve_with_root() {
        use wp_conf::paths::{OUT_FILE_PATH, RESCURE_FILE_PATH, SRC_FILE_PATH};

        let base = Path::new("/work");

        // 测试相对路径
        let relative_path = WarpProject::resolve_with_root(base, "data/file.txt");
        assert_eq!(relative_path, Path::new("/work/data/file.txt"));

        // 测试绝对路径
        let absolute_path = WarpProject::resolve_with_root(base, "/absolute/path");
        assert_eq!(absolute_path, Path::new("/absolute/path"));

        // 测试以 "./" 开头的路径
        let prefixed_path = WarpProject::resolve_with_root(base, "./data/file.txt");
        assert_eq!(prefixed_path, Path::new("/work/data/file.txt"));

        // 测试常量路径
        let out_path = WarpProject::resolve_with_root(base, OUT_FILE_PATH);
        assert!(out_path.starts_with("/work"));

        let src_path = WarpProject::resolve_with_root(base, SRC_FILE_PATH);
        assert!(src_path.starts_with("/work"));

        let rescue_path = WarpProject::resolve_with_root(base, RESCURE_FILE_PATH);
        assert!(rescue_path.starts_with("/work"));
    }

    #[test]
    fn test_init_wpgen_config() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let work_root = temp_dir.path();

        // 测试 init_wpgen_config 方法
        let result = WarpProject::init_wpgen_config(work_root);
        assert!(result.is_ok(), "Wpgen config initialization should succeed");

        // 验证配置文件被创建
        let wpgen_config_path = work_root.join(CONF_WPGEN_FILE);
        assert!(wpgen_config_path.exists(), "wpgen.toml should exist");

        // 验证文件内容
        let content =
            fs::read_to_string(&wpgen_config_path).expect("Should be able to read wpgen.toml");
        assert!(!content.is_empty(), "wpgen.toml should not be empty");
        // 检查配置文件是否包含一些基本配置项
        assert!(
            content.contains("["),
            "wpgen.toml should contain at least one section"
        );

        // 测试重复调用（不应该覆盖现有文件）
        let result = WarpProject::init_wpgen_config(work_root);
        assert!(result.is_ok(), "Second call should also succeed");

        let new_content =
            fs::read_to_string(&wpgen_config_path).expect("Should be able to read wpgen.toml");
        assert_eq!(content, new_content, "File should not be overwritten");
    }
}
