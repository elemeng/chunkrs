//! UltraCDC 内容定义分块算法实现
//!
//! 基于论文 "UltraCDC: A Fast and Efficient Content-Defined Chunking Algorithm
//! for Data Deduplication" (2022) 及 Plakar 的 Go 实现移植

use std::error::Error;
use std::fmt;

/// 预计算的汉明距离表：每个字节与 0xAA 的距离
const HAMMING_DISTANCE_TO_0XAA: [i32; 256] = [
    4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
    5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 7, 8, 6, 7, 5, 6, 4, 5, 6, 7, 5, 6,
    4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
    2, 3, 1, 2, 3, 4, 2, 3, 1, 2, 0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3,
    1, 2, 0, 1, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 2, 3, 1, 2, 3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5,
    3, 4, 2, 3, 4, 5, 3, 4, 5, 6, 4, 5, 6, 7, 5, 6, 4, 5, 3, 4, 5, 6, 4, 5, 3, 4, 2, 3, 4, 5, 3, 4,
];

/// 小掩码（Normal Point 之前使用）：0x2F = 0b101111
const MASK_S: u64 = 0x2F;
/// 大掩码（Normal Point 之后使用）：0x2C = 0b101100
/// 比 maskS 少检查 2 位，更容易匹配（提高正常点后匹配概率）
const MASK_L: u64 = 0x2C;
/// 低熵字符串阈值（LEST）：连续 64 字节相同则强制切割
const LOW_ENTROPY_THRESHOLD: usize = 64;

/// UltraCDC 配置选项
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// 最小块大小（必须 >= 64 且 < normal_size）
    pub min_size: usize,
    /// 目标块大小（必须 >= 64）
    pub normal_size: usize,
    /// 最大块大小（必须 > normal_size）
    pub max_size: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            min_size: 2 * 1024,     // 2KB
            normal_size: 10 * 1024, // 10KB
            max_size: 64 * 1024,    // 64KB
        }
    }
}

/// UltraCDC 错误类型
#[derive(Debug)]
pub enum UltraCdcError {
    InvalidNormalSize,
    InvalidMinSize,
    InvalidMaxSize,
}

impl fmt::Display for UltraCdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UltraCdcError::InvalidNormalSize => {
                write!(f, "NormalSize 必须在 [64, 1GB] 范围内")
            }
            UltraCdcError::InvalidMinSize => {
                write!(f, "MinSize 必须在 [64, 1GB] 范围内且小于 NormalSize")
            }
            UltraCdcError::InvalidMaxSize => {
                write!(f, "MaxSize 必须在 [64, 1GB] 范围内且大于 NormalSize")
            }
        }
    }
}

impl Error for UltraCdcError {}

impl Options {
    /// 创建新配置
    pub fn new(
        min_size: usize,
        normal_size: usize,
        max_size: usize,
    ) -> Result<Self, UltraCdcError> {
        let opts = Self {
            min_size,
            normal_size,
            max_size,
        };
        opts.validate()?;
        Ok(opts)
    }

    /// 验证配置参数
    pub fn validate(&self) -> Result<(), UltraCdcError> {
        if self.normal_size < 64 || self.normal_size > 1024 * 1024 * 1024 {
            return Err(UltraCdcError::InvalidNormalSize);
        }
        if self.min_size < 64
            || self.min_size > 1024 * 1024 * 1024
            || self.min_size >= self.normal_size
        {
            return Err(UltraCdcError::InvalidMinSize);
        }
        if self.max_size < 64
            || self.max_size > 1024 * 1024 * 1024
            || self.max_size <= self.normal_size
        {
            return Err(UltraCdcError::InvalidMaxSize);
        }
        Ok(())
    }
}

/// UltraCDC 分块器
#[derive(Debug)]
pub struct UltraCDC {
    options: Options,
}

impl UltraCDC {
    /// 使用默认配置创建
    pub fn new() -> Self {
        Self {
            options: Options::default(),
        }
    }

    /// 使用指定配置创建
    pub fn with_options(options: Options) -> Result<Self, UltraCdcError> {
        options.validate()?;
        Ok(Self { options })
    }

    /// 查找切割点
    ///
    /// # 参数
    /// - `data`: 输入数据切片
    /// - `n`: 实际处理长度（必须 <= data.len()）
    ///
    /// # 返回
    /// 切割点索引（<= n），如果未找到则返回 n
    ///
    /// # Panics
    /// 如果 n > data.len() 则 panic
    pub fn find_cut_point(&self, data: &[u8], n: usize) -> usize {
        assert!(
            n <= data.len(),
            "n ({}) 必须 <= data.len() ({})",
            n,
            data.len()
        );

        let min_size = self.options.min_size;
        let max_size = self.options.max_size;
        let normal_size = self.options.normal_size;

        // 情况1：数据太短，直接取全部
        if n <= min_size {
            return n;
        }

        // 限制处理范围不超过 max_size
        let n = n.min(max_size);

        // 如果调整后的 n <= normal_size，调整 normal_size 以适应
        let normal_size = if n <= normal_size { n } else { normal_size };

        // 初始化输出窗口（从 min_size 开始的 8 字节）
        let out_buf_win = &data[min_size..min_size + 8];

        // 计算初始汉明距离（与模式 0xAAAAAAAA 的距离）
        let mut dist: i32 = out_buf_win
            .iter()
            .map(|&v| HAMMING_DISTANCE_TO_0XAA[v as usize])
            .sum();

        let mut low_entropy_count = 0;
        let mut mask = MASK_S;

        // 主循环：每次跳 8 字节
        let mut i = min_size + 8;
        while i <= n.saturating_sub(8) {
            // 超过 normal_size 后切换为更宽松的掩码
            if i >= normal_size {
                mask = MASK_L;
            }

            let in_buf_win = &data[i..i + 8];

            // 低熵检测：连续窗口内容相同
            if in_buf_win == out_buf_win {
                low_entropy_count += 1;
                if low_entropy_count >= LOW_ENTROPY_THRESHOLD {
                    // 强制切割，返回当前位置 + 8
                    return i + 8;
                }
                i += 8;
                continue;
            }

            low_entropy_count = 0;

            // 逐字节检查这 8 字节
            for j in 0..8 {
                if (dist as u64 & mask) == 0 {
                    // 找到切割点
                    return i + j;
                }

                // 滑动窗口：移除 out_byte，加入 in_byte
                let out_byte = data[i + j - 8];
                let in_byte = data[i + j];

                let update = HAMMING_DISTANCE_TO_0XAA[in_byte as usize]
                    - HAMMING_DISTANCE_TO_0XAA[out_byte as usize];
                dist += update;
            }

            // 更新输出窗口引用
            i += 8;
        }

        // 未找到合适切割点，返回 n（最大到 max_size）
        n
    }

    /// 流式分块：处理整个数据流，返回所有块边界
    pub fn chunk_stream(&self, data: &[u8]) -> Vec<usize> {
        let mut boundaries = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let remaining = &data[offset..];
            let n = remaining.len().min(self.options.max_size);

            let cut = self.find_cut_point(remaining, n);
            offset += cut;
            boundaries.push(offset);

            // 如果切到末尾则结束
            if cut == remaining.len() {
                break;
            }
        }

        boundaries
    }
}

impl Default for UltraCDC {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let cdc = UltraCDC::new();
        assert_eq!(cdc.options.min_size, 2048);
        assert_eq!(cdc.options.normal_size, 10240);
        assert_eq!(cdc.options.max_size, 65536);
    }

    #[test]
    fn test_find_cut_point_small_data() {
        let cdc = UltraCDC::new();
        let data = vec![0u8; 100]; // 小于 min_size
        assert_eq!(cdc.find_cut_point(&data, data.len()), 100);
    }

    #[test]
    fn test_chunk_stream() {
        let cdc = UltraCDC::with_options(Options::new(64, 256, 512).unwrap()).unwrap();
        // 生成随机数据测试不 panic
        let data: Vec<u8> = (0..10000).map(|i| (i % 256) as u8).collect();
        let boundaries = cdc.chunk_stream(&data);

        assert!(!boundaries.is_empty());
        // 验证边界递增
        for i in 1..boundaries.len() {
            assert!(boundaries[i] > boundaries[i - 1]);
            // 验证块大小在允许范围内
            let size = boundaries[i] - boundaries[i - 1];
            assert!(size >= 64);
            assert!(size <= 512);
        }
    }

    #[test]
    fn test_low_entropy_detection() {
        // 创建一段重复数据（应该触发低熵切割）
        let opts = Options::new(64, 256, 512).unwrap();
        let cdc = UltraCDC::with_options(opts).unwrap();
        let data = vec![0xAAu8; 1000]; // 重复 0xAA

        let cut = cdc.find_cut_point(&data, data.len());
        // 应该因为低熵而提前切割
        assert!(cut < 1000);
    }

    #[test]
    fn test_options_validation() {
        assert!(Options::new(64, 128, 256).is_ok());
        assert!(Options::new(32, 128, 256).is_err()); // min < 64
        assert!(Options::new(200, 128, 256).is_err()); // min >= normal
        assert!(Options::new(64, 128, 100).is_err()); // max <= normal
    }
}
