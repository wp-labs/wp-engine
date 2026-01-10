use std::path::Path;

use crate::connectors::{
    lint::lint_rows_from_root,
    templates::init_definitions,
    types::{LintRow, LintSeverity, SilentErrKind},
};
use crate::traits::{Checkable, Component};
use crate::types::CheckStatus;

use super::paths::ConnectorsPaths;
use wp_error::run_error::{RunReason, RunResult};

#[derive(Clone)]
#[allow(dead_code)] // paths field is used in tests but not detected
pub struct Connectors {
    pub paths: ConnectorsPaths,
}

impl Connectors {
    pub fn new(paths: ConnectorsPaths) -> Self {
        Self { paths }
    }

    pub fn lint_rows_from_root<P: AsRef<Path>>(&self, work_root: P) -> Vec<LintRow> {
        lint_rows_from_root(work_root)
    }

    pub fn init_definition<P: AsRef<Path>>(&self, work_root: P) -> RunResult<()> {
        // 仅生成 connectors/ 内的模板目录，避免在工作根制造 legacy source.d/sink.d
        init_definitions(work_root)
    }

    /// 检查连接器配置是否有效
    /// 返回检查结果和可能的错误信息
    pub fn check<P: AsRef<Path>>(&self, work_root: P) -> Result<bool, String> {
        let mut errors = Vec::new();
        for row in self.lint_rows_from_root(work_root.as_ref()) {
            if matches!(row.sev, LintSeverity::Error) {
                errors.push(format_lint_error(&row));
            }
        }
        if errors.is_empty() {
            Ok(true)
        } else {
            Err(format!(
                "connectors lint failed: {} error(s)\n{}",
                errors.len(),
                errors.join("\n")
            ))
        }
    }
}

fn format_lint_error(row: &LintRow) -> String {
    match row.silent_err {
        Some(SilentErrKind::BadIdChars) => {
            format!("{}: bad id chars: {} in {}", row.scope, row.id, row.file)
        }
        Some(SilentErrKind::SourcesIdMustEndSrc) => format!(
            "{}: id must end with _src: {} in {}",
            row.scope, row.id, row.file
        ),
        Some(SilentErrKind::SinksIdMustEndSink) => format!(
            "{}: id must end with _sink: {} in {}",
            row.scope, row.id, row.file
        ),
        None => format!(
            "{}: parse failed for {}: {}",
            row.scope,
            row.file,
            row.msg.replace("parse failed: ", ""),
        ),
    }
}

// Trait implementations for unified component interface
impl Component for Connectors {
    fn component_name(&self) -> &'static str {
        "Connectors"
    }
}

// Note: Connectors does not implement Checkable because its check() method
// requires a work_root parameter, which differs from the trait signature.
// The existing check(&self, work_root) method is preserved for compatibility.
