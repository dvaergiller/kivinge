use std::ops::Deref;

pub struct Table<Row: TableRow>(Vec<Row>);

impl<Row: TableRow> Deref for Table<Row> {
    type Target = Vec<Row>;
    fn deref(&self) -> &Vec<Row> {
        let Table(rows) = self;
        &rows
    }
}

pub trait TableRow {
    fn columns() -> Vec<&'static str>;
    fn try_get_field<Field: ToString>(&self, field: Field) -> Option<String>;
}

trait TableCell {
    fn format_cell(&self) -> String;
}

impl<C: TableCell + Default> TableCell for Option<C> {
    fn format_cell(&self) -> String {
        self.unwrap_or_default().format_cell()
    }
}

impl<C: ToString> TableCell for C {
    fn format_cell(&self) -> String {
        self.to_string()
    }
}
