//! Gate G0: compares the Loro and Yrs CRDTs on workloads shaped like ALTAIS
//! notes. Both engines run the same deterministic edit traces; the report
//! (`cargo run --release -p altais-crdt-bench --bin crdt-report`) measures
//! speed, stored size and merging, and the tests check that both engines end
//! with the same text.
//!
//! Positions are counted in Unicode scalar values. The traces use only
//! characters from the Basic Multilingual Plane (Slovak, Czech, Hungarian,
//! Polish, Russian, Chinese), so Yrs runs with UTF-16 offsets, which are then
//! the same numbers.

pub mod engines;
pub mod paragraphs;
pub mod trace;

pub use engines::{Engine, LoroEngine, YrsEngine};
pub use trace::{Op, Rng};
