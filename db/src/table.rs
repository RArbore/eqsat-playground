use std::collections::HashMap;

use util::interner::{IdentifierId, StringInterner};
use util::vec::VirtualVec;

const EMPTY: u32 = 0xFFFFFFFF;

pub struct Table<const DET_COLS: usize, const DEP_COLS: usize> {
    contents: VirtualVec<([u32; DET_COLS], [u32; DEP_COLS])>,
    determine_map: HashMap<&'static [u32; DET_COLS], &'static [u32; DEP_COLS]>,

    pub symbol: IdentifierId,

    pub num_allocated_rows: u32,
    pub num_free_rows: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowId(u32);

impl<const DET_COLS: usize, const DEP_COLS: usize> Table<DET_COLS, DEP_COLS> {
    pub fn new(symbol: IdentifierId) -> Self {
        const {
            assert!(DET_COLS > 0);
        }
        Self {
            contents: VirtualVec::new(),
            determine_map: HashMap::new(),

            symbol,

            num_allocated_rows: 0,
            num_free_rows: 0,
        }
    }

    pub fn insert_row(
        &mut self,
        determinant: &[u32; DET_COLS],
        dependent: &[u32; DEP_COLS],
    ) -> &[u32; DEP_COLS] {
        self.get_or_create_row(determinant, || *dependent)
    }

    pub fn get_or_create_row<F>(
        &mut self,
        determinant: &[u32; DET_COLS],
        dependent: F,
    ) -> &[u32; DEP_COLS]
    where
        F: FnOnce() -> [u32; DEP_COLS],
    {
        if let Some(dependent) = self.determine_map.get(determinant) {
            dependent
        } else {
            self.num_allocated_rows += 1;
            let idx = self.contents.len();
            self.contents.push((*determinant, dependent()));
            let row = unsafe { self.contents.static_ref(idx) };
            self.determine_map.insert(&row.0, &row.1);
            &row.1
        }
    }

    pub fn first_row(&self) -> Option<RowId> {
        for idx in 0..self.contents.len() {
            if self.contents[idx] != ([EMPTY; DET_COLS], [EMPTY; DEP_COLS]) {
                return Some(RowId(idx as u32));
            }
        }
        None
    }

    pub fn next_row(&self, row: RowId) -> Option<RowId> {
        for idx in (row.0 as usize + 1)..self.contents.len() {
            if self.contents[idx] != ([EMPTY; DET_COLS], [EMPTY; DEP_COLS]) {
                return Some(RowId(idx as u32));
            }
        }
        None
    }

    pub fn get_row(&self, row: RowId) -> ([u32; DET_COLS], [u32; DEP_COLS]) {
        self.contents[row.0 as usize]
    }

    pub fn delete_row(&mut self, row: RowId) -> bool {
        if self
            .determine_map
            .remove(&self.contents[row.0 as usize].0)
            .is_some()
        {
            self.num_allocated_rows -= 1;
            self.num_free_rows += 1;
            self.contents[row.0 as usize] = ([EMPTY; DET_COLS], [EMPTY; DEP_COLS]);
            true
        } else {
            false
        }
    }

    pub fn dump(&self, interner: &StringInterner) -> String {
        let mut s = String::new();
        let symbol = interner.get(self.symbol);
        let mut maybe_row_id = self.first_row();
        while let Some(row_id) = maybe_row_id {
            let row = self.get_row(row_id);
            s = format!("{}{}({:?}) -> {:?}\n", s, symbol, row.0, row.1);
            maybe_row_id = self.next_row(row_id);
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use util::arena::Arena;
    use util::interner::StringInterner;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn simple_table() {
        let mut buf: [u64; 1] = [0; 1];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);
        let mut table = Table::<2, 1>::new(interner.intern("blah"));
        assert_eq!(table.insert_row(&[0, 1], &[2]), &[2]);
        assert_eq!(table.insert_row(&[0, 2], &[3]), &[3]);
        assert_eq!(table.insert_row(&[0, 2], &[4]), &[3]);
        assert_eq!(table.insert_row(&[0, 1], &[5]), &[2]);
        assert_eq!(table.insert_row(&[1, 2], &[3]), &[3]);
        assert_eq!(table.num_allocated_rows, 3);
        assert_eq!(table.num_free_rows, 0);
        let id = table.first_row().unwrap();
        assert!(table.delete_row(id));
        assert!(!table.delete_row(id));
        assert_eq!(table.num_allocated_rows, 2);
        assert_eq!(table.num_free_rows, 1);
        assert_eq!(table.insert_row(&[0, 1], &[5]), &[5]);
        assert_eq!(table.insert_row(&[0, 1], &[7]), &[5]);
        assert_eq!(table.num_allocated_rows, 3);
        assert_eq!(table.num_free_rows, 1);
        let id = table.next_row(id).unwrap();
        assert!(table.delete_row(id));
        assert!(!table.delete_row(id));
        assert_eq!(table.num_allocated_rows, 2);
        assert_eq!(table.num_free_rows, 2);
        assert_eq!(table.insert_row(&[0, 2], &[7]), &[7]);
        assert_eq!(table.insert_row(&[0, 2], &[6]), &[7]);
        assert_eq!(table.num_allocated_rows, 3);
        assert_eq!(table.num_free_rows, 2);
    }
}
