//! A tool-calling agent loop that works with any inference engine.
//!
//! This crate holds everything the engine itself does not: the prompt for
//! each step (the model's own chat template with the tools, rendered by
//! lean-chat), the Qwen2.5-Instruct tool-call parser, the step-wise agent
//! loop, and an offline eval harness. The loop reaches the model only
//! through the [`generator::Generator`] trait (prompt text in, reply text
//! out). [`lean_generator::LeanGenerator`] implements it on lean's engine,
//! which picks its backend by capability.

pub mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub mod eval;
pub mod generator;
#[cfg(feature = "lean")]
pub mod lean_generator;
#[cfg(test)]
mod test_support;
