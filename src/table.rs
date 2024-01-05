use once_cell::sync::Lazy;

pub static TABLE_FORMAT: Lazy<prettytable::format::TableFormat> = Lazy::new(|| {
    use prettytable::format::{FormatBuilder, LinePosition, LineSeparator};

    FormatBuilder::new()
        .column_separator(' ')
        .separator(LinePosition::Top, LineSeparator::new('─', ' ', ' ', ' '))
        .separator(LinePosition::Title, LineSeparator::new('─', ' ', ' ', ' '))
        .separator(LinePosition::Intern, LineSeparator::new('┈', ' ', ' ', ' '))
        .separator(LinePosition::Bottom, LineSeparator::new('─', ' ', ' ', ' '))
        .padding(1, 1)
        .build()
});

pub struct Table {}

impl Table {
    pub fn new(titles: prettytable::Row) -> prettytable::Table {
        let mut table = prettytable::Table::new();
        table.set_format(*TABLE_FORMAT);
        table.set_titles(titles);

        table
    }
}
