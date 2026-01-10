# P0-2 阶段详细设计：统一错误处理策略

**优先级**: P0（最高）
**预计工作量**: 3-4 天
**风险等级**: 🟡 中
**依赖**: 无

---

## 一、问题陈述

### 1.1 当前状况

**三个 crates 混用多种错误类型**:

```rust
// wp-cli-core - 混用 3 种
pub fn func1() -> anyhow::Result<T>        // business/observability/*
pub fn func2() -> OrionConfResult<T>       // business/connectors/*
pub fn func3() -> wp_error::RunResult<T>   // 部分使用

// wp-config - 混用 2 种
pub fn func4() -> OrionConfResult<T>       // 主要使用
pub fn func5() -> Result<T, Box<dyn Error>> // 部分使用

// wp-proj - 混用 2 种
pub fn func6() -> RunResult<T>             // 主要使用
pub fn func7() -> Result<T>                // 少量使用
```

**统计数据**:
```bash
# 使用 anyhow::Result 的函数
wp-cli-core: ~15 处
wp-proj:     ~3 处

# 使用 OrionConfResult 的函数
wp-cli-core: ~20 处
wp-config:   ~45 处

# 使用 RunResult 的函数
wp-proj:     ~85 处
wp-cli-core: ~8 处
```

### 1.2 影响分析

**调用复杂度**:
```rust
// 当前需要多次转换
let sources = load_sources(path)
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;  // OrionConfResult → anyhow

let output = process_data(&data)?;  // anyhow::Result

let _ = validate_config(&cfg)
    .map_err(|e| anyhow::anyhow!(e.to_string()))?;  // RunResult → anyhow
```

**错误上下文丢失**:
- 通过 `.to_string()` 转换丢失了原始错误的 backtrace
- 无法使用 `Error::source()` 追踪错误链
- 日志记录不完整

---

## 二、设计方案

### 2.1 目标架构

**统一到 `wp_error::RunResult<T>`**:

```
所有公共 API 使用
   ↓
wp_error::RunResult<T>
   ↓
wp_error::RunError (StructError<RunReason>)
   ↓
自动转换所有外部错误类型
```

**原因选择 RunResult**:
1. ✅ 已在 wp-proj 广泛使用（85 处）
2. ✅ 提供结构化错误（RunReason 枚举）
3. ✅ 支持错误分类（conf/io/parse 等）
4. ✅ 与 orion-error 生态系统集成

### 2.2 转换策略

#### 2.2.1 为常见错误类型实现 From trait

```rust
// wp-error/src/run_error.rs (或新建 conversions.rs)

use orion_error::OrionError;

/// 从 OrionError 自动转换
impl From<orion_error::StructError<orion_error::OrionReason>> for RunReason {
    fn from(e: orion_error::StructError<orion_error::OrionReason>) -> Self {
        RunReason::from_conf(e.to_string())
    }
}

/// 从 anyhow::Error 自动转换
impl From<anyhow::Error> for RunReason {
    fn from(e: anyhow::Error) -> Self {
        // 保留错误链
        let chain: Vec<String> = e.chain()
            .map(|e| e.to_string())
            .collect();

        RunReason::from_conf(format!("错误: {}", chain.join(" -> ")))
    }
}

/// 从标准 IO 错误转换
impl From<std::io::Error> for RunReason {
    fn from(e: std::io::Error) -> Self {
        RunReason::from_io(e.to_string())
    }
}

/// 从 TOML 解析错误转换
impl From<toml::de::Error> for RunReason {
    fn from(e: toml::de::Error) -> Self {
        RunReason::from_parse(format!("TOML 解析错误: {}", e))
    }
}

/// 从 JSON 序列化错误转换
impl From<serde_json::Error> for RunReason {
    fn from(e: serde_json::Error) -> Self {
        RunReason::from_conf(format!("JSON 错误: {}", e))
    }
}
```

#### 2.2.2 辅助宏简化转换

```rust
// wp-error/src/macros.rs (新建)

/// 快速创建 RunResult 错误
///
/// # 示例
/// ```
/// run_err!(conf, "配置文件不存在: {}", path);
/// run_err!(io, "无法读取文件");
/// run_err!(parse, "解析失败，行 {}", line_num);
/// ```
#[macro_export]
macro_rules! run_err {
    (conf, $($arg:tt)*) => {
        Err($crate::run_error::RunReason::from_conf(format!($($arg)*)).to_err())
    };
    (io, $($arg:tt)*) => {
        Err($crate::run_error::RunReason::from_io(format!($($arg)*)).to_err())
    };
    (parse, $($arg:tt)*) => {
        Err($crate::run_error::RunReason::from_parse(format!($($arg)*)).to_err())
    };
}

/// 为 Result 添加错误上下文
///
/// # 示例
/// ```
/// load_config(path).context("加载配置失败")?;
/// parse_data(content).context("解析第 {} 行失败", line)?;
/// ```
pub trait ResultExt<T> {
    fn context(self, msg: impl Into<String>) -> RunResult<T>;
    fn with_context<F>(self, f: F) -> RunResult<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: std::error::Error,
{
    fn context(self, msg: impl Into<String>) -> RunResult<T> {
        self.map_err(|e| {
            RunReason::from_conf(format!("{}: {}", msg.into(), e)).to_err()
        })
    }

    fn with_context<F>(self, f: F) -> RunResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            RunReason::from_conf(format!("{}: {}", f(), e)).to_err()
        })
    }
}
```

---

## 三、实施步骤

### 3.1 准备阶段（半天）

**步骤 1**: 创建功能分支
```bash
git checkout -b refactor/p0-2-unified-errors
```

**步骤 2**: 审计当前错误使用
```bash
# 统计各类错误使用情况
grep -r "anyhow::Result\|OrionConfResult\|RunResult" \
  crates/wp-cli-core/src \
  crates/wp-config/src \
  crates/wp-proj/src \
  --include="*.rs" | wc -l
```

**步骤 3**: 确定基线测试
```bash
cargo test --workspace
```

### 3.2 实施阶段（3 天）

#### 阶段 1：增强 wp-error（1 天）

**步骤 1.1**: 实现错误转换 trait
```rust
// 在 wp-error crate 中添加
// src/conversions.rs (新建)

[上面 2.2.1 节的代码]
```

**步骤 1.2**: 添加辅助宏
```rust
// src/macros.rs (新建)

[上面 2.2.2 节的代码]
```

**步骤 1.3**: 更新 lib.rs
```rust
// wp-error/src/lib.rs

pub mod run_error;
pub mod conversions;  // 新增
pub mod macros;       // 新增

// 重新导出常用项
pub use run_error::{RunError, RunReason, RunResult};
pub use macros::ResultExt;
```

**步骤 1.4**: 测试转换
```rust
// wp-error/src/conversions.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_anyhow_preserves_chain() {
        let err = anyhow::anyhow!("inner")
            .context("middle")
            .context("outer");

        let run_reason = RunReason::from(err);
        let msg = run_reason.to_string();

        assert!(msg.contains("inner"));
        assert!(msg.contains("middle"));
        assert!(msg.contains("outer"));
    }

    #[test]
    fn from_io_error() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found"
        );

        let run_reason = RunReason::from(io_err);
        assert!(run_reason.to_string().contains("file not found"));
    }
}
```

#### 阶段 2：迁移 wp-cli-core（1 天）

**优先级顺序**:
1. 公共 API 函数（对外接口）
2. 内部模块接口
3. 私有函数（可选）

**步骤 2.1**: 迁移 business/observability
```rust
// business/observability/sinks.rs

// 修改前
pub fn process_group(...) -> anyhow::Result<GroupResult> {
    let data = load_data()?;  // anyhow propagation
    Ok(GroupResult { ... })
}

// 修改后
use wp_error::{RunResult, ResultExt};

pub fn process_group(...) -> RunResult<GroupResult> {
    let data = load_data()
        .context("加载组数据失败")?;  // 自动转换 + 上下文
    Ok(GroupResult { ... })
}
```

**步骤 2.2**: 迁移 business/connectors
```rust
// business/connectors/sinks.rs

// 修改前
pub fn validate_routes(...) -> OrionConfResult<()> {
    // ...
}

// 修改后
use wp_error::RunResult;

pub fn validate_routes(...) -> RunResult<()> {
    // OrionConfResult 错误会自动转换为 RunResult
    // ...
}
```

**步骤 2.3**: 更新 Cargo.toml
```toml
# crates/wp-cli-core/Cargo.toml

[dependencies]
wp-error = { path = "../wp-error" }
# anyhow 可以保留用于内部，但公共 API 不使用
anyhow = "1.0"  # 仅内部使用
```

**步骤 2.4**: 逐个模块编译检查
```bash
cargo check -p wp-cli-core --lib
cargo clippy -p wp-cli-core -- -W clippy::all
```

#### 阶段 3：迁移 wp-config（半天）

**步骤 3.1**: 更新 loader 模块
```rust
// src/loader/mod.rs

// 修改前
pub trait ConfStdOperation {
    fn load(path: &Path) -> OrionConfResult<Self>;
}

// 修改后
use wp_error::RunResult;

pub trait ConfStdOperation {
    fn load(path: &Path) -> RunResult<Self>;
}
```

**步骤 3.2**: 更新 structure 模块验证
```rust
// src/structure/sink/instance.rs

// 修改前
impl Validate for SinkInstanceConf {
    fn validate(&self) -> OrionConfResult<()> {
        // ...
    }
}

// 修改后
use wp_error::RunResult;

impl Validate for SinkInstanceConf {
    fn validate(&self) -> RunResult<()> {
        // OrionError 自动转换
        // ...
    }
}
```

#### 阶段 4：迁移 wp-proj（半天）

**步骤 4.1**: 检查现有使用
```bash
# wp-proj 已经大量使用 RunResult，检查是否有遗漏
grep -r "Result<" crates/wp-proj/src --include="*.rs" \
  | grep -v "RunResult" \
  | grep "pub fn"
```

**步骤 4.2**: 迁移剩余部分
```rust
// 将所有 anyhow::Result 改为 RunResult
// 通常只需要更改类型签名，实现自动转换
```

### 3.3 测试和验证（半天）

**步骤 3.1**: 运行完整测试套件
```bash
cargo test --workspace
```

**步骤 3.2**: 检查警告
```bash
cargo clippy --workspace -- -W clippy::all
```

**步骤 3.3**: 手动验证错误消息
```bash
# 故意触发错误，检查消息质量
wproj prj check /nonexistent/path
```

---

## 四、文件变更清单

### 4.1 新建文件

| 文件 | 说明 |
|------|------|
| `wp-error/src/conversions.rs` | 错误类型转换实现 |
| `wp-error/src/macros.rs` | 辅助宏定义 |

### 4.2 修改文件（示例）

| 文件 | 变更类型 | 预计改动 |
|------|---------|----------|
| `wp-error/src/lib.rs` | 导出新模块 | +5 行 |
| `wp-cli-core/src/business/observability/*.rs` | 返回类型 | ~15 处 |
| `wp-cli-core/src/business/connectors/*.rs` | 返回类型 | ~20 处 |
| `wp-config/src/loader/*.rs` | Trait 定义 | ~3 处 |
| `wp-config/src/structure/**/*.rs` | 返回类型 | ~10 处 |
| `wp-proj/src/**/*.rs` | 清理遗留 | ~5 处 |

---

## 五、测试策略

### 5.1 单元测试

**wp-error 新增测试**:
```rust
// wp-error/src/conversions.rs

#[cfg(test)]
mod tests {
    #[test]
    fn from_各种错误类型() { ... }

    #[test]
    fn 错误链保留() { ... }

    #[test]
    fn context_宏工作() { ... }
}
```

**现有测试保持通过**:
- wp-cli-core: 8 个测试模块
- wp-config: 测试模块
- wp-proj: 63 个单元测试 + 9 个文档测试

### 5.2 集成测试

**错误传播验证**:
```rust
#[test]
fn error_propagation_across_crates() {
    // wp-proj 调用 wp-cli-core
    // wp-cli-core 调用 wp-config
    // 确保错误正确传播且保留上下文
}
```

### 5.3 错误消息质量测试

```rust
#[test]
fn error_messages_are_helpful() {
    let result = some_failing_operation();
    let err = result.unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("上下文信息"));
    assert!(msg.contains("原始错误"));
}
```

---

## 六、风险评估

### 6.1 高风险点

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **API 破坏性变更** | 高 | 1. From trait 自动转换<br>2. 分阶段迁移<br>3. 保留兼容层（如需要） |
| **错误信息质量下降** | 中 | 1. 保留错误链<br>2. 添加上下文<br>3. 手动测试错误消息 |
| **性能开销** | 低 | 1. From trait 是零成本抽象<br>2. 只在错误路径执行 |

### 6.2 回滚策略

**阶段性提交**:
```bash
# 每完成一个 crate 的迁移就提交
git add wp-error
git commit -m "feat(wp-error): add error conversions"

git add wp-cli-core
git commit -m "refactor(wp-cli-core): unify to RunResult"

# 如果出问题，可以逐个回滚
git revert HEAD~1  # 只回滚 wp-cli-core
```

---

## 七、迁移指南

### 7.1 快速参考

**从 anyhow::Result 迁移**:
```rust
// 修改前
pub fn func() -> anyhow::Result<T> {
    let data = load()?;
    Ok(data)
}

// 修改后
use wp_error::{RunResult, ResultExt};

pub fn func() -> RunResult<T> {
    let data = load()
        .context("加载失败")?;  // 自动转换 + 上下文
    Ok(data)
}
```

**从 OrionConfResult 迁移**:
```rust
// 修改前
pub fn func() -> OrionConfResult<T> {
    validate_config(cfg)?;  // OrionError
    Ok(result)
}

// 修改后
use wp_error::RunResult;

pub fn func() -> RunResult<T> {
    validate_config(cfg)?;  // 自动转换
    Ok(result)
}
```

**手动创建错误**:
```rust
// 使用宏
use wp_error::run_err;

if invalid {
    return run_err!(conf, "配置无效: {}", reason);
}

// 或使用传统方式
return Err(RunReason::from_conf("错误").to_err());
```

### 7.2 最佳实践

1. **为外部错误添加上下文**:
```rust
fs::read_to_string(path)
    .context("读取配置文件失败")?;
```

2. **使用具体的错误类型**:
```rust
// 好
RunReason::from_conf("配置错误")
RunReason::from_io("IO 错误")
RunReason::from_parse("解析错误")

// 避免
RunReason::from_conf("发生错误")  // 太泛泛
```

3. **保留错误链**:
```rust
// From trait 自动保留
let result: RunResult<_> = anyhow_func()
    .context("操作失败")?;  // 保留完整错误链
```

---

## 八、验收标准

### 8.1 功能标准

- ✅ 所有公共 API 使用 `RunResult<T>`
- ✅ 外部错误自动转换（无需手动 `.map_err()`）
- ✅ 错误消息包含完整上下文
- ✅ 错误链完整保留

### 8.2 质量标准

- ✅ 所有测试通过（100%）
- ✅ 无编译警告
- ✅ 错误消息人类可读
- ✅ 文档覆盖新增 API

### 8.3 代码指标

- ✅ `.map_err()` 使用减少 >80%
- ✅ 错误类型一致性 100%
- ✅ 新增代码 <200 行（conversions + macros）

---

## 九、后续改进

完成统一后，可以进一步优化：

1. **结构化错误**:
```rust
pub enum AppError {
    Config(ConfigError),
    Io(IoError),
    Parse(ParseError),
}
```

2. **错误码系统**:
```rust
pub struct RunError {
    code: &'static str,  // "E001"
    message: String,
    source: Option<Box<dyn Error>>,
}
```

3. **本地化支持**:
```rust
pub trait LocalizedError {
    fn message(&self, locale: &str) -> String;
}
```

---

## 十、检查清单

### 实施前
- [ ] 审查设计文档
- [ ] 统计当前错误类型使用
- [ ] 确定迁移优先级
- [ ] 创建功能分支

### 实施中
- [ ] 实现错误转换 trait
- [ ] 添加辅助宏
- [ ] 迁移 wp-cli-core
- [ ] 迁移 wp-config
- [ ] 清理 wp-proj

### 验证
- [ ] 所有测试通过
- [ ] 无编译警告
- [ ] 手动测试错误消息质量
- [ ] 代码审查

### 完成
- [ ] 更新文档
- [ ] 提交清晰的 commit
- [ ] 创建 PR（如需要）

---

**设计文档版本**: 1.0
**创建日期**: 2026-01-10
**预计工作量**: 3-4 天
**风险等级**: 🟡 中

**审查意见**:
```
[请在此处添加审查意见]

批准 / 需要修改 / 拒绝

签名：_______________  日期：_______________
```

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
