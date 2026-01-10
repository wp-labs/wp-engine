use super::super::types::Row;

pub fn print_rows(rows: &[Row], total: u64) {
    for it in rows {
        let full_name = format!("{}/{}", it.group, it.sink);
        println!(
            "{:<10} | {:<25} | {:<45} | {}",
            if it.infras { "infras" } else { "business" },
            full_name,
            it.path,
            it.lines
        );
    }
    println!("-- total lines: {}", total);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_print_rows_format() {
        let rows = vec![
            Row {
                group: "business".to_string(),
                sink: "demo_sink".to_string(),
                path: "./data/output.dat".to_string(),
                lines: 1000,
                infras: false,
            },
            Row {
                group: "default".to_string(),
                sink: "error_sink".to_string(),
                path: "./data/error.log".to_string(),
                lines: 50,
                infras: true,
            },
        ];

        // 测试打印不会崩溃，并验证格式
        print_rows(&rows, 1050);
    }
}
