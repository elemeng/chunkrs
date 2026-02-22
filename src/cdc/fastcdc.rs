//! FastCDC rolling hash implementation.

use crate::cdc::tables::{MASKS, GEAR_TABLE_SHIFTED};

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
    pub fn new(min_size: usize, avg_size: usize, max_size: usize, normalization_level: u8) -> Self {
        Self::with_key(min_size, avg_size, max_size, normalization_level, None)
    }

    pub fn with_key(
        min_size: usize,
        avg_size: usize,
        max_size: usize,
        normalization_level: u8,
        key: Option<[u8; 32]>,
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
        let gear_table_shifted = if let Some(k) = key {
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

    pub fn reset(&mut self) {
        self.hash = 0;
        self.bytes_since_boundary = 0;
    }

    pub fn update(&mut self, byte: u8) -> bool {
        self.bytes_since_boundary += 1;

        let byte_idx = byte as usize;
        let gear = self.gear_table_shifted[byte_idx];
        self.hash = self.hash.wrapping_add(gear);

        if self.bytes_since_boundary < self.min_size {
            return false;
        }

        if self.bytes_since_boundary >= self.max_size {
            return self.emit_boundary();
        }

        let mask = if self.bytes_since_boundary < self.avg_size {
            self.mask_s
        } else {
            self.mask_l
        };

        if (self.hash & mask) == 0 {
            self.emit_boundary()
        } else {
            false
        }
    }

    fn emit_boundary(&mut self) -> bool {
        self.bytes_since_boundary = 0;
        self.hash = 0;
        true
    }

    #[allow(dead_code)]
    pub fn find_boundary(&mut self, data: &[u8]) -> Option<usize> {
        for (i, &byte) in data.iter().enumerate() {
            if self.update(byte) {
                return Some(i + 1);
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn bytes_since_boundary(&self) -> usize {
        self.bytes_since_boundary
    }

    #[allow(dead_code)]
    pub fn hash(&self) -> u64 {
        self.hash
    }

    #[allow(dead_code)]
    pub fn min_size(&self) -> usize {
        self.min_size
    }

    #[allow(dead_code)]
    pub fn avg_size(&self) -> usize {
        self.avg_size
    }

    #[allow(dead_code)]
    pub fn max_size(&self) -> usize {
        self.max_size
    }
}

impl Default for FastCdc {
    fn default() -> Self {
        Self::new(
            crate::config::DEFAULT_MIN_CHUNK_SIZE,
            crate::config::DEFAULT_AVG_CHUNK_SIZE,
            crate::config::DEFAULT_MAX_CHUNK_SIZE,
            crate::config::DEFAULT_NORMALIZATION_LEVEL,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fastcdc_min_size_constraint() {
        let mut cdc = FastCdc::new(4, 16, 64, 1);

        for _ in 0..3 {
            assert!(!cdc.update(0xFF), "No boundary before min_size");
        }
    }

    #[test]
    fn test_fastcdc_boundary_detection() {
        let mut cdc = FastCdc::new(4, 16, 64, 1);

        let mut found_boundary = false;
        for i in 0..200 {
            if cdc.update((i % 256) as u8) {
                found_boundary = true;
                break;
            }
        }
        assert!(found_boundary, "Must find boundary within 200 bytes");
    }

    #[test]
    fn test_fastcdc_max_size_enforcement() {
        let mut cdc = FastCdc::new(2, 8, 8, 1);

        for _ in 0..7 {
            assert!(!cdc.update(0xFF), "No boundary before max_size");
        }

        assert!(cdc.update(0xFF), "Boundary at max_size");
    }

    #[test]
    fn test_fastcdc_reset() {
        let mut cdc = FastCdc::new(4, 16, 64, 1);

        for i in 0..100 {
            cdc.update(i as u8);
        }

        cdc.reset();

        assert_eq!(cdc.bytes_since_boundary(), 0);
        assert_eq!(cdc.hash(), 0);
    }

    #[test]
    fn test_fastcdc_default() {
        let cdc = FastCdc::default();
        assert_eq!(cdc.min_size(), 4 * 1024);
        assert_eq!(cdc.avg_size(), 16 * 1024);
        assert_eq!(cdc.max_size(), 64 * 1024);
    }

    #[test]
    fn test_fastcdc_keyed_deterministic() {
        #[cfg(feature = "keyed-cdc")]
        {
            let key = [42u8; 32];
            let data = b"The quick brown fox jumps over the lazy dog";

            let mut cdc1 = FastCdc::with_key(4, 16, 64, 1, Some(key));
            let mut cdc2 = FastCdc::with_key(4, 16, 64, 1, Some(key));

            let boundaries1: Vec<bool> = data.iter().map(|&b| cdc1.update(b)).collect();
            let boundaries2: Vec<bool> = data.iter().map(|&b| cdc2.update(b)).collect();

            assert_eq!(boundaries1, boundaries2, "Keyed CDC should be deterministic");
        }
    }

    #[test]
    fn test_fastcdc_keyed_unique() {
        #[cfg(feature = "keyed-cdc")]
        {
            let key1 = [1u8; 32];
            let key2 = [2u8; 32];
            let data = b"Test data for uniqueness check";

            let mut cdc1 = FastCdc::with_key(4, 16, 64, 1, Some(key1));
            let mut cdc2 = FastCdc::with_key(4, 16, 64, 1, Some(key2));

            let boundaries1: Vec<bool> = data.iter().map(|&b| cdc1.update(b)).collect();
            let boundaries2: Vec<bool> = data.iter().map(|&b| cdc2.update(b)).collect();

            assert_ne!(
                boundaries1, boundaries2,
                "Different keys should produce different boundaries"
            );
        }
    }

    #[test]
    fn test_fastcdc_normalization_level_0() {
        let mut cdc = FastCdc::new(4, 16, 64, 0);

        let mut found_boundary = false;
        for i in 0..200 {
            if cdc.update((i % 256) as u8) {
                found_boundary = true;
                break;
            }
        }
        assert!(found_boundary, "Level 0 should still find boundaries");
    }

    #[test]
    fn test_fastcdc_normalization_level_2() {
        let mut cdc = FastCdc::new(4, 16, 64, 2);

        let mut found_boundary = false;
        for i in 0..200 {
            if cdc.update((i % 256) as u8) {
                found_boundary = true;
                break;
            }
        }
        assert!(found_boundary, "Level 2 should still find boundaries");
    }
}
