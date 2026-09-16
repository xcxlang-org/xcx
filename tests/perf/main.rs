//! Machine-independent performance/resource guards: ratio assertions only
//! (self-timed programs and memory-growth ratios) — never absolute
//! wall-clock thresholds, so the guards hold on any hardware.

#[path = "../common/mod.rs"]
mod common;

mod perf_guards;
