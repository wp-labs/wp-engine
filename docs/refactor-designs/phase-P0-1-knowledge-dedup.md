# P0-1 阶段详细设计：消除知识库操作重复

**优先级**: P0（最高）
**预计工作量**: 2-3 天
**风险等级**: 🟢 低
**依赖**: 无

---

## 一、问题陈述

### 1.1 当前状况

**重复位置**:
```
wp-proj/src/models/knowledge.rs        - 237 行（完整实现）
wp-cli-core/src/knowdb/mod.rs          - 223 行（完整实现）
```

**重复内容** (~180 行代码):

1. **类型定义重复**:
```rust
// 两个文件中完全相同的定义
pub struct TableCheck {
    pub name: String,
    pub dir: PathBuf,
    pub create_ok: bool,
    pub insert_ok: bool,
    pub data_ok: bool,
    pub columns_ok: bool,
}

pub struct CheckReport {
    pub total: usize,
    pub ok: usize,
    pub fail: usize,
    pub tables: Vec<TableCheck>,
}

pub struct CleanReport {
    pub removed_models_dir: bool,
    pub removed_authority_cache: bool,
    pub not_found_models: bool,
}
```

2. **函数实现重复**:
```rust
// 完全相同的实现逻辑
pub fn init(work_root: &str, full: bool) -> Result<()>
pub fn check(work_root: &str, dict: &EnvDict) -> Result<CheckReport>
pub fn clean(work_root: &str) -> Result<CleanReport>
```

### 1.2 影响分析

**维护成本**:
- Bug 修复需要同步两处
- 测试需要维护两套
- 文档可能不一致

**当前使用情况**:
```bash
# wp-proj 的调用
crates/wp-proj/src/project/warp.rs:63  - Knowledge::new()
crates/wp-proj/src/project/init.rs:    - knowledge.init()

# wp-cli-core 的调用
crates/wp-cli-core/src/knowdb/mod.rs:  - 自身实现
```

---

## 二、设计方案

### 2.1 目标架构

**原则**: Single Source of Truth

```
┌──────────────────────────────────────┐
│     wp-proj/src/models/knowledge.rs  │
│  (薄包装层 - 50 行)                   │
│  - Component trait 实现               │
│  - 委托调用 wp-cli-core               │
│  - 错误类型转换                       │
└────────────┬─────────────────────────┘
             │ 委托
             ↓
┌──────────────────────────────────────┐
│     wp-cli-core/src/knowdb/mod.rs    │
│  (唯一实现 - 223 行)                  │
│  - 所有业务逻辑                       │
│  - 类型定义                           │
│  - 数据库操作                         │
└──────────────────────────────────────┘
```

### 2.2 接口设计

#### 2.2.1 保留在 wp-cli-core 的公共 API

```rust
// wp-cli-core/src/knowdb/mod.rs

pub struct TableCheck {
    pub name: String,
    pub dir: PathBuf,
    pub create_ok: bool,
    pub insert_ok: bool,
    pub data_ok: bool,
    pub columns_ok: bool,
}

pub struct CheckReport {
    pub total: usize,
    pub ok: usize,
    pub fail: usize,
    pub tables: Vec<TableCheck>,
}

pub struct CleanReport {
    pub removed_models_dir: bool,
    pub removed_authority_cache: bool,
    pub not_found_models: bool,
}

/// 初始化知识库
///
/// # 参数
/// - `work_root`: 项目根目录
/// - `full`: 是否完整初始化（包括示例数据）
pub fn init(work_root: &str, full: bool) -> Result<()>;

/// 检查知识库状态
///
/// # 参数
/// - `work_root`: 项目根目录
/// - `dict`: 环境变量字典
pub fn check(work_root: &str, dict: &EnvDict) -> Result<CheckReport>;

/// 清理知识库数据
///
/// # 参数
/// - `work_root`: 项目根目录
pub fn clean(work_root: &str) -> Result<CleanReport>;
```

#### 2.2.2 wp-proj 的新实现（薄包装）

```rust
// wp-proj/src/models/knowledge.rs

use crate::traits::Component;
use wp_error::run_error::{RunReason, RunResult};
use orion_variate::EnvDict;

// 重新导出类型（避免破坏现有 API）
pub use wp_cli_core::knowdb::{TableCheck, CheckReport, CleanReport};

/// 知识库管理组件
///
/// 提供知识库的初始化、检查和清理功能。
/// 实现委托给 wp-cli-core::knowdb。
#[derive(Debug, Clone, Default)]
pub struct Knowledge;

impl Knowledge {
    /// 创建新的知识库管理实例
    pub fn new() -> Self {
        Self
    }

    /// 初始化知识库
    ///
    /// # 参数
    /// - `work_root`: 项目根目录
    /// - `full`: 是否完整初始化（包括示例数据）
    ///
    /// # 示例
    /// ```no_run
    /// use wp_proj::models::Knowledge;
    ///
    /// let kb = Knowledge::new();
    /// kb.init("./my-project", false)?;
    /// # Ok::<(), wp_error::run_error::RunError>(())
    /// ```
    pub fn init(&self, work_root: &str, full: bool) -> RunResult<()> {
        wp_cli_core::knowdb::init(work_root, full)
            .map_err(|e| RunReason::from_conf(format!("知识库初始化失败: {}", e)).to_err())
    }

    /// 检查知识库状态
    ///
    /// # 参数
    /// - `work_root`: 项目根目录
    /// - `dict`: 环境变量字典
    pub fn check(&self, work_root: &str, dict: &EnvDict) -> RunResult<CheckReport> {
        wp_cli_core::knowdb::check(work_root, dict)
            .map_err(|e| RunReason::from_conf(format!("知识库检查失败: {}", e)).to_err())
    }

    /// 清理知识库数据
    ///
    /// # 参数
    /// - `work_root`: 项目根目录
    pub fn clean(&self, work_root: &str) -> RunResult<CleanReport> {
        wp_cli_core::knowdb::clean(work_root)
            .map_err(|e| RunReason::from_conf(format!("知识库清理失败: {}", e)).to_err())
    }
}

impl Component for Knowledge {
    fn component_name(&self) -> &'static str {
        "Knowledge"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_component_name() {
        let kb = Knowledge::new();
        assert_eq!(kb.component_name(), "Knowledge");
    }
}
```

---

## 三、实施步骤

### 3.1 准备阶段（30 分钟）

**步骤 1**: 创建功能分支
```bash
git checkout -b refactor/p0-1-knowledge-dedup
```

**步骤 2**: 备份当前实现
```bash
cp crates/wp-proj/src/models/knowledge.rs \
   crates/wp-proj/src/models/knowledge.rs.backup
```

**步骤 3**: 运行基线测试
```bash
cargo test -p wp-proj --lib
cargo test -p wp-cli-core --lib
```

预期：所有测试通过

### 3.2 实施阶段（2-3 天）

#### 阶段 1：确保 wp-cli-core API 稳定（半天）

**步骤 1.1**: 检查 wp-cli-core 的公共 API
```bash
# 确认以下函数已导出
grep "pub fn init\|pub fn check\|pub fn clean" \
  crates/wp-cli-core/src/knowdb/mod.rs
```

**步骤 1.2**: 添加详细文档（如果缺失）
```rust
// 在 wp-cli-core/src/knowdb/mod.rs 中添加

/// 初始化知识库
///
/// 在指定的工作根目录下创建知识库结构，包括：
/// - models/ 目录
/// - authority/ 缓存目录
/// - 初始表结构
///
/// # 参数
/// - `work_root`: 项目根目录路径
/// - `full`: 是否包含示例数据
///
/// # 错误
/// - 目录创建失败
/// - 数据库初始化失败
pub fn init(work_root: &str, full: bool) -> Result<()> {
    // 现有实现
}

// 为 check() 和 clean() 添加类似文档
```

**步骤 1.3**: 运行 wp-cli-core 测试
```bash
cargo test -p wp-cli-core knowdb
```

#### 阶段 2：重构 wp-proj 实现（1 天）

**步骤 2.1**: 重写 `knowledge.rs`

创建新文件内容（完整代码见上面 2.2.2 节）

**步骤 2.2**: 更新 Cargo.toml 依赖（如果需要）
```toml
# crates/wp-proj/Cargo.toml
[dependencies]
wp-cli-core = { path = "../wp-cli-core" }
wp-error = { path = "../wp-error" }
orion-variate = { workspace = true }
```

**步骤 2.3**: 检查编译
```bash
cargo check -p wp-proj
```

预期输出：
```
   Compiling wp-proj v1.8.0
    Finished dev [unoptimized + debuginfo] target(s) in X.XXs
```

#### 阶段 3：测试和验证（半天）

**步骤 3.1**: 运行单元测试
```bash
cargo test -p wp-proj models::knowledge
```

**步骤 3.2**: 运行集成测试
```bash
cargo test -p wp-proj --test '*'
```

**步骤 3.3**: 手动测试（可选）
```bash
cd crates/wp-proj
cargo run --example knowledge_test  # 如果有示例
```

#### 阶段 4：清理和文档（半天）

**步骤 4.1**: 删除备份文件
```bash
rm crates/wp-proj/src/models/knowledge.rs.backup
```

**步骤 4.2**: 更新 CHANGELOG（如果存在）
```markdown
## [Unreleased]

### Changed
- `wp_proj::models::Knowledge` 现在委托给 `wp_cli_core::knowdb`
- 减少 ~180 行重复代码
- API 保持向后兼容
```

**步骤 4.3**: 提交更改
```bash
git add -A
git commit -m "refactor(wp-proj): 消除知识库操作重复，委托给 wp-cli-core

- 将 Knowledge 实现改为薄包装
- 删除重复的 TableCheck, CheckReport, CleanReport 定义
- 重新导出 wp-cli-core 类型以保持向后兼容
- 减少 ~180 行重复代码

测试状态：
✅ 所有单元测试通过
✅ 集成测试通过
✅ API 向后兼容

🤖 Generated with [Claude Code](https://claude.com/claude-code)
Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 四、文件变更清单

### 4.1 修改的文件

| 文件 | 变更类型 | 行数变化 | 说明 |
|------|---------|----------|------|
| `wp-proj/src/models/knowledge.rs` | 🔄 重写 | 237 → 65 | 改为薄包装 |
| `wp-cli-core/src/knowdb/mod.rs` | ➕ 增强 | +10 | 添加文档注释 |

**详细变更**:

```diff
# wp-proj/src/models/knowledge.rs

- 删除 ~180 行重复的实现代码
- 删除 TableCheck, CheckReport, CleanReport 类型定义
+ 添加 pub use wp_cli_core::knowdb::{...} 重新导出
+ 保留 Knowledge struct，实现 Component trait
+ 所有方法委托给 wp_cli_core::knowdb::*
+ 添加错误转换（anyhow::Error → RunResult）
```

### 4.2 不需要修改的文件

以下文件**无需修改**（向后兼容）:
```
✅ crates/wp-proj/src/project/warp.rs    - Knowledge::new() 调用不变
✅ crates/wp-proj/src/project/init.rs    - knowledge.init() 调用不变
✅ 所有测试文件                          - API 签名保持一致
```

---

## 五、测试策略

### 5.1 单元测试

**现有测试保持**:
```rust
// wp-cli-core/src/knowdb/mod.rs 中的测试
#[cfg(test)]
mod tests {
    // 现有测试不变，继续通过
}
```

**新增测试**:
```rust
// wp-proj/src/models/knowledge.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn knowledge_delegates_to_cli_core() {
        let kb = Knowledge::new();
        let temp = tempdir().unwrap();
        let path = temp.path().to_str().unwrap();

        // 测试委托是否工作
        let result = kb.init(path, false);
        assert!(result.is_ok(), "委托调用应该成功");
    }

    #[test]
    fn knowledge_component_trait() {
        let kb = Knowledge::new();
        assert_eq!(kb.component_name(), "Knowledge");
    }

    #[test]
    fn error_conversion_works() {
        let kb = Knowledge::new();
        // 使用无效路径触发错误
        let result = kb.check("", &EnvDict::default());
        assert!(result.is_err(), "应该返回 RunResult 错误");
    }
}
```

### 5.2 集成测试

**验证点**:
1. ✅ WarpProject 可以正常创建 Knowledge 组件
2. ✅ init() 调用成功创建知识库
3. ✅ check() 返回正确的 CheckReport
4. ✅ clean() 清理文件

**测试用例**:
```rust
// tests/integration_knowledge.rs (新建或更新)

#[test]
fn knowledge_integration_with_warp_project() {
    let temp = tempdir().unwrap();
    let project = WarpProject::bare(temp.path());

    let kb = project.knowledge();

    // 初始化
    kb.init(temp.path().to_str().unwrap(), false).unwrap();

    // 检查
    let report = kb.check(
        temp.path().to_str().unwrap(),
        &EnvDict::default()
    ).unwrap();

    assert!(report.total > 0, "应该有检查项");

    // 清理
    let clean_result = kb.clean(temp.path().to_str().unwrap()).unwrap();
    assert!(clean_result.removed_models_dir || !clean_result.removed_models_dir);
}
```

### 5.3 手动测试清单

- [ ] 在干净的项目中运行 `wproj prj init`
- [ ] 运行 `wproj prj check` 验证知识库检查
- [ ] 运行 `wproj data clean` 验证清理功能
- [ ] 验证错误消息清晰易懂

---

## 六、风险评估与缓解

### 6.1 风险矩阵

| 风险 | 概率 | 影响 | 等级 | 缓解措施 |
|------|------|------|------|----------|
| API 不兼容 | 低 | 高 | 🟡 中 | 重新导出类型，保持签名一致 |
| 错误转换丢失信息 | 低 | 中 | 🟢 低 | 在转换时添加上下文 |
| 性能下降 | 极低 | 低 | 🟢 低 | 委托调用无额外开销 |
| 测试遗漏 | 低 | 中 | 🟢 低 | 保留所有现有测试 |

### 6.2 回滚策略

**如果出现问题**:

1. **立即回滚**:
```bash
git revert HEAD
git push origin refactor/p0-1-knowledge-dedup
```

2. **恢复备份**:
```bash
cp crates/wp-proj/src/models/knowledge.rs.backup \
   crates/wp-proj/src/models/knowledge.rs
```

3. **验证回滚**:
```bash
cargo test -p wp-proj
```

### 6.3 问题应对

**问题 1**: 编译失败
- **原因**: 类型导入缺失
- **解决**: 检查 `pub use` 语句，确保所有类型都重新导出

**问题 2**: 测试失败
- **原因**: 错误类型不匹配
- **解决**: 确认 `.map_err()` 正确转换为 RunResult

**问题 3**: 行为不一致
- **原因**: wp-cli-core 实现与 wp-proj 旧实现有差异
- **解决**: 对比两边逻辑，必要时修复 wp-cli-core

---

## 七、验收标准

### 7.1 功能标准

- ✅ 所有 Knowledge 方法正常工作（init, check, clean）
- ✅ 错误消息清晰且包含上下文
- ✅ Component trait 正确实现
- ✅ 向后兼容（现有调用代码无需修改）

### 7.2 质量标准

- ✅ 代码行数减少 >70%（237 → <70 行）
- ✅ 所有单元测试通过（63 个）
- ✅ 所有集成测试通过
- ✅ 文档覆盖率 100%（所有公共 API 有文档）
- ✅ 无编译警告

### 7.3 性能标准

- ✅ 委托调用无可测量的性能开销（<1ms）
- ✅ 内存使用无增加

---

## 八、成功指标

**代码指标**:
- 重复代码行数：237 → 0（-100%）
- wp-proj/knowledge.rs 行数：237 → 65（-73%）
- 总代码行数减少：~180 行

**质量指标**:
- 测试通过率：100%
- 文档覆盖率：100%
- 编译警告：0

**时间指标**:
- 预计工作量：2-3 天
- 实际工作量：_____ 天（待填写）

---

## 九、后续步骤

完成后，此重构将为以下工作铺路：

1. **P0-2 阶段**: 统一错误处理
   - 已经在 Knowledge 中使用了 RunResult 转换
   - 可作为其他组件的参考模式

2. **P1 阶段**: 其他重复代码消除
   - 建立了委托模式的最佳实践
   - 证明了薄包装的可行性

3. **文档改进**:
   - 建立了详细文档注释的标准
   - 为其他模块树立榜样

---

## 十、检查清单

### 开始前
- [ ] 创建功能分支
- [ ] 备份现有文件
- [ ] 运行基线测试
- [ ] 阅读完整设计文档

### 实施中
- [ ] 更新 wp-cli-core 文档
- [ ] 重写 wp-proj/knowledge.rs
- [ ] 添加重新导出
- [ ] 实现错误转换
- [ ] 保持 Component trait

### 完成前
- [ ] 运行所有测试
- [ ] 检查无编译警告
- [ ] 手动测试关键功能
- [ ] 更新 CHANGELOG
- [ ] 代码审查（自查或同行）

### 提交前
- [ ] 删除备份文件
- [ ] Git 提交消息清晰
- [ ] 推送到远程分支
- [ ] 创建 Pull Request（如需要）

---

**设计文档版本**: 1.0
**创建日期**: 2026-01-10
**作者**: Claude Code
**审查状态**: ⏳ 待审查

**审查意见**:
```
[请在此处添加审查意见]

批准 / 需要修改 / 拒绝

签名：_______________  日期：_______________
```

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
