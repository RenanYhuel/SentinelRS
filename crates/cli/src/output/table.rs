use comfy_table::{presets::UTF8_FULL, Attribute, Cell, Color, ContentArrangement, Table};

pub fn build_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    let cells: Vec<Cell> = headers
        .iter()
        .map(|h| Cell::new(h).fg(Color::Cyan).add_attribute(Attribute::Bold))
        .collect();
    table.set_header(cells);
    table
}
