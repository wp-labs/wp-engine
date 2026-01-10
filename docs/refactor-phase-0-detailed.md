# 阶段 0: 准备工作 - 详细执行方案

> **目标**: 确保重构前有充分的安全保障，建立测试基线和 API 契约

**预计时间**: 2-3 小时
**风险等级**: 🟢 低
**可回滚**: ✅ 是

---

## 任务清单

### 任务 0.1: 创建功能分支 ⏱️ 5 分钟

**操作步骤**:
```bash
# 1. 确保当前在主分支，且工作区干净
git checkout develop/1.8
git status  # 确认无未提交更改

# 2. 拉取最新代码
git pull origin develop/1.8

# 3. 创建重构分支
git checkout -b refactor/simplify-cli-architecture

# 4. 推送到远程（可选，用于备份）
git push -u origin refactor/simplify-cli-architecture
```

**验证**:
```bash
git branch  # 应显示 * refactor/simplify-cli-architecture
```

---

### 任务 0.2: 记录当前测试基线 ⏱️ 30 分钟

**操作步骤**:

#### Step 1: 运行所有测试并记录结果

```bash
# 运行完整测试套件
cargo test --workspace 2>&1 | tee docs/baseline-test-report.txt

# 记录测试统计
echo "=== Test Baseline ===" > docs/refactor-baseline.md
echo "Date: $(date)" >> docs/refactor-baseline.md
echo "" >> docs/refactor-baseline.md
echo "## Test Results" >> docs/refactor-baseline.md
cargo test --workspace 2>&1 | grep -E "test result|running" >> docs/refactor-baseline.md
```

#### Step 2: 记录编译时间基线

```bash
# 清理构建缓存
cargo clean

# 记录编译时间
echo "" >> docs/refactor-baseline.md
echo "## Build Time Baseline" >> docs/refactor-baseline.md
echo "### Clean build:" >> docs/refactor-baseline.md
time cargo build --release 2>&1 | tail -n 3 >> docs/refactor-baseline.md

echo "### Incremental build:" >> docs/refactor-baseline.md
touch crates/wp-cli-core/src/lib.rs
time cargo build --release 2>&1 | tail -n 3 >> docs/refactor-baseline.md
```

#### Step 3: 记录代码行数基线

```bash
echo "" >> docs/refactor-baseline.md
echo "## Code Metrics Baseline" >> docs/refactor-baseline.md

# 统计各 crate 的代码行数
for crate in wp-cli-core wp-cli-utils wp-config; do
    echo "### $crate:" >> docs/refactor-baseline.md
    find crates/$crate/src -name "*.rs" | xargs wc -l | tail -n 1 >> docs/refactor-baseline.md
done

# 统计总行数
echo "### Total:" >> docs/refactor-baseline.md
find crates/wp-cli-{core,utils} crates/wp-config/src -name "*.rs" | xargs wc -l | tail -n 1 >> docs/refactor-baseline.md
```

**预期输出示例**:
```
docs/refactor-baseline.md:
=== Test Baseline ===
Date: 2026-01-10 14:30:00

## Test Results
running 45 tests
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

## Build Time Baseline
### Clean build:
real    2m15.234s
user    8m32.156s
sys     0m45.321s

### Incremental build:
real    0m5.123s
user    0m12.456s
sys     0m2.345s

## Code Metrics Baseline
### wp-cli-core:
    1234 total
### wp-cli-utils:
    567 total
### wp-config:
    2345 total
### Total:
    4146 total
```

---

### 任务 0.3: 记录公共 API 契约 ⏱️ 1 小时

**目标**: 确保重构后不破坏公共接口

#### Step 1: 提取 wp-cli-core 公共 API

创建 `docs/api-contract-cli-core.md`:

```bash
cat > docs/api-contract-cli-core.md << 'EOF'
# wp-cli-core 公共 API 契约

## 必须保持兼容的公共接口

### connectors::sources

```rust
pub fn list_connectors(
    work_root: &str,
    eng_conf: &EngineConfig,
    dict: &EnvDict,
) -> OrionConfResult<Vec<ConnectorListRow>>

pub fn route_table(
    work_root: &str,
    eng_conf: &EngineConfig,
    path_like: Option<&str>,
    dict: &EnvDict,
) -> OrionConfResult<Vec<RouteRow>>
```

### connectors::sinks

```rust
pub fn validate_routes(work_root: &str) -> OrionConfResult<()>

pub fn list_connectors_usage(
    work_root: &str,
) -> OrionConfResult<(BTreeMap<String, ConnectorRec>, Vec<(String, String, String)>)>

pub fn route_table(
    work_root: &str,
    group_filters: &[String],
    sink_filters: &[String],
) -> OrionConfResult<Vec<RouteRow>>

pub fn load_connectors_map(work_root: &str) -> OrionConfResult<BTreeMap<String, ConnectorRec>>
```

### obs::stat

```rust
pub fn stat_src_file(
    work_root: &str,
    eng_conf: &EngineConfig,
) -> Result<Option<SrcLineReport>>

pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Vec<wpcnt_lib::types::Row>, u64)>

pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Option<SrcLineReport>, Vec<wpcnt_lib::types::Row>, u64)>
```

## 数据结构

### ConnectorListRow
```rust
pub struct ConnectorListRow {
    pub id: String,
    pub kind: String,
    pub allow_override: Vec<String>,
    pub detail: String,
    pub refs: usize,
}
```

### RouteRow (sources)
```rust
pub struct RouteRow {
    pub key: String,
    pub connect: String,
    pub kind: String,
    pub enabled: bool,
    pub detail: String,
}
```

### RouteRow (sinks)
```rust
pub struct RouteRow {
    pub scope: String,
    pub group: String,
    pub full_name: String,
    pub name: String,
    pub connector: String,
    pub target: String,
    pub fmt: String,
    pub detail: String,
    pub rules: Vec<String>,
    pub oml: Vec<String>,
}
```
EOF
```

#### Step 2: 提取 wp-cli-utils 公共 API

创建 `docs/api-contract-cli-utils.md`:

```bash
cat > docs/api-contract-cli-utils.md << 'EOF'
# wp-cli-utils 公共 API 契约

## 必须保持兼容的公共接口

### sources

```rust
pub fn list_file_sources_with_lines(
    work_root: &Path,
    eng_conf: &EngineConfig,
    ctx: &Ctx,
) -> Option<SrcLineReport>

pub fn total_input_from_wpsrc(
    work_root: &Path,
    engine_conf: &EngineConfig,
    ctx: &Ctx,
) -> Option<u64>
```

### validate

```rust
pub fn validate_groups(
    groups: &[GroupAccum],
    total_override: Option<u64>,
) -> ValidateReport
```

### stats

```rust
pub fn load_stats_file(path: &Path) -> Result<StatsFile>
pub fn group_input(stats: &StatsFile, filters: &[String]) -> Vec<GroupAccum>
```

### scan

```rust
pub fn process_group(
    name: &str,
    expect: Option<GroupExpectSpec>,
    sinks: Vec<SinkInstanceConf>,
    is_infra: bool,
    ctx: &Ctx,
    rows: &mut Vec<Row>,
    total: &mut u64,
) -> Result<()>
```

### pretty

```rust
pub fn print_src_files_table(report: &SrcLineReport)
pub fn print_validate_report(report: &ValidateReport)
pub fn print_rows(rows: &[Row])
// ... 其他 print 函数
```

### fsutils

```rust
pub fn resolve_path(path: &str, work_root: &Path) -> PathBuf
pub fn count_lines_file(path: &Path) -> Result<u64>
```

### banner

```rust
pub fn print_banner(version: &str)
pub fn split_quiet_args(args: Vec<String>) -> (Vec<String>, bool)
```

## 数据结构

### SrcLineReport
```rust
pub struct SrcLineReport {
    pub total_enabled_lines: u64,
    pub items: Vec<SrcLineItem>,
}

pub struct SrcLineItem {
    pub key: String,
    pub path: String,
    pub enabled: bool,
    pub lines: Option<u64>,
    pub error: Option<String>,
}
```

### ValidateReport
```rust
pub struct ValidateReport {
    pub items: Vec<ValidateItem>,
}

pub struct ValidateItem {
    pub group: String,
    pub sink: Option<String>,
    pub msg: String,
    pub severity: Severity,
}
```
EOF
```

#### Step 3: 提取 wp-config 公共 API

创建 `docs/api-contract-config.md`:

```bash
cat > docs/api-contract-config.md << 'EOF'
# wp-config 公共 API 契约

## 必须保持兼容的公共接口

### sources

```rust
pub fn find_connectors_dir(path: &Path) -> Option<PathBuf>
pub fn load_connectors_for(dir: &Path, env: &EnvDict) -> Result<BTreeMap<String, SourceConnector>>
// ... 完整列表见现有代码
```

### sinks

```rust
pub fn load_route_files_from(dir: &Path, env: &EnvDict) -> Result<Vec<RouteFile>>
pub fn load_sink_defaults(sink_root: &str, env: &EnvDict) -> Result<Option<DefaultsBody>>
pub fn build_route_conf_from(...) -> Result<RouteConf>
pub fn load_business_route_confs(sink_root: &str, env: &EnvDict) -> Result<Vec<RouteConf>>
pub fn load_infra_route_confs(sink_root: &str, env: &EnvDict) -> Result<Vec<RouteConf>>
// ... 完整列表见现有代码
```

### connectors

```rust
pub fn load_connector_defs_from_dir(...) -> Result<Vec<ConnectorDef>>
pub fn param_value_from_toml(value: &toml::Value) -> ParamValue
pub fn param_map_to_table(params: &ParamMap) -> toml::Table
```

### engine

```rust
impl EngineConfig {
    pub fn init(work_root: &str) -> Self
    pub fn src_root(&self) -> &str
    pub fn sink_root(&self) -> &str
}
```

## 注意事项

- ⚠️ wp-config 将是重构中最稳定的部分，尽量不修改其公共接口
- ✅ 可以新增接口，但不能删除或修改现有接口签名
EOF
```

---

### 任务 0.4: 创建重构检查清单 ⏱️ 15 分钟

创建 `docs/refactor-checklist.md`:

```bash
cat > docs/refactor-checklist.md << 'EOF'
# 重构检查清单

## 阶段 0: 准备工作
- [ ] 创建功能分支 `refactor/simplify-cli-architecture`
- [ ] 记录测试基线（docs/baseline-test-report.txt）
- [ ] 记录编译时间基线（docs/refactor-baseline.md）
- [ ] 记录代码行数基线
- [ ] 记录公共 API 契约
  - [ ] wp-cli-core API (docs/api-contract-cli-core.md)
  - [ ] wp-cli-utils API (docs/api-contract-cli-utils.md)
  - [ ] wp-config API (docs/api-contract-config.md)
- [ ] 创建本检查清单

## 每个阶段的通用检查项

### 编码前
- [ ] 阅读并理解本阶段的详细方案
- [ ] 创建本阶段的子分支（可选）
- [ ] 提交当前所有更改

### 编码中
- [ ] 按照方案逐步修改代码
- [ ] 每个小步骤后编译检查
- [ ] 及时提交有意义的变更

### 编码后
- [ ] 运行 `cargo build --workspace`，确保编译通过
- [ ] 运行 `cargo test --workspace`，确保所有测试通过
- [ ] 运行 `cargo clippy --workspace`，检查代码质量
- [ ] 对比 API 契约，确保没有破坏公共接口
- [ ] 更新文档（如有必要）
- [ ] 提交本阶段的更改
- [ ] 标记本阶段为完成

## 回滚准备

### 如何回滚到任意阶段
```bash
# 查看提交历史
git log --oneline

# 回滚到特定提交
git reset --hard <commit-hash>

# 或者创建新分支从头开始
git checkout -b refactor/retry develop/1.8
```

### 紧急回滚到主分支
```bash
git checkout develop/1.8
git branch -D refactor/simplify-cli-architecture
```

## 风险监控

每个阶段完成后检查：
- [ ] 编译时间变化 < 10%
- [ ] 测试通过率 = 100%
- [ ] 代码覆盖率没有下降
- [ ] 没有新的编译警告

## 里程碑

- [ ] 阶段 0 完成: 准备工作 ✅
- [ ] 阶段 1 完成: 统一参数合并逻辑
- [ ] 阶段 2 完成: 移除业务逻辑下沉
- [ ] 阶段 3 完成: 缩短调用链
- [ ] 阶段 4 完成: 创建新目录结构
- [ ] 阶段 5 完成: 迁移代码
- [ ] 阶段 6 完成: 清理和验证
- [ ] 最终验证: 所有目标达成
EOF
```

---

### 任务 0.5: 补充关键路径的集成测试 ⏱️ 1 小时

**目标**: 确保重构后主要功能不被破坏

#### Step 1: 为 source 统计添加集成测试

创建或更新 `crates/wp-cli-core/tests/integration_stat_sources.rs`:

```rust
// crates/wp-cli-core/tests/integration_stat_sources.rs
use std::fs;
use std::path::PathBuf;
use wp_cli_core::obs::stat;
use wp_conf::engine::EngineConfig;

fn create_test_env() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();

    // 创建目录结构
    fs::create_dir_all(root.join("connectors/source.d")).unwrap();
    fs::create_dir_all(root.join("topology/sources")).unwrap();

    // 创建 connector 定义
    fs::write(
        root.join("connectors/source.d/file.toml"),
        r#"
[[connectors]]
id = "test_file"
type = "file"
allow_override = ["path"]

[connectors.default_params]
fmt = "json"
"#,
    )
    .unwrap();

    // 创建 wpsrc.toml
    fs::write(
        root.join("topology/sources/wpsrc.toml"),
        r#"
[[sources]]
key = "test_source"
connect = "test_file"
enable = true
params_override = { path = "test_data.log" }
"#,
    )
    .unwrap();

    // 创建测试数据文件
    fs::write(
        root.join("test_data.log"),
        "line1\nline2\nline3\n",
    )
    .unwrap();

    (temp, root)
}

#[test]
fn test_stat_src_file_integration() {
    let (_temp, root) = create_test_env();
    let eng_conf = EngineConfig::init(root.to_str().unwrap());

    let result = stat::stat_src_file(root.to_str().unwrap(), &eng_conf);

    assert!(result.is_ok());
    let report = result.unwrap();
    assert!(report.is_some());

    let report = report.unwrap();
    assert_eq!(report.items.len(), 1);
    assert_eq!(report.items[0].key, "test_source");
    assert_eq!(report.items[0].lines, Some(3));
    assert_eq!(report.total_enabled_lines, 3);
}
```

#### Step 2: 为 sink 路由验证添加集成测试

创建 `crates/wp-cli-core/tests/integration_sinks_validation.rs`:

```rust
// crates/wp-cli-core/tests/integration_sinks_validation.rs
use std::fs;
use wp_cli_core::connectors::sinks;

fn create_sink_test_env() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().to_path_buf();

    // 创建目录
    fs::create_dir_all(root.join("connectors/sink.d")).unwrap();
    fs::create_dir_all(root.join("models/sinks/business.d")).unwrap();

    // Connector 定义
    fs::write(
        root.join("connectors/sink.d/test.toml"),
        r#"
[[connectors]]
id = "test_sink"
type = "file"
allow_override = ["file"]
"#,
    )
    .unwrap();

    // 有效的路由配置
    fs::write(
        root.join("models/sinks/business.d/valid.toml"),
        r#"
version = "2.0"

[sink_group]
name = "test_group"
rule = ["/test/*"]

[[sink_group.sinks]]
name = "sink1"
connect = "test_sink"
params = { file = "output.txt" }
"#,
    )
    .unwrap();

    (temp, root)
}

#[test]
fn test_validate_routes_success() {
    let (_temp, root) = create_sink_test_env();
    let result = sinks::validate_routes(root.to_str().unwrap());
    assert!(result.is_ok());
}

#[test]
fn test_validate_routes_oml_rule_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("connectors/sink.d")).unwrap();
    fs::create_dir_all(root.join("models/sinks/business.d")).unwrap();

    fs::write(
        root.join("connectors/sink.d/test.toml"),
        r#"
[[connectors]]
id = "test_sink"
type = "file"
allow_override = ["file"]
"#,
    )
    .unwrap();

    // 冲突配置: 同时有 OML 和 RULE
    fs::write(
        root.join("models/sinks/business.d/invalid.toml"),
        r#"
version = "2.0"

[sink_group]
name = "conflict_group"
oml = ["model1"]
rule = ["/test/*"]

[[sink_group.sinks]]
name = "sink1"
connect = "test_sink"
params = { file = "output.txt" }
"#,
    )
    .unwrap();

    let result = sinks::validate_routes(root.to_str().unwrap());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("OML and RULE cannot be used together"));
}
```

#### Step 3: 运行新测试

```bash
# 运行新增的集成测试
cargo test --package wp-cli-core --test integration_stat_sources
cargo test --package wp-cli-core --test integration_sinks_validation

# 确保所有测试通过
cargo test --workspace
```

---

## 阶段 0 完成验证

完成所有任务后，执行以下验证：

```bash
# 1. 检查所有文档已创建
ls -la docs/refactor-*.md docs/api-contract-*.md docs/baseline-test-report.txt

# 2. 检查测试基线已记录
cat docs/refactor-baseline.md

# 3. 检查分支已创建
git branch | grep refactor/simplify-cli-architecture

# 4. 检查测试通过
cargo test --workspace

# 5. 提交阶段 0 的所有更改
git add .
git commit -m "refactor(phase-0): prepare for architecture simplification

- Create refactor branch
- Record test baseline and metrics
- Document public API contracts
- Add integration tests for critical paths
- Create refactor checklist
"

# 6. 推送到远程（可选）
git push origin refactor/simplify-cli-architecture
```

---

## 预期产出

完成阶段 0 后，应该有以下文件：

```
docs/
├── refactor-baseline.md              # 测试和性能基线
├── baseline-test-report.txt          # 完整测试输出
├── api-contract-cli-core.md          # wp-cli-core API 契约
├── api-contract-cli-utils.md         # wp-cli-utils API 契约
├── api-contract-config.md            # wp-config API 契约
└── refactor-checklist.md             # 重构检查清单

crates/wp-cli-core/tests/
├── integration_stat_sources.rs       # Source 统计集成测试
└── integration_sinks_validation.rs   # Sink 验证集成测试
```

---

## 时间估算明细

| 任务 | 预计时间 | 实际时间 |
|------|---------|---------|
| 0.1 创建分支 | 5 分钟 | ___ |
| 0.2 记录基线 | 30 分钟 | ___ |
| 0.3 记录 API | 1 小时 | ___ |
| 0.4 创建检查清单 | 15 分钟 | ___ |
| 0.5 补充测试 | 1 小时 | ___ |
| **总计** | **2.5-3 小时** | ___ |

---

## 批准检查点

**请确认以下内容后批准执行**:

- [ ] 我理解阶段 0 的目标是建立安全保障
- [ ] 我同意创建新的功能分支进行重构
- [ ] 我同意记录当前状态作为基线
- [ ] 我同意补充必要的集成测试
- [ ] 本阶段的所有操作都是安全的，可以随时回滚

**批准方式**:
- ✅ 批准执行: 请回复 "批准阶段 0"
- ❌ 需要调整: 请说明需要修改的部分
- ⏸️ 暂缓执行: 请说明原因

---

**下一阶段预告**:
阶段 1 将统一参数合并逻辑，这是最低风险、最高收益的改进点。
