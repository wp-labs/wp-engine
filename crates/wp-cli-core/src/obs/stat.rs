use anyhow::Result;
use std::path::Path;
use wp_conf::{
    engine::EngineConfig,
    sinks::{load_business_route_confs, load_infra_route_confs},
};
use wpcnt_lib::{SrcLineReport, list_file_sources_with_lines};

/// Sources (file) only
pub fn stat_src_file(work_root: &str, eng_conf: &EngineConfig) -> Result<Option<SrcLineReport>> {
    let ctx = wpcnt_lib::types::Ctx::new(work_root.to_string());
    Ok(wpcnt_lib::list_file_sources_with_lines(
        Path::new(work_root),
        eng_conf,
        &ctx,
    ))
}

/// Sinks (file-like) only; caller must pass resolved sink_root
pub fn stat_sink_file(
    sink_root: &Path,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Vec<wpcnt_lib::types::Row>, u64)> {
    use wpcnt_lib as wlib;
    if !(sink_root.join("business.d").exists() || sink_root.join("infra.d").exists()) {
        anyhow::bail!(
            "缺少 sinks 配置目录：在 '{}' 下未发现 business.d/ 或 infra.d/",
            sink_root.display()
        );
    }
    let mut rows = Vec::new();
    let mut total = 0u64;
    for conf in load_business_route_confs(sink_root.to_string_lossy().as_ref())? {
        let g = conf.sink_group;
        if !wlib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = wlib::process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            false,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    for conf in load_infra_route_confs(sink_root.to_string_lossy().as_ref())? {
        let g = conf.sink_group;
        if !wlib::is_match(g.name().as_str(), &ctx.group_filters) {
            continue;
        }
        let _ = wlib::process_group(
            g.name(),
            g.expect().clone(),
            g.sinks().clone(),
            true,
            ctx,
            &mut rows,
            &mut total,
        );
    }
    Ok((rows, total))
}

/// Combined: src-file + sink-file; requires work_root and sink_root
pub fn stat_file_combined(
    work_root: &str,
    eng_conf: &EngineConfig,
    ctx: &wpcnt_lib::types::Ctx,
) -> Result<(Option<SrcLineReport>, Vec<wpcnt_lib::types::Row>, u64)> {
    let src_rep = list_file_sources_with_lines(Path::new(work_root), eng_conf, ctx);
    let sink_root = Path::new(work_root).join(eng_conf.sink_root());
    let (rows, total) = stat_sink_file(&sink_root, ctx)?;
    Ok((src_rep, rows, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("{}_{}", prefix, nanos));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write_sink_connectors(base: &std::path::Path) {
        let cdir = base.join("connectors").join("sink.d");
        fs::create_dir_all(&cdir).unwrap();
        fs::write(
            cdir.join("file.toml"),
            r#"[[connectors]]
id = "file_sink"
type = "file"
allow_override = ["path","fmt","base","file"]
"#,
        )
        .unwrap();
    }

    fn write_route_with_path(sink_root: &std::path::Path) -> std::path::PathBuf {
        let biz = sink_root.join("business.d");
        fs::create_dir_all(&biz).unwrap();
        let fp = biz.join("demo.toml");
        fs::write(
            &fp,
            r#"version = "2.0"

[sink_group]
name = "demo"
oml  = []

[[sink_group.sinks]]
name = "json"
connect = "file_sink"
params = { base = ".", file = "o1.dat" }
"#,
        )
        .unwrap();
        fp
    }

    fn write_src_connectors(base: &std::path::Path) {
        let cdir = base.join("connectors").join("source.d");
        fs::create_dir_all(&cdir).unwrap();
        fs::write(
            cdir.join("c1.toml"),
            r#"[[connectors]]
id = "c1"
type = "file"
allow_override = ["path"]
"#,
        )
        .unwrap();
    }

    fn write_wpsrc(work_root: &std::path::Path) {
        let dir = work_root.join("models").join("sources");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("wpsrc.toml"),
            r#"[[sources]]
key = "s1"
connect = "c1"
params_override = { path = "src1.dat" }
"#,
        )
        .unwrap();
    }

    #[test]
    fn stat_sink_and_combined_ok() {
        let root = tmp_dir("wpcore_stat");
        write_sink_connectors(&root);
        write_src_connectors(&root);
        write_wpsrc(&root);
        // build case sink tree
        let sink_root = root.join("usecase").join("d").join("c").join("sink");
        write_route_with_path(&sink_root);
        // create example files with lines
        fs::write(root.join("o1.dat"), b"a\nb\n").unwrap();
        fs::write(root.join("src1.dat"), b"x\ny\nz\n").unwrap();

        let ctx = wpcnt_lib::types::Ctx::new(root.to_string_lossy().to_string());
        // sink-only stat
        let (rows, total) = stat_sink_file(&sink_root, &ctx).expect("sink stat");
        assert!(!rows.is_empty());
        assert_eq!(total, 2);

        // combined stat
        let (src_rep, rows2, total2) =
            stat_file_combined(root.to_string_lossy().as_ref(), &sink_root, &ctx)
                .expect("combined");
        assert_eq!(rows2.len(), rows.len());
        assert_eq!(total2, total);
        assert_eq!(src_rep.unwrap().total_enabled_lines, 3);
    }
}
