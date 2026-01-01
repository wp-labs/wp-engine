pub mod defs;
mod toml;

pub use defs::{
    ConnectorTomlFile, param_map_from_table_ref, param_map_to_table, param_value_from_toml,
};
pub use toml::load_connector_defs_from_dir;
pub use wp_connector_api::{
    ConnectorDef, ConnectorScope, ParamMap, SinkDefProvider, SourceDefProvider,
    parammap_from_toml_table as param_map_from_table,
};
