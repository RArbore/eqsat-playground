#![feature(proc_macro_expand)]

use std::collections::BTreeMap;
use std::fs::read_to_string;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use serde::Deserialize;
use syn::{Ident, TypePath, parse_str};

#[derive(Clone, Debug, Deserialize)]
struct TablesSpec(BTreeMap<String, RelationSpec>);

#[derive(Clone, Debug, Deserialize)]
struct RelationSpec {
    symbol: String,
    determinant: Vec<ColumnSpec>,
    dependent: Option<Vec<ColumnSpec>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ColumnSpec {
    name: String,
    sort: String,
    variadic: Option<bool>,
}

#[proc_macro]
pub fn define_database(input: TokenStream) -> TokenStream {
    let input = input.to_string();
    let toml = read_to_string(&input[1..(input.len() - 1)]).unwrap();
    let spec = toml::from_str::<TablesSpec>(&toml).unwrap();
    let default_dep_spec = vec![ColumnSpec {
        name: "root".to_string(),
        sort: "::util::union_find::ClassId".to_string(),
        variadic: None,
    }];

    let mut db_fields = vec![];
    let mut lowercase_relations = vec![];
    let mut uppercase_relations = vec![];
    let mut symbol_fields = vec![];
    let mut symbols = vec![];
    let mut insert_arms = vec![];
    let mut row_enum_fields = vec![];
    for (name, spec) in spec.0.clone() {
        let lowercase_relation_name = Ident::new(&name.to_ascii_lowercase(), Span::call_site());
        let uppercase_relation_name = Ident::new(&name, Span::call_site());
        let symbol_field = Ident::new(&format!("{}_symbol", name.to_ascii_lowercase()), Span::call_site());
        symbol_fields.push(symbol_field.clone());
        symbols.push(spec.symbol);
        let dependent = spec.dependent.unwrap_or(default_dep_spec.clone());
        let det_cols = spec.determinant.len();
        let dep_cols = dependent.len();
        db_fields.push(quote! {
            #lowercase_relation_name: Option<::db::table::Table<#det_cols, #dep_cols>>,
            #symbol_field: ::util::interner::IdentifierId,
        });

        let field_name = Ident::new(&name.to_ascii_lowercase(), Span::call_site());
        let symbol_field = Ident::new(&format!("{}_symbol", name.to_ascii_lowercase()), Span::call_site());
        let mut field_names = vec![];
        let mut args = vec![];
        let mut det_names = vec![];
        let mut dep_names = vec![];
        let mut merge_exprs = vec![];
        let mut arg_idxs = vec![];
        let mut ret_idxs = vec![];
        for (idx, column) in spec.determinant.clone().into_iter().enumerate() {
            let name = Ident::new(&column.name, Span::call_site());
            let sort: TypePath = parse_str(&column.sort).unwrap();
            field_names.push(quote! {#name});
            args.push(quote! {
                #name: #sort
            });
            det_names.push(name);
            arg_idxs.push(idx);
        }
        for (idx, column) in dependent
            .into_iter()
            .enumerate()
        {
            let name = Ident::new(&column.name, Span::call_site());
            let sort: TypePath = parse_str(&column.sort).unwrap();
            field_names.push(quote! {#name});
            args.push(quote! {
                #name: #sort
            });
            ret_idxs.push(idx);
            dep_names.push(name);
            if column.sort == "::util::union_find::ClassId" {
                merge_exprs.push(quote!(self.uf.merge(
                    unsafe { ::core::mem::transmute(dep[#idx]) },
                    unsafe { ::core::mem::transmute(new_dep[#idx]) },
                );));
            } else {
                panic!()
            }
        }
        insert_arms.push(quote! {
            Row::#uppercase_relation_name { #(#field_names),* } => {
                let det = [#(unsafe { ::core::mem::transmute(#det_names) }),*];
                let dep = [#(unsafe { ::core::mem::transmute(#dep_names) }),*];
                let new_dep = self.#field_name.get_or_insert_with(|| ::db::table::Table::new(self.#symbol_field)).insert_row(&det, &dep);
                if new_dep != &dep {
                    #(#merge_exprs)*
                }
                #(let #det_names = unsafe { ::core::mem::transmute(det[#arg_idxs]) };)*
                #(let #dep_names = unsafe { ::core::mem::transmute(new_dep[#ret_idxs]) };)*
                Row::#uppercase_relation_name { #(#field_names),* }
            }
        });
        row_enum_fields.push(quote!(#(#args),*));
        lowercase_relations.push(lowercase_relation_name);
        uppercase_relations.push(uppercase_relation_name);
    }

    quote! {
        struct Database {
            #(#db_fields)*
            uf: ::util::union_find::UnionFind,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Row {
            #(#uppercase_relations { #row_enum_fields }),*
        }

        impl Database {
            fn new(interner: &mut ::util::interner::StringInterner) -> Self {
                Self {
                    uf: ::util::union_find::UnionFind::new(),
                    #(#lowercase_relations: None),*,
                    #(#symbol_fields: interner.intern(#symbols)),*,
                }
            }

            fn insert(&mut self, row: Row) -> Row {
                match row {
                    #(#insert_arms),*
                }
            }
        }
    }
    .into()
}
