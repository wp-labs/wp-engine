use crate::connectors::{ConnectorDef, ConnectorTomlFile};
use crate::structure::GroupExpectSpec;
use crate::structure::SinkExpectOverride;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use toml::value::Table;
use wp_connector_api::ParamMap;

pub type ConnectorFile = ConnectorTomlFile;
pub type ConnectorRec = ConnectorDef;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteFile {
    #[serde(default)]
    pub version: Option<String>,
    pub sink_group: RouteGroup,
    /// 原始文件路径（IO 层注入；用于错误上下文）
    #[serde(skip)]
    pub origin: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteGroup {
    pub name: String,
    #[serde(default)]
    pub parallel: Option<usize>,
    #[serde(rename = "match")]
    #[serde(default)]
    pub _match: Option<toml::value::Table>,
    /// 顶层扁平写法：oml/rule；可为字符串或字符串数组
    #[serde(default)]
    pub oml: Option<StringOrArray>,
    #[serde(default)]
    pub rule: Option<StringOrArray>,
    /// 组级标签
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub expect: Option<GroupExpectSpec>,
    #[serde(default)]
    pub sinks: Vec<RouteSink>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteSink {
    #[serde(rename = "use", alias = "connect", alias = "connector")]
    connect: String,
    /// 同一 sink_group 内唯一名称（配置字段仍为 `name`）
    #[serde(default, rename = "name")]
    inner_name: Option<String>,
    // 统一写法：params（不再兼容 params_override）
    #[serde(default, rename = "params")]
    params: ParamMap,
    /// sink 级标签
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    expect: Option<SinkExpectOverride>,
    #[serde(default)]
    filter: Option<String>,
    /// 当 cond 结果等于该值时投递；默认为 true
    #[serde(default = "crate_default_true")]
    filter_expect: bool,
}

impl RouteSink {
    pub fn use_id(&self) -> &str {
        self.connect.as_str()
    }
    pub fn inner_name(&self) -> Option<&str> {
        self.inner_name.as_deref()
    }
    pub fn params(&self) -> &ParamMap {
        &self.params
    }
    pub fn expect(&self) -> Option<&SinkExpectOverride> {
        self.expect.as_ref()
    }
    pub fn filter_path(&self) -> Option<&str> {
        self.filter.as_deref()
    }
    pub fn tags(&self) -> Option<&Vec<String>> {
        self.tags.as_ref()
    }
    pub fn filter_expect(&self) -> bool {
        self.filter_expect
    }
}

fn crate_default_true() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum StringOrArray {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrArray {
    pub fn as_vec(&self) -> Vec<String> {
        match self {
            StringOrArray::Single(s) => vec![s.clone()],
            StringOrArray::Multiple(v) => v.clone(),
        }
    }
}

// 为了向后兼容，保留旧名称的别名
#[deprecated(note = "Use StringOrArray instead")]
pub type StrOrVec = StringOrArray;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DefaultsBody {
    #[serde(default)]
    pub tags: Option<Vec<String>>, // 每层 <=4；留给上层合并
    pub expect: GroupExpectSpec,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DefaultsFile {
    pub defaults: DefaultsBody,
}
