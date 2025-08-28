use std::collections::HashMap;

use util::vec::VirtualVec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnType {
    EClass,
    Identifier,
    Constant,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Schema<const DET_COLS: usize, const DEP_COLS: usize> {
    pub determinant_columns: [ColumnType; DET_COLS],
    pub dependent_columns: [ColumnType; DEP_COLS],
}

pub struct Table<const DET_COLS: usize, const DEP_COLS: usize> {
    contents: VirtualVec<([u32; DET_COLS], [u32; DEP_COLS])>,
    determine_map: HashMap<&'static [u32; DET_COLS], &'static [u32; DEP_COLS]>,
}

impl<const DET_COLS: usize, const DEP_COLS: usize> Table<DET_COLS, DEP_COLS> {
    pub fn new() -> Self {
        Self {
            contents: VirtualVec::new(),
            determine_map: HashMap::new(),
        }
    }

    pub fn insert_row(&mut self, determinant: &[u32; DET_COLS], dependent: &[u32; DEP_COLS]) -> &[u32; DEP_COLS] {
        self.get_or_create_row(determinant, || *dependent)
    }

    pub fn get_or_create_row<F>(&mut self, determinant: &[u32; DET_COLS], dependent: F) -> &[u32; DEP_COLS]
        where F: FnOnce() -> [u32; DEP_COLS]
    {
        if let Some(dependent) = self.determine_map.get(determinant) {
            dependent
        } else {
            let idx = self.contents.len();
            self.contents.push((*determinant, dependent()));
            let row = unsafe { self.contents.static_ref(idx) };
            self.determine_map.insert(&row.0, &row.1);
            &row.1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_table() {
        let mut table = Table::<2, 1>::new();
        assert_eq!(table.insert_row(&[0, 1], &[2]), &[2]);
        assert_eq!(table.insert_row(&[0, 2], &[3]), &[3]);
        assert_eq!(table.insert_row(&[0, 2], &[4]), &[3]);
        assert_eq!(table.insert_row(&[0, 1], &[5]), &[2]);
        assert_eq!(table.insert_row(&[1, 2], &[3]), &[3]);
    }
}
