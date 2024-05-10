use colored::Colorize;
use prettytable::{cell, Row, Table};
use prettytable::format::consts::FORMAT_CLEAN;

pub fn header(text: &str) -> String {
    const ARROW: &str = "==>";

    format!("{} {text}", ARROW.truecolor(144, 168, 89))
}

pub fn table(values: &[String], max_width: u16) -> Table {
    const RIGHT_PADDING: usize = 2;

    let mut table = Table::new();
    let mut format = *FORMAT_CLEAN;
    format.padding(0, RIGHT_PADDING);

    table.set_format(format);
    table.unset_titles();

    let chunk_size = calculate_chunk_size(values, RIGHT_PADDING, max_width);
    for row in values.chunks(chunk_size) {
        let row: Vec<_> = row.iter().map(|n| cell!(n)).collect();

        table.add_row(Row::new(row));
    }

    table
}

fn calculate_chunk_size(values: &[String], padding: usize, max_width: u16) -> usize {
    let mut lens: Vec<_> = values.iter().map(|v| v.len()).collect();

    lens.sort_unstable_by(|a, b| b.cmp(a));

    let mut chunk_size = 1;
    let mut row_len = 0;

    for len in lens {
        if row_len + len + chunk_size * padding < max_width.into() {
            chunk_size += 1;
            row_len += len;
        } else {
            break;
        }
    }

    chunk_size
}