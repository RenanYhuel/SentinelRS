use comfy_table::{Table, ContentArrangement, presets::UTF8_FULL};

pub fn build_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(headers);
    table
}
