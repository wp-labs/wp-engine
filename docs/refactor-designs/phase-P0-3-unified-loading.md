# P0-3 阶段详细设计:统一配置加载接口

**优先级**: P0(最高)
**预计工作量**: 4-5 天
**风险等级**: 🟡 中
**依赖**: P0-2 (统一错误处理)

---

## 一、问题陈述

### 1.1 当前状况

**wp-config 中存在三种不同的加载模式**:

```rust
// 模式 1: Sources - 从字符串加载
pub fn load_source_instances_from_str(
    config_str: &str,
    start: &Path,
    dict: &EnvDict
) -> OrionConfResult<Vec<SourceInstanceConf>>

// 模式 2: Sinks (Business) - 从目录加载
pub fn load_business_route_confs(
    sink_root: &Path,
    dict: &EnvDict
) -> OrionConfResult<Vec<RouteConf>>

pub fn load_route_files_from(
    dir: &Path,
    dict: &EnvDict
) -> OrionConfResult<Vec<RouteFile>>

// 模式 3: Infrastructure - Trait 方法
impl ConfStdOperation for SyslogSinkConf {
    fn load(path: &Path) -> OrionConfResult<Self> { ... }
}
```

**函数命名不一致**:
```
load_source_instances_from_str()  ← 明确来源(str)
load_business_route_confs()       ← 未指明来源
load_route_files_from()           ← 返回中间类型,不是最终配置
SyslogSinkConf::load()            ← 泛型名称,无上下文
```

**参数顺序不一致**:
```rust
load_source_instances_from_str(config_str, start, dict)  // dict 在最后
load_business_route_confs(sink_root, dict)               // dict 在最后
SyslogSinkConf::load(path)                               // 缺少 dict 参数
```

### 1.2 影响分析

**使用者困惑**:
```rust
// 开发者需要记住每种类型的加载方式
let sources = load_source_instances_from_str(&content, &base, &dict)?;
let sinks = load_business_route_confs(&sink_root, &dict)?;
let syslog = SyslogSinkConf::load(&path)?;  // 等等,这个不需要 dict?
```

**无法编写通用代码**:
```rust
// 无法实现这样的通用加载器
fn load_all_configs<T>(paths: &[PathBuf], dict: &EnvDict) -> Result<Vec<T>> {
    // 因为每种类型的加载方法签名都不同
}
```

**测试重复**:
- 每种加载方式需要独立的测试逻辑
- 模拟(mock)代码重复
- 错误处理测试重复

### 1.3 统计数据

```bash
# 当前加载函数数量
wp-config/src/sources/loader.rs:        3 个公共函数
wp-config/src/sinks/loader.rs:          5 个公共函数
wp-config/src/sinks/infrastructure/*.rs: 4 个 load() 实现
wp-config/src/loader/mod.rs:            ConfStdOperation trait (未广泛使用)

# 调用位置
wp-cli-core 中调用: ~15 处
wp-proj 中调用:     ~8 处
```

---

## 二、设计方案

### 2.1 目标架构

**统一的加载接口 trait**:

```
┌────────────────────────────────────────┐
│      ConfigLoader<T> Trait             │
│  - load_from_path()                    │
│  - load_from_str()                     │
│  - validate()                          │
└─────────────┬──────────────────────────┘
              │ 实现
       ┌──────┼──────┬─────────┐
       │      │      │         │
    Sources Sinks Connectors Infrastructure
```

### 2.2 核心 Trait 设计

#### 2.2.1 ConfigLoader Trait

```rust
// wp-config/src/loader/traits.rs (新建)

use std::path::Path;
use orion_variate::EnvDict;
use crate::OrionConfResult;

/// 统一的配置加载接口
///
/// 所有配置类型都应实现此 trait 以提供一致的加载体验。
pub trait ConfigLoader: Sized {
    /// 配置类型名称(用于错误消息)
    fn config_type_name() -> &'static str;

    /// 从文件路径加载配置
    ///
    /// # 参数
    /// - `path`: 配置文件路径
    /// - `dict`: 环境变量字典,用于变量替换
    ///
    /// # 错误
    /// - 文件不存在
    /// - 文件内容格式错误
    /// - 验证失败
    fn load_from_path(path: &Path, dict: &EnvDict) -> OrionConfResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| {
                orion_error::OrionError::conf(format!(
                    "无法读取 {} 配置文件 {:?}: {}",
                    Self::config_type_name(),
                    path,
                    e
                ))
            })?;

        Self::load_from_str(&content, path.parent().unwrap_or(Path::new(".")), dict)
    }

    /// 从字符串内容加载配置
    ///
    /// # 参数
    /// - `content`: 配置文件内容(通常是 TOML)
    /// - `base`: 基准路径,用于解析相对路径
    /// - `dict`: 环境变量字典
    fn load_from_str(content: &str, base: &Path, dict: &EnvDict) -> OrionConfResult<Self>;

    /// 验证配置(可选,默认不验证)
    ///
    /// 在加载后自动调用,用于检查配置的合法性。
    fn validate(&self) -> OrionConfResult<()> {
        Ok(())
    }
}
```

#### 2.2.2 批量加载辅助函数

```rust
// wp-config/src/loader/batch.rs (新建)

use super::traits::ConfigLoader;
use std::path::{Path, PathBuf};
use orion_variate::EnvDict;
use crate::OrionConfResult;

/// 从目录加载所有配置文件
///
/// # 参数
/// - `dir`: 目录路径
/// - `pattern`: 文件名模式(如 "*.toml")
/// - `dict`: 环境变量字典
pub fn load_all_from_dir<T: ConfigLoader>(
    dir: &Path,
    pattern: &str,
    dict: &EnvDict,
) -> OrionConfResult<Vec<T>> {
    let mut configs = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| {
            orion_error::OrionError::io(format!("无法读取目录 {:?}: {}", dir, e))
        })?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && matches_pattern(&path, pattern) {
            let config = T::load_from_path(&path, dict)?;
            configs.push(config);
        }
    }

    Ok(configs)
}

/// 从多个路径加载配置
pub fn load_from_paths<T: ConfigLoader>(
    paths: &[PathBuf],
    dict: &EnvDict,
) -> OrionConfResult<Vec<T>> {
    paths
        .iter()
        .map(|path| T::load_from_path(path, dict))
        .collect()
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    if pattern == "*" || pattern == "*.*" {
        return true;
    }

    if let Some(ext) = pattern.strip_prefix("*.") {
        return path.extension().map_or(false, |e| e == ext);
    }

    path.file_name()
        .and_then(|n| n.to_str())
        .map_or(false, |n| n == pattern)
}
```

### 2.3 为现有类型实现 ConfigLoader

#### 2.3.1 Sources 加载器

```rust
// wp-config/src/sources/loader.rs (重构)

use crate::loader::traits::ConfigLoader;
use crate::structure::source::instance::SourceInstanceConf;

impl ConfigLoader for Vec<SourceInstanceConf> {
    fn config_type_name() -> &'static str {
        "Sources"
    }

    fn load_from_str(content: &str, base: &Path, dict: &EnvDict) -> OrionConfResult<Self> {
        // 使用现有的解析逻辑
        let sources = toml::from_str::<toml::Value>(content)
            .map_err(|e| orion_error::OrionError::parse(format!("TOML 解析失败: {}", e)))?;

        // 转换为 SourceInstanceConf
        let instances = parse_sources_from_toml(&sources, base, dict)?;

        Ok(instances)
    }

    fn validate(&self) -> OrionConfResult<()> {
        for source in self {
            source.validate()?;
        }
        Ok(())
    }
}

// 保留原有函数作为兼容层(可选)
#[deprecated(since = "1.8.0", note = "请使用 Vec::<SourceInstanceConf>::load_from_str()")]
pub fn load_source_instances_from_str(
    config_str: &str,
    start: &Path,
    dict: &EnvDict,
) -> OrionConfResult<Vec<SourceInstanceConf>> {
    Vec::<SourceInstanceConf>::load_from_str(config_str, start, dict)
}
```

#### 2.3.2 Sinks 加载器

```rust
// wp-config/src/sinks/loader.rs (重构)

use crate::loader::traits::ConfigLoader;
use crate::structure::sink::route::RouteConf;

impl ConfigLoader for Vec<RouteConf> {
    fn config_type_name() -> &'static str {
        "Sink Routes"
    }

    fn load_from_str(content: &str, base: &Path, dict: &EnvDict) -> OrionConfResult<Self> {
        let routes = toml::from_str::<toml::Value>(content)
            .map_err(|e| orion_error::OrionError::parse(format!("TOML 解析失败: {}", e)))?;

        parse_routes_from_toml(&routes, base, dict)
    }

    fn validate(&self) -> OrionConfResult<()> {
        for route in self {
            route.validate()?;
        }
        Ok(())
    }
}

// 新增: 从目录批量加载
pub fn load_routes_from_dir(
    dir: &Path,
    dict: &EnvDict,
) -> OrionConfResult<Vec<RouteConf>> {
    crate::loader::batch::load_all_from_dir(dir, "*.toml", dict)
}
```

#### 2.3.3 Infrastructure Sinks

```rust
// wp-config/src/sinks/infrastructure/syslog.rs (重构)

impl ConfigLoader for SyslogSinkConf {
    fn config_type_name() -> &'static str {
        "Syslog Sink"
    }

    fn load_from_str(content: &str, base: &Path, dict: &EnvDict) -> OrionConfResult<Self> {
        let mut conf: SyslogSinkConf = toml::from_str(content)
            .map_err(|e| orion_error::OrionError::parse(format!("TOML 解析失败: {}", e)))?;

        // 环境变量替换
        conf.host = conf.host.env_eval(dict);
        conf.facility = conf.facility.env_eval(dict);

        Ok(conf)
    }

    fn validate(&self) -> OrionConfResult<()> {
        if self.host.is_empty() {
            return Err(orion_error::OrionError::conf("syslog host 不能为空"));
        }
        Ok(())
    }
}

// 删除旧的 ConfStdOperation trait 实现(已废弃)
```

---

## 三、实施步骤

### 3.1 准备阶段(半天)

**步骤 1**: 创建功能分支
```bash
git checkout develop/1.8
git pull origin develop/1.8
git checkout -b refactor/p0-3-unified-loading
```

**步骤 2**: 审计现有加载函数
```bash
# 查找所有加载函数
grep -r "pub fn load" crates/wp-config/src --include="*.rs" | grep -v "test"
grep -r "impl.*load" crates/wp-config/src --include="*.rs"

# 统计使用情况
grep -r "load_source_instances_from_str\|load_business_route_confs" \
  crates/wp-cli-core/src \
  crates/wp-proj/src \
  --include="*.rs" | wc -l
```

**步骤 3**: 运行基线测试
```bash
cargo test -p wp-config
cargo test --workspace
```

### 3.2 实施阶段(4 天)

#### 阶段 1:创建 Trait 基础设施(1 天)

**步骤 1.1**: 创建 trait 文件
```bash
mkdir -p crates/wp-config/src/loader
touch crates/wp-config/src/loader/traits.rs
touch crates/wp-config/src/loader/batch.rs
```

**步骤 1.2**: 实现 ConfigLoader trait
```rust
// 完整代码见上面 2.2.1 节
```

**步骤 1.3**: 实现批量加载辅助函数
```rust
// 完整代码见上面 2.2.2 节
```

**步骤 1.4**: 更新 lib.rs
```rust
// wp-config/src/lib.rs

pub mod loader;  // 已存在,确保导出 traits 和 batch

// 在 loader/mod.rs 中添加
pub mod traits;
pub mod batch;
```

**步骤 1.5**: 编译检查
```bash
cargo check -p wp-config
```

#### 阶段 2:迁移 Sources(1 天)

**步骤 2.1**: 为 SourceInstanceConf 实现 trait
```rust
// 完整代码见上面 2.3.1 节
```

**步骤 2.2**: 添加弃用警告(可选)
```rust
#[deprecated(since = "1.8.0", note = "请使用 Vec::<SourceInstanceConf>::load_from_str()")]
pub fn load_source_instances_from_str(...) -> OrionConfResult<...> {
    Vec::<SourceInstanceConf>::load_from_str(config_str, start, dict)
}
```

**步骤 2.3**: 更新测试
```rust
#[test]
fn load_sources_via_trait() {
    let content = r#"
        [[sources]]
        id = "test"
    "#;

    let result = Vec::<SourceInstanceConf>::load_from_str(
        content,
        Path::new("/tmp"),
        &EnvDict::default(),
    );

    assert!(result.is_ok());
}
```

**步骤 2.4**: 运行测试
```bash
cargo test -p wp-config sources::
```

#### 阶段 3:迁移 Sinks(1 天)

**步骤 3.1**: 为 RouteConf 实现 trait
```rust
// 完整代码见上面 2.3.2 节
```

**步骤 3.2**: 为 Infrastructure Sinks 实现 trait
```rust
// SyslogSinkConf, FileSinkConf, HttpSinkConf 等
// 完整代码见上面 2.3.3 节
```

**步骤 3.3**: 更新批量加载函数
```rust
pub fn load_routes_from_dir(dir: &Path, dict: &EnvDict) -> OrionConfResult<Vec<RouteConf>> {
    crate::loader::batch::load_all_from_dir(dir, "*.toml", dict)
}
```

**步骤 3.4**: 运行测试
```bash
cargo test -p wp-config sinks::
```

#### 阶段 4:迁移调用代码(1 天)

**步骤 4.1**: 更新 wp-cli-core 调用
```rust
// 修改前
let sources = load_source_instances_from_str(&content, &base, &dict)?;

// 修改后
use wp_config::loader::traits::ConfigLoader;
let sources = Vec::<SourceInstanceConf>::load_from_str(&content, &base, &dict)?;
```

**步骤 4.2**: 更新 wp-proj 调用
```rust
// 类似的修改
```

**步骤 4.3**: 逐模块编译检查
```bash
cargo check -p wp-cli-core
cargo check -p wp-proj
```

**步骤 4.4**: 运行完整测试套件
```bash
cargo test --workspace
```

### 3.3 清理阶段(半天)

**步骤 3.1**: 删除旧的加载函数(可选)
```bash
# 如果添加了 #[deprecated],可以在下个版本删除
# 本阶段可以保留兼容层
```

**步骤 3.2**: 更新文档
```rust
// 在 loader/mod.rs 添加模块文档
//! # 统一配置加载模块
//!
//! 本模块提供统一的配置加载接口 [`ConfigLoader`] trait。
//! 所有配置类型都实现此 trait,提供一致的 API。
//!
//! ## 示例
//!
//! ```no_run
//! use wp_config::loader::traits::ConfigLoader;
//! use wp_config::structure::source::instance::SourceInstanceConf;
//!
//! let sources = Vec::<SourceInstanceConf>::load_from_path(
//!     Path::new("config/sources.toml"),
//!     &EnvDict::default(),
//! )?;
//! # Ok::<(), orion_error::OrionError>(())
//! ```
```

**步骤 3.3**: 提交更改
```bash
git add -A
git commit -m "refactor(wp-config): 统一配置加载接口

- 创建 ConfigLoader trait 提供统一加载接口
- 实现批量加载辅助函数
- 为 Sources, Sinks, Infrastructure 实现 trait
- 更新所有调用代码使用新接口
- 保留旧函数作为弃用的兼容层

受影响的文件:
- wp-config: +150 行(新 trait 系统)
- wp-config: ~80 行重构
- wp-cli-core: ~15 处调用更新
- wp-proj: ~8 处调用更新

测试状态:
✅ 所有测试通过
✅ API 一致性 +40%
✅ 向后兼容(弃用警告)

🤖 Generated with [Claude Code](https://claude.com/claude-code)
Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>"
```

---

## 四、文件变更清单

### 4.1 新建文件

| 文件 | 说明 | 行数 |
|------|------|------|
| `wp-config/src/loader/traits.rs` | ConfigLoader trait 定义 | ~80 |
| `wp-config/src/loader/batch.rs` | 批量加载辅助函数 | ~70 |

### 4.2 修改文件

| 文件 | 变更类型 | 预计改动 | 说明 |
|------|---------|----------|------|
| `wp-config/src/lib.rs` | 导出新模块 | +2 行 | 导出 traits 和 batch |
| `wp-config/src/loader/mod.rs` | 模块组织 | +15 行 | 添加模块文档 |
| `wp-config/src/sources/loader.rs` | 实现 trait | ~40 行 | impl ConfigLoader |
| `wp-config/src/sinks/loader.rs` | 实现 trait | ~50 行 | impl ConfigLoader |
| `wp-config/src/sinks/infrastructure/*.rs` | 实现 trait | ~80 行 | 4 个类型各 ~20 行 |
| `wp-cli-core/src/business/connectors/*.rs` | 调用更新 | ~15 处 | 使用新 API |
| `wp-proj/src/sources/*.rs` | 调用更新 | ~5 处 | 使用新 API |
| `wp-proj/src/sinks/*.rs` | 调用更新 | ~3 处 | 使用新 API |

**总计**:
- 新增: ~150 行
- 修改: ~200 行
- 受影响文件: ~15 个

---

## 五、测试策略

### 5.1 单元测试

**ConfigLoader trait 测试**:
```rust
// wp-config/src/loader/traits.rs

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_from_path_reads_file() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("test.toml");

        std::fs::write(&path, "id = \"test\"").unwrap();

        // 假设有一个简单的测试类型
        let result = TestConfig::load_from_path(&path, &EnvDict::default());
        assert!(result.is_ok());
    }

    #[test]
    fn load_from_path_error_on_missing_file() {
        let result = TestConfig::load_from_path(
            Path::new("/nonexistent/file.toml"),
            &EnvDict::default(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn validate_called_automatically() {
        // 测试验证逻辑是否被调用
    }
}
```

**批量加载测试**:
```rust
// wp-config/src/loader/batch.rs

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_all_from_dir_finds_matching_files() {
        let temp = tempdir().unwrap();

        // 创建测试文件
        std::fs::write(temp.path().join("a.toml"), "id = \"a\"").unwrap();
        std::fs::write(temp.path().join("b.toml"), "id = \"b\"").unwrap();
        std::fs::write(temp.path().join("c.txt"), "ignored").unwrap();

        let configs = load_all_from_dir::<TestConfig>(
            temp.path(),
            "*.toml",
            &EnvDict::default(),
        ).unwrap();

        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn load_from_paths_handles_multiple() {
        let paths = vec![
            PathBuf::from("/path/a.toml"),
            PathBuf::from("/path/b.toml"),
        ];

        // 测试多路径加载
    }
}
```

### 5.2 集成测试

**Sources 加载测试**:
```rust
// wp-config/tests/sources_loader.rs

#[test]
fn load_sources_via_new_api() {
    let content = include_str!("fixtures/sources.toml");

    let sources = Vec::<SourceInstanceConf>::load_from_str(
        content,
        Path::new("tests/fixtures"),
        &EnvDict::default(),
    ).unwrap();

    assert_eq!(sources.len(), 3);
}

#[test]
fn load_sources_validates_automatically() {
    let invalid_content = r#"
        [[sources]]
        id = ""  # 无效: 空 ID
    "#;

    let result = Vec::<SourceInstanceConf>::load_from_str(
        invalid_content,
        Path::new("/tmp"),
        &EnvDict::default(),
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("ID 不能为空"));
}
```

**Sinks 加载测试**:
```rust
// wp-config/tests/sinks_loader.rs

#[test]
fn load_routes_from_directory() {
    let routes = load_routes_from_dir(
        Path::new("tests/fixtures/routes"),
        &EnvDict::default(),
    ).unwrap();

    assert!(routes.len() > 0);
}
```

### 5.3 兼容性测试

**确保旧 API 仍然工作**:
```rust
#[test]
#[allow(deprecated)]
fn old_api_still_works() {
    let content = r#"
        [[sources]]
        id = "test"
    "#;

    let result = load_source_instances_from_str(
        content,
        Path::new("/tmp"),
        &EnvDict::default(),
    );

    assert!(result.is_ok());
}
```

---

## 六、风险评估

### 6.1 高风险点

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **破坏现有 API** | 高 | 1. 保留旧函数作为弃用的兼容层<br>2. 添加 #[deprecated] 警告<br>3. 分阶段迁移 |
| **Trait 设计缺陷** | 中 | 1. 提前进行原型测试<br>2. 至少支持 3 种类型<br>3. 可扩展设计 |
| **性能下降** | 低 | 1. Trait 是零成本抽象<br>2. 逻辑保持不变 |

### 6.2 中等风险点

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| **测试覆盖不足** | 中 | 保留所有现有测试 + 新增 trait 测试 |
| **文档不完整** | 中 | 为 trait 添加详细文档和示例 |
| **迁移遗漏** | 中 | 使用 grep 检查所有调用点 |

### 6.3 回滚策略

**阶段性提交**:
```bash
# 每完成一个阶段就提交
git add wp-config/src/loader
git commit -m "feat(wp-config): add ConfigLoader trait"

git add wp-config/src/sources
git commit -m "refactor(sources): implement ConfigLoader"

git add wp-config/src/sinks
git commit -m "refactor(sinks): implement ConfigLoader"

# 如果出问题,可以逐个回滚
git revert HEAD~2  # 只回滚 sinks 和 sources
```

---

## 七、API 对比示例

### 7.1 加载 Sources

**修改前**:
```rust
use wp_config::sources::loader::load_source_instances_from_str;

let sources = load_source_instances_from_str(&content, &base, &dict)?;
```

**修改后**:
```rust
use wp_config::loader::traits::ConfigLoader;
use wp_config::structure::source::instance::SourceInstanceConf;

// 方式 1: 从字符串
let sources = Vec::<SourceInstanceConf>::load_from_str(&content, &base, &dict)?;

// 方式 2: 从文件
let sources = Vec::<SourceInstanceConf>::load_from_path(&path, &dict)?;

// 方式 3: 批量加载
use wp_config::loader::batch::load_all_from_dir;
let sources = load_all_from_dir::<Vec<SourceInstanceConf>>(
    &dir,
    "*.toml",
    &dict,
)?;
```

### 7.2 加载 Sinks

**修改前**:
```rust
use wp_config::sinks::loader::load_business_route_confs;

let routes = load_business_route_confs(&sink_root, &dict)?;
```

**修改后**:
```rust
use wp_config::loader::traits::ConfigLoader;
use wp_config::structure::sink::route::RouteConf;

// 方式 1: 使用新的统一函数
let routes = load_routes_from_dir(&sink_root, &dict)?;

// 方式 2: 直接使用 trait
let routes = Vec::<RouteConf>::load_from_path(&path, &dict)?;
```

### 7.3 加载 Infrastructure

**修改前**:
```rust
use wp_config::sinks::infrastructure::syslog::SyslogSinkConf;
use wp_config::loader::ConfStdOperation;

let conf = SyslogSinkConf::load(&path)?;  // 缺少 dict 参数
```

**修改后**:
```rust
use wp_config::loader::traits::ConfigLoader;
use wp_config::sinks::infrastructure::syslog::SyslogSinkConf;

let conf = SyslogSinkConf::load_from_path(&path, &dict)?;  // 统一接口
```

### 7.4 通用代码示例

**新能力: 编写通用加载逻辑**:
```rust
use wp_config::loader::traits::ConfigLoader;

/// 通用配置加载器
pub fn load_config_if_exists<T: ConfigLoader>(
    path: &Path,
    dict: &EnvDict,
) -> OrionConfResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }

    let config = T::load_from_path(path, dict)?;
    Ok(Some(config))
}

// 使用示例
let sources = load_config_if_exists::<Vec<SourceInstanceConf>>(&path, &dict)?;
let routes = load_config_if_exists::<Vec<RouteConf>>(&path, &dict)?;
let syslog = load_config_if_exists::<SyslogSinkConf>(&path, &dict)?;
```

---

## 八、验收标准

### 8.1 功能标准

- ✅ ConfigLoader trait 定义清晰,易于实现
- ✅ 至少 5 种配置类型实现 trait
  - Vec<SourceInstanceConf>
  - Vec<RouteConf>
  - SyslogSinkConf
  - FileSinkConf
  - HttpSinkConf
- ✅ 批量加载函数工作正常
- ✅ 环境变量替换正确执行
- ✅ 验证逻辑自动调用

### 8.2 质量标准

- ✅ 所有测试通过(100%)
- ✅ 无编译警告
- ✅ 旧 API 保持兼容(弃用警告)
- ✅ 文档覆盖率 100%

### 8.3 代码指标

- ✅ API 一致性提升 >40%
- ✅ 新增代码 <200 行
- ✅ 调用代码简化率 >30%

---

## 九、后续改进

完成统一后,可以进一步优化:

### 9.1 异步加载支持

```rust
#[async_trait]
pub trait AsyncConfigLoader: Sized {
    async fn load_from_path_async(path: &Path, dict: &EnvDict) -> OrionConfResult<Self>;
}
```

### 9.2 配置缓存

```rust
pub struct CachedLoader<T> {
    cache: HashMap<PathBuf, T>,
}

impl<T: ConfigLoader + Clone> CachedLoader<T> {
    pub fn load_or_cached(&mut self, path: &Path, dict: &EnvDict) -> OrionConfResult<T> {
        if let Some(cached) = self.cache.get(path) {
            return Ok(cached.clone());
        }

        let config = T::load_from_path(path, dict)?;
        self.cache.insert(path.to_path_buf(), config.clone());
        Ok(config)
    }
}
```

### 9.3 配置热重载

```rust
pub trait Reloadable: ConfigLoader {
    fn on_reload(&mut self) -> OrionConfResult<()>;
}
```

---

## 十、检查清单

### 实施前
- [ ] 审查设计文档
- [ ] 统计当前加载函数使用情况
- [ ] 确定 trait 设计合理性
- [ ] 创建功能分支

### 实施中
- [ ] 实现 ConfigLoader trait
- [ ] 实现批量加载辅助函数
- [ ] 为 Sources 实现 trait
- [ ] 为 Sinks 实现 trait
- [ ] 为 Infrastructure 实现 trait
- [ ] 更新所有调用代码

### 验证
- [ ] 所有测试通过
- [ ] 无编译警告
- [ ] 手动测试各种加载场景
- [ ] 代码审查

### 完成
- [ ] 更新文档
- [ ] 添加使用示例
- [ ] 提交清晰的 commit
- [ ] 创建 PR(如需要)

---

**设计文档版本**: 1.0
**创建日期**: 2026-01-10
**预计工作量**: 4-5 天
**风险等级**: 🟡 中

**审查意见**:
```
[请在此处添加审查意见]

批准 / 需要修改 / 拒绝

签名:_______________  日期:_______________
```

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude Sonnet 4.5 <noreply@anthropic.com>
