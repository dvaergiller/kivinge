use proc_macro::TokenStream;
use syn::{parse_macro_input, Data, DeriveInput};
use quote::quote;

#[proc_macro_derive(TableRow, attributes(table_row))]
pub fn derive_table_row(row: TokenStream) -> TokenStream {
    let ast: DeriveInput = parse_macro_input!(row);
    let type_name = &ast.ident;
    match ast.data {
        Data::Struct(struct_data) => {
            let cols = struct_data.fields.iter().filter_map(|f| f.ident.clone());
            let cols2 = struct_data.fields.iter().filter_map(|f| f.ident.clone());
            quote!{
                impl TableRow for #type_name {
                    fn columns() -> Vec<&'static str> {
                        vec![#(stringify!(#cols)),*]
                    }
                    fn try_get_field<Field: ToString>(&self, field: Field) -> Option<String> {
                        match field.to_string().as_str() {
                            #(stringify!(#cols2) => Some(self.#cols2.format_cell())),*,
                            _ => None
                        }
                    }
                }
            }
        },
        _ => quote!{ compile_error!("TableRow can only be derived from struct types") }
    }.into()
}
