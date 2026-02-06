//! Thread-local buffer pool for efficient memory reuse.
//!
//! This module provides a thread-local buffer pool to minimize allocations
//! during chunking operations. Buffers are reused within each thread, reducing
//! the overhead of repeated allocations and deallocations.
//!
//! # Performance Benefits
//!
//! - **Reduced allocations**: Reuses buffers instead of allocating new ones
//! - **Thread-local**: No synchronization overhead
//! - **Bounded pool**: Limits memory usage per thread
//! - **Automatic cleanup**: Buffers returned when dropped

use std::cell::RefCell;

/// Default buffer size for pooled buffers.
pub const DEFAULT_BUFFER_SIZE: usize = 64 * 1024; // 64 KiB

/// Maximum number of buffers to keep per thread.
pub const MAX_POOL_SIZE: usize = 4;

/// A reusable byte buffer.
///
/// `Buffer` wraps a `Vec<u8>` and automatically returns it to the thread-local
/// pool when dropped, allowing it to be reused for future operations.
///
/// # Example
///
/// ```ignore
/// use chunkrs::buffer::Buffer;
///
/// let mut buf = Buffer::take();
/// buf.extend_from_slice(b"hello world");
/// buf.clear();
/// // Buffer is returned to pool when dropped
/// ```
pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    /// Takes a buffer from the thread-local pool or creates a new one.
    ///
    /// This method tries to reuse a buffer from the pool if available,
    /// otherwise creates a new buffer with the default capacity.
    ///
    /// # Returns
    ///
    /// A `Buffer` that can be used for temporary storage
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::buffer::Buffer;
    ///
    /// let buf = Buffer::take();
    /// assert!(buf.data.capacity() >= 64 * 1024);
    /// ```
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
    ///
    /// This removes all data from the buffer but preserves its capacity,
    /// allowing it to be reused efficiently.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use chunkrs::buffer::Buffer;
    ///
    /// let mut buf = Buffer::take();
    /// buf.extend_from_slice(b"hello world");
    /// assert_eq!(buf.data.len(), 11);
    ///
    /// buf.clear();
    /// assert!(buf.data.is_empty());
    /// assert!(buf.data.capacity() >= 64 * 1024);
    /// ```
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
//
// This uses a thread-local storage to keep a small pool of buffers per thread.
// Each buffer is a Vec<u8> that can be reused for temporary storage.
//
// The pool is bounded by MAX_POOL_SIZE to prevent unbounded memory growth.
// Buffers that are too large (more than 2x DEFAULT_BUFFER_SIZE) are not
// returned to the pool to avoid wasting memory.
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
