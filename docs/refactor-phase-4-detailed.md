# 阶段 4: 创建新目录结构 - 详细执行方案

> **目标**: 在 wp-cli-core 中建立清晰的模块结构，为阶段 5 代码迁移做准备

**预计时间**: 30-45 分钟
**风险等级**: 🟢 低
**可回滚**: ✅ 是
**前置条件**: ✅ 阶段 3 已完成

---

## 背景分析

### 当前目录结构

#### wp-cli-core 结构 (阶段 3 完成后)

```
wp-cli-core/src/
├── business/
│   └── observability/
│       ├── mod.rs
│       ├── sources.rs
│       └── sinks.rs
├── connectors/
│   ├── mod.rs
│   ├── sources.rs
│   └── sinks.rs
├── data/
│   ├── mod.rs
│   └── clean.rs
├── knowdb/
│   └── mod.rs
├── obs/
│   ├── mod.rs
│   └── validate.rs
└── lib.rs
```

#### wp-cli-utils 结构 (待迁移)

```
wp-cli-utils/src/
├── banner.rs
├── fsutils.rs
├── pretty/
│   ├── mod.rs
│   ├── helpers.rs
│   ├── sinks.rs
│   ├── sources.rs
│   └── validate.rs
├── stats.rs
├── types.rs
├── validate.rs
└── lib.rs
```

### 问题分析

**当前问题**:
1. 🔴 **模块分类不清**: connectors 既有业务逻辑又在顶层
2. 🔴 **obs 模块定位模糊**: 只剩 validate，应该归入 business
3. 🔴 **缺少 utils 层**: 没有为工具函数准备目录结构
4. 🔴 **utils crate 待迁移**: wp-cli-utils 的内容需要迁移目标

### 目标架构

```
wp-cli-core/src/
├── business/              # 业务逻辑层
│   ├── mod.rs
│   ├── connectors/        # 连接器业务逻辑（新增）
│   │   ├── mod.rs
│   │   ├── sinks.rs      （从顶层移入）
│   │   └── sources.rs    （从顶层移入）
│   └── observability/     # 可观测性业务逻辑（已有）
│       ├── mod.rs
│       ├── sources.rs
│       ├── sinks.rs
│       └── validate.rs   （从 obs 移入）
│
├── utils/                 # 工具函数层（新增）
│   ├── mod.rs
│   ├── banner.rs         （预留，阶段 5 迁移）
│   ├── fs/               （预留，阶段 5 迁移）
│   │   └── mod.rs
│   ├── pretty/           （预留，阶段 5 迁移）
│   │   └── mod.rs
│   ├── stats/            （预留，阶段 5 迁移）
│   │   └── mod.rs
│   ├── types.rs          （预留，阶段 5 迁移）
│   └── validate/         （预留，阶段 5 迁移）
│       └── mod.rs
│
├── data/                  # 数据操作（保持）
│   ├── mod.rs
│   └── clean.rs
│
├── knowdb/                # 知识库（保持）
│   └── mod.rs
│
└── lib.rs                 # 主入口
```

**改进**:
- ✅ **层次清晰**: business 和 utils 分层明确
- ✅ **职责明确**: connectors 归入 business 层
- ✅ **结构完整**: 为 utils 迁移准备好目录
- ✅ **易于扩展**: 每个子模块有独立目录

---

## 任务分解

### 任务 4.1: 规划目录结构 ⏱️ 5 分钟

#### Step 1: 确认模块分类

**Business 层模块**:
- `business/connectors/` - 连接器列表、路由表业务逻辑
- `business/observability/` - 统计、验证业务逻辑

**Utils 层模块**:
- `utils/banner.rs` - 横幅打印工具
- `utils/fs/` - 文件系统工具 (fsutils)
- `utils/pretty/` - 美化输出工具
- `utils/stats/` - 统计计算工具
- `utils/types.rs` - 共享类型定义
- `utils/validate/` - 验证工具函数

**保持不变的模块**:
- `data/` - 数据清理功能
- `knowdb/` - 知识库功能

#### Step 2: 绘制依赖关系图

```
┌──────────────────────────────────────┐
│            lib.rs                     │
│  (对外暴露统一API)                    │
└─────┬──────────────┬─────────────────┘
      │              │
      ↓              ↓
┌───────────┐  ┌──────────────┐
│ business/ │  │   utils/     │
│           │  │              │
│ ├─ conn   │  │ ├─ banner    │
│ └─ obs    │  │ ├─ fs        │
│           │  │ ├─ pretty    │
│           │  │ ├─ stats     │
│           │  │ ├─ types     │
│           │  │ └─ validate  │
└─────┬─────┘  └──────┬───────┘
      │                │
      │                │
      ↓                ↓
┌────────────────────────┐
│   wp-config            │
│   wpcnt_lib            │
│   (底层 crates)        │
└────────────────────────┘
```

**依赖规则**:
- business 可以依赖 utils
- utils 不能依赖 business
- lib.rs 从 business 和 utils re-export

---

### 任务 4.2: 创建 utils 目录结构 ⏱️ 10 分钟

#### Step 1: 创建 utils 主模块

```bash
mkdir -p crates/wp-cli-core/src/utils
```

#### Step 2: 创建 utils/mod.rs

```rust
//! Utility functions and helper modules
//!
//! This module provides various utility functions that are used across
//! the CLI application but don't contain business logic.

// 阶段 5 将迁移这些模块
// pub mod banner;
// pub mod fs;
// pub mod pretty;
// pub mod stats;
// pub mod types;
// pub mod validate;
```

**说明**:
- 现在只创建占位符
- 阶段 5 会实际迁移代码并启用这些模块

#### Step 3: 创建子目录和占位符

```bash
# 创建子目录
mkdir -p crates/wp-cli-core/src/utils/fs
mkdir -p crates/wp-cli-core/src/utils/pretty
mkdir -p crates/wp-cli-core/src/utils/stats
mkdir -p crates/wp-cli-core/src/utils/validate

# 创建占位符 mod.rs
touch crates/wp-cli-core/src/utils/fs/mod.rs
touch crates/wp-cli-core/src/utils/pretty/mod.rs
touch crates/wp-cli-core/src/utils/stats/mod.rs
touch crates/wp-cli-core/src/utils/validate/mod.rs
```

#### Step 4: 创建占位符内容

**utils/fs/mod.rs**:
```rust
//! File system utility functions
//!
//! This module will contain file operations, path resolution, and
//! line counting utilities.
//!
//! (Content will be migrated in Phase 5)
```

**utils/pretty/mod.rs**:
```rust
//! Pretty-printing and formatting utilities
//!
//! This module provides functions for formatting and displaying
//! various data structures in a human-readable format.
//!
//! (Content will be migrated in Phase 5)
```

**utils/stats/mod.rs**:
```rust
//! Statistical calculation utilities
//!
//! This module provides functions for computing statistics, percentages,
//! and other mathematical operations.
//!
//! (Content will be migrated in Phase 5)
```

**utils/validate/mod.rs**:
```rust
//! Validation utility functions
//!
//! This module provides functions for validating data integrity,
//! checking thresholds, and other validation operations.
//!
//! (Content will be migrated in Phase 5)
```

---

### 任务 4.3: 创建 business/connectors 目录 ⏱️ 10 分钟

#### Step 1: 创建目录

```bash
mkdir -p crates/wp-cli-core/src/business/connectors
```

#### Step 2: 移动 connectors 模块到 business 层

这个任务包含：
1. 移动 `src/connectors/sources.rs` → `src/business/connectors/sources.rs`
2. 移动 `src/connectors/sinks.rs` → `src/business/connectors/sinks.rs`
3. 创建 `src/business/connectors/mod.rs`
4. 更新 `src/business/mod.rs`
5. 删除 `src/connectors/` 目录

**具体操作**:

```bash
# 移动文件
mv crates/wp-cli-core/src/connectors/sources.rs crates/wp-cli-core/src/business/connectors/
mv crates/wp-cli-core/src/connectors/sinks.rs crates/wp-cli-core/src/business/connectors/
mv crates/wp-cli-core/src/connectors/mod.rs crates/wp-cli-core/src/business/connectors/

# 删除旧目录
rmdir crates/wp-cli-core/src/connectors
```

#### Step 3: 更新 business/mod.rs

```rust
//! Business logic layer for CLI operations

pub mod connectors;     // 新增
pub mod observability;
```

#### Step 4: 更新 lib.rs 的导入

```rust
// 之前
pub mod connectors;

// 之后（移除，因为已经在 business 下）
// connectors 现在通过 business 访问
```

如果有 re-export，需要更新：

```rust
// 之前
pub use connectors::sources::{list_connectors, route_table};

// 之后
pub use business::connectors::sources::{list_connectors, route_table};
// 或者在 business/mod.rs 中 re-export
```

---

### 任务 4.4: 移动 obs/validate 到 business/observability ⏱️ 10 分钟

#### Step 1: 移动文件

```bash
mv crates/wp-cli-core/src/obs/validate.rs crates/wp-cli-core/src/business/observability/
```

#### Step 2: 更新 business/observability/mod.rs

```rust
//! Observability business logic for sources and sinks

mod sources;
mod sinks;
mod validate;  // 新增

pub use sources::{
    SrcLineItem,
    SrcLineReport,
    list_file_sources_with_lines,
    total_input_from_wpsrc,
};
pub use sinks::{
    ResolvedSinkLite,
    collect_sink_statistics,
    process_group,
    process_group_v2,
};
pub use validate::{  // 新增
    build_groups_v2,
    // ... 其他 validate 导出
};
```

#### Step 3: 更新 validate.rs 中的导入

如果 validate.rs 中有相对路径导入，需要更新：

```rust
// 检查是否有类似这样的导入
use crate::business::observability::{process_group, ...};
```

#### Step 4: 删除 obs 目录

```bash
# 删除 obs/mod.rs
rm crates/wp-cli-core/src/obs/mod.rs

# 删除 obs 目录
rmdir crates/wp-cli-core/src/obs
```

#### Step 5: 更新 lib.rs

```rust
// 之前
pub mod obs;

// 之后（移除）
// obs 模块已经合并到 business/observability
```

#### Step 6: 更新所有引用 obs::validate 的代码

查找并更新：

```bash
rg "obs::validate" crates/wp-cli-core crates/wp-proj --type rust
```

更新为：

```rust
// 之前
use wp_cli_core::obs::validate;

// 之后
use wp_cli_core::business::observability;
// 或者
use wp_cli_core::business::observability::build_groups_v2;
```

---

### 任务 4.5: 更新 lib.rs 统一导出 ⏱️ 5 分钟

#### 更新后的 lib.rs 结构

```rust
// Module declarations
pub mod business;
pub mod data;
pub mod knowdb;
pub mod utils;  // 新增（虽然现在是空的）

// Re-export business layer functions for convenience
pub use business::observability::{
    build_groups_v2,
    collect_sink_statistics,
    list_file_sources_with_lines,
    process_group,
    total_input_from_wpsrc,
    SrcLineReport,
};

pub use business::connectors::sources::{
    list_connectors,
    route_table,
};

pub use business::connectors::sinks::{
    // ... sink 相关导出
};

// Utils layer (will be populated in Phase 5)
// pub use utils::...
```

**设计原则**:
- lib.rs 提供扁平化的 API
- 用户可以直接 `use wp_cli_core::function_name`
- 也可以通过完整路径 `use wp_cli_core::business::observability::function_name`

---

### 任务 4.6: 编译验证 ⏱️ 5 分钟

```bash
# 编译检查
cargo build --workspace

# 运行测试
cargo test --workspace 2>&1 | tee /tmp/phase4-test-results.txt

# 验证
grep "test result:" /tmp/phase4-test-results.txt
```

**预期结果**:
- ✅ 所有文件编译通过
- ✅ 所有测试通过 (908+ tests)
- ✅ 无编译警告

---

### 任务 4.7: 更新文档 ⏱️ 5 分钟

#### 创建 migration-guide-phase-4.md

```markdown
# 阶段 4 迁移指南

## 模块路径变化

### connectors 模块

**之前**:
```rust
use wp_cli_core::connectors::sources::list_connectors;
```

**之后**:
```rust
use wp_cli_core::business::connectors::sources::list_connectors;
// 或直接使用 re-export
use wp_cli_core::list_connectors;
```

### obs::validate 模块

**之前**:
```rust
use wp_cli_core::obs::validate::build_groups_v2;
```

**之后**:
```rust
use wp_cli_core::business::observability::build_groups_v2;
// 或直接使用 re-export
use wp_cli_core::build_groups_v2;
```

## 新增目录结构

```
wp-cli-core/src/
├── business/
│   ├── connectors/      ← 从顶层移入
│   └── observability/   ← validate 合并进来
└── utils/               ← 新增（为阶段 5 准备）
```
```

---

## 完成验证清单

- [ ] ✅ utils/ 目录结构已创建
- [ ] ✅ connectors 已移到 business/connectors/
- [ ] ✅ validate 已移到 business/observability/
- [ ] ✅ obs/ 目录已删除
- [ ] ✅ lib.rs 导出已更新
- [ ] ✅ 所有测试通过
- [ ] ✅ 无编译警告
- [ ] ✅ 文档已更新
- [ ] ✅ 更改已提交

---

## 时间估算

| 任务 | 预计时间 | 实际时间 |
|------|---------|---------|
| 4.1 规划目录结构 | 5 分钟 | ___ |
| 4.2 创建 utils 目录 | 10 分钟 | ___ |
| 4.3 移动 connectors | 10 分钟 | ___ |
| 4.4 移动 validate | 10 分钟 | ___ |
| 4.5 更新 lib.rs | 5 分钟 | ___ |
| 4.6 编译验证 | 5 分钟 | ___ |
| 4.7 更新文档 | 5 分钟 | ___ |
| **总计** | **30-45 分钟** | ___ |

---

## 风险评估

### 风险等级: 🟢 低

**原因**:
- 只是移动文件和创建空目录
- 不改变任何功能逻辑
- 影响范围明确且可控

### 潜在问题

1. **导入路径错误**
   - **症状**: 编译错误，找不到模块
   - **解决**: 全局搜索并更新导入路径

2. **Re-export 遗漏**
   - **症状**: 外部 crate 编译失败
   - **解决**: 在 lib.rs 添加缺失的 re-export

3. **相对路径问题**
   - **症状**: 模块内部引用失败
   - **解决**: 更新为正确的相对路径

---

## 回滚方案

```bash
# 查看提交历史
git log --oneline

# 回滚到阶段 3
git reset --hard <phase-3-commit-hash>

# 或创建回滚提交
git revert HEAD
```

---

## 批准检查点

**请确认以下内容后批准执行**:

- [ ] 我理解阶段 4 的目标是创建目录结构
- [ ] 我同意将 connectors 移到 business 层
- [ ] 我同意将 validate 合并到 observability
- [ ] 我同意创建 utils 层目录（阶段 5 填充内容）
- [ ] 本阶段的所有操作都是安全的，可以随时回滚

**批准方式**:
- ✅ 批准执行: 请回复 "批准阶段 4" 或 "同意"
- ❌ 需要调整: 请说明需要修改的部分
- ⏸️ 暂缓执行: 请说明原因

---

**准备好开始了吗？** 🚀
