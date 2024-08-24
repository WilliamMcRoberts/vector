use std::{
    alloc::{alloc, dealloc, handle_alloc_error, realloc, Layout},
    mem,
    ops::{Deref, DerefMut},
    ptr::{copy, read, write, NonNull},
};

#[allow(dead_code)]
struct Vector<T> {
    ptr: NonNull<T>,
    len: usize,
    capacity: usize,
}

#[allow(dead_code)]
impl<T> Vector<T> {
    pub fn new() -> Self {
        assert!(
            mem::size_of::<T>() != 0,
            "Cannot allocate memory for zero sized types"
        );
        Self {
            ptr: NonNull::dangling(),
            len: 0usize,
            capacity: 0usize,
        }
    }

    fn grow(&mut self) {
        let (new_cap, new_layout) = if self.capacity == 0 {
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

        let new_ptr = if self.capacity == 0 {
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

    pub fn push(&mut self, elem: T) {
        if self.len == self.capacity {
            self.grow();
        }

        unsafe {
            write(self.ptr.as_ptr().add(self.len), elem);
        }

        // Can't fail, we'll OOM first.
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            unsafe { Some(read(self.ptr.as_ptr().add(self.len))) }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        // Note: `<=` because it's valid to insert after everything
        // which would be equivalent to push.
        assert!(index <= self.len, "index out of bounds");
        if self.len == self.capacity {
            self.grow();
        }

        unsafe {
            // ptr::copy(src, dest, len): "copy from src to dest len elems"
            copy(
                self.ptr.as_ptr().add(index),
                self.ptr.as_ptr().add(index + 1),
                self.len - index,
            );
            write(self.ptr.as_ptr().add(index), elem);
        }

        self.len += 1;
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T> Drop for Vector<T> {
    fn drop(&mut self) {
        if self.capacity != 0 {
            while let Some(_) = self.pop() {}
            let layout = Layout::array::<T>(self.capacity).unwrap();
            unsafe {
                dealloc(self.ptr.as_ptr() as *mut u8, layout);
            }
        }
    }
}

impl<T> Deref for Vector<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for Vector<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

unsafe impl<T: Send> Send for Vector<T> {}
unsafe impl<T: Sync> Sync for Vector<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut v: Vector<usize> = Vector::new();
        v.push(16 as usize);
        assert_eq!(v.len(), 1);
        assert_eq!(v.capacity(), 4);
        v.push(20 as usize);
        v.push(21 as usize);
        v.push(23 as usize);
        assert_eq!(v.len(), 4);
        assert_eq!(v.capacity(), 4);
        v.push(28 as usize);
        assert_eq!(v.len(), 5);
        assert_eq!(v.capacity(), 8);
        v[2] = 20;
    }
}
