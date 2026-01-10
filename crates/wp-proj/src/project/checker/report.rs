use super::options::CheckComponents;
use super::types::{Cell, Row};
use comfy_table::{Cell as TCell, ContentArrangement, Table, presets::UTF8_FULL};

pub fn build_detail_table(rows: &[Row], comps: &CheckComponents) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        TCell::new("Category"),
        TCell::new("Item"),
        TCell::new("Data"),
        TCell::new("Result"),
    ]);
    for row in rows {
        for entry in detail_entries_for(row, comps) {
            table.add_row(vec![
                TCell::new(entry.category),
                TCell::new(entry.item),
                TCell::new(entry.data),
                TCell::new(entry.result),
            ]);
        }
    }
    table
}

pub fn component_cells<'a>(row: &'a Row, comps: &CheckComponents) -> Vec<(&'static str, &'a Cell)> {
    let mut cells = Vec::new();
    if comps.engine {
        cells.push(("Config", &row.conf));
    }
    if comps.connectors {
        cells.push(("Connectors", &row.connectors));
    }
    if comps.sources {
        cells.push(("Sources", &row.sources));
    }
    if comps.sinks {
        cells.push(("Sinks", &row.sinks));
    }
    if comps.wpl {
        cells.push(("WPL", &row.wpl));
    }
    if comps.oml {
        cells.push(("OML", &row.oml));
    }
    cells
}

struct DetailEntry {
    category: String,
    item: String,
    data: String,
    result: String,
}

fn detail_entries_for(row: &Row, comps: &CheckComponents) -> Vec<DetailEntry> {
    let mut entries = Vec::new();
    let cat = |section: &str| format!("{} / {}", row.path, section);
    if comps.engine {
        let config_data = row
            .conf_detail
            .clone()
            .unwrap_or_else(|| cell_data(&row.conf));
        entries.push(DetailEntry {
            category: cat("Config"),
            item: "Engine config".into(),
            data: config_data,
            result: status_mark(&row.conf).to_string(),
        });
    }

    if comps.connectors {
        if let Some(stats) = &row.connector_counts {
            entries.push(DetailEntry {
                category: cat("Connectors"),
                item: "Source connectors".into(),
                data: format!("{} defs / {} refs", stats.source_defs, stats.source_refs),
                result: status_mark(&row.connectors).to_string(),
            });
            entries.push(DetailEntry {
                category: cat("Connectors"),
                item: "Sink connectors".into(),
                data: format!("{} defs / {} routes", stats.sink_defs, stats.sink_routes),
                result: status_mark(&row.connectors).to_string(),
            });
        } else {
            entries.push(DetailEntry {
                category: cat("Connectors"),
                item: "Summary".into(),
                data: cell_data(&row.connectors),
                result: status_mark(&row.connectors).to_string(),
            });
        }
    }

    if comps.sources {
        if let Some(breakdown) = &row.source_checks {
            entries.push(DetailEntry {
                category: cat("Sources"),
                item: "Structure".into(),
                data: cell_data(&breakdown.syntax),
                result: status_mark(&breakdown.syntax).to_string(),
            });
            entries.push(DetailEntry {
                category: cat("Sources"),
                item: "Runtime".into(),
                data: cell_data(&breakdown.runtime),
                result: status_mark(&breakdown.runtime).to_string(),
            });
        } else {
            entries.push(DetailEntry {
                category: cat("Sources"),
                item: "Summary".into(),
                data: cell_data(&row.sources),
                result: status_mark(&row.sources).to_string(),
            });
        }
    }

    if comps.sinks {
        entries.push(DetailEntry {
            category: cat("Sinks"),
            item: "Targets".into(),
            data: cell_data(&row.sinks),
            result: status_mark(&row.sinks).to_string(),
        });
    }
    if comps.wpl {
        entries.push(DetailEntry {
            category: cat("WPL"),
            item: "Models".into(),
            data: cell_data(&row.wpl),
            result: status_mark(&row.wpl).to_string(),
        });
    }
    if comps.oml {
        entries.push(DetailEntry {
            category: cat("OML"),
            item: "Models".into(),
            data: cell_data(&row.oml),
            result: status_mark(&row.oml).to_string(),
        });
    }

    entries
}

fn status_mark(cell: &Cell) -> &'static str {
    if cell.ok { "✓" } else { "✗" }
}

fn cell_data(cell: &Cell) -> String {
    cell.msg.clone().unwrap_or_else(|| "ok".to_string())
}
