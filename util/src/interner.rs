use core::hash::Hash;
use std::collections::HashMap;

use crate::arena::{Arena, BrandedArenaId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IdentifierId(u32);

impl IdentifierId {
    pub fn idx(&self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug)]
pub struct StringInterner<'a, 'b> {
    str_to_id: HashMap<&'a str, IdentifierId>,
    id_to_str: Vec<&'a str>,
    arena: &'b Arena<'a>,
}

impl<'a, 'b> StringInterner<'a, 'b> {
    pub fn new(arena: &'b Arena<'a>) -> Self {
        Self {
            str_to_id: HashMap::new(),
            id_to_str: vec![],
            arena,
        }
    }

    pub fn intern(&mut self, string: &str) -> IdentifierId {
        if let Some(id) = self.str_to_id.get(string) {
            *id
        } else {
            let in_arena = self.arena.new_ref(string);
            let id = IdentifierId(self.id_to_str.len().try_into().unwrap());
            self.str_to_id.insert(in_arena, id);
            self.id_to_str.push(in_arena);
            id
        }
    }

    pub fn get(&self, id: IdentifierId) -> &'a str {
        self.id_to_str[id.0 as usize]
    }

    pub fn num_idens(&self) -> usize {
        self.id_to_str.len()
    }
}

#[derive(Debug)]
pub struct Interner<'a, 'b, T> {
    obj_to_id: HashMap<&'a T, BrandedArenaId<T>>,
    id_to_obj: Vec<&'a T>,
    arena: &'b Arena<'a>,
}

impl<'a, 'b, T: Eq + Hash> Interner<'a, 'b, T> {
    pub fn new(arena: &'b Arena<'a>) -> Self {
        Self {
            obj_to_id: HashMap::new(),
            id_to_obj: vec![],
            arena,
        }
    }

    pub fn intern(&mut self, obj: T) -> BrandedArenaId<T> {
        if let Some(id) = self.obj_to_id.get(&obj) {
            *id
        } else {
            let in_arena = self.arena.alloc(obj);
            let arena_ref = self.arena.get(in_arena);
            self.obj_to_id.insert(arena_ref, in_arena);
            self.id_to_obj.push(arena_ref);
            in_arena
        }
    }

    pub fn get(&self, id: BrandedArenaId<T>) -> &'a T {
        self.arena.get(id)
    }

    pub fn num_objs(&self) -> usize {
        self.id_to_obj.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_strings() {
        let mut buf: [u64; 4] = [0; 4];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = StringInterner::new(&arena);
        let str1 = "short";
        let str2 = "quite a long string";
        let id1 = interner.intern(str1);
        let id2 = interner.intern(str2);
        assert_ne!(id1, id2);
        let id3 = interner.intern(str1);
        let id4 = interner.intern(str2);
        assert_ne!(id3, id4);
        assert_eq!(id1, id3);
        assert_eq!(id2, id4);
        assert_eq!(interner.get(id1), str1);
        assert_eq!(interner.get(id2), str2);
        assert_eq!(interner.get(id3), str1);
        assert_eq!(interner.get(id4), str2);
    }

    #[test]
    fn intern_objs() {
        let mut buf: [u64; 4] = [0; 4];
        let arena = Arena::new_backed(&mut buf);
        let mut interner = Interner::<(i32, i32)>::new(&arena);
        let id1 = interner.intern((0, 1));
        let id2 = interner.intern((2, 3));
        assert_ne!(id1, id2);
        let id3 = interner.intern((0, 1));
        let id4 = interner.intern((2, 3));
        assert_ne!(id3, id4);
        assert_eq!(id1, id3);
        assert_eq!(id2, id4);
        assert_eq!(*interner.get(id1), (0, 1));
        assert_eq!(*interner.get(id2), (2, 3));
        assert_eq!(*interner.get(id3), (0, 1));
        assert_eq!(*interner.get(id4), (2, 3));
    }
}
