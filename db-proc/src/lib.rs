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
    for (name, spec) in spec.0.clone() {
        let relation_name = Ident::new(&name.to_ascii_lowercase(), Span::call_site());
        let dependent = spec.dependent.unwrap_or(default_dep_spec.clone());
        let det_cols = spec.determinant.len();
        let dep_cols = dependent.len();
        db_fields.push(quote! {
            #relation_name: ::db::table::Table<#det_cols, #dep_cols>,
        });
    }

    let mut impl_fns = vec![];
    for (name, spec) in spec.0 {
        let field_name = Ident::new(&name.to_ascii_lowercase(), Span::call_site());
        let fn_name = Ident::new(
            &format!("create_{}", name.to_ascii_lowercase()),
            Span::call_site(),
        );
        let mut args = vec![];
        let mut det_names = vec![];
        let mut dep_names = vec![];
        let mut merge_exprs = vec![];
        for column in spec.determinant.clone() {
            let name = Ident::new(&column.name, Span::call_site());
            let sort: TypePath = parse_str(&column.sort).unwrap();
            args.push(quote! {
                #name: #sort
            });
            det_names.push(name);
        }
        for (idx, column) in spec
            .dependent
            .unwrap_or(default_dep_spec.clone())
            .into_iter()
            .enumerate()
        {
            let name = Ident::new(&column.name, Span::call_site());
            let sort: TypePath = parse_str(&column.sort).unwrap();
            args.push(quote! {
                #name: #sort
            });
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
        impl_fns.push(quote! {
            fn #fn_name(&mut self, #(#args),*) {
                let det = [#(unsafe { ::core::mem::transmute(#det_names) }),*];
                let dep = [#(unsafe { ::core::mem::transmute(#dep_names) }),*];
                let new_dep = self.#field_name.insert_row(&det, &dep);
                if new_dep != &dep {
                    #(#merge_exprs)*
                }
            }
        });
    }

    quote! {
        struct Database {
            #(#db_fields)*
            uf: ::util::union_find::UnionFind,
        }

        impl Database {
            #(#impl_fns)*
        }
    }
    .into()
}
