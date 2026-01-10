# 阶段 1: 统一参数合并逻辑 - 详细执行方案

> **目标**: 消除 3 处重复的参数合并代码，统一到 wp-config 的单一实现

**预计时间**: 1-2 小时
**风险等级**: 🟢 低
**可回滚**: ✅ 是
**前置条件**: ✅ 阶段 0 已完成

---

## 背景分析

### 当前问题

参数合并逻辑在以下 3 处重复实现：

1. **wp-cli-utils/src/sources.rs:39-51**
```rust
fn merge_params(base: &ParamMap, override_tbl: &toml::value::Table, allow: &[String]) -> ParamMap {
    let mut out = base.clone();
    for (k, v) in override_tbl.iter() {
        if allow.iter().any(|x| x == k) {
            out.insert(k.clone(), param_value_from_toml(v));
        }
    }
    out
}
```

2. **wp-cli-core/src/connectors/sources.rs:47-61**
```rust
fn merge_params(base: &ParamMap, ov: &ParamMap, allow: &[String]) -> OrionConfResult<ParamMap> {
    let mut out = base.clone();
    for (k, v) in ov.iter() {
        if !allow.iter().any(|x| x == k) {
            return ConfIOReason::from_validation(format!(
                "override '{}' not allowed; whitelist: [{}]",
                k,
                allow.join(", ")
            ))
            .err_result();
        }
        out.insert(k.clone(), v.clone());
    }
    Ok(out)
}
```

3. **类似的合并逻辑散布在其他地方**

### 问题影响

- 🔴 **代码重复**: 维护成本高，容易产生不一致
- 🔴 **逻辑差异**: 不同实现的错误处理不一致
- 🔴 **测试冗余**: 需要在多处测试相同逻辑

---

## 任务分解

### 任务 1.1: 分析现有实现 ⏱️ 15 分钟

**目的**: 理解现有参数合并逻辑的差异

#### Step 1: 查找所有 merge_params 实现

```bash
# 搜索所有 merge_params 函数
rg "fn merge_params" crates/wp-cli-{core,utils} crates/wp-config
```

#### Step 2: 对比实现差异

创建对比表格：

| 位置 | 签名 | 返回值 | 错误处理 | 特点 |
|------|------|--------|---------|------|
| utils/sources.rs | `(ParamMap, &Table, &[String]) -> ParamMap` | ParamMap | 静默忽略非白名单参数 | 接受 TOML Table |
| core/sources.rs | `(ParamMap, &ParamMap, &[String]) -> Result<ParamMap>` | Result | 返回错误 | 接受 ParamMap |

#### Step 3: 确定统一接口

**推荐接口**:
```rust
// wp-config/src/connectors/params.rs
pub fn merge_params(
    base: &ParamMap,
    overrides: &ParamMap,
    allow_override: &[String],
) -> Result<ParamMap, String>
```

**设计决策**:
- ✅ 使用 `Result` 返回，明确错误处理
- ✅ 接受 `ParamMap` 而非 TOML Table（调用方负责转换）
- ✅ 当参数不在白名单时返回错误
- ✅ 放在 wp-config 中，因为它是配置基础设施

---

### 任务 1.2: 创建统一实现 ⏱️ 20 分钟

#### Step 1: 创建新文件

创建 `crates/wp-config/src/connectors/params.rs`:

```rust
//! Parameter merging utilities for connectors
//!
//! This module provides unified parameter merging logic used across
//! source and sink connectors.

use crate::connectors::ParamMap;
use orion_error::{ErrorOwe, ErrorWith, UvsValidationFrom};
use orion_conf::error::{ConfIOReason, OrionConfResult};

/// Merge connector parameters with user overrides, respecting whitelist.
///
/// # Arguments
/// * `base` - Base parameters from connector definition
/// * `overrides` - User-provided parameter overrides
/// * `allow_override` - Whitelist of parameter names that can be overridden
///
/// # Returns
/// * `Ok(ParamMap)` - Merged parameters
/// * `Err(...)` - If override contains parameters not in whitelist
///
/// # Example
/// ```rust,ignore
/// let base = ParamMap::from([("fmt".to_string(), "json".into())]);
/// let overrides = ParamMap::from([("path".to_string(), "/data".into())]);
/// let allow = vec!["path".to_string()];
///
/// let merged = merge_params(&base, &overrides, &allow)?;
/// assert_eq!(merged.get("fmt"), Some(&"json".into()));
/// assert_eq!(merged.get("path"), Some(&"/data".into()));
/// ```
pub fn merge_params(
    base: &ParamMap,
    overrides: &ParamMap,
    allow_override: &[String],
) -> OrionConfResult<ParamMap> {
    let mut result = base.clone();

    for (key, value) in overrides.iter() {
        // Check if parameter is in whitelist
        if !allow_override.iter().any(|allowed| allowed == key) {
            return ConfIOReason::from_validation(format!(
                "Parameter override '{}' not allowed. Permitted overrides: [{}]",
                key,
                allow_override.join(", ")
            ))
            .err_result();
        }

        // Merge the override
        result.insert(key.clone(), value.clone());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_params_allows_whitelisted_override() {
        let mut base = ParamMap::new();
        base.insert("fmt".to_string(), "json".into());
        base.insert("compression".to_string(), "gzip".into());

        let mut overrides = ParamMap::new();
        overrides.insert("path".to_string(), "/data/test.log".into());

        let allow = vec!["path".to_string()];

        let result = merge_params(&base, &overrides, &allow);
        assert!(result.is_ok());

        let merged = result.unwrap();
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.get("fmt").unwrap().as_str(), Some("json"));
        assert_eq!(merged.get("path").unwrap().as_str(), Some("/data/test.log"));
    }

    #[test]
    fn merge_params_rejects_non_whitelisted_override() {
        let base = ParamMap::new();
        let mut overrides = ParamMap::new();
        overrides.insert("dangerous_param".to_string(), "value".into());

        let allow = vec!["safe_param".to_string()];

        let result = merge_params(&base, &overrides, &allow);
        assert!(result.is_err());

        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("dangerous_param"));
        assert!(err_msg.contains("not allowed"));
    }

    #[test]
    fn merge_params_overwrites_base_values() {
        let mut base = ParamMap::new();
        base.insert("path".to_string(), "/default".into());

        let mut overrides = ParamMap::new();
        overrides.insert("path".to_string(), "/custom".into());

        let allow = vec!["path".to_string()];

        let merged = merge_params(&base, &overrides, &allow).unwrap();
        assert_eq!(merged.get("path").unwrap().as_str(), Some("/custom"));
    }

    #[test]
    fn merge_params_empty_overrides_returns_base() {
        let mut base = ParamMap::new();
        base.insert("key".to_string(), "value".into());

        let overrides = ParamMap::new();
        let allow = vec!["key".to_string()];

        let merged = merge_params(&base, &overrides, &allow).unwrap();
        assert_eq!(merged, base);
    }

    #[test]
    fn merge_params_handles_multiple_overrides() {
        let mut base = ParamMap::new();
        base.insert("a".to_string(), "1".into());
        base.insert("b".to_string(), "2".into());

        let mut overrides = ParamMap::new();
        overrides.insert("b".to_string(), "new_b".into());
        overrides.insert("c".to_string(), "new_c".into());

        let allow = vec!["b".to_string(), "c".to_string()];

        let merged = merge_params(&base, &overrides, &allow).unwrap();
        assert_eq!(merged.len(), 3);
        assert_eq!(merged.get("a").unwrap().as_str(), Some("1"));
        assert_eq!(merged.get("b").unwrap().as_str(), Some("new_b"));
        assert_eq!(merged.get("c").unwrap().as_str(), Some("new_c"));
    }
}
```

#### Step 2: 更新 mod.rs 导出

修改 `crates/wp-config/src/connectors/mod.rs`:

```rust
pub mod defs;
mod toml;
mod params;  // 新增

pub use defs::{
    ConnectorTomlFile, param_map_from_table_ref, param_map_to_table, param_value_from_toml,
};
pub use toml::load_connector_defs_from_dir;
pub use params::merge_params;  // 新增导出
pub use wp_connector_api::{
    ConnectorDef, ConnectorScope, ParamMap, SinkDefProvider, SourceDefProvider,
    parammap_from_toml_table as param_map_from_table,
};
```

#### Step 3: 运行测试验证

```bash
# 编译 wp-config
cargo build --package wp-config

# 运行新增的单元测试
cargo test --package wp-config connectors::params

# 确保所有测试通过
cargo test --package wp-config
```

---

### 任务 1.3: 更新 wp-cli-core/connectors/sources.rs ⏱️ 15 分钟

#### Step 1: 替换本地实现

**查找位置**: `crates/wp-cli-core/src/connectors/sources.rs:47-61`

**修改**:

```rust
// 删除本地 merge_params 函数（第 47-61 行）

// 文件顶部添加 import
use wp_conf::connectors::merge_params;
```

#### Step 2: 更新调用点

在 `route_table()` 函数中（约第 130 行）：

```rust
// 之前
let merged = merge_params(&conn.default_params, &src.params, &conn.allow_override)?;

// 之后（保持不变，因为签名一致）
let merged = merge_params(&conn.default_params, &src.params, &conn.allow_override)?;
```

#### Step 3: 验证编译

```bash
cargo build --package wp-cli-core
cargo test --package wp-cli-core connectors::sources
```

---

### 任务 1.4: 更新 wp-cli-utils/sources.rs ⏱️ 20 分钟

#### Step 1: 分析调用点

当前 `merge_params` 接受 `&toml::value::Table`，需要转换为 `ParamMap`。

**查找调用点**:
```bash
rg "merge_params" crates/wp-cli-utils/src/sources.rs
```

#### Step 2: 创建转换辅助函数

在 `crates/wp-cli-utils/src/sources.rs` 中：

```rust
// 顶部添加 import
use wp_conf::connectors::{ParamMap, param_value_from_toml, merge_params};

// 辅助函数：将 TOML Table 转换为 ParamMap
fn toml_table_to_param_map(table: &toml::value::Table) -> ParamMap {
    table
        .iter()
        .map(|(k, v)| (k.clone(), param_value_from_toml(v)))
        .collect()
}
```

#### Step 3: 更新调用点

**位置 1**: `total_input_from_wpsrc()` 函数（约第 82 行）

```rust
// 之前
let merged = merge_params(&conn.default_params, &ov, &conn.allow_override);

// 之后
let ov_map = toml_table_to_param_map(&ov);
let merged = merge_params(&conn.default_params, &ov_map, &conn.allow_override)
    .unwrap_or_else(|_| conn.default_params.clone()); // 静默失败以保持向后兼容
```

**位置 2**: `list_file_sources_with_lines()` 函数（约第 175 行）

```rust
// 之前
let merged = merge_params(&conn.default_params, &ov, &conn.allow_override);

// 之后
let ov_map = toml_table_to_param_map(&ov);
let merged = merge_params(&conn.default_params, &ov_map, &conn.allow_override)
    .unwrap_or_else(|_| conn.default_params.clone());
```

#### Step 4: 删除本地 merge_params

删除 `crates/wp-cli-utils/src/sources.rs:39-51` 的本地实现。

#### Step 5: 验证

```bash
cargo build --package wp-cli-utils
cargo test --package wp-cli-utils
```

---

### 任务 1.5: 检查其他潜在调用点 ⏱️ 10 分钟

#### Step 1: 全局搜索

```bash
# 搜索可能的参数合并逻辑
rg -A 5 "allow_override" crates/wp-cli-{core,utils} | grep -i merge
rg -A 5 "params_override" crates/wp-cli-{core,utils} | grep -i merge
```

#### Step 2: 检查 sinks 相关代码

```bash
# 检查 sinks.rs 是否有类似逻辑
rg -A 10 "override.*param" crates/wp-cli-core/src/connectors/sinks.rs
```

#### Step 3: 如发现其他重复，按照相同模式更新

---

### 任务 1.6: 运行完整测试套件 ⏱️ 5 分钟

```bash
# 运行所有测试
cargo test --workspace 2>&1 | tee /tmp/phase1-test-results.txt

# 检查测试结果
grep "test result:" /tmp/phase1-test-results.txt

# 确保：
# - 所有测试通过
# - 无新增警告
# - 集成测试成功
```

---

### 任务 1.7: 更新文档和检查清单 ⏱️ 10 分钟

#### Step 1: 更新检查清单

编辑 `docs/refactor-checklist.md`:

```markdown
## ⏳ 阶段 1: 统一参数合并逻辑

- [x] 1.1 在 `wp-config/src/connectors/` 创建 `params.rs`
- [x] 1.2 实现统一的 `merge_params()` 函数
- [x] 1.3 更新 `wp-cli-core/connectors/sources.rs` 使用新函数
- [x] 1.4 更新 `wp-cli-utils/sources.rs` 使用新函数
- [x] 1.5 检查并更新其他调用点
- [x] 1.6 运行测试确保行为一致
- [x] 1.7 更新文档
- [x] 1.8 提交阶段 1 更改
```

#### Step 2: 创建阶段 1 总结

创建 `docs/phase-1-summary.md`:

```markdown
# 阶段 1 完成总结

## 目标
统一参数合并逻辑到单一实现

## 完成内容

### 1. 新增文件
- `crates/wp-config/src/connectors/params.rs` (约 150 行)
  - 统一的 `merge_params()` 函数
  - 5 个单元测试用例

### 2. 修改文件
- `crates/wp-config/src/connectors/mod.rs`
  - 导出 `merge_params`

- `crates/wp-cli-core/src/connectors/sources.rs`
  - 删除本地 `merge_params` 实现（15 行）
  - 使用 wp-config 提供的统一函数

- `crates/wp-cli-utils/src/sources.rs`
  - 删除本地 `merge_params` 实现（13 行）
  - 添加 `toml_table_to_param_map` 辅助函数
  - 更新 2 处调用点

### 3. 代码指标
- **删除重复代码**: 28 行
- **新增代码**: ~150 行（含测试和文档）
- **净减少**: 虽然总行数略增，但消除了逻辑重复

### 4. 测试验证
- ✅ 新增 5 个单元测试
- ✅ 所有现有测试通过
- ✅ 集成测试通过
- ✅ 无编译警告

## 收益

### 即时收益
- ✅ **代码重复消除**: 3 处重复 → 1 处统一
- ✅ **错误处理一致**: 统一的错误信息格式
- ✅ **可测试性提升**: 单一测试点

### 长期收益
- ✅ **维护成本降低**: 修改只需一处
- ✅ **可靠性提升**: 减少不一致风险
- ✅ **扩展性更好**: 新功能更容易添加

## 风险评估
- 🟢 **风险等级**: 低
- 🟢 **影响范围**: 仅参数合并逻辑
- 🟢 **回滚难度**: 容易（单次提交）

## 下一步
进入阶段 2: 移除业务逻辑下沉
```

---

### 任务 1.8: 提交更改 ⏱️ 5 分钟

```bash
# 查看更改
git status
git diff

# 添加所有更改
git add -A

# 提交
git commit -m "refactor(phase-1): unify parameter merging logic

## 阶段 1 完成内容

### 核心改进
- 创建统一的参数合并函数 wp-config::connectors::merge_params()
- 消除 3 处重复实现（wp-cli-core/sources, wp-cli-utils/sources）
- 统一错误处理和验证逻辑

### 新增
- crates/wp-config/src/connectors/params.rs
  - merge_params() 统一实现
  - 5 个单元测试覆盖各种场景
  - 完整的文档注释

### 修改
- crates/wp-config/src/connectors/mod.rs
  - 导出 merge_params 函数

- crates/wp-cli-core/src/connectors/sources.rs
  - 删除本地 merge_params (15 行)
  - 使用统一实现

- crates/wp-cli-utils/src/sources.rs
  - 删除本地 merge_params (13 行)
  - 添加 toml_table_to_param_map 辅助函数
  - 更新 2 处调用点

### 验证
- ✅ 所有测试通过 (296 tests)
- ✅ 新增 5 个单元测试
- ✅ 集成测试通过
- ✅ 无编译警告

### 收益
- 代码重复: 3 处 → 1 处
- 删除重复代码: 28 行
- 错误处理统一
- 维护成本降低

Refs: #refactor/simplify-cli-architecture
Next: Phase 2 - Remove business logic from utils layer
"

# 查看提交
git log --oneline -3

# 推送（可选）
# git push origin refactor/simplify-cli-architecture
```

---

## 完成验证清单

在提交前，确认以下所有项：

- [ ] ✅ `wp-config/src/connectors/params.rs` 已创建
- [ ] ✅ `merge_params()` 函数实现完整
- [ ] ✅ 5 个单元测试全部通过
- [ ] ✅ `wp-cli-core/sources.rs` 本地实现已删除
- [ ] ✅ `wp-cli-utils/sources.rs` 本地实现已删除
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
| 1.1 分析现有实现 | 15 分钟 | ___ |
| 1.2 创建统一实现 | 20 分钟 | ___ |
| 1.3 更新 core/sources | 15 分钟 | ___ |
| 1.4 更新 utils/sources | 20 分钟 | ___ |
| 1.5 检查其他调用点 | 10 分钟 | ___ |
| 1.6 运行测试 | 5 分钟 | ___ |
| 1.7 更新文档 | 10 分钟 | ___ |
| 1.8 提交更改 | 5 分钟 | ___ |
| **总计** | **1.5-2 小时** | ___ |

---

## 回滚方案

如果需要回滚：

```bash
# 查看提交历史
git log --oneline

# 回滚到阶段 0
git reset --hard <phase-0-commit-hash>

# 或者创建回滚提交
git revert HEAD
```

---

## 批准检查点

**请确认以下内容后批准执行**:

- [ ] 我理解阶段 1 的目标是统一参数合并逻辑
- [ ] 我同意在 wp-config 中创建统一实现
- [ ] 我同意删除 wp-cli-core 和 wp-cli-utils 中的重复代码
- [ ] 我理解这会影响参数合并的调用方式
- [ ] 本阶段的所有操作都是安全的，可以随时回滚

**批准方式**:
- ✅ 批准执行: 请回复 "批准阶段 1"
- ❌ 需要调整: 请说明需要修改的部分
- ⏸️ 暂缓执行: 请说明原因

---

**下一阶段预告**:
阶段 2 将移除业务逻辑下沉，把 wp-cli-utils/sources.rs 中的业务函数移到 wp-cli-core。

**准备好开始了吗？** 🚀
