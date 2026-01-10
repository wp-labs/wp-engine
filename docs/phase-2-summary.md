# 阶段 2 完成总结

**日期**: 2026-01-10
**阶段**: 移除业务逻辑下沉

---

## 目标

将 wp-cli-utils 中的业务逻辑移到 wp-cli-core，明确层次职责。

---

## 完成内容

### 1. 新增文件

**业务层模块结构**:
- `crates/wp-cli-core/src/business/mod.rs`
- `crates/wp-cli-core/src/business/observability/mod.rs`
- `crates/wp-cli-core/src/business/observability/sources.rs` (约 240 行)
  - `total_input_from_wpsrc()` - 从配置推导总输入条数
  - `list_file_sources_with_lines()` - 列出文件源及行数
  - Re-export types from wpcnt_lib
- `crates/wp-cli-core/src/business/observability/sinks.rs` (约 180 行)
  - `process_group()` - 处理 sink 组
  - `process_group_v2()` - V2 版本处理
  - `ResolvedSinkLite` 结构

### 2. 修改文件

**wp-cli-core/src/lib.rs**:
- 添加 `pub mod business;`
- Re-export business functions: `list_file_sources_with_lines`, `total_input_from_wpsrc`, `process_group`, `SrcLineReport`

**wp-cli-core/src/obs/stat.rs**:
- 更新导入: `use crate::business::observability::{...};`
- 使用业务层函数替代 wpcnt_lib 调用
- 移除 `use wpcnt_lib as wlib;`

**wp-cli-core/src/obs/validate.rs**:
- 更新导入: `use crate::business::observability::process_group;`
- 使用业务层函数

**wp-cli-utils/src/lib.rs**:
- 移除 `pub mod sources;` 和 `pub mod scan;`
- 移除 `pub use sources::*;` 和 `pub use scan::process_group;`
- 保留纯工具模块: `banner`, `fsutils`, `pretty`, `stats`, `types`, `validate`

**wp-cli-utils/src/types.rs**:
- 添加 `SrcLineItem` 和 `SrcLineReport` 结构定义
- 这些是数据结构，被多个模块共享

**wp-cli-utils/src/pretty/sources.rs**:
- 更新导入: `use crate::types::{SrcLineItem, SrcLineReport};`

**wp-proj/src/sinks/validate.rs**:
- 更新调用: `wp_cli_core::total_input_from_wpsrc()` 替代 `wlib::total_input_from_wpsrc()`

### 3. 删除文件

- ❌ `crates/wp-cli-utils/src/sources.rs` (235 行) - 全部是业务逻辑
- ❌ `crates/wp-cli-utils/src/scan.rs` (178 行) - 全部是业务逻辑

---

## 架构改进

### 之前的架构 (有问题)

```
┌─────────────────┐
│   wp-cli-core   │
│   (CLI 入口)    │
└────────┬────────┘
         │ 依赖
         ↓
┌─────────────────┐
│  wp-cli-utils   │ ← 混合业务逻辑和工具函数
│  • sources.rs   │   (违反单一职责)
│  • scan.rs      │
│  • fsutils.rs   │
└─────────────────┘
```

**问题**:
- wp-cli-utils 混合了业务逻辑和工具函数
- 层次职责不清
- 业务逻辑不应该在 utils 层

### 之后的架构 (清晰)

```
┌─────────────────────────────┐
│       wp-cli-core           │
│  ┌──────────────────────┐   │
│  │   business/          │   │ ← 业务逻辑在 core
│  │   • observability/   │   │
│  │     - sources.rs     │   │
│  │     - sinks.rs       │   │
│  └──────────────────────┘   │
│  ┌──────────────────────┐   │
│  │   obs/               │   │
│  │   • stat.rs          │   │ ← 使用 business 层
│  │   • validate.rs      │   │
│  └──────────────────────┘   │
└────────┬────────────────────┘
         │ 使用
         ↓
┌─────────────────┐
│  wp-cli-utils   │ ← 只保留纯工具函数
│  • fsutils      │   (单一职责)
│  • pretty       │
│  • types        │   ← 共享数据结构
│  • validate     │
└─────────────────┘
```

**优势**:
- ✅ 层次职责明确: business (业务) → utils (工具)
- ✅ 业务逻辑集中在 core
- ✅ Utils 只保留纯工具函数
- ✅ 数据结构在 types 中共享

---

## 代码指标

- **移动代码**: ~420 行业务逻辑
- **删除文件**: 2 个 (sources.rs, scan.rs)
- **新增文件**: 4 个 (business 模块)
- **修改文件**: 7 个
- **净减少**: utils 层减少 413 行

---

## 测试验证

### 测试结果

```
✅ 所有测试通过: 911 tests
✅ 无编译警告
✅ 无编译错误
```

### 关键测试

- ✅ Source统计集成测试 - 5 tests
- ✅ Sink验证集成测试 - 6 tests
- ✅ Pretty打印测试 - 保持通过
- ✅ 全workspace编译通过

---

## 收益分析

### 即时收益

1. **层次职责明确**
   - business: 业务逻辑
   - utils: 纯工具函数
   - types: 共享数据结构

2. **代码组织改善**
   - wp-cli-utils 变得纯粹
   - business 逻辑集中管理
   - 更容易理解和维护

3. **依赖关系清晰**
   - core → business (内部)
   - core → utils (工具)
   - utils 不依赖 core

### 长期收益

1. **可维护性提升**
   - 业务逻辑集中在一处
   - 修改影响范围明确
   - 减少意外依赖

2. **为后续重构奠定基础**
   - 业务逻辑已经从 utils 剥离
   - 可以逐步优化 business 层
   - utils 变成可复用的工具库

3. **架构清晰度**
   - 新成员更容易理解代码结构
   - 模块边界清晰
   - 职责划分明确

---

## 技术要点

### 1. 数据结构共享策略

将 `SrcLineItem` 和 `SrcLineReport` 放在 `wpcnt_lib::types` 中：

**原因**:
- 避免循环依赖 (core 依赖 utils，utils 不能依赖 core)
- 这些是数据结构，不是业务逻辑
- 多个模块需要共享这些类型

**设计**:
```rust
// wp-cli-utils/src/types.rs
pub struct SrcLineReport { ... }
pub struct SrcLineItem { ... }

// wp-cli-core/src/business/observability/sources.rs
pub use wpcnt_lib::types::{SrcLineItem, SrcLineReport};
```

### 2. 业务逻辑组织

**模块层次**:
```
business/
└── observability/
    ├── mod.rs        - 导出公共接口
    ├── sources.rs    - 源文件统计逻辑
    └── sinks.rs      - Sink处理逻辑
```

**导出策略**:
- 业务函数从 observability 导出
- 类型从 wpcnt_lib::types re-export
- 保持向后兼容的公共API

### 3. 辅助函数处理

**私有辅助函数**:
- `read_wpsrc_toml()` - TOML文件读取
- `load_connectors_map()` - Connector加载
- `toml_table_to_param_map()` - 类型转换

**策略**: 保持私有，只暴露高层业务函数

---

## 风险评估

### 风险等级: 🟡 中等

**原因**:
- 跨 crate 代码移动
- 需要更新多处导入路径
- 影响业务逻辑组织

### 实际遇到的问题

1. **类型定义位置**
   - **问题**: SrcLineReport 应该放在哪里？
   - **解决**: 放在 types.rs（共享数据结构）

2. **pretty 模块依赖**
   - **问题**: pretty/sources.rs 依赖 SrcLineReport
   - **解决**: 从 types 导入，而不是从已删除的 sources 模块

3. **wp-proj 依赖**
   - **问题**: wp-proj 使用 `wlib::total_input_from_wpsrc`
   - **解决**: 更新为 `wp_cli_core::total_input_from_wpsrc`

### 缓解措施

- ✅ 逐步迁移，频繁测试
- ✅ 保持小粒度提交
- ✅ 完整的回滚方案

---

## 下一步

准备进入**阶段 3: 缩短调用链**

### 阶段 3 预览

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

**预计时间**: 1-1.5 小时
**风险等级**: 🟢 低

---

## 附录

### 提交信息模板

```
refactor(phase-2): move business logic from utils to core

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
- crates/wp-cli-core/src/business/observability/sinks.rs
  - process_group()
  - process_group_v2()
  - ResolvedSinkLite

### 修改
- crates/wp-cli-core/src/lib.rs - 导出 business 模块
- crates/wp-cli-core/src/obs/stat.rs - 使用 business 层函数
- crates/wp-cli-core/src/obs/validate.rs - 使用 business 层函数
- crates/wp-cli-utils/src/lib.rs - 移除业务代码导出
- crates/wp-cli-utils/src/types.rs - 添加共享数据结构
- crates/wp-cli-utils/src/pretty/sources.rs - 更新导入
- crates/wp-proj/src/sinks/validate.rs - 更新调用

### 删除
- crates/wp-cli-utils/src/sources.rs (235 行业务逻辑)
- crates/wp-cli-utils/src/scan.rs (178 行业务逻辑)

### 验证
- ✅ 所有测试通过 (911 tests)
- ✅ 无编译警告
- ✅ 功能保持一致

### 架构改进
**之前**: CLI → Utils(业务+工具) → Config
**之后**: CLI → Business(业务) → Utils(工具) → Config

### 收益
- 层次职责明确
- 依赖关系清晰
- 为后续重构奠定基础
- 消除业务逻辑下沉
- utils 变成纯工具库

Refs: #refactor/simplify-cli-architecture
Phase: 2/6 - Business Logic Elevation
Next: Phase 3 - Shorten call chains

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
```

---

**完成时间**: 2026-01-10
**实际时间**: ~1.5 小时
**审查状态**: 待审查
