//! Native child workloads for the Automexia process-control tests.

#![forbid(unsafe_code)]

mod workload;

pub use workload::Workload;

#[cfg(windows)]
pub const EXECUTABLE: &str = env!("AUTOMEXIA_PROCESS_TEST_FIXTURE_EXECUTABLE");
