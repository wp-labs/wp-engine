//! Sources Management Module
//!
//! This module provides comprehensive source management functionality including
//! validation, initialization, and routing operations for data sources
//! in the Warp Flow System.

use orion_conf::TomlIO;
use orion_error::{ErrorConv, ToStructError, UvsConfFrom};
use orion_variate::EnvDict;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use wp_cli_core::connectors::sources as sources_core;
use wp_conf::sources::types::{SourceItem, WarpSources};
use wp_conf::{engine::EngineConfig, sources::build::load_source_instances_from_file};
use wp_engine::facade::config::WPSRC_TOML;
use wp_engine::sources::SourceConfigParser;
use wp_error::run_error::{RunReason, RunResult};

// Re-export modules and types
pub use super::source_builder::source_builders;

/// Constants for default source configurations
pub const DEFAULT_FILE_SOURCE_KEY: &str = "file_1";
pub const DEFAULT_FILE_SOURCE_PATH: &str = "gen.dat";
pub const DEFAULT_SYSLOG_SOURCE_ID: &str = "syslog_1";
pub const DEFAULT_SYSLOG_HOST: &str = "0.0.0.0";
pub const DEFAULT_SYSLOG_PORT: i64 = 1514;

/// Sources management system for data source operations
///
/// The `Sources` struct provides a centralized interface for managing all
/// source-related operations including validation, initialization, and routing
/// of data sources within the project.
#[derive(Clone)]
pub struct Sources {
    work_root: PathBuf,
    eng_conf: Arc<EngineConfig>,
}

impl Sources {
    /// Creates a new Sources instance
    pub fn new<P: AsRef<Path>>(work_root: P, eng_conf: Arc<EngineConfig>) -> Self {
        Self {
            work_root: work_root.as_ref().to_path_buf(),
            eng_conf,
        }
    }

    pub(crate) fn update_engine_conf(&mut self, eng_conf: Arc<EngineConfig>) {
        self.eng_conf = eng_conf;
    }

    fn sources_root(&self) -> PathBuf {
        let raw = self.eng_conf.src_root();
        let candidate = Path::new(raw);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.work_root.join(candidate)
        }
    }

    fn wpsrc_path(&self) -> PathBuf {
        self.sources_root().join(WPSRC_TOML)
    }

    pub fn check(&self) -> RunResult<()> {
        let wpsrc_path = self.wpsrc_path();

        // Verify configuration file exists
        if !wpsrc_path.exists() {
            return Err(RunReason::from_conf(format!(
                "Configuration error: wpsrc.toml file does not exist: {:?}",
                wpsrc_path
            ))
            .to_err());
        }

        // Parse and validate configuration
        self.validate_wpsrc_config(&self.work_root, &wpsrc_path)?;

        // Attempt to build specifications to ensure they are valid
        self.build_source_specs(&wpsrc_path)?;

        println!("✓ Sources configuration validation passed");
        Ok(())
    }

    pub fn check_sources_config(&self) -> Result<bool, String> {
        let wpsrc_path = self.wpsrc_path();
        if !wpsrc_path.exists() {
            return Err("Configuration error: wpsrc.toml file does not exist".to_string());
        }

        self.parse_config_only(&self.work_root, &wpsrc_path)
            .map(|_| true)
            .map_err(|e| format!("Parse sources failed: {}", e))
    }

    pub fn init(&self) -> RunResult<()> {
        let wpsrc_dir = self.sources_root();
        let wpsrc_path = wpsrc_dir.join(WPSRC_TOML);

        // Ensure parent directory exists
        self.ensure_directory_exists(&wpsrc_path)?;

        // Load existing configuration or create new one
        let mut sources_config = self.load_or_create_config(&wpsrc_path)?;

        // Add default sources if they don't exist
        self.add_default_sources(&mut sources_config)?;

        // Save configuration
        sources_config.save_toml(&wpsrc_path).map_err(|e| {
            RunReason::from_conf(format!("Failed to save configuration: {}", e)).to_err()
        })?;

        println!("✓ Sources initialization completed");
        Ok(())
    }

    fn validate_wpsrc_config(&self, work_root: &Path, wpsrc_path: &Path) -> RunResult<()> {
        let parser = SourceConfigParser::new(work_root.to_path_buf());

        // Load configuration from TOML file
        let sources_config = WarpSources::load_toml(wpsrc_path).err_conv()?;

        // Serialize configuration to validate structure
        let config_content = toml::to_string_pretty(&sources_config).map_err(|e| {
            RunReason::from_conf(format!("Failed to serialize config: {}", e)).to_err()
        })?;

        // Parse and validate the configuration content
        parser
            .parse_and_validate_only(&config_content)
            .map_err(|e| {
                RunReason::from_conf(format!("Sources config validation failed: {}", e)).to_err()
            })?;

        Ok(())
    }

    fn parse_config_only(&self, work_root: &Path, wpsrc_path: &Path) -> RunResult<()> {
        let parser = SourceConfigParser::new(work_root.to_path_buf());

        // Load configuration from TOML file, or use empty config if file doesn't exist
        let sources_config = if wpsrc_path.exists() {
            WarpSources::load_toml(wpsrc_path).map_err(|e| {
                RunReason::from_conf(format!("Failed to load wpsrc.toml: {}", e)).to_err()
            })?
        } else {
            WarpSources { sources: vec![] }
        };

        // Serialize configuration for parsing
        let config_content = toml::to_string_pretty(&sources_config).map_err(|e| {
            RunReason::from_conf(format!("Failed to serialize config: {}", e)).to_err()
        })?;

        // Parse and validate the configuration structure
        parser
            .parse_and_validate_only(&config_content)
            .map_err(|e| {
                RunReason::from_conf(format!("Configuration parsing failed: {}", e)).to_err()
            })?;

        Ok(())
    }

    /// Builds source specifications for validation
    fn build_source_specs(&self, wpsrc_path: &Path) -> RunResult<()> {
        let env_dict = EnvDict::new();
        let _specs = load_source_instances_from_file(wpsrc_path, &env_dict).map_err(|e| {
            RunReason::from_conf(format!("Failed to build source specs: {}", e)).to_err()
        })?;
        Ok(())
    }

    /// Loads existing configuration or creates new empty one
    fn load_or_create_config(&self, config_path: &Path) -> RunResult<WarpSources> {
        if config_path.exists() {
            WarpSources::load_toml(config_path).map_err(|e| {
                RunReason::from_conf(format!("Failed to load existing config: {}", e)).to_err()
            })
        } else {
            Ok(WarpSources { sources: vec![] })
        }
    }

    fn add_default_sources(&self, config: &mut WarpSources) -> RunResult<()> {
        let default_sources = vec![
            // Add a default file source that reads from gen.dat
            source_builders::file_source(DEFAULT_FILE_SOURCE_KEY, DEFAULT_FILE_SOURCE_PATH),
            // Add a default syslog TCP source (disabled by default)
            source_builders::syslog_tcp_source(
                DEFAULT_SYSLOG_SOURCE_ID,
                DEFAULT_SYSLOG_HOST,
                DEFAULT_SYSLOG_PORT,
            )
            .with_enable(Some(false)),
        ];

        // Add each source if it doesn't already exist
        for source_item in default_sources {
            Self::ensure_source_exists(config, source_item);
        }

        Ok(())
    }

    /// Adds a new source only if an entry with the same key is not present
    fn ensure_source_exists(config: &mut WarpSources, source_item: SourceItem) {
        if config.sources.iter().any(|s| s.key == source_item.key) {
            return;
        }
        config.sources.push(source_item);
    }

    // =================== PROJECT MANAGEMENT ===================

    /// Ensures parent directory exists for configuration file
    fn ensure_directory_exists(&self, config_path: &Path) -> RunResult<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RunReason::from_conf(format!("Failed to create directory: {}", e)).to_err()
            })?;
        }
        Ok(())
    }

    // =================== DISPLAY METHODS ===================

    /// Displays sources information in JSON format
    pub fn display_as_json(&self, rows: &[sources_core::RouteRow]) {
        let json_rows: Vec<serde_json::Value> = rows
            .iter()
            .map(|r| {
                serde_json::json!({
                    "key": r.key,
                    "kind": r.kind,
                    "enabled": r.enabled,
                    "detail": r.detail
                })
            })
            .collect();

        println!("{}", serde_json::to_string_pretty(&json_rows).unwrap());
    }

    /// Displays sources information in table format
    pub fn display_as_table(&self, rows: &[sources_core::RouteRow]) {
        use comfy_table::{Cell as TCell, ContentArrangement, Table};

        let mut table = Table::new();
        table.load_preset(comfy_table::presets::UTF8_FULL);
        table.set_content_arrangement(ContentArrangement::Dynamic);
        table.set_width(120);
        table.set_header(vec![
            TCell::new("key"),
            TCell::new("kind"),
            TCell::new("on"),
            TCell::new("detail"),
        ]);

        for row in rows {
            table.add_row(vec![
                TCell::new(&row.key),
                TCell::new(&row.kind),
                TCell::new(if row.enabled { "on" } else { "off" }),
                TCell::new(&row.detail),
            ]);
        }

        println!("{}", table);
        println!("total: {}", rows.len());
    }
}

// =================== TESTS ===================

#[cfg(test)]
mod tests {

    use crate::test_utils::temp_workdir;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_sources_creation() {
        let temp = temp_workdir();
        let eng = std::sync::Arc::new(EngineConfig::init(temp.path()));
        let _sources = Sources::new(temp.path(), eng);
        assert!(true); // Basic test to ensure struct can be created
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_FILE_SOURCE_KEY, "file_1");
        assert_eq!(DEFAULT_SYSLOG_SOURCE_ID, "syslog_1");
        assert_eq!(DEFAULT_SYSLOG_HOST, "0.0.0.0");
        assert_eq!(DEFAULT_SYSLOG_PORT, 1514);
    }

    #[test]
    fn add_default_sources_skips_existing_entries() {
        let mut config = WarpSources {
            sources: Vec::new(),
        };
        // first insert default file source manually with custom param
        let mut custom = source_builders::file_source(DEFAULT_FILE_SOURCE_KEY, "custom.dat");
        custom.params.insert("base".into(), json!("custom_base"));
        config.sources.push(custom);

        Sources::ensure_source_exists(
            &mut config,
            source_builders::file_source(DEFAULT_FILE_SOURCE_KEY, DEFAULT_FILE_SOURCE_PATH),
        );

        let stored = config
            .sources
            .iter()
            .find(|s| s.key == DEFAULT_FILE_SOURCE_KEY)
            .unwrap();
        assert_eq!(
            stored.params.get("base").and_then(|v| v.as_str()),
            Some("custom_base")
        );
    }
}
