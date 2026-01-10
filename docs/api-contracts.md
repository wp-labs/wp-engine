# 公共 API 契约文档

> **重要**: 重构过程中必须保持这些公共接口的兼容性

**日期**: 2026-01-10
**目的**: 确保重构不破坏现有 API

---

## wp-cli-core 公共 API

### connectors::sources

```rust
// 列出 source connectors 及其使用情况
pub fn list_connectors(
    work_root: &str,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> OrionConfResult<Vec<ConnectorListRow>>

// 生成 source 路由表
pub fn route_table(
    work_root: &str,
    eng_conf: &EngineConfig,
    path_like: Option<&str>,
    dict: &EnvDict,
) -> OrionConfResult<Vec<RouteRow>>

// 数据结构
pub struct ConnectorListRow {
    pub id: String,
    pub kind: String,
    pub allow_override: Vec<String>,
    pub detail: String,
    pub refs: usize,
}

pub struct RouteRow {
    pub key: String,
    pub connect: String,
    pub kind: String,
    pub enabled: bool,
    pub detail: String,
}
```

### connectors::sinks

```rust
// 验证所有 sink 路由配置
pub fn validate_routes(work_root: &str) -> OrionConfResult<()>

// 列出 sink connectors 使用情况
pub fn list_connectors_usage(
    work_root: &str,
) -> OrionConfResult<(BTreeMap<String, ConnectorRec>, Vec<(String, String, String)>)>

// 生成 sink 路由表
pub fn route_table(
    work_root: &str,
    group_filters: &[String],
    sink_filters: &[String],
) -> OrionConfResult<Vec<RouteRow>>

// 加载 connectors 映射
pub fn load_connectors_map(
    work_root: &str,
) -> OrionConfResult<BTreeMap<String, ConnectorRec>>

// 数据结构
pub struct RouteRow {
    pub scope: String,
    pub group: String,
    pub full_name: String,
    pub name: String,
    pub connector: String,
    pub target: String,
    pub fmt: String,
    pub detail: String,
    pub rules: Vec<String>,
    pub oml: Vec<String>,
}
```

### obs::stat

```rust
// 统计 source 文件
pub fn stat_src_file(
    work_root: &str,
    eng_conf: &EngineConfig,
) -> Result<Option<SrcLineReport>>

// 统计 sink 文件
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Vec<wpcnt_lib::types::Row>, u64)>

// 综合统计
pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Option<SrcLineReport>, Vec<wpcnt_lib::types::Row>, u64)>
```

---

## wp-cli-utils 公共 API

### sources

```rust
// 列出文件源及行数
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<SrcLineReport>

// 统计总输入行数
pub fn total_input_from_wpsrc(
    work_root: &Path,
    engine_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<u64>

// 数据结构
pub struct SrcLineReport {
    pub total_enabled_lines: u64,
    pub items: Vec<SrcLineItem>,
}

pub struct SrcLineItem {
    pub key: String,
    pub path: String,
    pub enabled: bool,
    pub lines: Option<u64>,
    pub error: Option<String>,
}
```

### validate

```rust
// 验证分组比例
pub fn validate_groups(
    groups: &[GroupAccum],
    total_override: Option<u64>,
) -> ValidateReport

// 数据结构
pub struct ValidateReport {
    pub items: Vec<ValidateItem>,
}

pub struct ValidateItem {
    pub group: String,
    pub sink: Option<String>,
    pub msg: String,
    pub severity: Severity,
}
```

### stats & scan

```rust
// 加载统计文件
pub fn load_stats_file(path: &Path) -> Result<StatsFile>

// 分组输入数据
pub fn group_input(stats: &StatsFile, filters: &[String]) -> Vec<GroupAccum>

// 处理单个分组
pub fn process_group(
    name: &str,
    expect: Option<GroupExpectSpec>,
    sinks: Vec<SinkInstanceConf>,
    is_infra: bool,
    ctx: &Ctx,
    rows: &mut Vec<Row>,
    total: &mut u64,
) -> Result<()>
```

### pretty

```rust
// 格式化输出函数
pub fn print_src_files_table(report: &SrcLineReport)
pub fn print_validate_report(report: &ValidateReport)
pub fn print_rows(rows: &[Row])
// ... 其他 print_* 函数
```

### fsutils

```rust
// 解析文件路径
pub fn resolve_path(path: &str, work_root: &Path) -> PathBuf

// 统计文件行数
pub fn count_lines_file(path: &Path) -> Result<u64>
```

### banner

```rust
// 打印 banner
pub fn print_banner(version: &str)

// 分离静默参数
pub fn split_quiet_args(args: Vec<String>) -> (Vec<String>, bool)
```

---

## wp-config 公共 API

### sources

```rust
// 定位 connectors 目录
pub fn find_connectors_dir(path: &Path) -> Option<PathBuf>

// 加载 source connectors
pub fn load_connectors_for(
    dir: &Path,
    env: &EnvDict,
) -> Result<BTreeMap<String, SourceConnector>>

// 加载 wpsrc 配置
impl WpSourcesConfig {
    pub fn env_load_toml(path: &Path, env: &EnvDict) -> Result<Self>
}
```

### sinks

```rust
// 从目录加载路由文件
pub fn load_route_files_from(
    dir: &Path,
    env: &EnvDict,
) -> Result<Vec<RouteFile>>

// 加载 sink 默认配置
pub fn load_sink_defaults(
    sink_root: &str,
    env: &EnvDict,
) -> Result<Option<DefaultsBody>>

// 从路由文件构建配置
pub fn build_route_conf_from(
    route: &RouteFile,
    defaults: Option<&DefaultsBody>,
    connectors: &BTreeMap<String, ConnectorRec>,
) -> Result<RouteConf>

// 加载业务路由配置
pub fn load_business_route_confs(
    sink_root: &str,
    env: &EnvDict,
) -> Result<Vec<RouteConf>>

// 加载基础设施路由配置
pub fn load_infra_route_confs(
    sink_root: &str,
    env: &EnvDict,
) -> Result<Vec<RouteConf>>

// 定位 connectors 基础目录
pub fn find_connectors_base_dir(path: &Path) -> Option<PathBuf>
```

### connectors

```rust
// 从目录加载 connector 定义
pub fn load_connector_defs_from_dir(
    dir: &Path,
    scope: ConnectorScope,
    env: &EnvDict,
) -> Result<Vec<ConnectorDef>>

// TOML 值转参数值
pub fn param_value_from_toml(value: &toml::Value) -> ParamValue

// 参数映射转 TOML 表
pub fn param_map_to_table(params: &ParamMap) -> toml::Table
```

### engine

```rust
// 引擎配置
impl EngineConfig {
    pub fn init(work_root: &str) -> Self
    pub fn src_root(&self) -> &str
    pub fn sink_root(&self) -> &str
}
```

---

## 兼容性要求

### 必须保持

- ✅ 所有公共函数签名保持不变
- ✅ 所有公共数据结构保持不变
- ✅ 返回值类型保持不变
- ✅ 错误类型保持不变

### 允许修改

- ✅ 内部实现细节
- ✅ 私有函数和模块
- ✅ 性能优化
- ✅ 代码组织结构（模块位置）

### 可以新增

- ✅ 新的公共函数
- ✅ 新的工具函数
- ✅ 优化的实现路径

---

## 验证方法

重构完成后，运行以下检查：

```bash
# 1. 确保所有测试通过
cargo test --workspace

# 2. 检查公共 API 未变更
# 对比 lib.rs 中的 pub use 语句

# 3. 确保文档编译通过
cargo doc --workspace --no-deps

# 4. 检查没有新的编译警告
cargo clippy --workspace
```

---

**维护**: 每次修改公共 API 时，必须更新此文档
