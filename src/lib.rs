use std::{
    alloc::{alloc, Layout},
    ptr::NonNull,
};

struct Vector<T> {
    len: usize,
    capacity: usize,
    ptr: NonNull<T>,
}

impl<T> Vector<T> {
    pub fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            capacity: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        assert_ne!(std::mem::size_of::<T>(), 0, "No zero sized types");
        match self.capacity() {
            0 => {
                let layout = Layout::array::<T>(4).expect("Could not allocated memory");
                let ptr = unsafe { alloc(layout) } as *mut T;
                let ptr = NonNull::new(ptr).expect("Could not allocate memory");
                unsafe { ptr.as_ptr().write(item) };
                self.ptr = ptr;
                self.capacity = 4;
                self.len = 1;
            }
            _ if self.len >= self.capacity => todo!(),
            _ => {
                let offset = self
                    .len
                    .checked_mul(std::mem::size_of::<T>())
                    .expect("Cannot allocate memory.");
                assert!(offset < isize::MAX as usize, "Wrapped isize");
                unsafe {
                    self.ptr.as_ptr().add(self.len).write(item);
                    self.len += 1;
                }
            }
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.len
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
        assert_eq!(v.capacity(), 4);
        v.push(20 as usize);
        v.push(21 as usize);
        v.push(23 as usize);
        assert_eq!(v.len(), 4);
        assert_eq!(v.capacity(), 4);
    }
}
