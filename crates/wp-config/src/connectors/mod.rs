pub mod defs;
mod toml;

pub use defs::{param_map_from_table_ref, param_map_to_table, param_value_from_toml, ConnectorTomlFile};
pub use toml::load_connector_defs_from_dir;
pub use wp_connector_api::{
    ConnectorDef, ConnectorDefProvider, ConnectorScope, ParamMap,
    parammap_from_toml_table as param_map_from_table,
};
