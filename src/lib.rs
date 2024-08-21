use std::{
    alloc::{alloc, realloc, Layout},
    ptr::NonNull,
};

#[allow(dead_code)]
struct Vector<T> {
    len: usize,
    capacity: usize,
    ptr: NonNull<T>,
}

#[allow(dead_code)]
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
            _ if self.len >= self.capacity => {
                let new_capacity = self
                    .capacity
                    .checked_mul(2)
                    .expect("Could not allocate memory.");
                let size = std::mem::size_of::<T>() * self.capacity();
                let align = std::mem::align_of::<T>();
                size.checked_add(size % align)
                    .expect("Could not allocate memory.");
                unsafe {
                    let layout = Layout::from_size_align_unchecked(size, align);
                    let new_size = std::mem::size_of::<T>() * new_capacity;
                    let ptr = realloc(self.ptr.as_ptr() as *mut u8, layout, new_size);
                    let ptr = NonNull::new(ptr as *mut T).expect("Could not allocate memory");
                    ptr.as_ptr().add(self.len).write(item);
                    self.ptr = ptr;
                    self.len += 1;
                    self.capacity = new_capacity;
                }
            }
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
        v.push(28 as usize);
        assert_eq!(v.len(), 5);
        assert_eq!(v.capacity(), 8);
        // v[2] = 2;
    }
}
