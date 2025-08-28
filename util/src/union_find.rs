use crate::vec::{VirtualVec};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClassId(u32);

pub struct UnionFind {
    vec: VirtualVec<ClassId>,
}

impl UnionFind {
    pub fn new() -> Self {
        Self {
            vec: VirtualVec::new(),
        }
    }

    pub fn makeset(&self) -> ClassId {
        let len = self.vec.len();
        let id = ClassId(len.try_into().unwrap());
        self.vec.push(id);
        id
    }

    pub fn find(&self, mut id: ClassId) -> ClassId {
        loop {
            let parent = self.vec[id.0 as usize];
            if parent == id {
                return id;
            } else {
                id = parent;
            }
        }
    }

    pub fn merge(&mut self, a: ClassId, b: ClassId) -> ClassId {
        let fa = self.find(a);
        let fb = self.find(b);
        self.vec[fa.0 as usize] = fb;
        fb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore)]
    fn union_find() {
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
        uf.merge(x, y);
        assert_eq!(uf.find(x), uf.find(y));
        assert_ne!(uf.find(x), uf.find(z));
        uf.merge(x, z);
        assert_eq!(uf.find(x), uf.find(z));
        assert_eq!(uf.find(y), uf.find(z));
        assert_eq!(uf.find(y), uf.find(x));
    }
}
