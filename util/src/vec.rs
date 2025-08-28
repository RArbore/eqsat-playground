use core::borrow::{Borrow, BorrowMut};
use core::cell::Cell;
use core::fmt::{Debug, Error, Formatter};
use core::marker::PhantomData;
use core::mem::{align_of, needs_drop, size_of, take};
use core::ops::Index;
use core::ops::IndexMut;
use core::slice::{SliceIndex, from_raw_parts, from_raw_parts_mut};
use core::sync::atomic::Ordering;

use crate::arena::{Arena, ArenaInternal, MCOMMIT_GRANULARITY};

pub struct ArenaVec<'a, T> {
    contents: &'a mut [T],
    len: usize,
}

impl<'a, T> ArenaVec<'a, T> {
    pub fn new() -> Self {
        Self {
            contents: &mut [],
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a, T: Default> ArenaVec<'a, T> {
    pub fn push(&mut self, arena: &Arena<'a>, x: T) {
        if self.len < self.contents.len() {
            self.contents[self.len] = x;
        } else {
            let new_contents = arena.new_slice(if self.contents.is_empty() {
                4
            } else {
                self.contents.len() * 2
            });
            for i in 0..self.len {
                new_contents[i] = take(&mut self.contents[i]);
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

impl<'a, T: Debug> Debug for ArenaVec<'a, T> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), Error> {
        write!(fmt, "[")?;
        if !self.is_empty() {
            write!(fmt, "{:?}", self[0])?;
            for x in &self[1..] {
                write!(fmt, ", {:?}", x)?;
            }
        }
        write!(fmt, "]")
    }
}

impl<'a, T> Default for ArenaVec<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct VirtualVec<T> {
    arena: ArenaInternal<'static>,
    len: Cell<usize>,
    _phantom: PhantomData<T>,
}

impl<T> VirtualVec<T> {
    pub fn new() -> Self {
        const {
            assert!(!needs_drop::<T>());
        }
        let arena = ArenaInternal::new_virt(align_of::<T>());
        unsafe { arena.alloc_assume_aligned(MCOMMIT_GRANULARITY) };
        Self {
            arena,
            len: Cell::new(0),
            _phantom: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.len.get()
    }

    pub fn is_empty(&self) -> bool {
        self.len.get() == 0
    }

    pub fn push(&self, x: T) {
        let old_len = self.len.get();
        unsafe {
            assert!(old_len * size_of::<T>() <= self.arena.offset.load(Ordering::Relaxed));
            if old_len * size_of::<T>() == self.arena.offset.load(Ordering::Relaxed) {
                self.arena
                    .alloc_assume_aligned(self.arena.offset.load(Ordering::Relaxed));
            }
            (self.arena.ptr as *mut T).add(old_len).write(x);
        }
        self.len.set(old_len + 1);
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let new_len = self.len.get() - 1;
            let x = unsafe { (self.arena.ptr as *const T).add(new_len).read() };
            self.len.set(new_len);
            Some(x)
        }
    }

    pub unsafe fn static_ref(&self, idx: usize) -> &'static T {
        unsafe { &*(self.arena.ptr as *const T).add(idx) }
    }
}

impl<T> AsRef<[T]> for VirtualVec<T> {
    fn as_ref(&self) -> &[T] {
        self.borrow()
    }
}

impl<T> AsMut<[T]> for VirtualVec<T> {
    fn as_mut(&mut self) -> &mut [T] {
        self.borrow_mut()
    }
}

impl<T> Borrow<[T]> for VirtualVec<T> {
    fn borrow(&self) -> &[T] {
        unsafe { from_raw_parts(self.arena.ptr as *const T, self.len.get()) }
    }
}

impl<T> BorrowMut<[T]> for VirtualVec<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        unsafe { from_raw_parts_mut(self.arena.ptr as *mut T, self.len.get()) }
    }
}

impl<T, I: SliceIndex<[T]>> Index<I> for VirtualVec<T> {
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        Index::index(<VirtualVec<T> as Borrow<[T]>>::borrow(self), index)
    }
}

impl<T, I: SliceIndex<[T]>> IndexMut<I> for VirtualVec<T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        IndexMut::index_mut(<VirtualVec<T> as BorrowMut<[T]>>::borrow_mut(self), index)
    }
}

impl<T> Default for VirtualVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_vec() {
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

    #[test]
    #[cfg_attr(miri, ignore)]
    fn virtual_vec() {
        let mut vec = VirtualVec::new();
        assert_eq!(vec.as_ref(), &[]);
        assert_eq!(vec.len(), 0);
        vec.push(1);
        assert_eq!(vec.as_ref(), &[1]);
        assert_eq!(vec.len(), 1);
        vec.push(42);
        assert_eq!(vec.as_ref(), &[1, 42]);
        assert_eq!(vec.len(), 2);
        vec.pop();
        assert_eq!(vec.as_ref(), &[1]);
        assert_eq!(vec.len(), 1);
        vec.push(24);
        vec.push(4);
        vec.push(5);
        vec.push(7);
        vec.push(8);
        assert_eq!(vec.as_ref(), &[1, 24, 4, 5, 7, 8]);
        assert_eq!(vec.len(), 6);
    }
}
