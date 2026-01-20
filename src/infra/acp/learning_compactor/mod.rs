//! Learning compactor module for analyzing rejected feedback patterns.
//!
//! Uses an ACP agent to process feedback rejections and generate learned patterns
//! that guide future reviews to avoid unhelpful feedback.

mod client;
mod worker;

pub use worker::{LearningCompactionInput, run_learning_compaction};
