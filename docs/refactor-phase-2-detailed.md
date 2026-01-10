# 阶段 2: 移除业务逻辑下沉 - 详细执行方案

> **目标**: 将 wp-cli-utils 中的业务逻辑移到 wp-cli-core，明确层次职责

**预计时间**: 1.5-2 小时
**风险等级**: 🟡 中等
**可回滚**: ✅ 是
**前置条件**: ✅ 阶段 1 已完成

---

## 背景分析

### 当前问题

wp-cli-utils 当前混合了两类代码：

**业务逻辑层** (应该在 wp-cli-core):
- `sources.rs` - 文件源统计业务逻辑
  - `total_input_from_wpsrc()` - 从配置推导总输入条数
  - `list_file_sources_with_lines()` - 列出文件源及行数
- `scan.rs` - Sink 组处理业务逻辑
  - `process_group()` - 处理 sink 组
  - `process_group_v2()` - V2 版本处理

**工具层** (应该保留在 utils):
- `fsutils.rs` - 文件系统工具
- `pretty.rs` - 格式化输出
- `types.rs` - 数据结构定义
- `banner.rs` - 横幅打印
- `stats.rs` - 统计文件读取
- `validate.rs` - 验证逻辑

### 架构问题

```
当前架构（有问题）:
┌─────────────────┐
│   wp-cli-core   │
│   (CLI 入口)    │
└────────┬────────┘
         │ 调用
         ↓
┌─────────────────┐
│  wp-cli-utils   │ ← 混合了业务逻辑和工具函数
│  • sources.rs   │   （违反单一职责原则）
│  • scan.rs      │
│  • fsutils.rs   │
└─────────────────┘

目标架构（清晰）:
┌─────────────────────────────┐
│       wp-cli-core           │
│  ┌──────────────────────┐   │
│  │   business/          │   │ ← 业务逻辑在 core
│  │   • observability/   │   │
│  │   • connectors/      │   │
│  └──────────────────────┘   │
└────────┬────────────────────┘
         │ 使用
         ↓
┌─────────────────┐
│  wp-cli-utils   │ ← 只保留纯工具函数
│  • fsutils      │   （明确的工具层）
│  • pretty       │
│  • types        │
└─────────────────┘
```

---

## 任务分解

### 任务 2.1: 分析业务函数依赖 ⏱️ 15 分钟

#### Step 1: 确定需要移动的函数

**sources.rs**:
```rust
// 需要移动到 core
pub fn total_input_from_wpsrc(...) -> Option<u64>
pub fn list_file_sources_with_lines(...) -> Option<SrcLineReport>

// 依赖的类型
pub struct SrcLineItem { ... }
pub struct SrcLineReport { ... }
```

**scan.rs**:
```rust
// 需要移动到 core
pub fn process_group(...) -> GroupAccum
pub fn process_group_v2(...) -> GroupAccum
pub struct ResolvedSinkLite { ... }
```

#### Step 2: 分析依赖关系

```bash
# 检查 sources.rs 的依赖
rg "use.*fsutils" crates/wp-cli-utils/src/sources.rs
rg "use.*types" crates/wp-cli-utils/src/sources.rs

# 检查 scan.rs 的依赖
rg "use.*fsutils" crates/wp-cli-utils/src/scan.rs
rg "use.*types" crates/wp-cli-utils/src/scan.rs
```

**依赖分析结果**:
- sources.rs 依赖: `fsutils`, `types::Ctx`, `wp_conf`
- scan.rs 依赖: `fsutils`, `types::{Ctx, GroupAccum, Row, SinkAccum}`

**策略**: 保留 `types` 在 utils 中（作为共享数据结构），业务逻辑移到 core 后导入 `wpcnt_lib::types`

#### Step 3: 确定调用点

```bash
# 查找 list_file_sources_with_lines 的调用点
rg "list_file_sources_with_lines" crates/wp-cli-core

# 查找 total_input_from_wpsrc 的调用点
rg "total_input_from_wpsrc" crates/wp-cli-core

# 查找 process_group 的调用点
rg "process_group" crates/wp-cli-core
```

**调用点**:
- `wp-cli-core/src/obs/stat.rs` - 主要调用点

---

### 任务 2.2: 创建 core 业务模块结构 ⏱️ 10 分钟

#### Step 1: 创建目录结构

```bash
# 创建 business 模块目录
mkdir -p crates/wp-cli-core/src/business
mkdir -p crates/wp-cli-core/src/business/observability
```

#### Step 2: 创建模块文件

**`crates/wp-cli-core/src/business/mod.rs`**:
```rust
//! Business logic layer for CLI operations
//!
//! This module contains domain-specific business logic that orchestrates
//! configuration loading, data processing, and result aggregation.

pub mod observability;
```

**`crates/wp-cli-core/src/business/observability/mod.rs`**:
```rust
//! Observability business logic for sources and sinks
//!
//! This module provides high-level business functions for collecting
//! observability data about sources and sinks.

mod sources;
mod sinks;

pub use sources::{
    SrcLineItem, SrcLineReport,
    list_file_sources_with_lines,
    total_input_from_wpsrc,
};
pub use sinks::{
    ResolvedSinkLite,
    process_group,
    process_group_v2,
};
```

#### Step 3: 更新 lib.rs

修改 `crates/wp-cli-core/src/lib.rs`:
```rust
pub mod business;      // 新增
pub mod connectors;
pub mod obs;

// Re-export business functions for convenience
pub use business::observability::{
    list_file_sources_with_lines,
    total_input_from_wpsrc,
    process_group,
    SrcLineReport,
};
```

---

### 任务 2.3: 移动 sources 业务逻辑 ⏱️ 20 分钟

#### Step 1: 创建 sources.rs

创建 `crates/wp-cli-core/src/business/observability/sources.rs`:

```rust
//! Source file observability functions
//!
//! This module provides business logic for analyzing source configurations
//! and counting lines in source files.

use orion_variate::EnvDict;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use wp_conf::connectors::{ParamMap, param_value_from_toml, merge_params};
use wp_conf::engine::EngineConfig;
use wpcnt_lib::types::Ctx;
use wpcnt_lib::fsutils::{count_lines_file, resolve_path};

type SrcConnectorRec = wp_conf::sources::SourceConnector;

/// A flattened row for source line information
#[derive(Debug, Serialize)]
pub struct SrcLineItem {
    pub key: String,
    pub path: String,
    pub enabled: bool,
    pub lines: Option<u64>,
    pub error: Option<String>,
}

/// Report of source file line counts
#[derive(Debug, Serialize)]
pub struct SrcLineReport {
    pub total_enabled_lines: u64,
    pub items: Vec<SrcLineItem>,
}

// 私有辅助函数
fn read_wpsrc_toml(work_root: &Path, engine_conf: &EngineConfig) -> Option<String> {
    let modern = work_root.join(engine_conf.src_root()).join("wpsrc.toml");
    if modern.exists() {
        return std::fs::read_to_string(&modern).ok();
    }
    None
}

fn load_connectors_map(
    base_dir: &Path,
    dict: &EnvDict,
) -> Option<BTreeMap<String, SrcConnectorRec>> {
    wp_conf::sources::load_connectors_for(base_dir, dict).ok()
}

fn toml_table_to_param_map(table: &toml::value::Table) -> ParamMap {
    table
        .iter()
        .map(|(k, v)| (k.clone(), param_value_from_toml(v)))
        .collect()
}

/// 从 wpsrc 配置推导总输入条数（仅统计启用的文件源）
pub fn total_input_from_wpsrc(
    work_root: &Path,
    engine_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<u64> {
    let content = read_wpsrc_toml(work_root, engine_conf)?;
    let toml_val: toml::Value = toml::from_str(&content).ok()?;
    let mut sum = 0u64;

    if let Some(arr) = toml_val.get("sources").and_then(|v| v.as_array()) {
        // load connectors once
        let conn_dir =
            wp_conf::find_connectors_base_dir(&ctx.work_root.join("sources"), "source.d");
        let conn_map = conn_dir
            .as_ref()
            .and_then(|p| load_connectors_map(p.as_path(), dict))
            .unwrap_or_default();

        for item in arr {
            // v2: prefer connect flow
            if let Some(conn_id) = item.get("connect").and_then(|v| v.as_str()) {
                let enabled = item.get("enable").and_then(|v| v.as_bool()).unwrap_or(true);
                if !enabled {
                    continue;
                }
                if let Some(conn) = conn_map.get(conn_id) {
                    if conn.kind.eq_ignore_ascii_case("file") {
                        // 支持 params_override 与 params 两种写法
                        let ov = item
                            .get("params_override")
                            .or_else(|| item.get("params"))
                            .and_then(|v| v.as_table())
                            .cloned()
                            .unwrap_or_default();
                        let ov_map = toml_table_to_param_map(&ov);
                        let merged = merge_params(&conn.default_params, &ov_map, &conn.allow_override)
                            .unwrap_or_else(|_| conn.default_params.clone());

                        // 支持 path 或 base+file 两种写法
                        let maybe_path = merged
                            .get("path")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                            .or_else(|| {
                                let b = merged.get("base").and_then(|v| v.as_str());
                                let f = merged.get("file").and_then(|v| v.as_str());
                                match (b, f) {
                                    (Some(b), Some(f)) => {
                                        Some(std::path::Path::new(b).join(f).display().to_string())
                                    }
                                    _ => None,
                                }
                            });
                        if let Some(path) = maybe_path {
                            let pathbuf = resolve_path(&path, &ctx.work_root);
                            if let Ok(n) = count_lines_file(&pathbuf) {
                                sum += n;
                            }
                        }
                    }
                }
            }
        }
        return Some(sum);
    }
    None
}

/// 返回所有文件源（包含未启用）的行数信息；total 仅统计启用项
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<SrcLineReport> {
    let content = read_wpsrc_toml(work_root, eng_conf)?;
    let toml_val: toml::Value = toml::from_str(&content).ok()?;
    let mut items = Vec::new();
    let mut total = 0u64;

    if let Some(arr) = toml_val.get("sources").and_then(|v| v.as_array()) {
        // load connectors once
        let conn_dir =
            wp_conf::find_connectors_base_dir(&ctx.work_root.join("sources"), "source.d");
        let conn_map = conn_dir
            .as_ref()
            .and_then(|p| load_connectors_map(p.as_path(), dict))
            .unwrap_or_default();

        for it in arr {
            let conn_id = match it.get("connect").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => continue, // 不兼容旧写法
            };
            let key = it
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let enabled = it.get("enable").and_then(|v| v.as_bool()).unwrap_or(true);

            // 支持 params_override 与 params 两种写法
            let ov = it
                .get("params_override")
                .or_else(|| it.get("params"))
                .and_then(|v| v.as_table())
                .cloned()
                .unwrap_or_default();

            if let Some(conn) = conn_map.get(conn_id) {
                if !conn.kind.eq_ignore_ascii_case("file") {
                    continue;
                }
                let ov_map = toml_table_to_param_map(&ov);
                let merged = merge_params(&conn.default_params, &ov_map, &conn.allow_override)
                    .unwrap_or_else(|_| conn.default_params.clone());

                // 支持 path 或 base+file 两种写法
                let path_str = merged
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        let b = merged.get("base").and_then(|v| v.as_str());
                        let f = merged.get("file").and_then(|v| v.as_str());
                        match (b, f) {
                            (Some(b), Some(f)) => {
                                Some(std::path::Path::new(b).join(f).display().to_string())
                            }
                            _ => None,
                        }
                    })
                    .unwrap_or_default();

                let pbuf = resolve_path(&path_str, &ctx.work_root);
                if enabled {
                    match count_lines_file(&pbuf) {
                        Ok(n) => {
                            total += n;
                            items.push(SrcLineItem {
                                key,
                                path: pbuf.display().to_string(),
                                enabled,
                                lines: Some(n),
                                error: None,
                            });
                        }
                        Err(e) => {
                            items.push(SrcLineItem {
                                key,
                                path: pbuf.display().to_string(),
                                enabled,
                                lines: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                } else {
                    items.push(SrcLineItem {
                        key,
                        path: pbuf.display().to_string(),
                        enabled,
                        lines: None,
                        error: None,
                    });
                }
            }
        }
        return Some(SrcLineReport {
            total_enabled_lines: total,
            items,
        });
    }
    None
}
```

#### Step 2: 编译验证

```bash
# 编译 wp-cli-core
cargo build --package wp-cli-core
```

---

### 任务 2.4: 移动 sinks 业务逻辑 ⏱️ 20 分钟

#### Step 1: 创建 sinks.rs

创建 `crates/wp-cli-core/src/business/observability/sinks.rs`:

```rust
//! Sink group processing business logic
//!
//! This module provides functions for processing sink groups and
//! collecting line count statistics.

use wpcnt_lib::fsutils::{count_lines_file, is_match, resolve_path};
use wpcnt_lib::types::{Ctx, GroupAccum, Row, SinkAccum};

/// Process a sink group and collect line count statistics
pub fn process_group(
    group_name: &str,
    expect: Option<wp_conf::structure::GroupExpectSpec>,
    sinks: Vec<wp_conf::structure::SinkInstanceConf>,
    framework: bool,
    ctx: &Ctx,
    rows: &mut Vec<Row>,
    total: &mut u64,
) -> GroupAccum {
    let mut gacc = GroupAccum::new(group_name.to_string(), expect);
    for s in sinks.into_iter() {
        if !is_match(s.name().as_str(), &ctx.sink_filters) {
            continue;
        }
        let kind = s.resolved_kind_str();
        if !(kind.eq_ignore_ascii_case("file") || kind.eq_ignore_ascii_case("test_rescue")) {
            continue;
        }
        // Resolve V2 style path
        let params = s.resolved_params_table();
        let raw_path = if params.contains_key("base") || params.contains_key("file") {
            let base = params
                .get("base")
                .and_then(|v| v.as_str())
                .unwrap_or("./data/out_dat");
            let file = params
                .get("file")
                .and_then(|v| v.as_str())
                .unwrap_or("out.dat");
            std::path::Path::new(base).join(file).display().to_string()
        } else {
            params
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("./data/out_dat/out.dat")
                .to_string()
        };
        if let Some(substr) = &ctx.path_like {
            if !raw_path.contains(substr) {
                continue;
            }
        }
        let prefer = resolve_path(&raw_path, &ctx.work_root);
        match count_lines_file(&prefer) {
            Ok(n) => {
                *total += n;
                let sink_name = s.name().clone();
                let sink_expect = s.expect.clone();
                if !ctx.total_only {
                    rows.push(Row::ok(
                        group_name.to_string(),
                        sink_name.clone(),
                        prefer,
                        framework,
                        n,
                    ));
                }
                gacc.add_sink(SinkAccum {
                    name: sink_name,
                    lines: n,
                    expect: sink_expect,
                });
            }
            Err(_e) => {
                let sink_name = s.name().clone();
                let sink_expect = s.expect.clone();
                if !ctx.total_only {
                    rows.push(Row::err(
                        group_name.to_string(),
                        sink_name.clone(),
                        prefer,
                        framework,
                    ));
                }
                gacc.add_sink(SinkAccum {
                    name: sink_name,
                    lines: 0,
                    expect: sink_expect,
                });
            }
        }
    }
    gacc
}

/// V2: process using resolved route info (no dependency on SinkUseConf)
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct ResolvedSinkLite {
    pub name: String,
    pub kind: String,
    pub params: toml::value::Table,
}

/// V2 version of process_group using resolved sink information
pub fn process_group_v2(
    group_name: &str,
    expect: Option<wp_conf::structure::GroupExpectSpec>,
    sinks: Vec<ResolvedSinkLite>,
    framework: bool,
    ctx: &Ctx,
    rows: &mut Vec<Row>,
    total: &mut u64,
) -> GroupAccum {
    let mut gacc = GroupAccum::new(group_name.to_string(), expect);
    for s in sinks.into_iter() {
        if !is_match(s.name.as_str(), &ctx.sink_filters) {
            continue;
        }
        // Only file-like sinks contribute line counts
        if s.kind.eq_ignore_ascii_case("file") {
            // Resolve path: prefer base+file; fallback to path
            let path = if s.params.contains_key("base") || s.params.contains_key("file") {
                let base = s
                    .params
                    .get("base")
                    .and_then(|v| v.as_str())
                    .unwrap_or("./data/out_dat");
                let file = s
                    .params
                    .get("file")
                    .and_then(|v| v.as_str())
                    .unwrap_or("out.dat");
                std::path::Path::new(base).join(file).display().to_string()
            } else {
                s.params
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("./data/out_dat/out.dat")
                    .to_string()
            };
            if let Some(substr) = &ctx.path_like {
                if !path.contains(substr) {
                    continue;
                }
            }
            let prefer = resolve_path(&path, &ctx.work_root);
            match count_lines_file(&prefer) {
                Ok(n) => {
                    *total += n;
                    if !ctx.total_only {
                        rows.push(Row::ok(
                            group_name.to_string(),
                            s.name.clone(),
                            prefer,
                            framework,
                            n,
                        ));
                    }
                    gacc.add_sink(SinkAccum {
                        name: s.name,
                        lines: n,
                        expect: None,
                    });
                }
                Err(_e) => {
                    if !ctx.total_only {
                        rows.push(Row::err(
                            group_name.to_string(),
                            s.name.clone(),
                            prefer,
                            framework,
                        ));
                    }
                    gacc.add_sink(SinkAccum {
                        name: s.name,
                        lines: 0,
                        expect: None,
                    });
                }
            }
        }
    }
    gacc
}
```

---

### 任务 2.5: 更新 stat.rs 使用新实现 ⏱️ 15 分钟

#### Step 1: 修改导入

修改 `crates/wp-cli-core/src/obs/stat.rs`:

```rust
use anyhow::Result;
use orion_variate::EnvDict;
use std::path::Path;
use wp_conf::{
    engine::EngineConfig,
    sinks::{load_business_route_confs, load_infra_route_confs},
};
use wpcnt_lib::types::{Ctx, Row};

// 新增：从 business 模块导入
use crate::business::observability::{
    list_file_sources_with_lines,
    process_group,
    SrcLineReport,
};

/// Sources (file) only
pub fn stat_src_file(
    work_root: &str,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> Result<Option<SrcLineReport>> {
    let ctx = Ctx::new(work_root.to_string());
    Ok(list_file_sources_with_lines(
        Path::new(work_root),
        eng_conf,
        &ctx,
        dict,
    ))
}

/// Sinks (file-like) only; caller must pass resolved sink_root
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &Ctx,
) -> Result<(Vec<Row>, u64)> {
    if !(sink_root.join("business.d").exists() || sink_root.join("infra.d").exists()) {
        anyhow::bail!(
            "缺少 sinks 配置目录：在 '{}' 下未发现 business.d/ 或 infra.d/",
            sink_root.display()
        );
    }
    let mut rows = Vec::new();
    let mut total = 0u64;
    let env_dict = EnvDict::new();

    for conf in load_business_route_confs(sink_root.to_string_lossy().as_ref(), &env_dict)? {
        let g = conf.sink_group;
        if !wpcnt_lib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            false,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    for conf in load_infra_route_confs(sink_root.to_string_lossy().as_ref(), &env_dict)? {
        let g = conf.sink_group;
        if !wpcnt_lib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            true,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    Ok((rows, total))
}

/// Combined: src-file + sink-file; requires work_root and sink_root
pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Result<(Option<SrcLineReport>, Vec<Row>, u64)> {
    let src_rep = list_file_sources_with_lines(Path::new(work_root), eng_conf, ctx, dict);
    let sink_root = Path::new(work_root).join(eng_conf.sink_root());
    let (rows, total) = stat_sink_file(&sink_root, ctx)?;
    Ok((src_rep, rows, total))
}
```

---

### 任务 2.6: 更新 wp-cli-utils 移除业务代码 ⏱️ 15 分钟

#### Step 1: 从 sources.rs 移除业务函数

修改 `crates/wp-cli-utils/src/sources.rs`，删除以下内容：
- `total_input_from_wpsrc()` 函数
- `list_file_sources_with_lines()` 函数
- `SrcLineItem` 结构
- `SrcLineReport` 结构
- 所有相关的辅助函数

**保留**:
- 如果有其他工具函数（当前文件似乎只有业务逻辑，可能需要整个删除）

#### Step 2: 从 scan.rs 移除业务函数

修改 `crates/wp-cli-utils/src/scan.rs`，删除以下内容：
- `process_group()` 函数
- `process_group_v2()` 函数
- `ResolvedSinkLite` 结构

#### Step 3: 更新 lib.rs

修改 `crates/wp-cli-utils/src/lib.rs`:

```rust
pub mod banner;
pub mod fsutils;
pub mod pretty;
// pub mod scan;      // 移除或标记为废弃
// pub mod sources;   // 移除或标记为废弃
pub mod stats;
pub mod types;
pub mod validate;

pub use banner::{print_banner, split_quiet_args};
pub use fsutils::*;
pub use pretty::{
    print_rows, print_src_files_table, print_validate_evidence, print_validate_headline,
    print_validate_report, print_validate_tables, print_validate_tables_verbose,
};
// pub use scan::process_group;  // 移除
// pub use sources::*;            // 移除
pub use stats::{StatsFile, group_input, load_stats_file};
pub use types::*;
pub use validate::*;
```

**注意**: 如果 sources.rs 和 scan.rs 变为空文件，可以考虑直接删除。

---

### 任务 2.7: 运行完整测试套件 ⏱️ 5 分钟

```bash
# 运行所有测试
cargo test --workspace 2>&1 | tee /tmp/phase2-test-results.txt

# 检查测试结果
grep "test result:" /tmp/phase2-test-results.txt

# 确保：
# - 所有测试通过
# - 无新增警告
# - 集成测试成功
```

---

### 任务 2.8: 更新文档和检查清单 ⏱️ 10 分钟

#### Step 1: 更新检查清单

编辑 `docs/refactor-checklist.md`:

```markdown
## ✅ 阶段 2: 移除业务逻辑下沉

**目标**: 将 wp-cli-utils 中的业务逻辑移到 wp-cli-core

- [x] 2.1 分析业务函数依赖
- [x] 2.2 创建 core 业务模块结构
- [x] 2.3 移动 sources 业务逻辑
- [x] 2.4 移动 sinks 业务逻辑
- [x] 2.5 更新 stat.rs 使用新实现
- [x] 2.6 更新 wp-cli-utils 移除业务代码
- [x] 2.7 运行测试
- [x] 2.8 更新文档

**完成标准**:
- ✅ wp-cli-utils 不再包含业务逻辑
- ✅ 所有功能正常工作
- ✅ 所有测试通过 (911+ tests)
- ✅ 无编译警告
```

#### Step 2: 创建阶段 2 总结

创建 `docs/phase-2-summary.md`:

```markdown
# 阶段 2 完成总结

**日期**: 2026-01-10
**阶段**: 移除业务逻辑下沉

## 目标
将 wp-cli-utils 中的业务逻辑移到 wp-cli-core，明确层次职责。

## 完成内容

### 1. 新增文件
- `crates/wp-cli-core/src/business/mod.rs`
- `crates/wp-cli-core/src/business/observability/mod.rs`
- `crates/wp-cli-core/src/business/observability/sources.rs`
- `crates/wp-cli-core/src/business/observability/sinks.rs`

### 2. 修改文件
- `crates/wp-cli-core/src/lib.rs` - 导出 business 模块
- `crates/wp-cli-core/src/obs/stat.rs` - 使用 business 层函数
- `crates/wp-cli-utils/src/lib.rs` - 移除业务代码导出
- `crates/wp-cli-utils/src/sources.rs` - 删除业务函数
- `crates/wp-cli-utils/src/scan.rs` - 删除业务函数

### 3. 架构改进
**之前**:
- wp-cli-core 依赖 wp-cli-utils 的业务函数
- wp-cli-utils 混合工具和业务逻辑

**之后**:
- wp-cli-core 包含所有业务逻辑
- wp-cli-utils 只保留纯工具函数
- 层次清晰，职责明确

## 收益
- ✅ 层次职责明确
- ✅ 依赖关系清晰
- ✅ 为后续合并奠定基础
- ✅ 减少不合理的向下依赖
```

---

### 任务 2.9: 提交更改 ⏱️ 5 分钟

```bash
# 查看更改
git status
git diff

# 添加所有更改
git add -A

# 提交
git commit -m "refactor(phase-2): move business logic from utils to core

## 阶段 2 完成内容

### 核心改进
- 将业务逻辑从 wp-cli-utils 移到 wp-cli-core
- 建立清晰的业务层结构 (business/observability/)
- 明确 utils 层只保留工具函数

### 新增
- crates/wp-cli-core/src/business/mod.rs
- crates/wp-cli-core/src/business/observability/mod.rs
- crates/wp-cli-core/src/business/observability/sources.rs
  - list_file_sources_with_lines()
  - total_input_from_wpsrc()
  - SrcLineItem, SrcLineReport
- crates/wp-cli-core/src/business/observability/sinks.rs
  - process_group()
  - process_group_v2()
  - ResolvedSinkLite

### 修改
- crates/wp-cli-core/src/lib.rs
  - 导出 business 模块

- crates/wp-cli-core/src/obs/stat.rs
  - 从 business 层导入函数
  - 移除对 wp-cli-utils 业务函数的依赖

- crates/wp-cli-utils/src/sources.rs
  - 删除业务逻辑函数（保留工具函数）

- crates/wp-cli-utils/src/scan.rs
  - 删除业务逻辑函数

- crates/wp-cli-utils/src/lib.rs
  - 移除业务代码导出
  - 保留纯工具函数导出

### 验证
- ✅ 所有测试通过 (911+ tests)
- ✅ 无编译警告
- ✅ 功能保持一致

### 架构改进
之前: CLI → Utils(业务+工具) → Config
之后: CLI → Business(业务) → Utils(工具) → Config

### 收益
- 层次职责明确
- 依赖关系清晰
- 为后续重构奠定基础
- 消除不合理的向下依赖

Refs: #refactor/simplify-cli-architecture
Phase: 2/6 - Business Logic Elevation
Next: Phase 3 - Shorten call chains

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
"

# 查看提交
git log --oneline -3
```

---

## 完成验证清单

在提交前，确认以下所有项：

- [ ] ✅ business 模块结构已创建
- [ ] ✅ sources 业务逻辑已移到 core
- [ ] ✅ sinks 业务逻辑已移到 core
- [ ] ✅ stat.rs 已更新使用新路径
- [ ] ✅ wp-cli-utils 移除业务代码
- [ ] ✅ 所有调用点已更新
- [ ] ✅ `cargo test --workspace` 全部通过
- [ ] ✅ 无新增编译警告
- [ ] ✅ 检查清单已更新
- [ ] ✅ 总结文档已创建
- [ ] ✅ 更改已提交

---

## 时间估算

| 任务 | 预计时间 | 实际时间 |
|------|---------|---------|
| 2.1 分析业务函数依赖 | 15 分钟 | ___ |
| 2.2 创建业务模块结构 | 10 分钟 | ___ |
| 2.3 移动 sources 逻辑 | 20 分钟 | ___ |
| 2.4 移动 sinks 逻辑 | 20 分钟 | ___ |
| 2.5 更新 stat.rs | 15 分钟 | ___ |
| 2.6 清理 utils | 15 分钟 | ___ |
| 2.7 运行测试 | 5 分钟 | ___ |
| 2.8 更新文档 | 10 分钟 | ___ |
| 2.9 提交更改 | 5 分钟 | ___ |
| **总计** | **1.5-2 小时** | ___ |

---

## 风险评估

### 风险等级: 🟡 中等

**原因**:
- 涉及跨 crate 代码移动
- 需要更新多处导入路径
- 影响业务逻辑组织

**缓解措施**:
- 逐步迁移，每个函数单独验证
- 频繁运行测试
- 保持提交粒度小

### 潜在问题

1. **导入路径错误**
   - **症状**: 编译错误 E0433
   - **解决**: 仔细检查 use 语句

2. **类型不匹配**
   - **症状**: 编译错误 E0308
   - **解决**: 确保类型定义在正确位置

3. **测试失败**
   - **症状**: 集成测试找不到函数
   - **解决**: 更新 pub use 导出

---

## 回滚方案

如果需要回滚：

```bash
# 查看提交历史
git log --oneline

# 回滚到阶段 1
git reset --hard <phase-1-commit-hash>

# 或者创建回滚提交
git revert HEAD
```

---

## 下一步预告

**阶段 3: 缩短调用链**

**目标**: 减少不必要的中间层调用

**主要任务**:
1. 重写 `stat_src_file()` 直接调用 config 层
2. 重写 `stat_sink_file()` 直接调用 config 层
3. 移除不必要的包装函数
4. 简化调用路径

**预期效果**:
- 调用链深度从 5 层减少到 2-3 层
- 代码可读性提升
- 性能略有提升

---

## 批准检查点

**请确认以下内容后批准执行**:

- [ ] 我理解阶段 2 的目标是将业务逻辑移到 core
- [ ] 我同意创建 business/observability 模块结构
- [ ] 我同意从 wp-cli-utils 移除业务代码
- [ ] 我理解这会改变导入路径
- [ ] 本阶段的所有操作都是安全的，可以随时回滚

**批准方式**:
- ✅ 批准执行: 请回复 "批准阶段 2"
- ❌ 需要调整: 请说明需要修改的部分
- ⏸️ 暂缓执行: 请说明原因

---

**准备好开始了吗？** 🚀
