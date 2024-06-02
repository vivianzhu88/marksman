// view_utils.rs
use prettytable::{row, Table};
use prettytable::row::Row;
use prettytable::cell::Cell;
use crate::resy_client::ResySlot;

pub fn print_table(slots: &[ResySlot]) {
    let mut table = Table::new();
    table.add_row(row!["type", "start", "end", "min_sz", "max_sz", "qty", "id", "token"]);

    for slot in slots {
        table.add_row(Row::new(vec![
            Cell::new(&slot.slot_type),
            Cell::new(&slot.start),
            Cell::new(&slot.end),
            Cell::new(&slot.min_size.to_string()),
            Cell::new(&slot.max_size.to_string()),
            Cell::new(&slot.quantity.to_string()),
            Cell::new(&slot.id),
            Cell::new(&slot.token),
        ]));
    }

    table.printstd();
}
