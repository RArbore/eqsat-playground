#![feature(proc_macro_expand)]

use std::collections::HashMap;
use std::fs::read_to_string;

use proc_macro::TokenStream;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TablesSpec(HashMap<String, RelationSpec>);

#[derive(Debug, Deserialize)]
struct RelationSpec {
    symbol: String,
    determinant: Vec<ColumnSpec>,
}

#[derive(Debug, Deserialize)]
struct ColumnSpec {
    name: String,
    sort: String,
    variadic: Option<bool>,
}

#[proc_macro]
pub fn define_database(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let toml = read_to_string(&input[1..(input.len()-1)]).unwrap();
    let spec = toml::from_str::<TablesSpec>(&toml);
    eprintln!("{:?}", spec);
    TokenStream::new()
}
