use super::defs::ConnectorTomlFile;
use orion_conf::TomlIO;
use orion_conf::error::{ConfIOReason, OrionConfResult};
use orion_error::{ErrorOwe, ErrorWith, ToStructError, UvsValidationFrom};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use wp_connector_api::{ConnectorDef, ConnectorScope};

fn collect_connector_files(dir: &Path) -> OrionConfResult<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .owe_conf()
        .with(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|s| s == "toml").unwrap_or(false))
        .collect();
    files.sort();
    Ok(files)
}

pub fn load_connector_defs_from_dir(
    dir: &Path,
    scope: ConnectorScope,
) -> OrionConfResult<Vec<ConnectorDef>> {
    let mut map: BTreeMap<String, ConnectorDef> = BTreeMap::new();
    for fp in collect_connector_files(dir)? {
        let file: ConnectorTomlFile = ConnectorTomlFile::load_toml(&fp)?;
        for mut def in file.connectors {
            let origin = Some(fp.display().to_string());
            if map.contains_key(&def.id) {
                return ConfIOReason::from_validation(format!(
                    "duplicate connector id '{}' (file {})",
                    def.id,
                    fp.display()
                ))
                .err_result();
            }
            def.scope = scope;
            def.origin = origin;
            map.insert(def.id.clone(), def);
        }
    }
    Ok(map.into_values().collect())
}
