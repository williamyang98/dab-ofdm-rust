use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

/// A linear buffer that has a fixed capacity and a current size
pub struct LinearBucket<T> {
    data: Vec<T>,
    length: usize,
}

#[allow(unused)]
impl<T> LinearBucket<T> {
    /// Resets it to being empty
    pub fn reset(&mut self) {
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

    /// Returns immutable slice to valid data.
    pub fn iter(&self) -> &[T] {
        &self.data[..self.length]
    }

    /// Returns mutable slice to valid data. 
    pub fn iter_mut(&mut self) -> &mut [T] {
        &mut self.data[..self.length]
    }

    /// Returns immutable slice to entire internal buffer.
    pub fn raw_slice(&self) -> &[T] {
        &self.data
    }

    /// Returns mutable slice to entire internal buffer.
    pub fn raw_slice_mut(&mut self) -> &mut[T] {
        &mut self.data
    }
}

#[allow(unused)]
impl<T:Default+Copy+Clone> LinearBucket<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![T::default(); capacity],
            length: 0,
        }
    }

    /// Copies a array until the capacity has been reached.
    /// Returns the number of samples read from the array.
    pub fn consume(&mut self, buf: &[T]) -> usize {
        let remain = self.capacity() - self.length;
        let total_read = buf.len().min(remain);
        let dest_slice = self.length..self.length+total_read;
        let src_slice = 0..total_read;
        self.data[dest_slice].copy_from_slice(&buf[src_slice]);
        self.length += total_read;
        total_read
    }

    /// Copies data from a generic iterator.
    /// Returns the number of samples read from iterator.
    pub fn consume_from_iterator<I>(&mut self, mut iter: I) -> usize 
    where I: Iterator<Item = T> 
    {
        let mut total_read: usize = 0;
        loop {
            if self.is_full() {
                break;
            }
            let value = match iter.next() {
                None => break,
                Some(value) => value,
            };
            self.data[self.length] = value;
            self.length += 1;
        }

        total_read
    }
}

impl<T, U> Index<U> for LinearBucket<T> 
where U: SliceIndex<[T]> 
{
    type Output = U::Output;
    fn index(&self, index: U) -> &Self::Output {
        &self.iter()[index]
    }
}

impl<T, U> IndexMut<U> for LinearBucket<T>
where U: SliceIndex<[T]> 
{
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        &mut self.iter_mut()[index]
    }
}