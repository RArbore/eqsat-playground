use core::mem::transmute;

use db::table::Table;
use db::theory::{Theory, rebuild_table};
use util::interner::StringInterner;
use util::union_find::{ClassId, UnionFind};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Term {
    Constant {
        value: i32,
        root: ClassId,
    },
    Param {
        index: u32,
        root: ClassId,
    },
    Phi {
        lhs: ClassId,
        rhs: ClassId,
        root: ClassId,
    },
    Add {
        lhs: ClassId,
        rhs: ClassId,
        root: ClassId,
    },
}

fn constant_encode(term: &Term) -> ([u32; 1], [u32; 1]) {
    let Term::Constant { value, root } = term else {
        panic!()
    };
    unsafe { transmute(([*value], [*root])) }
}

fn param_encode(term: &Term) -> ([u32; 1], [u32; 1]) {
    let Term::Param { index, root } = term else {
        panic!()
    };
    unsafe { transmute(([*index], [*root])) }
}

fn phi_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::Phi { lhs, rhs, root } = term else {
        panic!()
    };
    unsafe { transmute(([*lhs, *rhs], [*root])) }
}

fn add_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::Add { lhs, rhs, root } = term else {
        panic!()
    };
    unsafe { transmute(([*lhs, *rhs], [*root])) }
}

fn constant_decode(det: &[u32; 1], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Constant {
            #[allow(unnecessary_transmutes)]
            value: transmute(det[0]),
            root: transmute(dep[0]),
        }
    }
}

fn param_decode(det: &[u32; 1], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Param {
            index: transmute(det[0]),
            root: transmute(dep[0]),
        }
    }
}

fn phi_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Phi {
            lhs: transmute(det[0]),
            rhs: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}

fn add_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Add {
            lhs: transmute(det[0]),
            rhs: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}

pub struct Graph {
    constant: Table<1, 1>,
    param: Table<1, 1>,
    phi: Table<2, 1>,
    add: Table<2, 1>,
    uf_theory: UFTheory,
}

impl Graph {
    pub fn new(interner: &mut StringInterner) -> Self {
        Self {
            constant: Table::new(interner.intern("cons")),
            param: Table::new(interner.intern("param")),
            phi: Table::new(interner.intern("ϕ")),
            add: Table::new(interner.intern("+")),
            uf_theory: UFTheory {
                uf: UnionFind::new(),
            },
        }
    }

    pub fn insert(&mut self, term: Term) -> Term {
        match &term {
            Term::Constant { .. } => {
                let (det, dep) = constant_encode(&term);
                let new_dep = self.constant.insert_row(&det, &dep).clone();
                if new_dep != dep {
                    unsafe { self.merge(transmute(new_dep[0]), transmute(dep[0])) };
                }
                constant_decode(&det, &new_dep)
            }
            Term::Param { .. } => {
                let (det, dep) = param_encode(&term);
                let new_dep = self.param.insert_row(&det, &dep).clone();
                if new_dep != dep {
                    unsafe { self.merge(transmute(new_dep[0]), transmute(dep[0])) };
                }
                param_decode(&det, &new_dep)
            }
            Term::Phi { .. } => {
                let (det, dep) = phi_encode(&term);
                let new_dep = self.phi.insert_row(&det, &dep).clone();
                if new_dep != dep {
                    unsafe { self.merge(transmute(new_dep[0]), transmute(dep[0])) };
                }
                phi_decode(&det, &new_dep)
            }
            Term::Add { .. } => {
                let (det, dep) = add_encode(&term);
                let new_dep = self.add.insert_row(&det, &dep).clone();
                if new_dep != dep {
                    unsafe { self.merge(transmute(new_dep[0]), transmute(dep[0])) };
                }
                add_decode(&det, &new_dep)
            }
        }
    }

    pub fn makeset(&mut self) -> ClassId {
        self.uf_theory.uf.makeset()
    }

    pub fn find(&self, id: ClassId) -> ClassId {
        self.uf_theory.uf.find(id)
    }

    pub fn merge(&self, a: ClassId, b: ClassId) -> ClassId {
        self.uf_theory.uf.merge(a, b)
    }

    pub fn rebuild(&mut self) {
        loop {
            let mut changed = false;

            changed = rebuild_table(
                &mut self.constant,
                &mut self.uf_theory,
                constant_encode,
                constant_decode,
            ) || changed;
            changed = rebuild_table(
                &mut self.param,
                &mut self.uf_theory,
                param_encode,
                param_decode,
            ) || changed;
            changed = rebuild_table(&mut self.phi, &mut self.uf_theory, phi_encode, phi_decode)
                || changed;
            changed = rebuild_table(&mut self.add, &mut self.uf_theory, add_encode, add_decode)
                || changed;

            if !changed {
                break;
            }
        }
    }

    pub fn dump(&self, interner: &StringInterner) -> String {
        format!(
            "{}{}{}{}",
            self.constant.dump(interner),
            self.param.dump(interner),
            self.phi.dump(interner),
            self.add.dump(interner)
        )
    }
}

struct UFTheory {
    uf: UnionFind,
}

impl Theory for UFTheory {
    type Term = Term;

    fn canonicalize(&mut self, term: &Self::Term) -> Self::Term {
        match term {
            Term::Constant { value, root } => Term::Constant {
                value: *value,
                root: self.uf.find(*root),
            },
            Term::Param { index, root } => Term::Param {
                index: *index,
                root: self.uf.find(*root),
            },
            Term::Phi { lhs, rhs, root } => Term::Phi {
                lhs: self.uf.find(*lhs),
                rhs: self.uf.find(*rhs),
                root: self.uf.find(*root),
            },
            Term::Add { lhs, rhs, root } => Term::Add {
                lhs: self.uf.find(*lhs),
                rhs: self.uf.find(*rhs),
                root: self.uf.find(*root),
            },
        }
    }
    fn solve(&mut self, lhs: &Self::Term, rhs: &Self::Term) {
        let lhs_root = match lhs {
            Term::Constant { root, .. } => *root,
            Term::Param { root, .. } => *root,
            Term::Phi { root, .. } => *root,
            Term::Add { root, .. } => *root,
        };
        let rhs_root = match rhs {
            Term::Constant { root, .. } => *root,
            Term::Param { root, .. } => *root,
            Term::Phi { root, .. } => *root,
            Term::Add { root, .. } => *root,
        };
        self.uf.merge(lhs_root, rhs_root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use util::arena::Arena;
    use util::interner::StringInterner;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn hash_cons() {
        let mut buf: [u64; 100] = [0; 100];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);

        let mut db = Graph::new(&mut interner);
        let root1 = db.makeset();
        let cons1 = db.insert(Term::Constant {
            value: 5i32,
            root: root1,
        });
        let root2 = db.makeset();
        let cons2 = db.insert(Term::Constant {
            value: 5i32,
            root: root2,
        });
        assert_ne!(root1, root2);
        assert_eq!(db.find(root1), db.find(root2));
        assert_eq!(
            cons1,
            Term::Constant {
                value: 5i32,
                root: root1
            }
        );
        assert_eq!(
            cons2,
            Term::Constant {
                value: 5i32,
                root: root1
            }
        );

        let root3 = db.makeset();
        let cons3 = db.insert(Term::Constant {
            value: 7i32,
            root: root3,
        });
        assert_ne!(root1, root3);
        assert_ne!(db.find(root1), db.find(root3));
        assert_eq!(
            cons3,
            Term::Constant {
                value: 7i32,
                root: root3
            }
        );

        let root4 = db.makeset();
        let add4 = db.insert(Term::Add {
            lhs: root1,
            rhs: root3,
            root: root4,
        });
        assert_eq!(
            add4,
            Term::Add {
                lhs: root1,
                rhs: root3,
                root: root4
            }
        );

        assert_eq!(db.add.num_allocated_rows, 1);
        assert_eq!(db.constant.num_allocated_rows, 2);
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn rebuild() {
        let mut buf: [u64; 100] = [0; 100];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);

        let mut db = Graph::new(&mut interner);
        let a = db.makeset();
        let b = db.makeset();
        let c = db.makeset();
        let d = db.makeset();
        let fab = db.makeset();
        let fcd = db.makeset();
        assert_ne!(a, c);
        assert_ne!(b, d);
        assert_ne!(fab, fcd);

        db.insert(Term::Add {
            lhs: a,
            rhs: b,
            root: fab,
        });
        db.insert(Term::Add {
            lhs: c,
            rhs: d,
            root: fcd,
        });
        let na = db.insert(Term::Constant {
            value: 2i32,
            root: a,
        });
        let nb = db.insert(Term::Constant {
            value: 3i32,
            root: b,
        });
        let nc = db.insert(Term::Constant {
            value: 2i32,
            root: c,
        });
        let nd = db.insert(Term::Constant {
            value: 3i32,
            root: d,
        });
        assert_eq!(
            na,
            Term::Constant {
                value: 2i32,
                root: a
            }
        );
        assert_eq!(
            nc,
            Term::Constant {
                value: 2i32,
                root: a
            }
        );
        assert_eq!(
            nb,
            Term::Constant {
                value: 3i32,
                root: b
            }
        );
        assert_eq!(
            nd,
            Term::Constant {
                value: 3i32,
                root: b
            }
        );
        assert_ne!(db.find(fab), db.find(fcd));

        db.rebuild();
        assert_eq!(db.find(fab), db.find(fcd));
    }
}
