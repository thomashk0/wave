use std::io;
use std::io::Read;

/// A very simple buffer around any type implementing the Read Trait.
///
/// This buffer is designed to support a producer/consumer workflow for streaming
/// parsers.
///
/// No specific optimisation have been done on this code.
#[derive(Debug)]
pub(crate) struct Buffer<R> {
    inner: R,
    offset: usize,
    size: usize,
    data: Vec<u8>,
}

impl<R: Read> Buffer<R> {
    pub fn with_capacity(capacity: usize, inner: R) -> Self {
        Buffer {
            inner,
            offset: 0,
            size: 0,
            data: Vec::with_capacity(capacity),
        }
    }

    fn capacity(&self) -> usize {
        self.data.len()
    }

    fn available(&self) -> usize {
        self.capacity() - (self.size + self.offset)
    }

    pub fn push(&mut self, elt: u8) {
        if self.available() == 0 {
            self.data.push(elt);
        } else {
            self.data[self.offset + self.size] = elt;
            self.size += 1;
        }
    }

    pub fn consume(&mut self, size: usize) {
        if size >= self.size {
            self.offset = 0;
            self.size = 0;
        } else {
            self.offset += size;
            self.size -= size;
        }
    }

    pub fn trim(&mut self) -> usize {
        let n = self
            .data()
            .iter()
            .take_while(|c| c.is_ascii_whitespace())
            .count();
        self.consume(n);
        n
    }

    pub fn shift(&mut self) {
        self.data.drain(0..self.offset);
        self.offset = 0;
    }

    pub fn refill(&mut self, size: usize) -> io::Result<usize> {
        let end = self.offset + self.size;
        if self.available() < size {
            self.data.resize(end + size, 0);
        }
        let n = self.inner.read(&mut self.data[end..end + size])?;
        self.size += n;
        Ok(n)
    }

    pub fn data(&self) -> &[u8] {
        &self.data[self.offset..self.offset + self.size]
    }

    pub fn len(&self) -> usize {
        self.size
    }
}
