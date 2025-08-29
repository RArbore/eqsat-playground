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
        let nroot1 = db.create_constant(5i32, root1);
        let root2 = db.uf.makeset();
        let nroot2 = db.create_constant(5i32, root2);
        assert_ne!(root1, root2);
        assert_eq!(db.uf.find(root1), db.uf.find(root2));
        assert_eq!(nroot1, nroot2);

        let root3 = db.uf.makeset();
        let nroot3 = db.create_constant(7i32, root3);
        assert_ne!(root1, root3);
        assert_ne!(db.uf.find(root1), db.uf.find(root3));
        assert_ne!(nroot1, nroot3);

        let root4 = db.uf.makeset();
        let nroot4 = db.create_add(nroot1, nroot3, root4);
        assert_ne!(db.uf.find(nroot1), db.uf.find(nroot4));
        assert_ne!(db.uf.find(nroot3), db.uf.find(nroot4));

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

        assert_eq!(db.create_add(a, b, fab), fab);
        assert_eq!(db.create_add(c, d, fcd), fcd);
        let na = db.create_constant(2i32, a);
        let nb = db.create_constant(3i32, b);
        let nc = db.create_constant(2i32, c);
        let nd = db.create_constant(3i32, d);
        assert_eq!(na, a);
        assert_eq!(nc, a);
        assert_ne!(nc, c);
        assert_eq!(nb, b);
        assert_eq!(nd, b);
        assert_ne!(nd, d);
        assert_ne!(db.uf.find(fab), db.uf.find(fcd));

        db.add.unwrap().rebuild();
        assert_eq!(db.uf.find(fab), db.uf.find(fcd));
    }
}
