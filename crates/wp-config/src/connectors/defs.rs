use serde::{Deserialize, Serialize};
use wp_connector_api::ConnectorDef;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorTomlFile {
    #[serde(default)]
    pub connectors: Vec<ConnectorDef>,
}
