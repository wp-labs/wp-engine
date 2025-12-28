use crate::connectors::ConnectorTomlFile;
use getset::WithSetters;
use serde::{Deserialize, Serialize};
use wp_connector_api::{ConnectorDef, ParamMap};

pub type SrcConnectorFileRec = ConnectorTomlFile;
pub type SourceConnector = ConnectorDef;

// V2 [[sources]] 项（应用层 SourceConfigParser 的配置数据结构迁移至配置层，便于统一装配）
#[derive(Debug, Clone, Deserialize, Serialize, WithSetters)]
pub struct WpSource {
    pub key: String,
    #[set_with = "pub"]
    #[serde(default)]
    pub enable: Option<bool>,
    pub connect: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, rename = "params", alias = "params_override")]
    pub params: ParamMap,
}

/// Deprecated alias: maintained for crates that still refer to `SourceItem`
pub type SourceItem = WpSource;

/// V2 源配置包装器：`[[sources]]` 列表
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WpSourcesConfig {
    #[serde(default)]
    pub sources: Vec<WpSource>,
}

/// Legacy alias for compatibility with tooling referencing `WarpSources`
pub type WarpSources = WpSourcesConfig;
