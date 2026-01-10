use comfy_table::{
    Cell, CellAlignment, ContentArrangement, Row as CRow, Table, presets::ASCII_MARKDOWN,
};
use super::super::types::SrcLineReport;

/// Print file sources (from wpsrc) in table form.
/// Columns: Key | Enabled | Lines | Path | Error
pub fn print_src_files_table(rep: &SrcLineReport) {
    let mut t = Table::new();
    t.load_preset(ASCII_MARKDOWN);
    t.set_content_arrangement(ContentArrangement::Dynamic);
    t.set_header(vec!["Key", "Enabled", "Lines", "Path", "Error"]);
    for it in &rep.items {
        let en = if it.enabled { "Y" } else { "N" };
        let lines = it
            .lines
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let err = it.error.clone().unwrap_or_else(|| "-".to_string());
        let mut row = CRow::new();
        for s in [it.key.clone(), en.to_string(), lines, it.path.clone(), err] {
            let align = if s.len() <= 6 {
                CellAlignment::Center
            } else {
                CellAlignment::Left
            };
            row.add_cell(Cell::new(s).set_alignment(align));
        }
        t.add_row(row);
    }
    println!("{}", t);
    println!("Total enabled lines: {}", rep.total_enabled_lines);
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::types::{SrcLineItem, SrcLineReport};

    #[test]
    fn print_sources_table_does_not_panic() {
        let rep = SrcLineReport {
            total_enabled_lines: 100,
            items: vec![
                SrcLineItem {
                    key: "file_1".into(),
                    path: "./data/in_dat/gen.dat".into(),
                    enabled: true,
                    lines: Some(100),
                    error: None,
                },
                SrcLineItem {
                    key: "file_2".into(),
                    path: "./data/in_dat/gen2.dat".into(),
                    enabled: false,
                    lines: None,
                    error: Some("not found".into()),
                },
            ],
        };
        // Only assert it doesn't panic (formatting to stdout)
        print_src_files_table(&rep);
    }
}
