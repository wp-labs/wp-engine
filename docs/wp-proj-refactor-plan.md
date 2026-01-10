# wp-proj 重构计划

**日期**: 2026-01-10
**目标**: 全面改进 wp-proj crate 的代码质量和架构
**策略**: 5 阶段渐进式重构

---

## 总体目标

1. ✅ 消除代码重复 (-75%)
2. ✅ 建立统一抽象
3. ✅ 标准化错误处理
4. ✅ 清理模块结构
5. ✅ 降低耦合度
6. ✅ 完善文档

---

## 阶段 1: 提取共同模式 (1-2 小时)

### 目标
消除路径解析和初始化逻辑的重复

### 任务清单

- [ ] 1.1 创建 `utils/path_resolver.rs`
  - [ ] 定义 `PathResolvable` trait
  - [ ] 实现通用路径解析逻辑

- [ ] 1.2 创建 `utils/template_init.rs`
  - [ ] 定义 `TemplateInitializer` struct
  - [ ] 实现统一的模板初始化流程

- [ ] 1.3 重构 `models/wpl.rs`
  - [ ] 实现 `PathResolvable`
  - [ ] 使用 `TemplateInitializer`
  - [ ] 移除重复逻辑

- [ ] 1.4 重构 `models/oml.rs`
  - [ ] 实现 `PathResolvable`
  - [ ] 使用 `TemplateInitializer`
  - [ ] 移除重复逻辑

- [ ] 1.5 重构 `sources/core.rs`
  - [ ] 实现 `PathResolvable`
  - [ ] 移除重复的路径逻辑

- [ ] 1.6 重构 `sinks/sink.rs`
  - [ ] 实现 `PathResolvable`
  - [ ] 移除重复的路径逻辑

- [ ] 1.7 运行测试验证
- [ ] 1.8 提交阶段 1 更改

### 验收标准
- ✅ 路径解析逻辑只存在 1 处
- ✅ 初始化逻辑统一
- ✅ 所有测试通过
- ✅ 代码减少 ~200 行

---

## 阶段 2: 创建组件抽象 (2-3 小时)

### 目标
为所有组件建立统一的接口

### 任务清单

- [ ] 2.1 定义核心 trait
  - [ ] 创建 `traits/component.rs`
  - [ ] 定义 `Component` trait
  - [ ] 定义 `HasExamples` trait
  - [ ] 定义 `HasStatistics` trait
  - [ ] 定义 `Checkable` trait

- [ ] 2.2 实现 trait - models
  - [ ] `Wpl` 实现所有 trait
  - [ ] `Oml` 实现所有 trait
  - [ ] `Knowledge` 实现 `Component` + `Checkable`

- [ ] 2.3 实现 trait - I/O
  - [ ] `Sources` 实现 `Component` + `HasStatistics`
  - [ ] `Sinks` 实现 `Component` + `HasStatistics`

- [ ] 2.4 实现 trait - connectors
  - [ ] `Connectors` 实现 `Component` + `Checkable`

- [ ] 2.5 重构 `WarpProject`
  - [ ] 使用 trait 统一处理组件
  - [ ] 简化初始化逻辑
  - [ ] 简化检查逻辑

- [ ] 2.6 运行测试验证
- [ ] 2.7 提交阶段 2 更改

### 验收标准
- ✅ 所有组件实现 `Component`
- ✅ WarpProject 代码简化
- ✅ 更容易扩展新组件
- ✅ 所有测试通过

---

## 阶段 3: 统一错误处理 (1 小时)

### 目标
标准化错误类型和转换方式

### 任务清单

- [ ] 3.1 审查现有错误处理
  - [ ] 识别所有错误类型使用
  - [ ] 识别所有转换方式

- [ ] 3.2 统一 `sources/core.rs`
  - [ ] 修改 `check_sources_config` 返回 `RunResult<bool>`
  - [ ] 统一使用 `.err_conv()`

- [ ] 3.3 统一 `sinks/validate.rs`
  - [ ] 标准化错误转换
  - [ ] 移除混用的转换方式

- [ ] 3.4 统一 `models/wpl.rs`
  - [ ] 标准化错误转换

- [ ] 3.5 统一 `models/oml.rs`
  - [ ] 标准化错误转换

- [ ] 3.6 更新 `utils/error_handler.rs`
  - [ ] 添加标准转换辅助函数
  - [ ] 添加文档说明

- [ ] 3.7 运行测试验证
- [ ] 3.8 提交阶段 3 更改

### 验收标准
- ✅ 统一使用 `RunResult<T>`
- ✅ 统一使用 `.err_conv()` 转换
- ✅ 错误处理一致性 > 95%
- ✅ 所有测试通过

---

## 阶段 4: 清理模块结构 (1 小时)

### 目标
解决模块混乱和代码重复问题

### 任务清单

- [ ] 4.1 合并 check 模块
  - [ ] 分析 `project/check/` 和 `project/checker/`
  - [ ] 决定合并策略
  - [ ] 移动文件到统一位置
  - [ ] 更新所有引用

- [ ] 4.2 创建 Manager trait
  - [ ] 定义 `ComponentManager` trait
  - [ ] `WParseManager` 实现 trait
  - [ ] `WpGenManager` 实现 trait
  - [ ] 提取共同逻辑

- [ ] 4.3 移动测试工具
  - [ ] 将 `project/tests.rs` 移到 `#[cfg(test)]`
  - [ ] 或移到 dev-dependencies

- [ ] 4.4 运行测试验证
- [ ] 4.5 提交阶段 4 更改

### 验收标准
- ✅ check 模块职责清晰
- ✅ Manager 有统一接口
- ✅ 测试代码分离
- ✅ 所有测试通过

---

## 阶段 5: 解耦和文档 (2 小时)

### 目标
降低耦合度，完善文档

### 任务清单

- [ ] 5.1 降低 wp-cli-core 耦合
  - [ ] 创建抽象层接口
  - [ ] 通过 trait 而非直接调用

- [ ] 5.2 减少 wp-engine facade 依赖
  - [ ] 审查 facade 使用
  - [ ] 提取必要接口

- [ ] 5.3 添加模块文档
  - [ ] `connectors/mod.rs` 添加 `//!` 注释
  - [ ] `sinks/mod.rs` 添加说明
  - [ ] `sources/mod.rs` 添加说明
  - [ ] `models/mod.rs` 添加说明

- [ ] 5.4 添加使用示例
  - [ ] `WarpProject` 添加 rustdoc 示例
  - [ ] `Component` trait 添加示例
  - [ ] 常用操作添加示例

- [ ] 5.5 生成文档验证
  - [ ] `cargo doc --no-deps`
  - [ ] 检查文档质量

- [ ] 5.6 运行测试验证
- [ ] 5.7 提交阶段 5 更改

### 验收标准
- ✅ 耦合度降低
- ✅ 文档覆盖 > 80%
- ✅ 所有公共 API 有文档
- ✅ 所有测试通过

---

## 最终验证

- [ ] 运行全量测试 `cargo test --package wp-proj`
- [ ] 运行 clippy `cargo clippy --package wp-proj`
- [ ] 生成文档 `cargo doc --package wp-proj --no-deps`
- [ ] 代码审查
- [ ] 性能对比（如需要）

---

## 预期成果

### 代码质量提升

| 指标 | 重构前 | 重构后 | 改进 |
|------|--------|--------|------|
| 代码重复 | 高 (4+ 处) | 低 (1 处) | -75% |
| 抽象层次 | 无 | Component trait | +100% |
| 错误一致性 | 60% | 95% | +35% |
| 模块清晰度 | 中 | 高 | +30% |
| 文档覆盖 | 40% | 80% | +40% |
| 可维护性 | 中 | 高 | +50% |

### 代码规模影响

- **删除重复代码**: ~300-400 行
- **新增抽象层**: ~200 行
- **净减少**: ~100-200 行
- **复杂度**: 显著降低

---

## 回滚策略

每个阶段独立提交，可单独回滚：

```bash
# 查看提交历史
git log --oneline --grep="refactor(wp-proj"

# 回滚到特定阶段
git reset --hard <stage-commit-hash>
```

---

## 注意事项

1. **保持测试通过**: 每阶段必须 100% 测试通过
2. **API 兼容**: 保持向后兼容
3. **增量提交**: 每阶段独立提交
4. **文档同步**: 及时更新文档

---

**创建日期**: 2026-01-10
**预计总时间**: 7-9 小时
**风险等级**: 🟡 中等

