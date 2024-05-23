// view_utils.rs
use prettytable::{row, Table};
use prettytable::row::Row;
use prettytable::cell::Cell;
use serde_json::Value;

pub fn print_table(slots: &[Value]) {
    let mut table = Table::new();
    table.add_row(row!["type", "start", "end", "max_sz", "min_sz", "qty", "id", "token"]);

    for slot in slots {
        if let (
            Some(slot_type), Some(start), Some(end), Some(min_size), Some(max_size), Some(quantity),
            Some(id), Some(token)
        ) = (
            slot.get("type"),
            slot.get("start"),
            slot.get("end"),
            slot.get("min_size"),
            slot.get("max_size"),

            slot.get("quantity"),
            slot.get("id"),
            slot.get("token"),
        ) {
            table.add_row(Row::new(vec![
                Cell::new(slot_type.as_str().unwrap_or("")),
                Cell::new(start.as_str().unwrap_or("")),
                Cell::new(&end.as_str().unwrap_or("")),
                Cell::new(&min_size.to_string()),
                Cell::new(&max_size.to_string()),
                Cell::new(&quantity.to_string()),
                Cell::new(&id.to_string()),
                Cell::new(token.as_str().unwrap_or("")),
            ]));
        }
    }

    table.printstd();
}
