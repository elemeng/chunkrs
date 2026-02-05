//! Thread-local buffer pool for efficient memory reuse.

use std::cell::RefCell;

/// Default buffer size for pooled buffers.
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024; // 64 KiB

/// Maximum number of buffers to keep per thread.
pub const MAX_POOL_SIZE: usize = 4;

/// A reusable byte buffer.
pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    /// Takes a buffer from the thread-local pool or creates a new one.
    pub fn take() -> Self {
        THREAD_BUFFER_POOL.with(|pool| {
            let mut pool = pool.borrow_mut();
            if let Some(data) = pool.pop() {
                Self { data }
            } else {
                Self {
                    data: Vec::with_capacity(DEFAULT_BUFFER_SIZE),
                }
            }
        })
    }

    /// Clears the buffer without deallocating.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Extends the buffer with data.
    #[allow(dead_code)]
    pub(crate) fn extend_from_slice(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        // Return the buffer to the pool if it's not too large
        if self.data.capacity() <= DEFAULT_BUFFER_SIZE * 2 {
            self.data.clear();
            THREAD_BUFFER_POOL.with(|pool| {
                let mut pool = pool.borrow_mut();
                if pool.len() < MAX_POOL_SIZE {
                    pool.push(std::mem::take(&mut self.data));
                }
            });
        }
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self::take()
    }
}

// Thread-local buffer pool
thread_local! {
    static THREAD_BUFFER_POOL: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_take() {
        let buf = Buffer::take();
        assert!(buf.data.capacity() >= DEFAULT_BUFFER_SIZE);
    }

    #[test]
    fn test_buffer_extend_and_clear() {
        let mut buf = Buffer::take();
        buf.extend_from_slice(b"hello world");
        assert_eq!(buf.data.len(), 11);

        buf.clear();
        assert!(buf.data.is_empty());
        // Capacity should be preserved
        assert!(buf.data.capacity() >= DEFAULT_BUFFER_SIZE);
    }

    #[test]
    fn test_buffer_reuse() {
        // Take a buffer, put some data in it, then drop it
        {
            let mut buf = Buffer::take();
            buf.extend_from_slice(b"test data");
        }

        // The buffer should be returned to the pool
        let buf2 = Buffer::take();
        // Buffer should be empty but have capacity
        assert!(buf2.data.is_empty());
        assert!(buf2.data.capacity() >= DEFAULT_BUFFER_SIZE);
    }
}
