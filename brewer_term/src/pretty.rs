use colored::Colorize;
use prettytable::{cell, Row, Table};
use prettytable::format::consts::FORMAT_CLEAN;

pub mod header {
    macro_rules! primary {
        ($($arg:tt)*) => {{
            use colored::Colorize;

            let res = format!($($arg)*);

            format!("{} {res}", "==>".truecolor(144, 168, 89))
        }}
    }

    macro_rules! warning {
        ($($arg:tt)*) => {{
            use colored::Colorize;

            let res = format!($($arg)*);

            format!("{} {res}", "==>".yellow())
        }}
    }

    macro_rules! error {
        ($($arg:tt)*) => {{
            use colored::Colorize;

            let res = format!($($arg)*);

            format!("{} {res}", "==>".red())
        }}
    }

    pub(crate) use primary;
    pub(crate) use warning;
    pub(crate) use error;
}

pub fn bool(b: bool) -> String {
    if b {
        "✔".green().to_string()
    } else {
        "✗".red().to_string()
    }
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
    let mut row_len = lens.first().cloned().unwrap_or_default();

    for len in lens {
        if row_len + len + padding < max_width.into() {
            chunk_size += 1;
            row_len += len + padding;
        } else {
            break;
        }
    }

    chunk_size
}