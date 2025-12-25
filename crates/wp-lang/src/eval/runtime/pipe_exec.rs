use crate::eval::runtime::field_pipe::PipeEnum;
use wp_model_core::model::DataField;
use wp_parser::WResult as ModalResult;

use super::field_pipe::{FieldIndex, FiledSetProcessor};
use once_cell::sync::OnceCell;

/// Heuristic thresholds to enable FieldIndex for Fun pipes.
/// Can be overridden via environment variables at process start:
/// - `WP_PIPE_FUN_THRESH` (default: 20)
/// - `WP_PIPE_FIELD_THRESH` (default: 1024)
struct Thresholds {
    fun: usize,
    fields: usize,
}

static PIPE_THRESHOLDS: OnceCell<Thresholds> = OnceCell::new();

#[inline]
fn thresholds() -> &'static Thresholds {
    PIPE_THRESHOLDS.get_or_init(|| Thresholds {
        fun: std::env::var("WP_PIPE_FUN_THRESH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20),
        fields: std::env::var("WP_PIPE_FIELD_THRESH")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1024),
    })
}

#[derive(Clone, Default)]
pub struct PipeExecutor {
    pipes: Vec<PipeEnum>,
}

impl PipeExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pipe(&mut self, pipe: PipeEnum) {
        self.pipes.push(pipe);
    }

    pub fn execute(&self, data: &mut Vec<DataField>) -> ModalResult<()> {
        // 简单启发式：Fun pipes 足够多且字段数较大时才构建索引，避免小规模时的克隆/哈希开销
        let fun_cnt = self
            .pipes
            .iter()
            .filter(|p| matches!(p, PipeEnum::Fun(_)))
            .count();

        let mut maybe_index: Option<FieldIndex> = None;
        let th = thresholds();
        if fun_cnt >= th.fun && data.len() >= th.fields {
            maybe_index = Some(FieldIndex::build(data));
        }

        for pipe in &self.pipes {
            match pipe {
                PipeEnum::Fun(_) => pipe.process(data, maybe_index.as_ref())?,
                PipeEnum::Group(_) => {
                    pipe.process(data, None)?;
                    // fields 可能被修改：如已启用索引则重建
                    if maybe_index.is_some() {
                        maybe_index = Some(FieldIndex::build(data));
                    }
                }
            }
        }
        Ok(())
    }
}
