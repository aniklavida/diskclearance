use std::fs::File;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use diskclearance_lib::boundary::cancellation::CancellationToken;
use diskclearance_lib::boundary::events::ScanProgressPayload;
use diskclearance_lib::boundary::throttle::{ProgressSink, ProgressThrottler};
use diskclearance_lib::duplicates::engine::{
    DuplicateDetectionMetrics, DuplicateDetectionOptions, detect_duplicates,
};
use diskclearance_lib::duplicates::hashing::{ChunkVerifyingReader, compute_full_hash_streaming};
use diskclearance_lib::performance::{FlatFixture, peak_resident_bytes, write_repeated_file};
use diskclearance_lib::platform::create_platform_adapter;
use diskclearance_lib::scan::traversal::{MemoryTracker, TraversalOptions, walk_roots};

struct CountingProgressSink {
    emissions: usize,
}

impl ProgressSink for CountingProgressSink {
    fn emit_progress(&mut self, _payload: ScanProgressPayload) {
        self.emissions += 1;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("scan") => {
            let entries = args
                .get(2)
                .and_then(|value| value.parse::<usize>().ok())
                .expect("scan requires entry count");
            measure_scan(entries);
        }
        Some("cancel") => {
            let entries = args
                .get(2)
                .and_then(|value| value.parse::<usize>().ok())
                .expect("cancel requires entry count");
            let after = args
                .get(3)
                .and_then(|value| value.parse::<usize>().ok())
                .expect("cancel requires cancellation point");
            measure_cancellation(entries, after);
        }
        Some("duplicate") => {
            let bytes = args
                .get(2)
                .and_then(|value| value.parse::<u64>().ok())
                .expect("duplicate requires byte count per file");
            measure_duplicates(bytes);
        }
        _ => {
            eprintln!("usage: measure_performance scan N | cancel N AFTER | duplicate BYTES");
            std::process::exit(2);
        }
    }
}

fn measure_scan(entries: usize) {
    let fixture = FlatFixture::new("scan", entries);
    let adapter = create_platform_adapter();
    let token = CancellationToken::new();
    let tracker = MemoryTracker::new();
    let first_verified = std::cell::RefCell::new(None);
    let started = Instant::now();
    let mut throttler = ProgressThrottler::new(
        Duration::from_millis(50),
        CountingProgressSink { emissions: 0 },
    );

    let result = walk_roots(
        TraversalOptions {
            session_id: "measure-scan".to_string(),
            roots: vec![fixture.root.clone()],
            adapter: adapter.as_ref(),
            cancellation_token: &token,
            tracker: Some(&tracker),
            hooks: None,
            collect_entries: false,
        },
        |payload| throttler.record(payload, Instant::now()),
        |_| {
            if first_verified.borrow().is_none() {
                *first_verified.borrow_mut() = Some(started.elapsed());
            }
        },
        |_| {},
    );
    throttler.flush();
    let elapsed = started.elapsed();
    let first_verified = first_verified.into_inner().expect("one verified result");
    let events = throttler.sink().emissions;

    println!("scan_entries={entries}");
    println!("scan_items={}", result.coverage.items_scanned);
    println!("scan_complete={}", result.coverage.is_complete);
    println!(
        "first_verified_ms={}",
        first_verified.as_secs_f64() * 1000.0
    );
    println!("scan_elapsed_ms={}", elapsed.as_secs_f64() * 1000.0);
    println!("peak_records={}", tracker.peak());
    println!("peak_rss_bytes={}", peak_resident_bytes().unwrap_or(0));
    println!("progress_events={events}");
    println!(
        "event_rate_per_second={}",
        events as f64 / elapsed.as_secs_f64()
    );
}

fn measure_cancellation(entries: usize, cancel_after: usize) {
    let fixture = FlatFixture::new("cancel", entries);
    let adapter = create_platform_adapter();
    let token = CancellationToken::new();
    let worker_token = token.clone();
    let verified = Arc::new(AtomicU64::new(0));
    let worker_verified = Arc::clone(&verified);
    let root = fixture.root.clone();

    let handle = std::thread::spawn(move || {
        walk_roots(
            TraversalOptions {
                session_id: "measure-cancel".to_string(),
                roots: vec![root],
                adapter: adapter.as_ref(),
                cancellation_token: &worker_token,
                tracker: None,
                hooks: None,
                collect_entries: false,
            },
            |_| {},
            move |_| {
                worker_verified.fetch_add(1, Ordering::SeqCst);
            },
            |_| {},
        )
    });

    while verified.load(Ordering::SeqCst) < cancel_after as u64 && !handle.is_finished() {
        std::thread::sleep(Duration::from_millis(1));
    }
    let cancellation_started = Instant::now();
    token.cancel();
    let result = handle.join().expect("measurement scan thread join");
    let latency = cancellation_started.elapsed();

    println!("cancel_fixture_entries={entries}");
    println!("cancel_requested_after={cancel_after}");
    println!("cancel_items={}", result.coverage.items_scanned);
    println!("cancel_complete={}", result.coverage.is_complete);
    println!("cancel_latency_ms={}", latency.as_secs_f64() * 1000.0);
    println!(
        "cancel_peak_rss_bytes={}",
        peak_resident_bytes().unwrap_or(0)
    );
}

fn measure_duplicates(bytes: u64) {
    let fixture = FlatFixture::new("duplicate", 0);
    let first = fixture.root.join("large_media_1.bin");
    let second = fixture.root.join("large_media_2.bin");
    write_repeated_file(&first, bytes, 0xA5);
    write_repeated_file(&second, bytes, 0xA5);

    let adapter = create_platform_adapter();
    let candidates = vec![first.clone(), second.clone()];
    let options = DuplicateDetectionOptions::default();
    let mut metrics = DuplicateDetectionMetrics::default();
    let staged_started = Instant::now();
    let groups = detect_duplicates(&candidates, adapter.as_ref(), &options, &mut metrics)
        .expect("staged duplicate detection");
    let staged_elapsed = staged_started.elapsed();

    let mut streamed_bytes = 0u64;
    let mut max_chunk = 0usize;
    let streamed_started = Instant::now();
    for path in &candidates {
        let file = File::open(path).expect("open benchmark media");
        let mut reader = ChunkVerifyingReader::new(file);
        let _ = compute_full_hash_streaming(&mut reader, |chunk| {
            max_chunk = max_chunk.max(chunk);
        })
        .expect("stream benchmark media");
        streamed_bytes += reader.total_bytes_read() as u64;
        max_chunk = max_chunk.max(reader.max_chunk_observed());
    }
    let streamed_elapsed = streamed_started.elapsed();

    println!("duplicate_files={}", candidates.len());
    println!("duplicate_bytes_per_file={bytes}");
    println!("duplicate_staged_groups={}", groups.len());
    println!(
        "duplicate_staged_full_hashes={}",
        metrics.full_hashes_computed
    );
    println!(
        "duplicate_staged_elapsed_ms={}",
        staged_elapsed.as_secs_f64() * 1000.0
    );
    println!("duplicate_streamed_bytes={streamed_bytes}");
    println!(
        "duplicate_streamed_elapsed_ms={}",
        streamed_elapsed.as_secs_f64() * 1000.0
    );
    println!("duplicate_max_chunk_bytes={max_chunk}");
    println!(
        "duplicate_peak_rss_bytes={}",
        peak_resident_bytes().unwrap_or(0)
    );
}
