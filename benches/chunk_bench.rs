//! Benchmarks for chunkrs.
//!
//! Run with:
//!     cargo bench
//!
//! Run with specific benchmark:
//!     cargo bench -- bench_chunker
//!
//! Run with keyed-cdc feature:
//!     cargo bench --features keyed-cdc

use bytes::Bytes;
use chunkrs::{ChunkConfig, Chunker};
use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

fn bench_chunker(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunker");

    // Different data sizes
    for size in [64 * 1024, 1024 * 1024, 10 * 1024 * 1024] {
        // Deterministic pseudo-random data
        let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            format!("random_{}mb", size / (1024 * 1024)),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut chunker = Chunker::new(ChunkConfig::default());
                    let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
                    let _final = chunker.finish();
                    black_box(chunks.len())
                });
            },
        );

        // All zeros (worst case for CDC)
        let zeros = vec![0u8; size];
        group.bench_with_input(
            format!("zeros_{}mb", size / (1024 * 1024)),
            &zeros,
            |b, data| {
                b.iter(|| {
                    let mut chunker = Chunker::new(ChunkConfig::default());
                    let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
                    let _final = chunker.finish();
                    black_box(chunks.len())
                });
            },
        );
    }

    group.finish();
}

fn bench_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("configs");
    let size = 1024 * 1024; // 1 MB
    let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

    // Small chunks
    group.bench_function("small_chunks", |b| {
        let config = ChunkConfig::new(2 * 1024, 8 * 1024, 32 * 1024).unwrap();
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    // Default chunks
    group.bench_function("default_chunks", |b| {
        let config = ChunkConfig::default();
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    // Large chunks
    group.bench_function("large_chunks", |b| {
        let config = ChunkConfig::new(64 * 1024, 256 * 1024, 1024 * 1024).unwrap();
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    // No hashing
    group.bench_function("no_hash", |b| {
        let config = ChunkConfig::default().with_hash_config(chunkrs::HashConfig::disabled());
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    group.finish();
}

#[cfg(feature = "keyed-cdc")]
fn bench_keyed_cdc(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyed_cdc");
    let size = 1024 * 1024; // 1 MB
    let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

    // Keyed CDC with random key
    group.bench_function("keyed", |b| {
        let key = [0u8; 32];
        let config = ChunkConfig::default().with_keyed_gear_table(Some(key));
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    // Non-keyed (baseline comparison)
    group.bench_function("non_keyed", |b| {
        let config = ChunkConfig::default().with_keyed_gear_table(None);
        b.iter(|| {
            let mut chunker = Chunker::new(config);
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    group.finish();
}

fn bench_streaming(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming");
    let size = 1024 * 1024; // 1 MB
    let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

    group.throughput(Throughput::Bytes(size as u64));
    group.bench_function("push_finish", |b| {
        b.iter(|| {
            let mut chunker = Chunker::new(ChunkConfig::default());
            let mut total = 0;
            let batch_size = 8192;

            for chunk in black_box(&data).chunks(batch_size) {
                let (chunks, _) = chunker.push(Bytes::copy_from_slice(chunk));
                total += chunks.len();
            }

            if chunker.finish().is_some() {
                total += 1;
            }

            black_box(total)
        });
    });

    group.bench_function("single_push", |b| {
        b.iter(|| {
            let mut chunker = Chunker::new(ChunkConfig::default());
            let (chunks, _) = chunker.push(Bytes::from(black_box(data.clone())));
            let _final = chunker.finish();
            black_box(chunks.len())
        });
    });

    group.finish();
}

// Conditionally include keyed-cdc benchmarks
criterion_group!(benches, bench_chunker, bench_configs, bench_streaming);

// Add keyed-cdc benchmarks only when feature is enabled
#[cfg(feature = "keyed-cdc")]
criterion_group!(keyed_benches, bench_keyed_cdc);

criterion_main!(benches);
