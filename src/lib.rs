use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    marker::PhantomData,
    mem::{forget, size_of},
    ops::{Deref, DerefMut},
    ptr::{copy, read, write, NonNull},
};

//////////////// Vector /////////////////////////////////
/////////////////////////////////////////////////////////

struct Vector<T> {
    buf: RawVec<T>,
    len: usize,
}

#[allow(dead_code)]
impl<T> Vector<T> {
    pub fn push(&mut self, elem: T) {
        if self.len == self.capacity() {
            self.buf.grow();
        }

        unsafe {
            write(self.ptr().add(self.len), elem);
        }

        // Can't fail, we'll OOM first.
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(read(self.ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        // Note: `<=` because it's valid to insert after everything
        // which would be equivalent to push.
        assert!(index <= self.len, "index out of bounds");
        if self.len == self.capacity() {
            self.buf.grow();
        }

        unsafe {
            // ptr::copy(src, dest, len): "copy from src to dest len elems"
            copy(
                self.ptr().add(index),
                self.ptr().add(index + 1),
                self.len - index,
            );
            write(self.ptr().add(index), elem);
        }

        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        // Note: `<` because it's *not* valid to remove after everything
        assert!(index < self.len, "index out of bounds");
        unsafe {
            self.len -= 1;
            let result = read(self.ptr().add(index));
            copy(
                self.ptr().add(index + 1),
                self.ptr().add(index),
                self.len - index,
            );
            result
        }
    }

    pub fn drain(&mut self) -> Drain<T> {
        let iter = unsafe { RawValIter::new(&self) };

        // this is a mem::forget safety thing. If Drain is forgotten, we just
        // leak the whole Vector's contents. Also we need to do this *eventually*
        // anyway, so why not do it now?
        self.len = 0;

        Drain {
            iter,
            vec: PhantomData,
        }
    }

    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn capacity(&self) -> usize {
        self.buf.capacity
    }

    pub fn new() -> Self {
        Vector {
            buf: RawVec::new(),
            len: 0,
        }
    }
}

impl<T> Deref for Vector<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr(), self.len) }
    }
}

impl<T> DerefMut for Vector<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr(), self.len) }
    }
}

impl<T> IntoIterator for Vector<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            let buf = read(&self.buf);
            forget(self);

            IntoIter { iter, _buf: buf }
        }
    }
}

//////////////// RawVec /////////////////////////////////
/////////////////////////////////////////////////////////

pub struct RawVec<T> {
    ptr: NonNull<T>,
    capacity: usize,
}

impl<T> RawVec<T> {
    pub fn new() -> Self {
        assert!(
            size_of::<T>() != 0,
            "Cannot allocate memory for zero sized types"
        );
        Self {
            ptr: NonNull::dangling(),
            capacity: 0usize,
        }
    }

    fn grow(&mut self) {
        let cur_cap_is_zero = || self.capacity == 0;
        let (new_cap, new_layout) = if cur_cap_is_zero() {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            // This can't overflow since self.cap <= isize::MAX.
            let new_cap = 2 * self.capacity;

            // `Layout::array` checks that the number of bytes is <= usize::MAX,
            // but this is redundant since old_layout.size() <= isize::MAX,
            // so the `unwrap` should never fail.
            let new_layout = Layout::array::<T>(new_cap).unwrap();
            (new_cap, new_layout)
        };

        // Ensure that the new allocation doesn't exceed `isize::MAX` bytes.
        assert!(
            new_layout.size() <= isize::MAX as usize,
            "Allocation too large"
        );

        let new_ptr = if cur_cap_is_zero() {
            unsafe { alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // If allocation fails, `new_ptr` will be null, in which case we abort.
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(p) => p,
            None => handle_alloc_error(new_layout),
        };
        self.capacity = new_cap;
    }
}

unsafe impl<T: Send> Send for RawVec<T> {}
unsafe impl<T: Sync> Sync for RawVec<T> {}

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

//////////////// IntoIter /////////////////////////////////
/////////////////////////////////////////////////////////

pub struct IntoIter<T> {
    _buf: RawVec<T>, // we don't actually care about this. Just need it to live.
    iter: RawValIter<T>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for _ in &mut *self {}
    }
}

//////////////// Drain /////////////////////////////////
/////////////////////////////////////////////////////////

pub struct Drain<'a, T: 'a> {
    vec: PhantomData<&'a mut Vector<T>>,
    iter: RawValIter<T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        for _ in &mut *self {}
    }
}

//////////////// RawValIter /////////////////////////////////
/////////////////////////////////////////////////////////

struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> RawValIter<T> {
    // unsafe to construct because it has no associated lifetimes.
    // This is necessary to store a RawValIter in the same struct as
    // its actual allocation. OK since it's a private implementation
    // detail.
    unsafe fn new(slice: &[T]) -> Self {
        RawValIter {
            start: slice.as_ptr(),
            end: if slice.len() == 0 {
                // if `len = 0`, then this is not actually allocated memory.
                // Need to avoid offsetting because that will give wrong
                // information to LLVM via GEP.
                slice.as_ptr()
            } else {
                slice.as_ptr().add(slice.len())
            },
        }
    }
}

impl<T> DoubleEndedIterator for RawValIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if size_of::<T>() == 0 {
                    self.end = (self.end as usize - 1) as *const _;
                    Some(read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    self.end = self.end.offset(-1);
                    Some(read(self.end))
                }
            }
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                if size_of::<T>() == 0 {
                    self.start = (self.start as usize + 1) as *const _;
                    Some(read(NonNull::<T>::dangling().as_ptr()))
                } else {
                    let old_ptr = self.start;
                    self.start = self.start.offset(1);
                    Some(read(old_ptr))
                }
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = size_of::<T>();
        let len =
            (self.end as usize - self.start as usize) / if elem_size == 0 { 1 } else { elem_size };
        (len, Some(len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut v: Vector<usize> = Vector::new();
        v.push(16 as usize);
        assert_eq!(v.len(), 1);
        assert_eq!(v.capacity(), 1);
        v.push(20 as usize);
        assert_eq!(v.len(), 2);
        assert_eq!(v.capacity(), 2);
        v.push(21 as usize);
        assert_eq!(v.len(), 3);
        assert_eq!(v.capacity(), 4);
        v.push(23 as usize);
        assert_eq!(v.len(), 4);
        assert_eq!(v.capacity(), 4);
        v.push(28 as usize);
        assert_eq!(v.len(), 5);
        assert_eq!(v.capacity(), 8);
        v[2] = 20;
    }
}
