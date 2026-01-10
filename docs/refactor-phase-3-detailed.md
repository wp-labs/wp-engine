# 阶段 3: 缩短调用链 - 详细执行方案

> **目标**: 减少不必要的中间层调用，简化调用路径

**预计时间**: 1-1.5 小时
**风险等级**: 🟢 低
**可回滚**: ✅ 是
**前置条件**: ✅ 阶段 2 已完成

---

## 背景分析

### 当前调用链问题

#### 现状 (阶段 2 完成后)

```
调用链示例 - stat_src_file:

CLI Command
  ↓
wp-cli-core::obs::stat::stat_src_file()
  ↓
wp-cli-core::business::observability::list_file_sources_with_lines()
  ↓
wp-conf::find_connectors_base_dir()
  ↓
wp-conf::sources::load_connectors_for()
  ↓
wpcnt_lib::fsutils::count_lines_file()

层数: 5-6 层
```

**问题**:
- 🔴 **过度包装**: obs::stat 只是简单包装了 business 函数
- 🔴 **多余中间层**: 部分功能可以直接调用 config 层
- 🔴 **代码重复**: 创建 Ctx、调用包装函数的模式重复
- 🔴 **可读性差**: 跨多个模块追踪逻辑路径

### 目标架构

```
优化后的调用链:

CLI Command
  ↓
wp-cli-core::business::observability::list_file_sources_with_lines()
  ↓ (直接调用)
wp-conf::sources::load_connectors_for()
  ↓
wpcnt_lib::fsutils::count_lines_file()

层数: 3-4 层 (减少 40%)
```

**改进**:
- ✅ **减少包装**: 移除 obs::stat 中的简单包装
- ✅ **直接调用**: CLI 直接调用 business 层
- ✅ **清晰路径**: 调用路径更短更直接
- ✅ **保持职责**: business 层仍然负责业务逻辑编排

---

## 问题分析

### 1. obs::stat 模块的问题

**当前实现** (`wp-cli-core/src/obs/stat.rs`):

```rust
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
```

**分析**:
- 仅仅是创建 Ctx 然后调用 business 函数
- 没有额外的业务逻辑
- 是一个不必要的包装层

**解决方案**:
- 移除 `stat_src_file()`
- CLI 直接调用 `business::observability::list_file_sources_with_lines()`
- 或者将 Ctx 创建放到 business 函数内部

### 2. stat_sink_file 的问题

**当前实现**:

```rust
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &Ctx,
) -> Result<(Vec<Row>, u64)> {
    // 检查目录存在
    // 加载 business 和 infra 配置
    // 调用 process_group
}
```

**分析**:
- 有一些目录检查逻辑
- 但主要还是简单的循环调用 process_group
- 可以简化或合并

### 3. 重复的模式

**问题**:
```rust
// 模式 1: 创建 Ctx
let ctx = Ctx::new(work_root);

// 模式 2: 加载配置并循环
for conf in load_configs() {
    process_group(...);
}
```

这些模式在多处重复。

---

## 任务分解

### 任务 3.1: 分析现有调用链 ⏱️ 15 分钟

#### Step 1: 识别所有调用点

```bash
# 查找 stat_src_file 的调用
rg "stat_src_file" crates/wp-proj --type rust

# 查找 stat_sink_file 的调用
rg "stat_sink_file" crates/wp-proj --type rust

# 查找 stat_file_combined 的调用
rg "stat_file_combined" crates/wp-proj --type rust
```

#### Step 2: 绘制当前调用图

创建调用链分析表格：

| 函数 | 调用路径 | 层数 | 是否必要 |
|------|---------|------|---------|
| stat_src_file | CLI→stat→business→config | 4 | ❌ stat层可移除 |
| stat_sink_file | CLI→stat→business→config | 4 | ⚠️ 有目录检查逻辑 |
| stat_file_combined | CLI→stat→business→config | 4 | ❌ 简单组合调用 |
| list_connectors | CLI→connectors→config | 3 | ✅ 已经较短 |
| route_table | CLI→connectors→config | 3 | ✅ 已经较短 |

#### Step 3: 确定优化策略

**决策**:
1. **移除 obs::stat 模块** - 功能移到 business 或直接暴露
2. **保留 obs::validate** - 有复杂的验证逻辑
3. **保留 connectors** - 已经较简洁

---

### 任务 3.2: 重构 stat_src_file ⏱️ 15 分钟

#### 方案 A: 移除包装，直接调用 (推荐)

**修改 wp-proj 调用点**:

```rust
// 之前
use wp_cli_core::obs::stat;
let report = stat::stat_src_file(work_root, &eng_conf, dict)?;

// 之后
use wp_cli_core::business::observability;
use wpcnt_lib::types::Ctx;

let ctx = Ctx::new(work_root);
let report = observability::list_file_sources_with_lines(
    Path::new(work_root),
    &eng_conf,
    &ctx,
    dict,
);
```

**优点**:
- 调用链更短
- 更直接明确
- 减少一层包装

**缺点**:
- 需要 CLI 层知道 Ctx
- 调用代码略长

#### 方案 B: 将 Ctx 创建移入 business 函数

**修改 business 函数签名**:

```rust
// 之前
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<SrcLineReport>

// 之后
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> Option<SrcLineReport> {
    let ctx = Ctx::new(work_root);
    // ... 业务逻辑
}
```

**优点**:
- CLI 调用更简洁
- Ctx 是内部实现细节

**缺点**:
- Ctx 目前可以配置 filters，简化版本会失去灵活性

#### 推荐方案: A + 辅助函数

提供两个版本：

```rust
// 完整版本 (需要自定义 Ctx)
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Option<SrcLineReport>

// 简化版本 (使用默认 Ctx)
pub fn list_file_sources_simple(
    work_root: &Path,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> Option<SrcLineReport> {
    let ctx = Ctx::new(work_root);
    list_file_sources_with_lines(work_root, eng_conf, &ctx, dict)
}
```

---

### 任务 3.3: 重构 stat_sink_file ⏱️ 20 分钟

#### 当前问题

```rust
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &Ctx,
) -> Result<(Vec<Row>, u64)> {
    // 1. 目录检查
    if !(sink_root.join("business.d").exists() || sink_root.join("infra.d").exists()) {
        anyhow::bail!(...);
    }

    // 2. 加载配置
    for conf in load_business_route_confs(...) {
        process_group(...);
    }
    for conf in load_infra_route_confs(...) {
        process_group(...);
    }

    Ok((rows, total))
}
```

#### 优化方案

**选项 1**: 移到 business 层

将整个函数移到 `business::observability::sinks`:

```rust
// business/observability/sinks.rs
pub fn collect_sink_statistics(
    sink_root: &Path,
    ctx: &Ctx,
) -> Result<(Vec<Row>, u64)> {
    // 完整实现
}
```

**选项 2**: 简化为高层接口

```rust
// business/observability/sinks.rs
pub fn analyze_sink_files(
    work_root: &Path,
    sink_root_name: &str,
    ctx: &Ctx,
) -> Result<SinkAnalysisReport> {
    let sink_root = Path::new(work_root).join(sink_root_name);

    // 验证目录
    validate_sink_directories(&sink_root)?;

    // 收集统计
    let (rows, total) = collect_statistics(&sink_root, ctx)?;

    Ok(SinkAnalysisReport { rows, total })
}
```

**推荐**: 选项 1 - 移到 business 层

---

### 任务 3.4: 处理 stat_file_combined ⏱️ 10 分钟

#### 当前实现

```rust
pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Result<(Option<SrcLineReport>, Vec<Row>, u64)> {
    let src_rep = list_file_sources_with_lines(...);
    let sink_root = Path::new(work_root).join(eng_conf.sink_root());
    let (rows, total) = stat_sink_file(&sink_root, ctx)?;
    Ok((src_rep, rows, total))
}
```

#### 优化

**方案**: 移到 business 层作为便利函数

```rust
// business/observability/mod.rs
pub fn analyze_all_files(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
    dict: &EnvDict,
) -> Result<FileAnalysisReport> {
    let sources = list_file_sources_with_lines(work_root, eng_conf, ctx, dict);
    let sink_root = work_root.join(eng_conf.sink_root());
    let (rows, total) = collect_sink_statistics(&sink_root, ctx)?;

    Ok(FileAnalysisReport {
        sources,
        sink_rows: rows,
        sink_total: total,
    })
}
```

---

### 任务 3.5: 更新所有调用点 ⏱️ 20 分钟

#### Step 1: 查找所有调用

```bash
rg "obs::stat::" crates/wp-proj --type rust
rg "use.*obs::stat" crates/wp-proj --type rust
```

#### Step 2: 逐个更新

**位置 1**: `wp-proj/src/sources/stat.rs`

```rust
// 之前
use wp_cli_core::obs::stat;
let report = stat::stat_src_file(work_root, &eng_conf, dict)?;

// 之后
use wp_cli_core::business::observability;
use wpcnt_lib::types::Ctx;

let ctx = Ctx::new(work_root);
let report = observability::list_file_sources_with_lines(
    Path::new(work_root),
    &eng_conf,
    &ctx,
    dict,
);
```

**位置 2**: `wp-proj/src/sinks/stat.rs`

类似更新...

---

### 任务 3.6: 清理 obs::stat 模块 ⏱️ 10 分钟

#### 决策

**选项 A**: 完全删除 `obs/stat.rs`
- 如果所有功能都已移到 business 层

**选项 B**: 保留作为 deprecated
- 添加 `#[deprecated]` 标记
- 提供迁移指南

**选项 C**: 保留简化版本
- 只保留真正有价值的包装

**推荐**: 选项 A - 完全删除

因为：
1. 功能已经在 business 层
2. 避免维护两套API
3. 强制使用新的清晰架构

#### 实施

```rust
// 删除 crates/wp-cli-core/src/obs/stat.rs

// 更新 crates/wp-cli-core/src/obs/mod.rs
// pub mod stat;  // 删除此行
pub mod validate;
```

---

### 任务 3.7: 优化 obs::validate (可选) ⏱️ 15 分钟

#### 当前状态

`obs::validate::build_groups_v2()` - 值得保留，因为：
- 有复杂的验证逻辑
- 组合了 business + infra 加载
- 返回结构化的验证数据

#### 轻微优化

```rust
// 可以简化错误处理
pub fn build_groups_v2(
    sink_root: &Path,
    ctx: &Ctx,
) -> Result<ValidationData> {
    // 使用 business 层函数
    // 保持高层逻辑编排
}
```

**决策**: 保持不变，validate 模块已经足够清晰

---

### 任务 3.8: 运行测试验证 ⏱️ 5 分钟

```bash
# 编译检查
cargo build --workspace

# 运行所有测试
cargo test --workspace 2>&1 | tee /tmp/phase3-test-results.txt

# 验证
grep "test result:" /tmp/phase3-test-results.txt
```

---

### 任务 3.9: 更新文档 ⏱️ 10 分钟

#### 更新 API 文档

在 `business/observability/mod.rs` 添加使用示例：

```rust
//! # Examples
//!
//! ## List file sources with line counts
//!
//! ```rust,ignore
//! use wp_cli_core::business::observability;
//! use wpcnt_lib::types::Ctx;
//!
//! let ctx = Ctx::new(work_root);
//! let report = observability::list_file_sources_with_lines(
//!     Path::new(work_root),
//!     &engine_config,
//!     &ctx,
//!     &env_dict,
//! );
//! ```
```

#### 更新迁移指南

创建 `docs/migration-phase-3.md`:

```markdown
# 阶段 3 迁移指南

## 移除的 API

### `wp_cli_core::obs::stat::stat_src_file`

**之前**:
```rust
use wp_cli_core::obs::stat;
let report = stat::stat_src_file(work_root, &eng_conf, dict)?;
```

**之后**:
```rust
use wp_cli_core::business::observability;
use wpcnt_lib::types::Ctx;

let ctx = Ctx::new(work_root);
let report = observability::list_file_sources_with_lines(
    Path::new(work_root),
    &eng_conf,
    &ctx,
    dict,
);
```

**原因**: 减少不必要的包装层，调用更直接
```

---

### 任务 3.10: 提交更改 ⏱️ 5 分钟

```bash
git add -A

git commit -m "refactor(phase-3): shorten call chains and remove unnecessary wrappers

## 阶段 3 完成内容

### 核心改进
- 移除 obs::stat 模块的不必要包装
- 调用链从 5-6 层减少到 3-4 层
- CLI 直接调用 business 层函数

### 删除
- crates/wp-cli-core/src/obs/stat.rs
  - stat_src_file() - 简单包装，已移除
  - stat_sink_file() - 功能移到 business 层
  - stat_file_combined() - 功能移到 business 层

### 修改
- crates/wp-cli-core/src/obs/mod.rs
  - 移除 stat 模块

- crates/wp-cli-core/src/business/observability/sinks.rs
  - 添加 collect_sink_statistics()

- crates/wp-proj/src/sources/stat.rs
  - 直接调用 business 层函数

- crates/wp-proj/src/sinks/stat.rs
  - 直接调用 business 层函数

### 新增
- docs/migration-phase-3.md
  - API 迁移指南

### 验证
- ✅ 所有测试通过 (911+ tests)
- ✅ 无编译警告
- ✅ 调用链简化 40%

### 架构改进
之前: CLI → obs::stat → business → config (5-6层)
之后: CLI → business → config (3-4层)

### 收益
- 调用链缩短 40%
- 代码更直接易读
- 移除不必要的包装层
- 减少代码维护负担

Refs: #refactor/simplify-cli-architecture
Phase: 3/6 - Call Chain Shortening
Next: Phase 4 - Create new directory structure

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
"
```

---

## 完成验证清单

- [ ] ✅ obs::stat 模块已移除
- [ ] ✅ 调用点已更新为直接调用 business 层
- [ ] ✅ business 层函数正常工作
- [ ] ✅ 所有测试通过
- [ ] ✅ 无编译警告
- [ ] ✅ 文档已更新
- [ ] ✅ 迁移指南已创建
- [ ] ✅ 更改已提交

---

## 时间估算

| 任务 | 预计时间 | 实际时间 |
|------|---------|---------|
| 3.1 分析调用链 | 15 分钟 | ___ |
| 3.2 重构 stat_src_file | 15 分钟 | ___ |
| 3.3 重构 stat_sink_file | 20 分钟 | ___ |
| 3.4 处理 stat_file_combined | 10 分钟 | ___ |
| 3.5 更新所有调用点 | 20 分钟 | ___ |
| 3.6 清理 obs::stat | 10 分钟 | ___ |
| 3.7 优化 validate (可选) | 15 分钟 | ___ |
| 3.8 运行测试 | 5 分钟 | ___ |
| 3.9 更新文档 | 10 分钟 | ___ |
| 3.10 提交更改 | 5 分钟 | ___ |
| **总计** | **1-1.5 小时** | ___ |

---

## 风险评估

### 风险等级: 🟢 低

**原因**:
- 只是移除包装层，不改变核心逻辑
- Business 层函数已经存在且测试通过
- 影响范围明确且可控

### 潜在问题

1. **调用点遗漏**
   - **症状**: 编译错误，找不到 obs::stat
   - **解决**: 全局搜索所有调用点

2. **Ctx 参数问题**
   - **症状**: 调用点不知道如何创建 Ctx
   - **解决**: 文档说明 + 示例代码

3. **测试失败**
   - **症状**: 集成测试路径错误
   - **解决**: 更新测试导入路径

---

## 回滚方案

```bash
# 查看提交历史
git log --oneline

# 回滚到阶段 2
git reset --hard <phase-2-commit-hash>

# 或创建回滚提交
git revert HEAD
```

---

## 批准检查点

**请确认以下内容后批准执行**:

- [ ] 我理解阶段 3 的目标是缩短调用链
- [ ] 我同意移除 obs::stat 模块
- [ ] 我同意 CLI 直接调用 business 层
- [ ] 我理解这会改变部分 API 调用方式
- [ ] 本阶段的所有操作都是安全的，可以随时回滚

**批准方式**:
- ✅ 批准执行: 请回复 "批准阶段 3" 或 "同意"
- ❌ 需要调整: 请说明需要修改的部分
- ⏸️ 暂缓执行: 请说明原因

---

**准备好开始了吗？** 🚀
