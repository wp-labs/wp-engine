# Warp Parse Engine

<div align="center">

![CI](https://github.com/wp-labs/wp-engine/workflows/CI/badge.svg)
[![codecov](https://codecov.io/gh/wp-labs/wp-engine/graph/badge.svg?token=6SVCXBHB6B)](https://codecov.io/gh/wp-labs/wp-engine)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

High-performance data parsing and processing engine built in Rust

</div>

## Overview

Warp Parse Engine (WP-Engine) is a high-performance, modular data parsing and processing engine designed for handling large-scale data streams with low latency and high throughput. It provides a domain-specific language (WPL) for defining parsing rules and supports multiple data formats and protocols.

## Features

- **High Performance**: Built with Rust for optimal performance and memory safety
- **Domain-Specific Language**: WPL (Warp Processing Language) for flexible rule definitions
- **Multi-format Support**: JSON, CSV, Protobuf, Syslog, and custom formats
- **Real-time Processing**: Stream processing with sub-millisecond latency
- **Extensible Architecture**: Plugin system for custom processors and sinks
- **Enterprise Ready**: Built-in monitoring, metrics, and fault tolerance

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/wp-labs/wp-engine.git
cd wp-engine

# Build the project
cargo build --release
```

### Basic Usage

```rust
use wp_engine::facade::WpApp;
use wp_engine::core::parser::wpl_engine::repo::WplRepository;

// Create a WPL rule
let rule = r#"
package /main {
    rule parse_log {
        timestamp "_" "request" "status" "response_time"
    }
}
"#;

// Build the parsing engine
let repo = WplRepository::from_wpl(rule)?;
let app = WpApp::from_config(repo)?;

// Process data
let input = "2024-01-01T12:00:00Z GET /api/users 200 45";
let result = app.process(input)?;
println!("Parsed: {:?}", result);
```

### Using CLI

```bash
# Parse data with WPL rules
wproj parse --rule rules.wpl --input data.log

# Generate rules from sample data
wproj gen-rule --sample sample.log --output rule.wpl

# Validate configuration
wproj check --config config.toml
```

## Architecture

```
wp-engine (root)
├── crates/                    # Core libraries
│   ├── wp-lang              # WPL language and parser
│   ├── wp-oml               # Object Modeling Language
│   ├── wp-parser            # Low-level parsing primitives
│   ├── wp-config            # Configuration management
│   ├── wp-data-utils        # Data structures and utilities
│   ├── wp-knowledge         # Knowledge database (KnowDB)
│   ├── wp-cli-core          # CLI shared infrastructure
│   ├── wp-cli-utils         # CLI utilities
│   ├── wp-proj              # Project management
│   ├── wp-stats             # Statistics collection
│   ├── orion_overload       # Common utilities
│   └── orion_exp           # Expression evaluation
├── src/                      # Main application
│   ├── core/               # Core engine
│   ├── runtime/            # Runtime components
│   ├── sources/            # Data sources
│   ├── sinks/              # Data sinks
│   ├── facade/             Public API
│   └── orchestrator/       # Orchestration
└── tests/                    # Integration tests
```

## WPL (Warp Processing Language)

WPL is a powerful domain-specific language for defining parsing rules:

```wpl
package /logs {
    rule nginx_access {
        # Parse timestamp
        timestamp "[" time(time_local) "]"
        # Parse remote address
        remote_addr chars@ip
        # Parse request line
        request_line "\"" chars@request "\""
        # Parse status
        status digit@3
        # Parse response size
        body_bytes_sent digits
    }

    rule extract_user {
        # Extract user ID from request
        request "^/api/users/" digits@user_id "/"?
    }

    # Define pipe processing
    pipe enrich {
        plg_pipe/geo_lookup(ip) -> country
        plg_pipe/user_agent_lookup(user_agent) -> device_type
    }
}
```

## Configuration

The engine uses TOML for configuration:

```toml
[engine]
workers = 4
buffer_size = 8192

[sources]
  [sources.tcp]
  type = "tcp"
  host = "0.0.0.0"
  port = 8080

  [sources.file]
  type = "file"
  path = "/var/log/nginx/access.log"

[sinks]
  [sinks.elasticsearch]
  type = "elasticsearch"
  url = "http://localhost:9200"
  index = "logs"

[monitoring]
  metrics_enabled = true
  prometheus_port = 9090
```

## Performance

- **Throughput**: Up to 1M events/second per core
- **Latency**: < 1ms for simple parsing rules
- **Memory**: < 100MB baseline for typical workloads
- **CPU**: Efficient use with SIMD optimizations

See [Benchmarks](./crates/wp-stats/BENCHMARKS.md) for detailed performance metrics.

## Plugins

Extend the engine with custom processors:

```rust
use wp_engine::plugin::{PipeProcessor, register_wpl_pipe};

struct CustomProcessor;

impl PipeProcessor for CustomProcessor {
    fn process(&self, data: RawData) -> WparseResult<RawData> {
        // Custom processing logic
        Ok(transformed_data)
    }

    fn name(&self) -> &'static str {
        "custom_processor"
    }
}

// Register the processor
register_wpl_pipe!("custom", || Hold::new(CustomProcessor));
```

## Feature Flags

- `default`: Community edition with core runtime
- `runtime-core`: Base runtime functionality
- `enterprise-backend`: Enterprise-only backend features
- `perf-ci`: Performance testing in CI
- `dev-tools`: Development utilities

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Install dependencies
cargo install cargo-watch cargo-expand

# Run tests
cargo test

# Run with auto-reload
cargo watch -x run

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

## Documentation

- [API Documentation](https://docs.wp-labs.com/wp-engine)
- [WPL Language Guide](docs/wpl-guide.md)
- [Plugin Development](docs/plugin-development.md)
- [Deployment Guide](docs/deployment.md)

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Support

- [Issues](https://github.com/wp-labs/wp-engine/issues)
- [Discussions](https://github.com/wp-labs/wp-engine/discussions)
- [Community Discord](https://discord.gg/wp-engine)

---

## Warp Parse Engine（Warp 解析引擎）

<div align="center">

[![Rust](https://img.shields.io/badge/rust-1.74+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/wp-labs/wp-engine)

用 Rust 构建的高性能数据解析和处理引擎

</div>

## 概述

Warp Parse Engine（WP-Engine）是一个高性能、模块化的数据解析和处理引擎，专为处理大规模数据流而设计，具有低延迟和高吞吐量的特点。它提供了领域特定语言（WPL）来定义解析规则，并支持多种数据格式和协议。

## 特性

- **高性能**：使用 Rust 构建，确保最佳性能和内存安全
- **领域特定语言**：WPL（Warp Processing Language）用于灵活的规则定义
- **多格式支持**：JSON、CSV、Protobuf、Syslog 和自定义格式
- **实时处理**：流处理，延迟低于毫秒级
- **可扩展架构**：插件系统支持自定义处理器和输出端
- **企业级就绪**：内置监控、指标和容错功能


## 架构

```
wp-engine (根目录)
├── crates/                    # 核心库
│   ├── wp-lang              # WPL 语言和解析器
│   ├── wp-oml               # 对象建模语言
│   ├── wp-parser            # 底层解析原语
│   ├── wp-config            # 配置管理
│   ├── wp-data-utils        # 数据结构和工具
│   ├── wp-knowledge         # 知识数据库 (KnowDB)
│   ├── wp-cli-core          # CLI 共享基础设施
│   ├── wp-cli-utils         # CLI 工具
│   ├── wp-proj              # 项目管理
│   ├── wp-stats             # 统计收集
│   ├── orion_overload       # 通用工具
│   └── orion_exp           # 表达式求值
├── src/                      # 主应用
│   ├── core/               # 核心引擎
│   ├── runtime/            # 运行时组件
│   ├── sources/            # 数据源
│   ├── sinks/              # 数据汇
│   ├── facade/             # 公共 API
│   └── orchestrator/       # 编排
└── tests/                    # 集成测试
```

## 许可证

本项目采用 Apache License 2.0 许可证 - 详情请参见 [LICENSE](LICENSE) 文件。

## 支持

- [问题反馈](https://github.com/wp-labs/wp-engine/issues)
- [讨论区](https://github.com/wp-labs/wp-engine/discussions)
- [社区 Discord](https://discord.gg/wp-engine)

---

**Warp Parse Dev Team**
