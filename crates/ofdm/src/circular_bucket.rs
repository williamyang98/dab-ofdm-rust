use std::ops::{Index, IndexMut};

/// A circular buffer that has a fixed capacity and current size
pub struct CircularBucket<T> {
    data: Vec<T>,
    index: usize,
    length: usize,
}

#[allow(unused)]
impl<T> CircularBucket<T> {
    /// Resets to an empty buffer starting at the zero index
    pub fn reset(&mut self) {
        self.index = 0;
        self.length = 0;
    }

    pub fn length(&self) -> usize {
        self.length
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    pub fn is_full(&self) -> bool {
        self.length == self.capacity()
    }

    /// Returns immutable iterator to valid data and wraps around as a circular buffer.
    pub fn iter<'a>(&'a self) -> Iter<'a,T> {
        let capacity = self.capacity();
        Iter {
            data: &self.data,
            index: self.index,
            capacity: capacity,
            remain_length: self.length,
        }
    }

    /// Returns immutable iterator to valid data and wraps around as a circular buffer.
    pub fn iter_mut<'a>(&'a mut self) -> IterMut<'a,T> {
        let capacity = self.capacity();
        IterMut {
            data: &mut self.data,
            index: self.index,
            capacity: capacity,
            remain_length: self.length,
        }
    }

    /// Returns immutable slice of entire internal buffer.
    pub fn raw_slice(&self) -> &[T] {
        &self.data
    }

    /// Returns mutable slice of entire internal buffer.
    pub fn raw_slice_mut(&mut self) -> &mut[T] {
        &mut self.data
    }
}

#[allow(unused)]
impl<T:Default+Copy+Clone> CircularBucket<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![T::default(); capacity],
            index: 0,
            length: 0,
        }
    }

    /// Copies an array into the circular buffer until the capacity has been reached.
    /// An additional argument is used to specify if it should continue if the current capacity is reached anyway.
    /// Returns the number of samples read from the array.
    pub fn consume(&mut self, buf: &[T], consume_all: bool) -> usize {
        let capacity = self.capacity();
        let remain = capacity - self.length;

        let total_read: usize = match consume_all {
            true => buf.len(),
            false => buf.len().min(remain),
        };

        for i in 0..total_read {
            self.data[self.index] = buf[i];
            self.index = (self.index + 1) % capacity;
        }
        self.length = usize::min(capacity, self.length+total_read);
        total_read
    }
}

pub struct Iter<'a, T> {
    data: &'a[T],
    index: usize,
    capacity: usize,
    remain_length: usize,
} 

pub struct IterMut<'a, T> {
    data: &'a mut[T],
    index: usize,
    capacity: usize,
    remain_length: usize,
}

impl<'a,T> Iterator for Iter<'a,T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remain_length == 0 {
            return None;
        }

        let value = &self.data[self.index];
        self.index = (self.index + 1) % self.capacity;
        self.remain_length -= 1;
        Some(value)
    }
}

impl<'a,T> Iterator for IterMut<'a,T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remain_length == 0 {
            return None;
        }

        // The invariant that we guarantee here is that we cannot return a mutable reference to the same value
        // This is done by incrementing the index to the circular buffer and never wrapping back to it
        let value = unsafe { 
            &mut *(&mut self.data[self.index] as *mut T)
        };
        self.index = (self.index + 1) % self.capacity;
        self.remain_length -= 1;
        Some(value)
    }
}

impl<T> Index<usize> for CircularBucket<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.length);
        let wrapped_index = (index + self.index) % self.capacity();
        &self.data[wrapped_index]
    }
}

impl<T> IndexMut<usize> for CircularBucket<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.length);
        let wrapped_index = (index + self.index) % self.capacity();
        &mut self.data[wrapped_index]
    }
}