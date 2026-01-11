# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.8.1] - 2024-01-11

### Added
- **P0-3**: `ConfigLoader` trait to unify configuration loading interface (crates/wp-config/src/loader/traits.rs)
- **P0-4**: `ComponentBase` trait system to standardize component architecture across wp-proj
- **P0-5**: Unified API consistency with new `fs` utilities module in wp-proj
- **P0-2**: Error conversion helpers module (`error_conv`, `error_handler`) to simplify error handling
- **P0-1**: Centralized knowledge base operations in wp-cli-core to eliminate duplication
- Comprehensive documentation comments for ConfigLoader trait
- Path normalization for log directory display to remove redundant `./` components (crates/wp-proj/src/utils/log_handler.rs:48-76)
- Test case `normalize_path_removes_current_dir_components` to verify path normalization

### Changed
- **Breaking**: EnvDict parameter now required in all configuration loading functions
  - `validate_routes(work_root: &str, env_dict: &EnvDict)` (wp-cli-core/src/business/connectors/sinks.rs:18)
  - `collect_sink_statistics(sink_root: &Path, ctx: &Ctx, dict: &EnvDict)` (wp-cli-core/src/business/observability/sinks.rs:21)
  - `load_warp_engine_confs(work_root: &str, dict: &EnvDict)` (src/orchestrator/config/models/warp_helpers.rs:17)
  - And 13 more functions across wp-proj and wp-cli-core
- **Architecture**: Enforced top-level EnvDict initialization pattern
  - EnvDict must be created at application entry point (e.g., `load_sec_dict()` in warp-parse)
  - Crate-level functions only accept `dict: &EnvDict` parameter, never create instances
  - This follows dependency injection pattern for better testability and clarity
- Source and sink factories now return multiple connector definitions instead of single instance
- Improved table formatting in CLI output for better readability

### Fixed
- Default sink path resolution now works correctly
- Engine configuration path normalization to handle `.` and `..` components properly
- Empty stat fields are now skipped during serialization
- Project initialization bug resolved
- Documentation test closure parameter issues in error_conv module
- Log directory paths now display correctly without `././` in output messages (crates/wp-proj/src/utils/log_handler.rs:96,102)
- Clippy warning `field_reassign_with_default` in wpgen configuration (crates/wp-config/src/generator/wpgen.rs:125)

### Refactored
- **wp-proj Stage 1**: Extracted common patterns to reduce code duplication
- **wp-proj Stage 2**: Implemented Component trait system for models, I/O, and connectors
- **wp-proj Stage 3**: Documented standard error handling patterns
- **wp-proj Stage 4**: Merged `check` and `checker` modules to eliminate responsibility overlap
- Knowledge base operations delegated from wp-proj to wp-cli-core

### Removed
- `EnvDictExt` trait removed from wp-config as it violated architectural separation
  - App layer (warp-parse, wpgen) is responsible for EnvDict creation
  - Crate layer (wp-engine, wp-proj, wp-config) only receives and uses EnvDict
- Documentation files: `envdict-ext-usage.md`, `envdict-ext-quickref.md`

## [1.8.0] - 2024-01-05

### Added
- Environment variable templating support via `orion-variate` integration
- `EnvDict` type for managing environment variables during configuration loading
- Environment variable substitution in configuration files using `${VAR}` syntax
- Three-level variable resolution: dict → system env → default value
- Tests for environment variable substitution in config loading
- Path resolution for relative configuration paths

### Changed
- Updated `orion_conf` dependency to version 0.4
- Updated `wp-infras` dependencies to track main branch
- License changed from MIT to SLv2 (Server License v2)
- Work root resolution now uses `Option<String>` for better API clarity
- Configuration loading functions now accept `EnvDict` parameter
- Replaced direct `toml::from_str` calls with `EnvTomlLoad::env_parse_toml`

### Fixed
- Work root validation issue (#56) - invalid work-root paths now properly handled
- Partial parsing handling improved with residue tracking and error logging

### Removed
- `Cargo.lock` removed from version control
- Unnecessary `provided_root` parameter removed from path resolution functions

## Version Comparison Links

[Unreleased]: https://github.com/wp-labs/wp-engine/compare/v1.8.1...HEAD
[1.8.1]: https://github.com/wp-labs/wp-engine/compare/v1.8.0-alpha...v1.8.1-alpha
[1.8.0]: https://github.com/wp-labs/wp-engine/compare/v1.7.0-alpha...v1.8.0-alpha
