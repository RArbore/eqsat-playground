use core::hash::Hash;
use core::mem::swap;
use std::collections::HashSet;

use util::union_find::{ClassId, UnionFind};

use crate::table::Table;

pub trait ENode: PartialEq {
    fn root(&self) -> ClassId;
    fn canonicalize(&self, uf: &mut UnionFind) -> Self;
}

pub fn rebuild_table<const DET_COLS: usize, const DEP_COLS: usize, T, E, D>(
    table: &mut Table<DET_COLS, DEP_COLS>,
    uf: &mut UnionFind,
    encode: E,
    decode: D,
) -> bool
where
    T: ENode,
    E: Fn(&T) -> ([u32; DET_COLS], [u32; DEP_COLS]),
    D: Fn(&[u32; DET_COLS], &[u32; DEP_COLS]) -> T,
{
    let mut ever_changed = false;
    loop {
        let mut changed = false;

        let mut maybe_row_id = table.first_row();
        while let Some(row_id) = maybe_row_id {
            let row = table.get_row(row_id);
            let term = decode(&row.0, &row.1);
            let canon_term = term.canonicalize(uf);
            if term != canon_term {
                changed = true;
                table.delete_row(row_id);
                let canon_row = encode(&canon_term);
                let new_dep = table.insert_row(&canon_row.0, &canon_row.1);
                if new_dep != &canon_row.1 {
                    let resident_term = decode(&canon_row.0, new_dep);
                    uf.merge(canon_term.root(), resident_term.root());
                }
            }

            maybe_row_id = table.next_row(row_id);
        }

        if !changed {
            break;
        } else {
            ever_changed = true;
        }
    }
    ever_changed
}

pub fn corebuild<T>(terms: Vec<T>, uf: &mut UnionFind)
where
    T: Clone + ENode + Eq + Hash,
{
    let num_classes = uf.num_classes();
    let mut last_uf = UnionFind::new_all_equals(num_classes);
    let mut next_uf = UnionFind::new_all_not_equals(num_classes);
    let mut observations = vec![HashSet::<T>::new(); num_classes as usize];

    loop {
        for term in &terms {
            observations[term.root().idx() as usize].insert(term.canonicalize(&mut last_uf));
        }

        for lhs in 0..num_classes {
            for rhs in 0..num_classes {
                if !observations[lhs as usize].is_disjoint(&observations[rhs as usize]) {
                    next_uf.merge(ClassId::new(lhs), ClassId::new(rhs));
                }
            }
        }

        if last_uf == next_uf {
            break;
        } else {
            swap(&mut last_uf, &mut next_uf);
            next_uf = UnionFind::new_all_not_equals(num_classes);
            observations.clear();
            observations.resize(num_classes as usize, HashSet::new());
        }
    }

    for idx in 0..num_classes {
        let id = ClassId::new(idx);
        let canon = last_uf.find(id);
        uf.merge(id, canon);
    }
}
