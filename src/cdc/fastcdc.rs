//! FastCDC rolling hash implementation.
//!
//! FastCDC uses a rolling hash to identify chunk boundaries based on content:
//! - Zero-padded masks for better deduplication
//! - Dual gear tables for faster hashing
//! - Normalized chunking with two-stage masks
//! - Deterministic: same input → same boundaries
//!
//! # References
//!
//! W. Xia et al., "The Design of Fast Content-Defined Chunking for Data
//! Deduplication Based Storage Systems," IEEE TPDS, vol. 31, no. 9, 2020.

use crate::cdc::tables::{GEAR_TABLE_SHIFTED, MASKS};

#[cfg(feature = "keyed-cdc")]
use crate::cdc::tables::generate_keyed_gear_table_shifted;

/// FastCDC rolling hash state.
#[derive(Debug)]
pub struct FastCdc {
    hash: u64,
    min_size: usize,
    avg_size: usize,
    max_size: usize,
    bytes_since_boundary: usize,
    mask_s: u64,
    mask_l: u64,
    gear_table_shifted: [u64; 256],
}

impl FastCdc {
    /// Creates a new FastCDC instance.
    #[allow(dead_code)]
    pub fn new(min_size: usize, avg_size: usize, max_size: usize, normalization_level: u8) -> Self {
        Self::with_key(min_size, avg_size, max_size, normalization_level, None)
    }

    /// Creates a new FastCDC instance with an optional key for keyed CDC.
    pub fn with_key(
        min_size: usize,
        avg_size: usize,
        max_size: usize,
        normalization_level: u8,
        _key: Option<[u8; 32]>,
    ) -> Self {
        let avg_bits = avg_size.trailing_zeros() as usize;
        let level = normalization_level as usize;

        let mask_s = if level == 0 {
            MASKS[avg_bits]
        } else {
            MASKS[avg_bits + level]
        };

        let mask_l = if level == 0 {
            MASKS[avg_bits]
        } else {
            MASKS[avg_bits - level]
        };

        #[cfg(feature = "keyed-cdc")]
        let gear_table_shifted = if let Some(k) = _key {
            generate_keyed_gear_table_shifted(k)
        } else {
            GEAR_TABLE_SHIFTED
        };

        #[cfg(not(feature = "keyed-cdc"))]
        let gear_table_shifted = GEAR_TABLE_SHIFTED;

        Self {
            hash: 0,
            min_size,
            avg_size,
            max_size,
            bytes_since_boundary: 0,
            mask_s,
            mask_l,
            gear_table_shifted,
        }
    }

    /// Resets the hash state.
    pub fn reset(&mut self) {
        self.hash = 0;
        self.bytes_since_boundary = 0;
    }

    /// Updates the hash with a new byte and returns true if a boundary is found.
    pub fn update(&mut self, byte: u8) -> bool {
        self.bytes_since_boundary = self.bytes_since_boundary.saturating_add(1);

        let byte_idx = byte as usize;
        let gear = self.gear_table_shifted[byte_idx];

        // Gear hash: hash = (hash >> 1) + gear_table[byte]
        self.hash = (self.hash >> 1).wrapping_add(gear);

        // Boundary detection
        if self.bytes_since_boundary < self.min_size {
            return false;
        }

        if self.bytes_since_boundary >= self.max_size {
            self.bytes_since_boundary = 0;
            return true;
        }

        if self.bytes_since_boundary >= self.avg_size {
            // Large mask check
            if (self.hash & self.mask_l) == 0 {
                self.bytes_since_boundary = 0;
                return true;
            }
        } else {
            // Small mask check
            if (self.hash & self.mask_s) == 0 {
                self.bytes_since_boundary = 0;
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastcdc_basic() {
        let mut cdc = FastCdc::new(4, 16, 64, 2);
        let data = [0u8; 100];
        let mut boundary_count = 0;

        for byte in data {
            if cdc.update(byte) {
                boundary_count += 1;
            }
        }

        assert!(boundary_count > 0, "Should find at least one boundary");
    }

    #[test]
    fn test_fastcdc_reset() {
        let mut cdc = FastCdc::new(4, 16, 64, 2);
        let data = [1u8; 100];

        for byte in data {
            cdc.update(byte);
        }

        cdc.reset();

        // After reset, should behave like fresh instance
        assert_eq!(cdc.bytes_since_boundary, 0);
    }

    #[test]
    fn test_fastcdc_min_size() {
        let min_size = 8;
        let mut cdc = FastCdc::new(min_size, 16, 64, 2);
        let data = [1u8; 20];

        for (i, byte) in data.iter().enumerate() {
            let found = cdc.update(*byte);
            if found {
                assert!(
                    i + 1 >= min_size,
                    "Boundary found before min_size at position {}",
                    i + 1
                );
            }
        }
    }

    #[test]
    fn test_fastcdc_max_size() {
        let max_size = 32;
        let mut cdc = FastCdc::new(4, 16, max_size, 2);
        let data = [1u8; 100];
        let mut boundary_count = 0;
        let mut last_boundary_pos = 0;

        for (i, byte) in data.iter().enumerate() {
            if cdc.update(*byte) {
                boundary_count += 1;
                let chunk_size = (i + 1 - last_boundary_pos) as usize;
                assert!(
                    chunk_size <= max_size,
                    "Chunk size {} exceeds max_size {}",
                    chunk_size,
                    max_size
                );
                last_boundary_pos = i + 1;
            }
        }
    }

    #[test]
    fn test_fastcdc_determinism() {
        let data = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let mut cdc1 = FastCdc::new(4, 8, 16, 2);
        let mut cdc2 = FastCdc::new(4, 8, 16, 2);
        let mut boundaries1 = Vec::new();
        let mut boundaries2 = Vec::new();

        for (i, byte) in data.iter().enumerate() {
            if cdc1.update(*byte) {
                boundaries1.push(i + 1);
            }
        }

        for (i, byte) in data.iter().enumerate() {
            if cdc2.update(*byte) {
                boundaries2.push(i + 1);
            }
        }

        assert_eq!(
            boundaries1, boundaries2,
            "Boundaries should be deterministic"
        );
    }
}
