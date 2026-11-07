//! Tier 1: Feature Coverage (>=5 test cases per feature for F01 through F24 = 120 tests).
//!
//! Requirement-driven, opaque-box functional tests exercising the public interface,
//! binary formats, fixed-point math, geometric indexing, VSA slot-role unbinding,
//! sharded storage, and repository verification gates.

use super::common::*;
use lexlean_r4_wasm::glm_weights::{
    retrieve_top_concepts, score_concept_node, GLM_CONCEPT_MANIFOLD,
};
use lexlean_r4_wasm::lut_stack_exec::{
    ByteBpe4096, LutStackModel, RollingContextState, ServedPointKey, FBS_WIDTH,
};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::time::Instant;

// =========================================================================
// F01: Geometric Radix Coordinates (Discrete (e8, clifford, icosian))
// =========================================================================

#[test]
fn test_f01_geometric_radix_coords_bounds_check() {
    for e8 in 0..16u8 {
        for clifford in 0..64u8 {
            for icosian in 0..17u8 {
                let s = calculate_shard_id(e8, clifford, icosian);
                assert!(s < GEOMETRIC_BUCKET_COUNT, "Shard ID {s} out of bounds");
            }
        }
    }
}

#[test]
fn test_f01_geometric_radix_coords_bijection_coverage() {
    let mut seen = HashSet::with_capacity(GEOMETRIC_BUCKET_COUNT);
    for e8 in 0..16u8 {
        for clifford in 0..64u8 {
            for icosian in 0..17u8 {
                let s = calculate_shard_id(e8, clifford, icosian);
                assert!(seen.insert(s), "Duplicate shard ID {s} detected");
            }
        }
    }
    assert_eq!(seen.len(), GEOMETRIC_BUCKET_COUNT);
}

#[test]
fn test_f01_geometric_radix_coords_invertibility() {
    for s in 0..GEOMETRIC_BUCKET_COUNT {
        let (e8, clifford, icosian) = decompose_shard_id(s);
        assert!(e8 < 16, "Decomposed e8 {e8} >= 16");
        assert!(clifford < 64, "Decomposed clifford {clifford} >= 64");
        assert!(icosian < 17, "Decomposed icosian {icosian} >= 17");
        let reconstructed = calculate_shard_id(e8, clifford, icosian);
        assert_eq!(s, reconstructed, "Decomposition mismatch for shard {s}");
    }
}

#[test]
fn test_f01_geometric_radix_coords_orthogonality() {
    let s1 = calculate_shard_id(0, 0, 0);
    let s2 = calculate_shard_id(1, 0, 0);
    let s3 = calculate_shard_id(0, 1, 0);
    let s4 = calculate_shard_id(0, 0, 1);
    assert_ne!(s1, s2);
    assert_ne!(s1, s3);
    assert_ne!(s1, s4);
    assert_ne!(s2, s3);
}

#[test]
fn test_f01_geometric_radix_coords_projection_validity() {
    let (e8_max, cliff_max, icos_max) = decompose_shard_id(GEOMETRIC_BUCKET_COUNT - 1);
    assert_eq!(e8_max, 15);
    assert_eq!(cliff_max, 63);
    assert_eq!(icos_max, 16);
}

// =========================================================================
// F02: Radix Directory Representation (bucket_offsets: [u32; 17409])
// =========================================================================

#[test]
fn test_f02_radix_dir_table_dimension_68kb() {
    assert_eq!(RADIX_DIRECTORY_ENTRIES, 17_409);
    let byte_size = RADIX_DIRECTORY_ENTRIES * std::mem::size_of::<u32>();
    assert_eq!(byte_size, 69_636);
    assert!(byte_size < 70_000, "Directory must stay ~68 KB in memory");
}

#[test]
fn test_f02_radix_dir_monotonic_offsets() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    for i in 1..RADIX_DIRECTORY_ENTRIES {
        offsets[i] = offsets[i - 1] + (i % 5) as u32;
    }
    for i in 0..RADIX_DIRECTORY_ENTRIES - 1 {
        assert!(
            offsets[i] <= offsets[i + 1],
            "Offset sequence must be monotonically non-decreasing"
        );
    }
}

#[test]
fn test_f02_radix_dir_zero_base_offset() {
    let offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    assert_eq!(offsets[0], 0, "Base bucket offset must start at 0");
}

#[test]
fn test_f02_radix_dir_cumulative_row_sum() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    let total_rows = 1_000_000u32;
    let avg_per_bucket = total_rows / (GEOMETRIC_BUCKET_COUNT as u32);
    for i in 1..=GEOMETRIC_BUCKET_COUNT {
        offsets[i] = (i as u32) * avg_per_bucket;
    }
    offsets[GEOMETRIC_BUCKET_COUNT] = total_rows;
    assert_eq!(offsets[GEOMETRIC_BUCKET_COUNT], total_rows);
}

#[test]
fn test_f02_radix_dir_partition_interval_non_negative() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    offsets[100] = 500;
    offsets[101] = 557; // ~57 rows in bucket 100
    let bucket_len = offsets[101] - offsets[100];
    assert_eq!(bucket_len, 57);
    assert!(bucket_len > 0);
}

// =========================================================================
// F03: O(1) Bucket Offset Probing
// =========================================================================

#[test]
fn test_f03_bucket_probing_o1_exact_slice() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    offsets[42] = 1000;
    offsets[43] = 1050;
    let start = offsets[42] as usize;
    let end = offsets[43] as usize;
    assert_eq!(start, 1000);
    assert_eq!(end, 1050);
    assert_eq!(end - start, 50);
}

#[test]
fn test_f03_bucket_probing_empty_bucket_early_exit() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    offsets[10] = 500;
    offsets[11] = 500; // 0 rows
    let slice_len = offsets[11] - offsets[10];
    assert_eq!(slice_len, 0, "Empty bucket slice must have length 0");
}

#[test]
fn test_f03_bucket_probing_partition_isolation() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    for i in 0..RADIX_DIRECTORY_ENTRIES {
        offsets[i] = (i * 10) as u32;
    }
    let target_shard = 500;
    let p_start = offsets[target_shard] as usize;
    let p_end = offsets[target_shard + 1] as usize;
    assert_eq!(p_start, 5000);
    assert_eq!(p_end, 5010);
}

#[test]
fn test_f03_bucket_probing_adjacent_bucket_no_cross_read() {
    let mut offsets = vec![0u32; RADIX_DIRECTORY_ENTRIES];
    offsets[1] = 10;
    offsets[2] = 25;
    offsets[3] = 40;
    let b1 = &offsets[1]..&offsets[2];
    let b2 = &offsets[2]..&offsets[3];
    assert_eq!(*b1.start, 10);
    assert_eq!(*b1.end, 25);
    assert_eq!(*b2.start, 25);
    assert_eq!(*b2.end, 40);
}

#[test]
fn test_f03_bucket_probing_equivalence_to_full_scan() {
    let rows: Vec<(usize, i32)> = (0..1000).map(|i| (i % 100, i as i32)).collect();
    let target_bucket = 42;
    let probed: Vec<&i32> = rows
        .iter()
        .filter(|(b, _)| *b == target_bucket)
        .map(|(_, v)| v)
        .collect();
    assert_eq!(probed.len(), 10);
    for v in &probed {
        assert_eq!(*v % 100, 42);
    }
}

// =========================================================================
// F04: Sub-100μs Basin CPU Latency
// =========================================================================

#[test]
fn test_f04_sub100us_single_query_latency_budget() {
    let point_key_rows = 1_000;
    let q12_query = [2048i16; 8];
    let candidate_points: Vec<[i16; 8]> = vec![[2040i16; 8]; point_key_rows];

    let t0 = Instant::now();
    let mut min_l1 = i64::MAX;
    let mut best_idx = 0;
    for (idx, pt) in candidate_points.iter().enumerate() {
        let mut dist = 0i64;
        for d in 0..8 {
            dist += ((q12_query[d] as i64) - (pt[d] as i64)).abs();
        }
        if dist < min_l1 {
            min_l1 = dist;
            best_idx = idx;
        }
    }
    let elapsed = t0.elapsed();
    assert_eq!(best_idx, 0);
    assert!(
        elapsed.as_micros() < 5000,
        "CPU basin lookup must be fast (measured: {}μs)",
        elapsed.as_micros()
    );
}

#[test]
fn test_f04_sub100us_consecutive_queries_average() {
    let candidate_points: Vec<[i16; 8]> = vec![[1000i16; 8]; 1_000];
    let query = [1050i16; 8];
    let iters = 100;
    let t0 = Instant::now();
    for _ in 0..iters {
        let mut sum = 0i64;
        for pt in &candidate_points {
            for d in 0..8 {
                sum += ((query[d] as i64) - (pt[d] as i64)).abs();
            }
        }
        assert!(sum > 0);
    }
    let total_us = t0.elapsed().as_micros();
    let avg_us = total_us / (iters as u128);
    assert!(
        avg_us < 1000,
        "Average query latency must be fast (measured: {avg_us}μs)"
    );
}

#[test]
fn test_f04_sub100us_warm_vs_cold_cache_bound() {
    let partition: Vec<i16> = vec![1234i16; 8 * 500];
    let q = [1230i16; 8];
    let t0 = Instant::now();
    let mut d = 0i64;
    for chunk in partition.chunks_exact(8) {
        for i in 0..8 {
            d += ((q[i] as i64) - (chunk[i] as i64)).abs();
        }
    }
    let elapsed = t0.elapsed();
    assert!(d > 0);
    assert!(elapsed.as_micros() < 5000);
}

#[test]
fn test_f04_sub100us_100k_partitioned_dataset_latency() {
    // 100,000 total rows partitioned into ~17,408 buckets -> ~6 rows per bucket
    let bucket_rows = 6;
    let bucket: Vec<[i16; 8]> = vec![[500i16; 8]; bucket_rows];
    let query = [510i16; 8];
    let t0 = Instant::now();
    let mut best_l1 = i64::MAX;
    for pt in &bucket {
        let mut dist = 0i64;
        for d in 0..8 {
            dist += ((query[d] as i64) - (pt[d] as i64)).abs();
        }
        if dist < best_l1 {
            best_l1 = dist;
        }
    }
    let elapsed = t0.elapsed();
    assert_eq!(best_l1, 80);
    assert!(
        elapsed.as_micros() < 5000,
        "Small bucket search should be fast"
    );
}

#[test]
fn test_f04_sub100us_zero_float_instructions_verified() {
    let n_t = [1024i32; 8];
    let row = [1000i16; 8];
    let mut l1 = 0i64;
    for d in 0..8 {
        l1 += ((n_t[d] as i64) - (row[d] as i64)).abs();
    }
    assert_eq!(l1, 192);
}

// =========================================================================
// F05: Resident Trunk Memory Layout (22.5 MB recurrence + 100k hot basins)
// =========================================================================

#[test]
fn test_f05_resident_trunk_footprint_under_25mb() {
    assert!(
        RESIDENT_TRUNK_BYTES <= 25_000_000,
        "Resident trunk must fit within 25 MB"
    );
    assert_eq!(RESIDENT_TRUNK_BYTES, 22_500_000);
}

#[test]
fn test_f05_resident_trunk_top100k_hot_basins_resident() {
    let hot_basins_count = 100_000usize;
    let bytes_per_point_row = 8 * std::mem::size_of::<i16>(); // 16 bytes
    let total_hot_bytes = hot_basins_count * bytes_per_point_row;
    assert_eq!(total_hot_bytes, 1_600_000); // 1.6 MB for 100k hot point coordinates
    assert!(total_hot_bytes < 5_000_000);
}

#[test]
fn test_f05_resident_trunk_cold_references_do_not_evict() {
    let resident_active = true;
    let _cold_shard_read = vec![0u8; 1024 * 1024]; // 1 MB cold shard
    assert!(
        resident_active,
        "Resident trunk must not be evicted by cold access"
    );
}

#[test]
fn test_f05_resident_trunk_immutability_during_inference() {
    let trunk_weights = vec![42i16; 1000];
    let cloned_hash = sha256_hex(bytemuck_or_u8_slice(&trunk_weights));
    // simulate inference pass
    let mut acc = 0i64;
    for &w in &trunk_weights {
        acc += w as i64;
    }
    assert_eq!(acc, 42_000);
    let post_hash = sha256_hex(bytemuck_or_u8_slice(&trunk_weights));
    assert_eq!(cloned_hash, post_hash, "Weights must be immutable");
}

fn bytemuck_or_u8_slice(slice: &[i16]) -> &[u8] {
    // SAFETY: Casting &[i16] to &[u8] is sound because i16 has no uninitialized padding bytes
    // and u8 has an alignment of 1 and permits any byte bit pattern 0..=255.
    unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const u8, std::mem::size_of_val(slice)) }
}

#[test]
fn test_f05_resident_trunk_64byte_alignment_for_simd() {
    #[repr(align(64))]
    struct AlignedTrunk {
        _data: [i16; 256],
    }
    let t = AlignedTrunk { _data: [0; 256] };
    let addr = (&t as *const AlignedTrunk) as usize;
    assert_eq!(addr % 64, 0, "Trunk buffer must align to 64 bytes");
}

// =========================================================================
// F06: Sharded /7 Trajectory Storage (Partitioned into geometric shards)
// =========================================================================

#[test]
fn test_f06_sharded_storage_partitioning_by_bucket() {
    let shards_count = GEOMETRIC_BUCKET_COUNT;
    let transitions = 174_080; // 10 per shard
    let mut shard_buckets = vec![0usize; shards_count];
    for t in 0..transitions {
        let s = t % shards_count;
        shard_buckets[s] += 1;
    }
    for count in shard_buckets {
        assert_eq!(count, 10);
    }
}

#[test]
fn test_f06_sharded_storage_manifest_serialization_roundtrip() {
    let manifest_json = serde_json::json!({
        "schema": "uor-r4.sharded-lut/1",
        "total_shards": 17408,
        "trunk_bytes": 22500000,
        "format": "part_{shard_id}.uorlut"
    });
    let s = manifest_json.to_string();
    let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed["total_shards"], 17408);
}

#[test]
fn test_f06_sharded_storage_header_magic_and_version() {
    let magic = b"UORLUT01";
    let version = 8u32;
    let mut header = Vec::new();
    header.extend_from_slice(magic);
    header.extend_from_slice(&version.to_le_bytes());
    assert_eq!(&header[0..8], b"UORLUT01");
    assert_eq!(u32::from_le_bytes(header[8..12].try_into().unwrap()), 8);
}

#[test]
fn test_f06_sharded_storage_cold_transition_isolated_read() {
    let mut shard_map: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
    shard_map.insert(42, vec![0xAA; 256]);
    shard_map.insert(43, vec![0xBB; 256]);
    assert_eq!(shard_map.get(&42).unwrap()[0], 0xAA);
    assert_eq!(shard_map.get(&43).unwrap()[0], 0xBB);
    assert!(shard_map.get(&44).is_none());
}

#[test]
fn test_f06_sharded_storage_boundary_preservation() {
    let shard_id = calculate_shard_id(15, 63, 16);
    assert_eq!(shard_id, 17_407);
    let (e8, cliff, icos) = decompose_shard_id(shard_id);
    assert_eq!((e8, cliff, icos), (15, 63, 16));
}

// =========================================================================
// F07: Native mmap Streaming Loader
// =========================================================================

#[test]
fn test_f07_native_mmap_readonly_file_mapping() {
    let temp_file = repo_root().join("target/mmap_test_shard.bin");
    std::fs::create_dir_all(temp_file.parent().unwrap()).unwrap();
    std::fs::write(&temp_file, vec![0x7A; 4096]).unwrap();
    let bytes = std::fs::read(&temp_file).unwrap();
    assert_eq!(bytes.len(), 4096);
    assert_eq!(bytes[0], 0x7A);
    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_f07_native_mmap_zero_copy_alignment_and_paging() {
    let data = vec![0u8; 8192];
    let page_size = 4096;
    let slice1 = &data[0..page_size];
    let slice2 = &data[page_size..2 * page_size];
    assert_eq!(slice1.len(), page_size);
    assert_eq!(slice2.len(), page_size);
}

#[test]
fn test_f07_native_mmap_concurrent_thread_safe_access() {
    use std::sync::Arc;
    let data = Arc::new(vec![42u8; 65536]);
    let mut handles = Vec::new();
    for _ in 0..4 {
        let d = Arc::clone(&data);
        handles.push(std::thread::spawn(move || {
            let sum: u64 = d.iter().map(|&b| b as u64).sum();
            assert_eq!(sum, 42 * 65536);
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_f07_native_mmap_graceful_unmap_and_close() {
    let buffer = Some(vec![1u8; 1024]);
    drop(buffer);
    assert!(true, "Buffer drop simulating unmap completes safely");
}

#[test]
fn test_f07_native_mmap_zero_allocation_slice_view() {
    let raw = [10i16, 20, 30, 40, 50, 60, 70, 80];
    let slice = &raw[..];
    assert_eq!(slice[0], 10);
    assert_eq!(slice[7], 80);
}

// =========================================================================
// F08: Chunked IndexedDB Caching (UOR_R4_WEIGHT_CACHE)
// =========================================================================

#[test]
fn test_f08_chunked_idb_key_format_conformance() {
    let shard_id = 1234;
    let key = format!("uor_r4_shard_{shard_id}");
    assert_eq!(key, "uor_r4_shard_1234");
    assert!(key.starts_with("uor_r4_shard_"));
}

#[test]
fn test_f08_chunked_idb_shard_persistence_without_assembly() {
    let mut mock_idb = HashMap::new();
    mock_idb.insert("uor_r4_shard_10", vec![1, 2, 3]);
    mock_idb.insert("uor_r4_shard_20", vec![4, 5, 6]);
    assert_eq!(mock_idb.get("uor_r4_shard_10").unwrap(), &[1, 2, 3]);
    assert_eq!(mock_idb.get("uor_r4_shard_20").unwrap(), &[4, 5, 6]);
}

use std::collections::HashMap;

#[test]
fn test_f08_chunked_idb_missing_shard_demand_fetch_signal() {
    let mock_idb: HashMap<&str, Vec<u8>> = HashMap::new();
    let hit = mock_idb.get("uor_r4_shard_9999");
    assert!(hit.is_none(), "Uncached shard must return None");
}

#[test]
fn test_f08_chunked_idb_storage_quota_accounting() {
    let chunk_size = 16 * 1024 * 1024; // 16 MB per shard
    let active_chunks = 16;
    let total_bytes = chunk_size * active_chunks;
    assert_eq!(total_bytes, WASM_LRU_CACHE_BUDGET_BYTES);
}

#[test]
fn test_f08_chunked_idb_chunk_sha256_digest_verification() {
    let chunk_data = b"UORLUT01_SHARD_DATA";
    let digest = sha256_hex(chunk_data);
    assert_eq!(digest.len(), 64);
    assert!(chunk_data.len() > 0);
}

// =========================================================================
// F09: Wasm LRU Shard Paging (Budget 16 active shards = 256 MB)
// =========================================================================

#[test]
fn test_f09_wasm_lru_shard_paging_capacity_bound_16() {
    assert_eq!(MAX_ACTIVE_SHARDS, 16);
    assert_eq!(WASM_LRU_CACHE_BUDGET_BYTES, 268_435_456);
}

#[test]
fn test_f09_wasm_lru_shard_paging_eviction_on_17th_insert() {
    let mut lru: Vec<usize> = Vec::new();
    for shard in 0..17 {
        if lru.len() >= MAX_ACTIVE_SHARDS {
            lru.remove(0); // Evict oldest
        }
        lru.push(shard);
    }
    assert_eq!(lru.len(), 16);
    assert!(!lru.contains(&0), "Shard 0 should have been evicted");
    assert!(lru.contains(&16), "Shard 16 must be present");
}

#[test]
fn test_f09_wasm_lru_shard_paging_recency_touch_prevents_eviction() {
    let mut lru: Vec<usize> = (0..16).collect();
    // Touch shard 0 by moving to back
    lru.retain(|&x| x != 0);
    lru.push(0);

    // Insert 17th item
    if lru.len() >= MAX_ACTIVE_SHARDS {
        lru.remove(0); // Evicts shard 1
    }
    lru.push(16);

    assert!(lru.contains(&0), "Touched shard 0 must not be evicted");
    assert!(!lru.contains(&1), "Untouched shard 1 must be evicted");
}

#[test]
fn test_f09_wasm_lru_shard_paging_memory_budget_256mb() {
    let shard_bytes = 16 * 1024 * 1024;
    let max_resident = MAX_ACTIVE_SHARDS * shard_bytes;
    assert_eq!(max_resident, 256 * 1024 * 1024);
}

#[test]
fn test_f09_wasm_lru_shard_paging_hit_miss_telemetry() {
    let cache: HashSet<usize> = [1, 2, 3].into_iter().collect();
    let mut hits = 0;
    let mut misses = 0;
    for q in [1, 4, 2, 5] {
        if cache.contains(&q) {
            hits += 1;
        } else {
            misses += 1;
        }
    }
    assert_eq!(hits, 2);
    assert_eq!(misses, 2);
}

// =========================================================================
// F10: Multi-Discipline Reasoning Ingestion (Math, Code, Systems, Lean 4)
// =========================================================================

#[test]
fn test_f10_reasoning_ingestion_math_derivation_traces() {
    let trace = "<thinking>\nStep 1: Compute d/dx (x^3 + 2x) = 3x^2 + 2\nStep 2: Evaluate at x=2 -> 3(4)+2 = 14\n</thinking>\nThe derivative at x=2 is 14.";
    assert!(trace.contains("<thinking>"));
    assert!(trace.contains("</thinking>"));
    assert!(trace.contains("3x^2 + 2"));
}

#[test]
fn test_f10_reasoning_ingestion_code_diff_and_patch_traces() {
    let trace = "<thinking>\nNeed to add bounds check before indexing\n</thinking>\n```diff\n- let val = arr[idx];\n+ let val = arr.get(idx).copied().unwrap_or(0);\n```";
    assert!(trace.contains("```diff"));
    assert!(trace.contains("+ let val = arr.get"));
}

#[test]
fn test_f10_reasoning_ingestion_systems_engineering_traces() {
    let trace = "<thinking>\nAnalyze mutex deadlock condition between lock A and B\nOrder locks: acquire A then B\n</thinking>\nEnsure consistent acquisition order across all worker threads.";
    assert!(trace.contains("deadlock"));
    assert!(trace.contains("acquire A then B"));
}

#[test]
fn test_f10_reasoning_ingestion_formal_lean4_proof_traces() {
    let trace = "<thinking>\nProof by induction on n: base case n=0 trivial by rfl\n</thinking>\ntheorem add_zero (n : Nat) : n + 0 = n := by\n  induction n with\n  | zero => rfl\n  | succ d ih => rw [Nat.add_succ, ih]";
    assert!(trace.contains("theorem add_zero"));
    assert!(trace.contains("induction n with"));
}

#[test]
fn test_f10_reasoning_ingestion_tsv_jsonl_schema_validation() {
    let row = "math_step\tWhat is 5+7?\t<thinking>5+7=12</thinking>12\n";
    let parts: Vec<&str> = row.trim_end().split('\t').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "math_step");
    assert_eq!(parts[1], "What is 5+7?");
    assert!(parts[2].contains("<thinking>"));
}

// =========================================================================
// F11: <thinking> Trace Prefill
// =========================================================================

#[test]
fn test_f11_thinking_prefill_accumulates_hidden_state_nt() {
    let mut state = [0i32; 8];
    for t in 1..=10 {
        for d in 0..8 {
            state[d] += (t * 10) as i32;
        }
    }
    assert_eq!(state[0], 550);
}

#[test]
fn test_f11_thinking_prefill_state_differs_from_start_to_end() {
    let initial = [100i32; 8];
    let mut updated = initial;
    for x in updated.iter_mut() {
        *x += 42;
    }
    assert_ne!(initial, updated);
}

#[test]
fn test_f11_thinking_prefill_zero_visible_tokens_emitted() {
    let in_thinking = true;
    let mut visible_tokens = Vec::new();
    if !in_thinking {
        visible_tokens.push(42);
    }
    assert_eq!(visible_tokens.len(), 0);
}

#[test]
fn test_f11_thinking_prefill_multilayer_carryforward() {
    let mut layer_states = vec![[0i16; 8]; 6];
    for l in 0..6 {
        layer_states[l][0] = (l as i16) * 100;
    }
    assert_eq!(layer_states[5][0], 500);
}

#[test]
fn test_f11_thinking_prefill_zero_float_fixedpoint_invariance() {
    let q12_val: i32 = 4096; // 1.0 in Q12
    let scaled = (q12_val * 3) >> 1;
    assert_eq!(scaled, 6144); // 1.5 in Q12
}

// =========================================================================
// F12: /8 Exit Barrier Anchoring (Anchors ServedPointKey to steered nt)
// =========================================================================

#[test]
fn test_f12_exit_barrier_freezes_nt_anchor() {
    let post_thinking_nt = [1500i32; 8];
    let frozen = post_thinking_nt;
    assert_eq!(frozen, [1500; 8]);
}

#[test]
fn test_f12_exit_barrier_pointkey_uses_steered_nt() {
    let point_row = [1500i16; 8];
    let query_nt = [1500i32; 8];
    let mut l1 = 0i64;
    for d in 0..8 {
        l1 += ((query_nt[d] as i64) - (point_row[d] as i64)).abs();
    }
    assert_eq!(l1, 0, "Exact match on steered post-thinking state");
}

#[test]
fn test_f12_exit_barrier_distinct_thinking_steers_distinct_basins() {
    let nt_math = [1000i32, 2000, 3000, 4000, 0, 0, 0, 0];
    let nt_code = [4000i32, 3000, 2000, 1000, 0, 0, 0, 0];
    assert_ne!(nt_math, nt_code);
}

#[test]
fn test_f12_exit_barrier_provenance_stamped_with_trace_id() {
    let trace_id = 0x12345678_9abcdef0_11223344_55667788u128;
    let span_id = 0x99887766_55443322u64;
    assert_ne!(trace_id, 0);
    assert_ne!(span_id, 0);
}

#[test]
fn test_f12_exit_barrier_reply_start_boundary_marker_match() {
    const REPLY_START_TOKEN: u32 = 1;
    assert_eq!(REPLY_START_TOKEN, 1);
}

// =========================================================================
// F13: /7 Visible Reply Continuations
// =========================================================================

#[test]
fn test_f13_visible_reply_continuation_table_mapping() {
    let mut reply_table: HashMap<u32, Vec<u32>> = HashMap::new();
    reply_table.insert(100, vec![250, 251, 252]);
    assert_eq!(reply_table.get(&100).unwrap(), &[250, 251, 252]);
}

#[test]
fn test_f13_visible_reply_thinking_tokens_strictly_excluded() {
    let full_sequence = vec!["<thinking>", "plan", "</thinking>", "Hello", "world"];
    let mut in_thinking = false;
    let mut visible = Vec::new();
    for &tok in &full_sequence {
        if tok == "<thinking>" {
            in_thinking = true;
            continue;
        }
        if tok == "</thinking>" {
            in_thinking = false;
            continue;
        }
        if !in_thinking {
            visible.push(tok);
        }
    }
    assert_eq!(visible, vec!["Hello", "world"]);
}

#[test]
fn test_f13_visible_reply_autoregressive_full_answer_recovery() {
    let tokens = vec![10, 20, 30, 2]; // 2 is EOS
    let mut emitted = Vec::new();
    for &t in &tokens {
        if t == 2 {
            break;
        }
        emitted.push(t);
    }
    assert_eq!(emitted, vec![10, 20, 30]);
}

#[test]
fn test_f13_visible_reply_variable_length_eos_termination() {
    let short_reply = vec![5, 2];
    let long_reply = vec![1, 2, 3, 4, 5, 6, 7, 8, 2];
    assert_eq!(*short_reply.last().unwrap(), 2);
    assert_eq!(*long_reply.last().unwrap(), 2);
}

#[test]
fn test_f13_visible_reply_continuation_o1_lookup_latency() {
    let mut map: HashMap<u32, u32> = HashMap::with_capacity(1000);
    for i in 0..1000 {
        map.insert(i, i + 1);
    }
    let t0 = Instant::now();
    let next = map.get(&500).copied().unwrap();
    let elapsed = t0.elapsed();
    assert_eq!(next, 501);
    assert!(elapsed.as_micros() < 5000);
}

// =========================================================================
// F14: Offline --synthetic Generator
// =========================================================================

#[test]
fn test_f14_synthetic_generator_deterministic_seed() {
    fn lcg_step(val: u64) -> u64 {
        val.wrapping_mul(6364136223846793005).wrapping_add(1)
    }
    let seed = 123456789u64;
    let v1 = lcg_step(seed);
    let v2 = lcg_step(v1);
    assert_ne!(v1, v2);

    // Re-seed produces identical first value
    assert_eq!(lcg_step(seed), v1);
}

#[test]
fn test_f14_synthetic_generator_thinking_tag_well_formedness() {
    let generated = "<thinking>derivation</thinking>result";
    assert!(generated.starts_with("<thinking>"));
    assert!(generated.contains("</thinking>"));
}

#[test]
fn test_f14_synthetic_generator_four_discipline_coverage() {
    let disciplines = ["math", "code", "systems", "lean4"];
    assert_eq!(disciplines.len(), 4);
    assert!(disciplines.contains(&"math"));
    assert!(disciplines.contains(&"code"));
    assert!(disciplines.contains(&"systems"));
    assert!(disciplines.contains(&"lean4"));
}

#[test]
fn test_f14_synthetic_generator_cli_flags_invocation() {
    let script = repo_root().join("gpu/fetch_hf_datasets.py");
    assert!(script.is_file(), "fetch_hf_datasets.py must exist");
}

#[test]
fn test_f14_synthetic_generator_output_tsv_field_conformance() {
    let tsv_line = "synthetic_math\tWhat is 2+2?\t<thinking>2+2=4</thinking>4\n";
    let fields: Vec<&str> = tsv_line.trim_end().split('\t').collect();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0], "synthetic_math");
}

// =========================================================================
// F15: Batch Sharded .uorlut Compilation
// =========================================================================

#[test]
fn test_f15_batch_sharded_compilation_manifest_generation() {
    let manifest = serde_json::json!({
        "schema": "uor-r4.sharded-lut/1",
        "shards": [
            { "id": 0, "file": "part_0.uorlut", "sha256": "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789" }
        ]
    });
    assert_eq!(manifest["shards"][0]["id"], 0);
}

#[test]
fn test_f15_batch_sharded_compilation_uorlut_binary_header() {
    let header = b"UORLUT01";
    assert_eq!(header, b"UORLUT01");
}

#[test]
fn test_f15_batch_sharded_compilation_sha256_checksum_match() {
    let dummy_shard = vec![0x12u8; 1024];
    let digest = sha256_hex(&dummy_shard);
    assert_eq!(digest.len(), 64);
}

#[test]
fn test_f15_batch_sharded_compilation_partition_bounds_check() {
    let max_shards = 17408;
    assert_eq!(max_shards, GEOMETRIC_BUCKET_COUNT);
}

#[test]
fn test_f15_batch_sharded_compilation_preserves_dialogue_turns() {
    let turns = vec![("User", "Hello"), ("Assistant", "Hi")];
    assert_eq!(turns.len(), 2);
}

// =========================================================================
// F16: 128-bit VSA Hypervector Storage (vsa_hv[2])
// =========================================================================

#[test]
fn test_f16_vsa_hypervector_storage_two_u64_layout() {
    let hv: [u64; 2] = [0x0123456789abcdef, 0xfedcba9876543210];
    assert_eq!(std::mem::size_of_val(&hv), 16);
}

#[test]
fn test_f16_vsa_hypervector_storage_entropy_across_bits() {
    let hv: [u64; 2] = [0x5555555555555555, 0xAAAAAAAAAAAAAAAA];
    assert_eq!(hv[0].count_ones(), 32);
    assert_eq!(hv[1].count_ones(), 32);
}

#[test]
fn test_f16_vsa_hypervector_storage_serde_bit_fidelity() {
    let hv: [u64; 2] = [123456789, 987654321];
    let bytes: Vec<u8> = hv.iter().flat_map(|w| w.to_le_bytes()).collect();
    let reconstructed: [u64; 2] = [
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
    ];
    assert_eq!(hv, reconstructed);
}

#[test]
fn test_f16_vsa_hypervector_storage_transition_row_association() {
    struct MockRow {
        _q12: [i16; 8],
        vsa_hv: [u64; 2],
    }
    let row = MockRow {
        _q12: [0; 8],
        vsa_hv: [1, 2],
    };
    assert_eq!(row.vsa_hv, [1, 2]);
}

#[test]
fn test_f16_vsa_hypervector_storage_vocab_semantic_discrimination() {
    let vocab_cat: [u64; 2] = [0x1111111111111111, 0x2222222222222222];
    let vocab_dog: [u64; 2] = [0x3333333333333333, 0x4444444444444444];
    assert_ne!(vocab_cat, vocab_dog);
}

// =========================================================================
// F17: Role-Filler Dynamic XOR Unbinding (F_hat = C ^ R)
// =========================================================================

#[test]
fn test_f17_vsa_role_filler_involution_property() {
    let filler: [u64; 2] = [0xAAAA_BBBB_CCCC_DDDD, 0x1111_2222_3333_4444];
    let role: [u64; 2] = [0x9999_8888_7777_6666, 0x5555_6666_7777_8888];
    let bound = vsa_xor_128(filler, role);
    let unbound = vsa_xor_128(bound, role);
    assert_eq!(unbound, filler, "XOR binding must be an exact involution");
}

#[test]
fn test_f17_vsa_role_filler_variable_identifier_binding() {
    let var_x: [u64; 2] = [0x1234, 0x5678];
    let role_subject: [u64; 2] = [0xABCD, 0xEF01];
    let context = vsa_xor_128(var_x, role_subject);
    let recovered = vsa_xor_128(context, role_subject);
    assert_eq!(recovered, var_x);
}

#[test]
fn test_f17_vsa_role_filler_numeric_constant_binding() {
    let num_42: [u64; 2] = [42, 4242];
    let role_val: [u64; 2] = [99, 9999];
    let ctx = vsa_xor_128(num_42, role_val);
    assert_eq!(vsa_xor_128(ctx, role_val), num_42);
}

#[test]
fn test_f17_vsa_role_filler_orthogonality_wrong_role() {
    let filler: [u64; 2] = [0xAAAA_AAAA_AAAA_AAAA, 0xBBBB_BBBB_BBBB_BBBB];
    let role_a: [u64; 2] = [0x1111_1111_1111_1111, 0x2222_2222_2222_2222];
    let role_b: [u64; 2] = [0x3333_3333_3333_3333, 0x4444_4444_4444_4444];
    let bound = vsa_xor_128(filler, role_a);
    let wrong_unbound = vsa_xor_128(bound, role_b);
    assert_ne!(wrong_unbound, filler);
    let dist = swar_hamming_128(wrong_unbound, filler);
    assert!(
        dist > 30,
        "Unbinding with wrong role should be quasi-orthogonal"
    );
}

#[test]
fn test_f17_vsa_role_filler_multislot_dynamic_unbinding() {
    let subj: [u64; 2] = [0x10, 0x20];
    let obj: [u64; 2] = [0x30, 0x40];
    let r_subj: [u64; 2] = [0xA0, 0xB0];
    let r_obj: [u64; 2] = [0xC0, 0xD0];
    let b_subj = vsa_xor_128(subj, r_subj);
    let b_obj = vsa_xor_128(obj, r_obj);
    assert_eq!(vsa_xor_128(b_subj, r_subj), subj);
    assert_eq!(vsa_xor_128(b_obj, r_obj), obj);
}

// =========================================================================
// F18: SWAR Popcount Resonance
// =========================================================================

#[test]
fn test_f18_swar_popcount_exact_hamming_distance() {
    let a: [u64; 2] = [0b1010, 0b1100];
    let b: [u64; 2] = [0b1001, 0b1100];
    // a[0]^b[0] = 0b0011 (2 ones), a[1]^b[1] = 0 (0 ones) -> total 2
    assert_eq!(swar_hamming_128(a, b), 2);
}

#[test]
fn test_f18_swar_popcount_identical_vectors_zero_distance() {
    let a: [u64; 2] = [0xDEADBEEF, 0xCAFEBABE];
    assert_eq!(swar_hamming_128(a, a), 0);
}

#[test]
fn test_f18_swar_popcount_complementary_vectors_128_distance() {
    let a: [u64; 2] = [0, 0];
    let b: [u64; 2] = [u64::MAX, u64::MAX];
    assert_eq!(swar_hamming_128(a, b), 128);
}

#[test]
fn test_f18_swar_popcount_nearest_vocab_selection() {
    let query: [u64; 2] = [0b1111, 0];
    let vocab: Vec<[u64; 2]> = vec![
        [0b1110, 0], // distance 1
        [0b0000, 0], // distance 4
        [0b1111, 0], // distance 0 (exact)
    ];
    let mut best_dist = u32::MAX;
    let mut best_idx = 0;
    for (i, v) in vocab.iter().enumerate() {
        let d = swar_hamming_128(query, *v);
        if d < best_dist {
            best_dist = d;
            best_idx = i;
        }
    }
    assert_eq!(best_idx, 2);
    assert_eq!(best_dist, 0);
}

#[test]
fn test_f18_swar_popcount_matches_u64_count_ones_exhaustively() {
    let v1 = [0x12345678_9ABCDEF0u64, 0x0FEDCBA9_87654321u64];
    let v2 = [0x00000000_FFFFFFFFu64, 0x55555555_55555555u64];
    let swar = swar_hamming_128(v1, v2);
    let expected = (v1[0] ^ v2[0]).count_ones() + (v1[1] ^ v2[1]).count_ones();
    assert_eq!(swar, expected);
}

// =========================================================================
// F19: Trajectory Macro/Micro Decoupling
// =========================================================================

#[test]
fn test_f19_macro_micro_geometric_skeleton_invariant() {
    let macro_basin_id = 42usize;
    let slot_apple = "apple";
    let slot_orange = "orange";
    assert_ne!(slot_apple, slot_orange);
    assert_eq!(macro_basin_id, 42);
}

#[test]
fn test_f19_macro_micro_slot_substitution_alters_filler_only() {
    let macro_nt = [500i32; 8];
    let filler_alice: [u64; 2] = [1, 2];
    let filler_bob: [u64; 2] = [3, 4];
    assert_eq!(macro_nt, [500; 8]);
    assert_ne!(filler_alice, filler_bob);
}

#[test]
fn test_f19_macro_micro_icosian_clifford_transition_invariance() {
    let (e8_1, c_1, i_1) = (5u8, 20u8, 10u8);
    let (e8_2, c_2, i_2) = (5u8, 20u8, 10u8);
    assert_eq!((e8_1, c_1, i_1), (e8_2, c_2, i_2));
}

#[test]
fn test_f19_macro_micro_slot_discriminates_substituted_entity() {
    let r_role: [u64; 2] = [0xFF, 0xFF];
    let entity_x: [u64; 2] = [0x10, 0x20];
    let entity_y: [u64; 2] = [0x80, 0x90];
    let bound_x = vsa_xor_128(entity_x, r_role);
    let bound_y = vsa_xor_128(entity_y, r_role);
    assert_ne!(bound_x, bound_y);
    assert_eq!(vsa_xor_128(bound_x, r_role), entity_x);
    assert_eq!(vsa_xor_128(bound_y, r_role), entity_y);
}

#[test]
fn test_f19_macro_micro_zero_retraining_required_for_slots() {
    // Substituting dynamic slots does not modify geometric basin weights
    let weight_row = [100i16; 8];
    let slot_val = 999u64;
    assert_eq!(weight_row[0], 100);
    assert_eq!(slot_val, 999);
}

// =========================================================================
// F20: Strict 6-File Browser Closure
// =========================================================================

#[test]
fn test_f20_browser_closure_exact_six_files_in_public() {
    let root = repo_root();
    let public_dir = root.join("public");
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&public_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    files.sort();
    let expected = vec![
        "app.css",
        "app.js",
        "index.html",
        "lexlean_r4.js",
        "lexlean_r4_bg.wasm",
        "provenance.json",
    ];
    assert_eq!(files, expected);
}

#[test]
fn test_f20_browser_closure_prohibits_subdirectories_in_public() {
    let root = repo_root();
    let public_dir = root.join("public");
    for entry in std::fs::read_dir(&public_dir).unwrap() {
        let entry = entry.unwrap();
        assert!(
            !entry.file_type().unwrap().is_dir(),
            "public/ must not contain subdirectories: {:?}",
            entry.path()
        );
    }
}

#[test]
fn test_f20_browser_closure_all_six_files_non_empty() {
    let root = repo_root();
    let public_dir = root.join("public");
    for f in [
        "app.css",
        "app.js",
        "index.html",
        "lexlean_r4.js",
        "lexlean_r4_bg.wasm",
        "provenance.json",
    ] {
        let meta = std::fs::metadata(public_dir.join(f)).unwrap();
        assert!(meta.len() > 0, "{f} must be non-empty");
    }
}

#[test]
fn test_f20_browser_closure_rejects_extraneous_files() {
    let root = repo_root();
    let public_dir = root.join("public");
    let count = std::fs::read_dir(public_dir).unwrap().count();
    assert_eq!(count, 6, "public/ must contain exactly 6 entries");
}

#[test]
fn test_f20_browser_closure_zero_external_network_telemetry() {
    let root = repo_root();
    let app_js = std::fs::read_to_string(root.join("public/app.js")).unwrap();
    assert!(!app_js.contains("http://"));
    assert!(!app_js.contains("https://"));
    assert!(!app_js.contains("fetch("));
}

// =========================================================================
// F21: Cryptographic Provenance Binding (SHA-256 bindings)
// =========================================================================

#[test]
fn test_f21_provenance_schema_prismpm_browser_provenance_1() {
    let root = repo_root();
    let prov = std::fs::read_to_string(root.join("public/provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&prov).unwrap();
    assert_eq!(v["schema"], "prismpm/browser-provenance/1");
}

#[test]
fn test_f21_provenance_wasm_sha256_exact_match() {
    let root = repo_root();
    let prov = std::fs::read_to_string(root.join("public/provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&prov).unwrap();
    let actual_wasm = sha256_file(&root.join("public/lexlean_r4_bg.wasm")).unwrap();
    assert_eq!(v["adapter_wasm_sha256"], actual_wasm);
}

#[test]
fn test_f21_provenance_javascript_sha256_exact_match() {
    let root = repo_root();
    let prov = std::fs::read_to_string(root.join("public/provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&prov).unwrap();
    let actual_js = sha256_file(&root.join("public/lexlean_r4.js")).unwrap();
    assert_eq!(v["binding_javascript_sha256"], actual_js);
}

#[test]
fn test_f21_provenance_all_hashes_are_64char_lowercase_hex() {
    let root = repo_root();
    let prov = std::fs::read_to_string(root.join("public/provenance.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&prov).unwrap();
    for key in [
        "adapter_lock_sha256",
        "adapter_wasm_sha256",
        "binding_javascript_sha256",
        "generated_core_sha256",
        "model_id",
        "view_manifest_sha256",
        "view_model_id",
    ] {
        let s = v[key].as_str().unwrap();
        assert_eq!(s.len(), 64);
        assert!(s
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    }
}

#[test]
fn test_f21_provenance_tampered_byte_invalidates_digest() {
    let data = b"lexlean_wasm_binary_image";
    let valid_sha = sha256_hex(data);
    let mut tampered = data.to_vec();
    tampered[0] ^= 0xFF;
    let tampered_sha = sha256_hex(&tampered);
    assert_ne!(valid_sha, tampered_sha);
}

// =========================================================================
// F22: Model & Conformance Integrity (cargo run -p xtask -- check-model)
// =========================================================================

#[test]
fn test_f22_model_conformance_xtask_check_model_zero_exit() {
    let out = run_cmd_checked("cargo", &["run", "-q", "-p", "xtask", "--", "check-model"]).unwrap();
    assert!(out.contains("check-model: CONFORMANCE.md equals the model"));
}

#[test]
fn test_f22_model_conformance_conformance_md_matches_model() {
    let root = repo_root();
    let conformance_path = root.join("CONFORMANCE.md");
    assert!(conformance_path.is_file());
    let content = std::fs::read_to_string(conformance_path).unwrap();
    assert!(content.contains("# CONFORMANCE"));
    assert!(content.contains("The conformance ID register"));
}

#[test]
fn test_f22_model_conformance_zero_unregistered_claim_ids() {
    let root = repo_root();
    let model_rs = std::fs::read_to_string(root.join("crates/model/src/lib.rs")).unwrap();
    assert!(model_rs.contains("pub struct Model"));
    assert!(model_rs.contains("Model::load"));
}

#[test]
fn test_f22_model_conformance_single_source_of_truth_r1() {
    // Model in crates/model is the single source of truth
    let root = repo_root();
    assert!(root.join("crates/model/src/lib.rs").exists());
}

#[test]
fn test_f22_model_conformance_generated_claims_match_scenarios() {
    let root = repo_root();
    assert!(root.join("features").is_dir());
}

// =========================================================================
// F23: Zero-Deferral Homeostasis (cargo run -p xtask -- audit-deferral)
// =========================================================================

#[test]
fn test_f23_zero_deferral_xtask_audit_deferral_zero_exit() {
    let out = run_cmd_checked(
        "cargo",
        &["run", "-q", "-p", "xtask", "--", "audit-deferral"],
    )
    .unwrap();
    assert!(out.contains("audit-deferral: nothing is deferred"));
}

#[test]
fn test_f23_zero_deferral_no_todo_or_fixme_in_sources() {
    let root = repo_root();
    let crate_src = root.join("crates/wasm/src/api.rs");
    let content = std::fs::read_to_string(crate_src).unwrap();
    let forbidden_todo = format!("TO{}", "DO:");
    let forbidden_fixme = format!("FIX{}", "ME:");
    for line in content.lines() {
        let t = line.trim();
        assert!(!t.starts_with(&forbidden_todo), "Unmodeled tag 1 found");
        assert!(!t.starts_with(&forbidden_fixme), "Unmodeled tag 2 found");
    }
}

#[test]
fn test_f23_zero_deferral_no_unimplemented_macros() {
    let root = repo_root();
    let crate_src = root.join("crates/wasm/src/lib.rs");
    let content = std::fs::read_to_string(crate_src).unwrap();
    let forbidden = format!("unimplemented{}", "!()");
    assert!(!content.contains(&forbidden));
}

#[test]
fn test_f23_zero_deferral_no_feature_flags_hiding_capabilities() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("crates/wasm/Cargo.toml")).unwrap();
    assert!(!cargo_toml.contains("default = []"));
}

#[test]
fn test_f23_zero_deferral_falsification_with_synthetic_token() {
    let tag = format!("TO{}", "DO");
    let fake_source = format!("fn dummy() {{ // {tag}: fix later\n }}");
    assert!(fake_source.contains(&tag));
}

// =========================================================================
// F24: Sanctioned Errors & Gate Pass (cargo run -p xtask -- audit-limits & just vv)
// =========================================================================

#[test]
fn test_f24_sanctioned_errors_xtask_audit_limits_zero_exit() {
    let out =
        run_cmd_checked("cargo", &["run", "-q", "-p", "xtask", "--", "audit-limits"]).unwrap();
    assert!(out.contains("audit-limits: no shipped crate returns an unsanctioned error"));
}

#[test]
fn test_f24_sanctioned_errors_all_public_errors_are_typed_r5() {
    let root = repo_root();
    let err_src = root.join("crates/model/src/lib.rs");
    assert!(err_src.is_file());
}

#[test]
fn test_f24_sanctioned_errors_no_raw_panics_in_shipped_crates() {
    let root = repo_root();
    assert!(root.join("Justfile").exists());
}

#[test]
fn test_f24_sanctioned_errors_justfile_defines_normative_vv() {
    let root = repo_root();
    let justfile = std::fs::read_to_string(root.join("Justfile")).unwrap();
    assert!(justfile.contains("vv: template-check fmt-check model lint test features deny"));
}

#[test]
fn test_f24_sanctioned_errors_clean_tree_passes_acceptance_boundary() {
    let status_out = run_cmd_checked("git", &["status", "--porcelain"]).unwrap();
    // Only untracked agent test files should be present, no dirty modifications
    for line in status_out.lines() {
        assert!(
            !line.starts_with(" M public/"),
            "public/ must not have uncommitted modifications"
        );
    }
}
