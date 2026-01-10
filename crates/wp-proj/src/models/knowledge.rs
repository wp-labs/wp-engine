use orion_conf::{EnvTomlLoad, UvsConfFrom};
use orion_error::ToStructError;
use orion_variate::EnvDict;
use wp_error::run_error::{RunReason, RunResult};

#[derive(Clone)]
pub struct Knowledge;

impl Knowledge {
    pub fn new() -> Self {
        Knowledge
    }
    pub fn init(&self, work_root: &str) -> RunResult<()> {
        wp_cli_core::knowdb::init(work_root, false)
            .map_err(|e| RunReason::from_conf(e.to_string()).to_err())
    }
}

use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use wp_knowledge::loader::KnowDbConf;

#[derive(Debug, Clone, Serialize)]
pub struct TableCheck {
    pub name: String,
    pub dir: String,
    pub create_ok: bool,
    pub insert_ok: bool,
    pub data_ok: bool,
    pub columns_ok: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CheckReport {
    pub total: usize,
    pub ok: usize,
    pub fail: usize,
    pub tables: Vec<TableCheck>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CleanReport {
    pub removed_models_dir: bool,
    pub removed_authority_cache: bool,
    pub not_found_models: bool,
}

/// Initialize knowledge DB skeleton under `<work_root>/models/knowledge`.
pub fn init(work_root: &str, full: bool) -> Result<()> {
    // build TOML body via simple structs to avoid drift
    #[derive(Serialize)]
    struct KnowdbToml {
        version: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        base_dir: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<LoadSpec>,
        #[serde(skip_serializing_if = "Option::is_none")]
        csv: Option<CsvSpec>,
        tables: Vec<TableSpec>,
    }
    #[derive(Serialize)]
    struct LoadSpec {
        transaction: bool,
        batch_size: usize,
        on_error: String,
    }
    #[derive(Serialize)]
    struct CsvSpec {
        has_header: bool,
        delimiter: String,
        encoding: String,
        trim: bool,
    }
    #[derive(Serialize)]
    struct TableSpec {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        dir: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data_file: Option<String>,
        columns: ColumnsSpec,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected_rows: Option<RowExpect>,
        #[serde(skip_serializing_if = "Option::is_none")]
        enabled: Option<bool>,
    }
    #[derive(Serialize)]
    struct ColumnsSpec {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        by_header: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        by_index: Vec<usize>,
    }
    #[derive(Serialize)]
    struct RowExpect {
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<usize>,
    }

    let spec = if full {
        KnowdbToml {
            version: 2,
            base_dir: Some(".".into()),
            default: Some(LoadSpec {
                transaction: true,
                batch_size: 2000,
                on_error: "fail".into(),
            }),
            csv: Some(CsvSpec {
                has_header: true,
                delimiter: ",".into(),
                encoding: "utf-8".into(),
                trim: true,
            }),
            tables: vec![TableSpec {
                name: "example".into(),
                dir: Some("example".into()),
                data_file: None,
                columns: ColumnsSpec {
                    by_header: vec!["name".into(), "pinying".into()],
                    by_index: vec![],
                },
                expected_rows: Some(RowExpect {
                    min: Some(1),
                    max: Some(100),
                }),
                enabled: Some(true),
            }],
        }
    } else {
        KnowdbToml {
            version: 2,
            base_dir: None,
            default: None,
            csv: None,
            tables: vec![TableSpec {
                name: "example".into(),
                dir: None,
                data_file: None,
                columns: ColumnsSpec {
                    by_header: vec!["name".into(), "pinying".into()],
                    by_index: vec![],
                },
                expected_rows: Some(RowExpect {
                    min: Some(1),
                    max: None,
                }),
                enabled: None,
            }],
        }
    };

    let wr = PathBuf::from(work_root);
    let models_dir = wr.join("models").join("knowledge");
    fs::create_dir_all(&models_dir)?;
    let body = toml::to_string_pretty(&spec).unwrap_or_else(|_| {
        "version = 2\n\n[[tables]]\nname = \"example\"\ncolumns.by_header = [\"name\", \"pinying\"]\n".to_string()
    });
    fs::write(models_dir.join("knowdb.toml"), body)?;
    // example dir and sample files
    let ex = models_dir.join("example");
    fs::create_dir_all(&ex)?;
    fs::write(
        ex.join("create.sql"),
        "CREATE TABLE IF NOT EXISTS {table} (\n  id      INTEGER PRIMARY KEY,\n  name    TEXT NOT NULL,\n  pinying TEXT NOT NULL\n);\nCREATE INDEX IF NOT EXISTS idx_{table}_name ON {table}(name);\n",
    )?;
    fs::write(
        ex.join("insert.sql"),
        "INSERT INTO {table} (name, pinying) VALUES (?1, ?2);\n",
    )?;
    fs::write(
        ex.join("data.csv"),
        "name,pinying\n令狐冲,linghuchong\n任盈盈,renyingying\n",
    )?;
    Ok(())
}

/// Check knowledge config and table files; returns structured table-level status.
pub fn check(work_root: &str, dict: &EnvDict) -> Result<CheckReport> {
    let wr = PathBuf::from(work_root);
    let conf_path = wr.join("models/knowledge/knowdb.toml");
    if !conf_path.exists() {
        anyhow::bail!("knowdb config not found: {}", conf_path.display());
    }
    let conf: KnowDbConf = KnowDbConf::env_load_toml(&conf_path, dict)?;
    if conf.version != 2 {
        anyhow::bail!("knowdb.version must be 2");
    }

    // base_dir relative to knowdb.toml
    let base_dir = conf_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(conf.base_dir);
    let mut rep = CheckReport::default();
    for t in conf.tables.into_iter().filter(|t| t.enabled) {
        let dir_name = t.dir.clone().unwrap_or(t.name.clone());
        let tdir = base_dir.join(&dir_name);
        let create_ok = tdir.join("create.sql").exists();
        let insert_ok = tdir.join("insert.sql").exists();
        let data_p = tdir.join(t.data_file.unwrap_or_else(|| "data.csv".into()));
        let data_ok = data_p.exists();
        let columns_ok = !t.columns.by_header.is_empty() || !t.columns.by_index.is_empty();
        let ok = create_ok && insert_ok && data_ok && columns_ok;
        if ok {
            rep.ok += 1;
        } else {
            rep.fail += 1;
        }
        rep.total += 1;
        rep.tables.push(TableCheck {
            name: t.name,
            dir: tdir.to_string_lossy().to_string(),
            create_ok,
            insert_ok,
            data_ok,
            columns_ok,
        });
    }
    Ok(rep)
}

/// Clean knowledge models directory and authority cache.
pub fn clean(work_root: &str) -> Result<CleanReport> {
    let wr = PathBuf::from(work_root);
    let models_dir = wr.join("models").join("knowledge");
    let mut rep = CleanReport::default();
    match std::fs::remove_dir_all(&models_dir) {
        Ok(_) => {
            rep.removed_models_dir = true;
        }
        Err(_) => {
            rep.not_found_models = !models_dir.exists();
            if !rep.not_found_models {
                anyhow::bail!("remove '{}' failed", models_dir.display());
            }
        }
    }
    let auth = wr.join(".run").join("authority.sqlite");
    if auth.exists() {
        let _ = std::fs::remove_file(&auth);
        rep.removed_authority_cache = true;
    }
    Ok(rep)
}
