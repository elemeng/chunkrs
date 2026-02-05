//! Benchmarks for chunkrs.
//!
//! Run with:
//!     cargo bench

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};

use chunkrs::{ChunkConfig, Chunker};

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
                    let chunker = Chunker::new(ChunkConfig::default());
                    let chunks = chunker.chunk_bytes(black_box(data.clone()));
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
                    let chunker = Chunker::new(ChunkConfig::default());
                    let chunks = chunker.chunk_bytes(black_box(data.clone()));
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
            let chunker = Chunker::new(config);
            let chunks = chunker.chunk_bytes(black_box(data.clone()));
            black_box(chunks.len())
        });
    });

    // Default chunks
    group.bench_function("default_chunks", |b| {
        let config = ChunkConfig::default();
        b.iter(|| {
            let chunker = Chunker::new(config);
            let chunks = chunker.chunk_bytes(black_box(data.clone()));
            black_box(chunks.len())
        });
    });

    // Large chunks
    group.bench_function("large_chunks", |b| {
        let config = ChunkConfig::new(64 * 1024, 256 * 1024, 1024 * 1024).unwrap();
        b.iter(|| {
            let chunker = Chunker::new(config);
            let chunks = chunker.chunk_bytes(black_box(data.clone()));
            black_box(chunks.len())
        });
    });

    // No hashing
    group.bench_function("no_hash", |b| {
        let config = ChunkConfig::default().with_hash_config(chunkrs::HashConfig::disabled());
        b.iter(|| {
            let chunker = Chunker::new(config);
            let chunks = chunker.chunk_bytes(black_box(data.clone()));
            black_box(chunks.len())
        });
    });

    group.finish();
}

fn bench_streaming(c: &mut Criterion) {
    use std::io::Read;

    let mut group = c.benchmark_group("streaming");
    let size = 1024 * 1024; // 1 MB
    let data: Vec<u8> = (0..size).map(|i| (i * 7 + 13) as u8).collect();

    group.throughput(Throughput::Bytes(size as u64));
    group.bench_function("iterator", |b| {
        b.iter(|| {
            let cursor = std::io::Cursor::new(black_box(&data));
            let chunker = Chunker::new(ChunkConfig::default());
            let mut count = 0;
            for chunk in chunker.chunk(cursor) {
                let _ = chunk.unwrap();
                count += 1;
            }
            black_box(count)
        });
    });

    group.bench_function("buffered", |b| {
        b.iter(|| {
            let mut cursor = std::io::Cursor::new(black_box(&data));
            let mut buf = vec![0u8; 64 * 1024];
            let mut total = 0usize;
            loop {
                let n = cursor.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                total += n;
            }
            black_box(total)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_chunker, bench_configs, bench_streaming);
criterion_main!(benches);
