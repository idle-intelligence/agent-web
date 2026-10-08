//! A tool-calling agent on top of the lean inference engine.
//!
//! This crate holds everything the engine itself does not: the
//! Qwen2.5-Instruct tool-calling chat template and output parser, the
//! step-wise agent loop, and an offline eval harness. It depends on
//! lean's public Rust API for the model forward pass, using lean's normal
//! engine selection (GPU when available, CPU otherwise); it never pins a
//! backend.

pub mod agent;
#[cfg(not(target_arch = "wasm32"))]
pub mod backend;
#[cfg(not(target_arch = "wasm32"))]
pub mod eval;
