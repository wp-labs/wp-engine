pub mod business;
pub mod data;
pub mod knowdb;
pub mod utils;

// Re-export business functions for convenience
pub use business::observability::{
    build_groups_v2,
    collect_sink_statistics,
    list_file_sources_with_lines,
    total_input_from_wpsrc,
    process_group,
    SrcLineReport,
};
