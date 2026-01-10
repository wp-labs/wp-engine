//! Observability business logic for sources and sinks
//!
//! This module provides high-level business functions for collecting
//! observability data about sources and sinks.

mod sources;
mod sinks;

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
