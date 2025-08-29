use db_proc::define_database;

define_database!("imp/src/ir.toml");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_cons() {
        let mut db = Database::new();
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
}
