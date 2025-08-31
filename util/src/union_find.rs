use core::cell::Cell;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ClassId(u32);

impl ClassId {
    pub fn new(idx: u32) -> Self {
        Self(idx)
    }

    pub fn idx(&self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionFind {
    vec: Vec<Cell<ClassId>>,
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
        }
    }

    pub fn new_all_not_equals(amount: u32) -> Self {
        Self {
            vec: (0..amount).map(|idx| Cell::new(ClassId(idx))).collect()
        }
    }

    pub fn new_all_equals(amount: u32) -> Self {
        Self {
            vec: vec![Cell::new(ClassId(0)); amount as usize],
        }
    }

    pub fn makeset(&mut self) -> ClassId {
        let len = self.vec.len();
        let id = ClassId(len.try_into().unwrap());
        self.vec.push(Cell::new(id));
        id
    }

    pub fn num_classes(&self) -> u32 {
        self.vec.len().try_into().unwrap()
    }

    pub fn find(&self, mut id: ClassId) -> ClassId {
        while id != self.parent(id) {
            self.set_parent(id, self.parent(self.parent(id)));
            id = self.parent(id);
        }
        id
    }

    #[inline]
    fn parent(&self, id: ClassId) -> ClassId {
        self.vec[id.0 as usize].get()
    }

    #[inline]
    fn set_parent(&self, id: ClassId, parent: ClassId) {
        self.vec[id.0 as usize].set(parent);
    }

    pub fn merge(&self, mut x: ClassId, mut y: ClassId) -> ClassId {
        while self.parent(x) != self.parent(y) {
            if self.parent(x) > self.parent(y) {
                if x == self.parent(x) {
                    self.set_parent(x, self.parent(y));
                    break;
                }
                let z = self.parent(x);
                self.set_parent(x, self.parent(y));
                x = z;
            } else {
                if y == self.parent(y) {
                    self.set_parent(y, self.parent(x));
                    break;
                }
                let z = self.parent(y);
                self.set_parent(y, self.parent(x));
                y = z;
            }
        }
        self.parent(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn simple_uf() {
        let mut uf = UnionFind::new();
        let x = uf.makeset();
        let y = uf.makeset();
        let z = uf.makeset();
        assert_ne!(x, y);
        assert_ne!(y, z);
        assert_ne!(z, x);
        assert_eq!(uf.find(x), x);
        assert_eq!(uf.find(y), y);
        assert_eq!(uf.find(z), z);
        assert_eq!(uf.merge(x, y), x);
        assert_eq!(uf.find(x), uf.find(y));
        assert_ne!(uf.find(x), uf.find(z));
        assert_eq!(uf.merge(x, z), x);
        assert_eq!(uf.find(x), uf.find(z));
        assert_eq!(uf.find(y), uf.find(z));
        assert_eq!(uf.find(y), uf.find(x));
    }

    #[test]
    #[cfg_attr(miri, ignore)]
    fn complex_uf() {
        let mut uf = UnionFind::new();
        let mut ids = vec![];
        for _ in 0..1000 {
            ids.push(uf.makeset());
        }
        for i in 0..999 {
            assert_ne!(uf.find(ids[i]), uf.find(ids[i + 1]));
        }
        for i in 0..500 {
            assert_eq!(uf.merge(ids[2 * i], ids[2 * i + 1]), ids[2 * i]);
        }
        for i in 0..500 {
            assert_eq!(uf.find(ids[2 * i]), uf.find(ids[2 * i + 1]));
            if i < 499 {
                assert_ne!(uf.find(ids[2 * i]), uf.find(ids[2 * i + 2]));
            }
        }
        for i in 0..499 {
            assert_eq!(uf.merge(ids[2 * i], ids[2 * i + 2]), ids[0]);
        }
        for i in 0..999 {
            assert_eq!(uf.find(ids[i]), uf.find(ids[999]));
        }
    }
}
