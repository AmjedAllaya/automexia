//! Reproducible CP5.0 research benchmark.
//!
//! This is intentionally a standalone workspace. It compares the existing
//! deterministic Quick Action matcher with nucleo-matcher without adding the
//! candidate dependency to any Automexia runtime, library, or release binary.

use std::hint::black_box;
use std::time::{Duration, Instant};

use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};

const CORPUS_SIZES: [usize; 3] = [32, 128, 512];
const WARMUPS: usize = 20;
const SAMPLES: usize = 200;
const MAX_P95: Duration = Duration::from_millis(16);
const QUERIES: [&str; 8] = [
    "kdep",
    "tfws",
    "auto env",
    "déplo",
    "δοκ",
    "東prod",
    "café",
    "service-0042",
];
const SEEDS: [&str; 12] = [
    "kubectl get deployments namespace production",
    "terraform workspace select production",
    "automexia environment cluster region project action",
    "déployer le service café production",
    "déployer le service café staging",
    "δοκιμή υπηρεσία παραγωγή",
    "東京 production cluster deploy",
    "aws eks update kubeconfig production",
    "az account set subscription staging",
    "gcloud config set project production",
    "ssh bastion production service",
    "service long-common-prefix environment provider region",
];

#[derive(Clone, Copy)]
struct Measurement {
    p50_ns: u128,
    p95_ns: u128,
    max_ns: u128,
    checksum: u64,
}

#[derive(Clone, Copy)]
struct IndexedCandidate<'a> {
    index: usize,
    value: &'a str,
}

impl AsRef<str> for IndexedCandidate<'_> {
    fn as_ref(&self) -> &str {
        self.value
    }
}
fn corpus(size: usize) -> Vec<String> {
    (0..size)
        .map(|index| {
            let seed = SEEDS[index % SEEDS.len()];
            format!(
                "{seed} service-{index:04} automexia-provider-environment-cluster-region-project"
            )
        })
        .collect()
}

// This mirrors automexia-devops/src/actions/activation.rs. The benchmark keeps
// the copy local so research cannot widen that crate's public API.
fn in_tree_score(query: &str, candidate: &str) -> Option<i32> {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let mut score = 0i32;
    let mut position = 0usize;
    let chars = candidate.char_indices().collect::<Vec<_>>();
    for needle in query.chars() {
        let relative = chars[position..]
            .iter()
            .position(|(_, value)| *value == needle)?;
        let index = position + relative;
        score += 100 - i32::try_from(relative.min(90)).unwrap_or(90);
        if index == 0
            || chars[index - 1].1.is_whitespace()
            || "-._/".contains(chars[index - 1].1)
        {
            score += 30;
        }
        position = index + 1;
    }
    Some(score - i32::try_from(candidate.chars().count().min(256)).unwrap_or(256))
}

fn in_tree_match_list(query: &str, candidates: &[String]) -> Vec<(usize, i32)> {
    let mut matches = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            in_tree_score(query, candidate).map(|score| (index, score))
        })
        .collect::<Vec<_>>();
    matches.sort_unstable_by(|left, right| {
        right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0))
    });
    matches
}

fn nucleo_match_list(query: &str, candidates: &[String]) -> Vec<(usize, u32)> {
    let pattern = Pattern::new(
        query,
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut matcher = Matcher::new(Config::DEFAULT);
    pattern
        .match_list(
            candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| IndexedCandidate {
                    index,
                    value: candidate,
                }),
            &mut matcher,
        )
        .into_iter()
        .map(|(candidate, score)| (candidate.index, score))
        .collect()
}

fn percentile(samples: &[Duration], numerator: usize, denominator: usize) -> u128 {
    let rank = (samples.len() * numerator).div_ceil(denominator);
    samples[rank.saturating_sub(1)].as_nanos()
}

fn measure<F>(mut operation: F) -> Measurement
where
    F: FnMut() -> u64,
{
    for _ in 0..WARMUPS {
        black_box(operation());
    }
    let mut elapsed = Vec::with_capacity(SAMPLES);
    let mut checksum = 0u64;
    for _ in 0..SAMPLES {
        let started = Instant::now();
        checksum = checksum.wrapping_add(black_box(operation()));
        elapsed.push(started.elapsed());
    }
    elapsed.sort_unstable();
    Measurement {
        p50_ns: percentile(&elapsed, 50, 100),
        p95_ns: percentile(&elapsed, 95, 100),
        max_ns: elapsed.last().copied().unwrap_or_default().as_nanos(),
        checksum,
    }
}

fn benchmark_in_tree(candidates: &[String]) -> u64 {
    QUERIES
        .iter()
        .map(|query| {
            let matches = in_tree_match_list(black_box(query), black_box(candidates));
            matches
                .iter()
                .fold(matches.len() as u64, |checksum, (index, score)| {
                    checksum
                        .wrapping_mul(16777619)
                        .wrapping_add(*index as u64)
                        .wrapping_add((*score as i64 as u64).rotate_left(7))
                })
        })
        .fold(0, u64::wrapping_add)
}

fn benchmark_nucleo(candidates: &[String]) -> u64 {
    QUERIES
        .iter()
        .map(|query| {
            let matches = nucleo_match_list(black_box(query), black_box(candidates));
            matches
                .iter()
                .fold(matches.len() as u64, |checksum, (index, score)| {
                    checksum
                        .wrapping_mul(16777619)
                        .wrapping_add(*index as u64)
                        .wrapping_add(u64::from(*score).rotate_left(7))
                })
        })
        .fold(0, u64::wrapping_add)
}

fn stale_generation(candidates: &[String], requested_generation: u64) -> usize {
    let mut processed = 0usize;
    for (index, candidate) in candidates.iter().enumerate() {
        // The fake generation changes deterministically after seven candidates.
        // A real bridge would read a pane-owned atomic generation here.
        let active_generation = if index < 7 {
            requested_generation
        } else {
            requested_generation.wrapping_add(1)
        };
        if active_generation != requested_generation {
            break;
        }
        black_box(in_tree_score("service", candidate));
        processed += 1;
    }
    processed
}

fn main() {
    if std::env::args_os().nth(1).as_deref()
        == Some(std::ffi::OsStr::new("--startup-probe"))
    {
        black_box(Matcher::new(Config::DEFAULT));
        return;
    }

    let mut rows = Vec::new();
    let mut exceeded = false;
    for size in CORPUS_SIZES {
        let candidates = corpus(size);
        let in_tree = measure(|| benchmark_in_tree(&candidates));
        let nucleo = measure(|| benchmark_nucleo(&candidates));
        exceeded |= Duration::from_nanos(in_tree.p95_ns as u64) > MAX_P95;
        exceeded |= Duration::from_nanos(nucleo.p95_ns as u64) > MAX_P95;
        rows.push((size, in_tree, nucleo));
    }
    let stale_candidates_processed = stale_generation(&corpus(512), 42);
    if stale_candidates_processed != 7 {
        eprintln!("stale generation was not rejected at the deterministic boundary");
        std::process::exit(2);
    }

    println!("{{");
    println!("  \"schema\": 1,");
    println!("  \"nucleo_version\": \"0.3.1\",");
    println!("  \"warmups\": {WARMUPS},");
    println!("  \"samples\": {SAMPLES},");
    println!("  \"queries_per_sample\": {},", QUERIES.len());
    println!("  \"rows\": [");
    for (row_index, (size, in_tree, nucleo)) in rows.iter().enumerate() {
        let separator = if row_index + 1 == rows.len() { "" } else { "," };
        println!(
            "    {{\"candidates\":{size},\"in_tree_p50_ns\":{},\"in_tree_p95_ns\":{},\"in_tree_max_ns\":{},\"in_tree_checksum\":{},\"nucleo_p50_ns\":{},\"nucleo_p95_ns\":{},\"nucleo_max_ns\":{},\"nucleo_checksum\":{}}}{separator}",
            in_tree.p50_ns,
            in_tree.p95_ns,
            in_tree.max_ns,
            in_tree.checksum,
            nucleo.p50_ns,
            nucleo.p95_ns,
            nucleo.max_ns,
            nucleo.checksum,
        );
    }
    println!("  ],");
    println!("  \"stale_candidates_before_cancel\": {stale_candidates_processed},");
    println!("  \"p95_ceiling_ms\": {},", MAX_P95.as_millis());
    println!("  \"within_ceiling\": {}", !exceeded);
    println!("}}");

    if exceeded {
        eprintln!("a matcher exceeded the 16 ms CP5.0 research ceiling");
        std::process::exit(3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_and_combining_corpus_is_searchable() {
        let candidates = corpus(32);
        assert!(!in_tree_match_list("déplo", &candidates).is_empty());
        assert!(!nucleo_match_list("déplo", &candidates).is_empty());
        assert!(!nucleo_match_list("café", &candidates).is_empty());
    }

    #[test]
    fn stale_generation_stops_before_the_eighth_candidate() {
        assert_eq!(stale_generation(&corpus(512), 9), 7);
    }

    #[test]
    fn corpus_sizes_and_sample_counts_are_fixed() {
        assert_eq!(CORPUS_SIZES, [32, 128, 512]);
        assert_eq!((WARMUPS, SAMPLES), (20, 200));
    }
}
