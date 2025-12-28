# Connector Definition Provider 提案

## 背景问题
- 当前 connector 元数据散落在 `connectors/source.d/*.toml`、`connectors/sink.d/*.toml` 中。每增加一种 connector（或修改默认参数）都必须修改模板文件乃至引擎代码，扩展性差。
- CLI/工具链对这些文件有硬依赖，不利于后续插件化或远程配置；也无法让外部团队独立贡献新的 connector 类型。

## 目标
- 以统一的数据结构表达 connector 定义（ID、类型、默认参数、允许覆写项等），并通过统一接口提供给引擎与工具链。
- 允许多个 Provider 并行注册，支持内置定义与插件定义共存。
- 引擎仅依赖运行时 Factory 注册；定义 Provider 仅为配置/模板层服务，与 runtime 解耦。

## 核心设计
```rust
pub enum ConnectorScope {
    Source,
    Sink,
}

pub struct ConnectorDef {
    pub id: String,
    pub kind: String,
    pub scope: ConnectorScope,
    pub allow_override: Vec<String>,
    pub default_params: toml::value::Table,
}

pub trait ConnectorDefProvider: Send + Sync + 'static {
    fn source_defs(&self) -> Vec<ConnectorDef>;
    fn sink_defs(&self) -> Vec<ConnectorDef>;
}
```

### Provider 注册 & 聚合
- 在 `connectors` 模块中新增注册表：
  ```rust
  pub fn register_def_provider(provider: Arc<dyn ConnectorDefProvider>);
  pub fn all_connector_defs() -> Vec<ConnectorDef>;
  ```
- `connectors/startup` 在初始化时注册内置 Provider（负责读取现有模板或内存表），插件可在加载阶段追加自己的 Provider。
- CLI/`wp-proj` 等工具通过 `all_connector_defs()` 获取所有定义，进行 lint、导出模板或直接提供给 orchestration 层。

### 与 Factory 的关系
- Provider 只描述“有哪些 connector 及其默认参数”；真正的构建逻辑仍由 `SourceFactory`/`SinkFactory` 实现并注册。
- 新增 connector 时，仅需：
  1. 实现工厂（`*Factory` + `*Spec` + Runtime）；
  2. 实现 `ConnectorDefProvider` 返回该 connector 的定义；
  3. 在插件/启动阶段注册 Provider 与 Factory。
- Engine 不需要再修改模板或硬编码定义。

## 兼容策略
- 提供 `TomlDefProvider`，读取现有 `connectors/source.d/*.toml`、`sink.d/*.toml` 并转成 `ConnectorDef`，保持现有用户不受影响。
- 新的 Provider 可以来自外部 crate、加载远程 JSON/TOML、甚至数据库；工具链无感，只依赖 trait 接口。

## 后续工作
- 落地注册表与 Provider trait，补充文档示例。
- 调整 `wp-proj`/CLI 代码，优先从 Provider 获取定义；如果无 Provider，再回退到旧目录结构。
- 梳理现有 connectors 模板，迁移到 Provider 中，以减少文件冗余。
