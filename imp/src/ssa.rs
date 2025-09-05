use core::mem::transmute;

use db::rebuild::{ENode, corebuild, rebuild_enode_table};
use db::table::Table;
use util::interner::StringInterner;
use util::union_find::{ClassId, UnionFind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Term {
    Constant {
        value: i32,
        root: ClassId,
    },
    Param {
        index: u32,
        root: ClassId,
    },

    Start {
        root: ClassId,
    },
    Region {
        lhs: ClassId,
        rhs: ClassId,
        root: ClassId,
    },
    Branch {
        pred: ClassId,
        cond: ClassId,
        root: ClassId,
    },
    ControlProj {
        pred: ClassId,
        index: u32,
        root: ClassId,
    },
    Finish {
        pred: ClassId,
        value: ClassId,
        root: ClassId,
    },

    Phi {
        region: ClassId,
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

impl ENode for Term {
    fn root(&self) -> ClassId {
        match self {
            Term::Constant { root, .. } => *root,
            Term::Param { root, .. } => *root,
            Term::Start { root, .. } => *root,
            Term::Region { root, .. } => *root,
            Term::Branch { root, .. } => *root,
            Term::ControlProj { root, .. } => *root,
            Term::Finish { root, .. } => *root,
            Term::Phi { root, .. } => *root,
            Term::Add { root, .. } => *root,
        }
    }

    fn canonicalize(&self, uf: &mut UnionFind) -> Self {
        match self {
            Term::Constant { value, root } => Term::Constant {
                value: *value,
                root: uf.find(*root),
            },
            Term::Param { index, root } => Term::Param {
                index: *index,
                root: uf.find(*root),
            },
            Term::Start { root } => Term::Start {
                root: uf.find(*root),
            },
            Term::Region { lhs, rhs, root } => Term::Region {
                lhs: uf.find(*lhs),
                rhs: uf.find(*rhs),
                root: uf.find(*root),
            },
            Term::Branch { pred, cond, root } => Term::Branch {
                pred: uf.find(*pred),
                cond: uf.find(*cond),
                root: uf.find(*root),
            },
            Term::ControlProj { pred, index, root } => Term::ControlProj {
                pred: uf.find(*pred),
                index: *index,
                root: uf.find(*root),
            },
            Term::Finish { pred, value, root } => Term::Finish {
                pred: uf.find(*pred),
                value: uf.find(*value),
                root: uf.find(*root),
            },
            Term::Phi {
                region,
                lhs,
                rhs,
                root,
            } => Term::Phi {
                region: uf.find(*region),
                lhs: uf.find(*lhs),
                rhs: uf.find(*rhs),
                root: uf.find(*root),
            },
            Term::Add { lhs, rhs, root } => Term::Add {
                lhs: uf.find(*lhs),
                rhs: uf.find(*rhs),
                root: uf.find(*root),
            },
        }
    }
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

fn start_encode(term: &Term) -> ([u32; 0], [u32; 1]) {
    let Term::Start { root } = term else { panic!() };
    unsafe { transmute(*root) }
}

fn region_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::Region { lhs, rhs, root } = term else {
        panic!()
    };
    unsafe { transmute(([*lhs, *rhs], [*root])) }
}

fn branch_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::Branch { pred, cond, root } = term else {
        panic!()
    };
    unsafe { transmute(([*pred, *cond], [*root])) }
}

fn control_proj_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::ControlProj { pred, index, root } = term else {
        panic!()
    };
    unsafe { transmute(([*pred, transmute(*index)], [*root])) }
}

fn finish_encode(term: &Term) -> ([u32; 2], [u32; 1]) {
    let Term::Finish { pred, value, root } = term else {
        panic!()
    };
    unsafe { transmute(([*pred, *value], [*root])) }
}

fn phi_encode(term: &Term) -> ([u32; 3], [u32; 1]) {
    let Term::Phi {
        region,
        lhs,
        rhs,
        root,
    } = term
    else {
        panic!()
    };
    unsafe { transmute(([*region, *lhs, *rhs], [*root])) }
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

fn start_decode(_det: &[u32; 0], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Start {
            root: transmute(dep[0]),
        }
    }
}

fn region_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Region {
            lhs: transmute(det[0]),
            rhs: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}

fn branch_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Branch {
            pred: transmute(det[0]),
            cond: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}

fn control_proj_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::ControlProj {
            pred: transmute(det[0]),
            index: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}
fn finish_decode(det: &[u32; 2], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Finish {
            pred: transmute(det[0]),
            value: transmute(det[1]),
            root: transmute(dep[0]),
        }
    }
}

fn phi_decode(det: &[u32; 3], dep: &[u32; 1]) -> Term {
    unsafe {
        Term::Phi {
            region: transmute(det[0]),
            lhs: transmute(det[1]),
            rhs: transmute(det[2]),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval {
    value: ClassId,
    low: i32,
    high: i32,
}

fn interval_encode(interval: &Interval) -> ([u32; 1], [u32; 2]) {
    unsafe { transmute(*interval) }
}

fn interval_decode(det: &[u32; 1], dep: &[u32; 2]) -> Interval {
    #[allow(unnecessary_transmutes)]
    unsafe {
        Interval {
            value: transmute(det[0]),
            low: transmute(dep[0]),
            high: transmute(dep[1]),
        }
    }
}

pub struct Graph {
    constant: Table<1, 1>,
    param: Table<1, 1>,
    start: Table<0, 1>,
    region: Table<2, 1>,
    branch: Table<2, 1>,
    control_proj: Table<2, 1>,
    finish: Table<2, 1>,
    phi: Table<3, 1>,
    add: Table<2, 1>,

    interval: Table<1, 2>,

    uf: UnionFind,
}

impl Graph {
    pub fn new(interner: &mut StringInterner) -> Self {
        Self {
            constant: Table::new(interner.intern("cons")),
            param: Table::new(interner.intern("param")),
            start: Table::new(interner.intern("start")),
            region: Table::new(interner.intern("region")),
            branch: Table::new(interner.intern("branch")),
            control_proj: Table::new(interner.intern("π")),
            finish: Table::new(interner.intern("finish")),
            phi: Table::new(interner.intern("ϕ")),
            add: Table::new(interner.intern("+")),

            interval: Table::new(interner.intern("[]")),

            uf: UnionFind::new(),
        }
    }

    pub fn insert(&mut self, term: Term) -> Term {
        match &term {
            Term::Constant { .. } => {
                let (det, dep) = constant_encode(&term);
                let new_dep = self.constant.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                constant_decode(&det, &new_dep)
            }
            Term::Param { .. } => {
                let (det, dep) = param_encode(&term);
                let new_dep = self.param.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                param_decode(&det, &new_dep)
            }
            Term::Start { .. } => {
                let (det, dep) = start_encode(&term);
                let new_dep = self.start.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                start_decode(&det, &new_dep)
            }
            Term::Region { .. } => {
                let (det, dep) = region_encode(&term);
                let new_dep = self.region.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                region_decode(&det, &new_dep)
            }
            Term::Branch { .. } => {
                let (det, dep) = branch_encode(&term);
                let new_dep = self.branch.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                branch_decode(&det, &new_dep)
            }
            Term::ControlProj { .. } => {
                let (det, dep) = control_proj_encode(&term);
                let new_dep = self.control_proj.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                control_proj_decode(&det, &new_dep)
            }
            Term::Finish { .. } => {
                let (det, dep) = finish_encode(&term);
                let new_dep = self.finish.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                finish_decode(&det, &new_dep)
            }
            Term::Phi { .. } => {
                let (det, dep) = phi_encode(&term);
                let new_dep = self.phi.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                phi_decode(&det, &new_dep)
            }
            Term::Add { .. } => {
                let (det, dep) = add_encode(&term);
                let new_dep = self.add.insert_row(&det, &dep, |new_dep, old_dep| {
                    if new_dep != old_dep {
                        unsafe { self.uf.merge(transmute(new_dep[0]), transmute(old_dep[0])) };
                    }
                    [old_dep[0]]
                }).clone();
                add_decode(&det, &new_dep)
            }
        }
    }

    pub fn makeset(&mut self) -> ClassId {
        self.uf.makeset()
    }

    pub fn find(&self, id: ClassId) -> ClassId {
        self.uf.find(id)
    }

    pub fn merge(&self, a: ClassId, b: ClassId) -> ClassId {
        self.uf.merge(a, b)
    }

    pub fn terms(&self) -> impl Iterator<Item = Term> + '_ {
        self.constant
            .iter()
            .map(|row| constant_decode(&row.0, &row.1))
            .chain(self.param.iter().map(|row| param_decode(&row.0, &row.1)))
            .chain(self.start.iter().map(|row| start_decode(&row.0, &row.1)))
            .chain(self.region.iter().map(|row| region_decode(&row.0, &row.1)))
            .chain(self.branch.iter().map(|row| branch_decode(&row.0, &row.1)))
            .chain(
                self.control_proj
                    .iter()
                    .map(|row| control_proj_decode(&row.0, &row.1)),
            )
            .chain(self.finish.iter().map(|row| finish_decode(&row.0, &row.1)))
            .chain(self.phi.iter().map(|row| phi_decode(&row.0, &row.1)))
            .chain(self.add.iter().map(|row| add_decode(&row.0, &row.1)))
    }

    pub fn rebuild(&mut self) {
        loop {
            let mut changed = false;

            corebuild(self.terms().collect(), &mut self.uf);

            changed = rebuild_enode_table(
                &mut self.constant,
                &mut self.uf,
                constant_encode,
                constant_decode,
            ) || changed;
            changed =
                rebuild_enode_table(&mut self.param, &mut self.uf, param_encode, param_decode) || changed;
            changed =
                rebuild_enode_table(&mut self.start, &mut self.uf, start_encode, start_decode) || changed;
            changed = rebuild_enode_table(&mut self.region, &mut self.uf, region_encode, region_decode)
                || changed;
            changed = rebuild_enode_table(&mut self.branch, &mut self.uf, branch_encode, branch_decode)
                || changed;
            changed = rebuild_enode_table(
                &mut self.control_proj,
                &mut self.uf,
                control_proj_encode,
                control_proj_decode,
            ) || changed;
            changed = rebuild_enode_table(&mut self.finish, &mut self.uf, finish_encode, finish_decode)
                || changed;
            changed = rebuild_enode_table(&mut self.phi, &mut self.uf, phi_encode, phi_decode) || changed;
            changed = rebuild_enode_table(&mut self.add, &mut self.uf, add_encode, add_decode) || changed;

            if !changed {
                break;
            }
        }
    }

    pub fn dump(&self, interner: &StringInterner) -> String {
        format!(
            "{}{}{}{}{}{}{}{}{}",
            self.constant.dump(interner),
            self.param.dump(interner),
            self.start.dump(interner),
            self.region.dump(interner),
            self.branch.dump(interner),
            self.control_proj.dump(interner),
            self.finish.dump(interner),
            self.phi.dump(interner),
            self.add.dump(interner)
        )
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
