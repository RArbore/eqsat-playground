use db_proc::define_database;

define_database!("imp/src/ir.toml");

#[cfg(test)]
mod tests {
    use super::*;

    use util::arena::Arena;
    use util::interner::StringInterner;

    #[test]
    fn hash_cons() {
        let mut buf: [u64; 100] = [0; 100];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);

        let mut db = Database::new(&mut interner);
        let root1 = db.uf.makeset();
        let cons1 = db.insert(Row::Constant { value: 5i32, root: root1 });
        let root2 = db.uf.makeset();
        let cons2 = db.insert(Row::Constant { value: 5i32, root: root2 });
        assert_ne!(root1, root2);
        assert_eq!(db.uf.find(root1), db.uf.find(root2));
        assert_eq!(cons1, Row::Constant { value: 5i32, root: root1 });
        assert_eq!(cons2, Row::Constant { value: 5i32, root: root1 });

        let root3 = db.uf.makeset();
        let cons3 = db.insert(Row::Constant { value: 7i32, root: root3 });
        assert_ne!(root1, root3);
        assert_ne!(db.uf.find(root1), db.uf.find(root3));
        assert_eq!(cons3, Row::Constant { value: 7i32, root: root3 });

        let root4 = db.uf.makeset();
        let add4 = db.insert(Row::Add { lhs: root1, rhs: root3, root: root4 });
        assert_eq!(add4, Row::Add { lhs: root1, rhs: root3, root: root4 });

        assert_eq!(db.add.unwrap().num_allocated_rows, 1);
        assert_eq!(db.constant.unwrap().num_allocated_rows, 2);
    }

    #[test]
    fn rebuild() {
        let mut buf: [u64; 100] = [0; 100];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);

        let mut db = Database::new(&mut interner);
        let a = db.uf.makeset();
        let b = db.uf.makeset();
        let c = db.uf.makeset();
        let d = db.uf.makeset();
        let fab = db.uf.makeset();
        let fcd = db.uf.makeset();
        assert_ne!(a, c);
        assert_ne!(b, d);
        assert_ne!(fab, fcd);

        db.insert(Row::Add { lhs: a, rhs: b, root: fab });
        db.insert(Row::Add { lhs: c, rhs: d, root: fcd });
        let na = db.insert(Row::Constant { value: 2i32, root: a });
        let nb = db.insert(Row::Constant { value: 3i32, root: b });
        let nc = db.insert(Row::Constant { value: 2i32, root: c });
        let nd = db.insert(Row::Constant { value: 3i32, root: d });
        assert_eq!(na, Row::Constant { value: 2i32, root: a });
        assert_eq!(nc, Row::Constant { value: 2i32, root: a });
        assert_eq!(nb, Row::Constant { value: 3i32, root: b });
        assert_eq!(nd, Row::Constant { value: 3i32, root: b });
        assert_ne!(db.uf.find(fab), db.uf.find(fcd));

        //db.add.unwrap().rebuild();
        //assert_eq!(db.uf.find(fab), db.uf.find(fcd));
    }
}
