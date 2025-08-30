use crate::table::Table;

pub trait Theory {
    type Term: PartialEq;

    fn canonicalize(&mut self, term: &Self::Term) -> Self::Term;
    fn solve(&mut self, lhs: &Self::Term, rhs: &Self::Term);
}

pub fn rebuild_table<const DET_COLS: usize, const DEP_COLS: usize, T, E, D>(
    table: &mut Table<DET_COLS, DEP_COLS>,
    theory: &mut T,
    encode: E,
    decode: D,
) -> bool
where
    T: Theory,
    E: Fn(&T::Term) -> ([u32; DET_COLS], [u32; DEP_COLS]),
    D: Fn(&[u32; DET_COLS], &[u32; DEP_COLS]) -> T::Term,
{
    let mut ever_changed = false;
    loop {
        let mut changed = false;

        let mut maybe_row_id = table.first_row();
        while let Some(row_id) = maybe_row_id {
            let row = table.get_row(row_id);
            let term = decode(&row.0, &row.1);
            let canon_term = theory.canonicalize(&term);
            if term != canon_term {
                changed = true;
                table.delete_row(row_id);
                let canon_row = encode(&canon_term);
                let new_dep = table.insert_row(&canon_row.0, &canon_row.1);
                if new_dep != &canon_row.1 {
                    let resident_term = decode(&canon_row.0, new_dep);
                    theory.solve(&canon_term, &resident_term);
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
