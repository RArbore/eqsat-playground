use core::borrow::{Borrow, BorrowMut};
use core::mem::take;
use core::ops::Index;
use core::ops::IndexMut;
use core::slice::SliceIndex;

use crate::arena::Arena;

pub struct ArenaVec<'a, T> {
    contents: &'a mut [T],
    len: usize,
}

impl<'a, T: Clone + Default> ArenaVec<'a, T> {
    pub fn new() -> Self {
        Self {
            contents: &mut [],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn push(&mut self, arena: &Arena<'a>, x: T) {
        if self.len < self.contents.len() {
            self.contents[self.len] = x;
        } else {
            let new_contents = arena.new_slice(
                if self.contents.is_empty() {
                    4
                } else {
                    self.contents.len() * 2
                },
                T::default(),
            );
            for i in 0..self.len {
                new_contents[i] = self.contents[i].clone();
            }
            self.contents = new_contents;
            self.contents[self.len] = x;
        }
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len > 0 {
            self.len -= 1;
            Some(take(&mut self.contents[self.len]))
        } else {
            None
        }
    }
}

impl<'a, T> AsRef<[T]> for ArenaVec<'a, T> {
    fn as_ref(&self) -> &[T] {
        self.borrow()
    }
}

impl<'a, T> AsMut<[T]> for ArenaVec<'a, T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.borrow_mut()
    }
}

impl<'a, T> Borrow<[T]> for ArenaVec<'a, T> {
    fn borrow(&self) -> &[T] {
        &self.contents[0..self.len]
    }
}

impl<'a, T> BorrowMut<[T]> for ArenaVec<'a, T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut self.contents[0..self.len]
    }
}

impl<'a, T, I: SliceIndex<[T]>> Index<I> for ArenaVec<'a, T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(<ArenaVec<'_, T> as Borrow<[T]>>::borrow(self), index)
    }
}

impl<'a, T, I: SliceIndex<[T]>> IndexMut<I> for ArenaVec<'a, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(<ArenaVec<'_, T> as BorrowMut<[T]>>::borrow_mut(self), index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_vec() {
        let mut buf: [u64; 64] = [0; 64];
        let arena = Arena::new_backed(&mut buf);
        let mut vec = ArenaVec::new();
        assert_eq!(vec.as_ref(), &[]);
        assert_eq!(vec.len(), 0);
        vec.push(&arena, 1);
        assert_eq!(vec.as_ref(), &[1]);
        assert_eq!(vec.len(), 1);
        vec.push(&arena, 42);
        assert_eq!(vec.as_ref(), &[1, 42]);
        assert_eq!(vec.len(), 2);
        vec.pop();
        assert_eq!(vec.as_ref(), &[1]);
        assert_eq!(vec.len(), 1);
        vec.push(&arena, 24);
        vec.push(&arena, 4);
        vec.push(&arena, 5);
        vec.push(&arena, 7);
        vec.push(&arena, 8);
        assert_eq!(vec.as_ref(), &[1, 24, 4, 5, 7, 8]);
        assert_eq!(vec.len(), 6);
    }
}
