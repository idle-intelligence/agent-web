//! `Generator` implementations built on lean's public native CPU and GPU
//! forward-pass APIs (`lean::cpu`, `lean::model`/`lean::engine`), and a
//! capability-based pick between them, mirroring lean's own CLI (which
//! takes `--engine gpu|cpu` and drives `lean::model::forward_prefill` or
//! `lean::cpu::forward_prefill` directly, picking the GPU path by default
//! and falling back to CPU when no adapter is available).
//!
//! Native only: wasm bindings for the agent live in the `agent-wasm` crate
//! and build their own generators against the same lean APIs, inside the
//! wasm-bindgen surface (which cannot share this module's `pollster`
//! blocking call).

use anyhow::Result;

use crate::agent::Generator;

/// Greedy decode on lean's native CPU engine: a fresh `CpuKvCache` sized
/// to `prompt_ids.len() + max_new_tokens` every call, matching the
/// agent's one-call-per-step usage (no KV-cache prefix reuse across
/// steps; agent turns are short enough that this is the simple, obviously
/// correct choice).
pub struct CpuGenerator {
    model: lean::cpu::CpuModel,
}

impl CpuGenerator {
    pub fn load(gguf_path: &str) -> Result<Self> {
        Ok(Self {
            model: lean::cpu::CpuModel::load(gguf_path)?,
        })
    }
}

impl Generator for CpuGenerator {
    fn generate(&mut self, prompt_ids: &[u32], max_new_tokens: usize) -> Result<Vec<u32>> {
        let max_ctx = prompt_ids.len() + max_new_tokens;
        let mut cache = lean::cpu::CpuKvCache::new(&self.model.config, max_ctx);
        let logits = lean::cpu::forward_prefill(&self.model, &mut cache, prompt_ids);
        let mut next_id = lean::model::argmax(&logits);
        let mut generated = Vec::with_capacity(max_new_tokens);
        for _ in 0..max_new_tokens {
            if self.model.config.eos_token_ids.contains(&next_id) {
                break;
            }
            generated.push(next_id);
            next_id = lean::cpu::forward_decode_step_argmax(&self.model, &mut cache, next_id);
        }
        Ok(generated)
    }
}

/// Greedy decode on lean's wgpu engine: builds a fresh rope table and
/// `KvCache` per call, blocking on lean's async forward functions with
/// `pollster` (native only, same pattern `lean-cli` uses).
pub struct GpuGenerator {
    engine: lean::engine::Engine,
    model: lean::model::GpuModel,
}

impl GpuGenerator {
    /// Fails if no wgpu adapter is available; callers should fall back to
    /// [`CpuGenerator`] in that case (see [`best_generator`]).
    pub fn load(gguf_path: &str) -> Result<Self> {
        let engine = pollster::block_on(lean::engine::Engine::new_async())?;
        let model = lean::model::GpuModel::load(&engine, gguf_path, true)?;
        Ok(Self { engine, model })
    }
}

impl Generator for GpuGenerator {
    fn generate(&mut self, prompt_ids: &[u32], max_new_tokens: usize) -> Result<Vec<u32>> {
        // A fresh generation must reset the model's buffer pool first, or
        // decode can rebind to a stale KvCache from a previous call.
        self.model.pool.reset();
        let max_ctx = prompt_ids.len() as u32 + max_new_tokens as u32 + 4;
        let mut cache = lean::model::KvCache::new(&self.engine, &self.model.config, max_ctx);
        let (cos, sin) = lean::model::build_rope_tables(
            self.model.config.head_dim,
            self.model.config.rope_theta,
            max_ctx as usize,
        );
        let cos_buf = self.engine.buf_f32(&cos, "rope_cos");
        let sin_buf = self.engine.buf_f32(&sin, "rope_sin");

        let logits = pollster::block_on(lean::model::forward_prefill(
            &self.engine,
            &self.model,
            &mut cache,
            prompt_ids,
            &cos_buf,
            &sin_buf,
            None,
        ));
        let mut next_id = lean::model::argmax(&logits);
        let mut generated = Vec::with_capacity(max_new_tokens);
        for _ in 0..max_new_tokens {
            if self.model.config.eos_token_ids.contains(&next_id) {
                break;
            }
            generated.push(next_id);
            next_id = pollster::block_on(lean::model::forward_decode_step_argmax(
                &self.engine,
                &self.model,
                &mut cache,
                next_id,
                &cos_buf,
                &sin_buf,
                None,
            ));
        }
        Ok(generated)
    }
}

/// Either engine, picked once at load time by capability (GPU when an
/// adapter is available, CPU otherwise) rather than by benchmarking.
pub enum AnyGenerator {
    Gpu(Box<GpuGenerator>),
    Cpu(Box<CpuGenerator>),
}

impl Generator for AnyGenerator {
    fn generate(&mut self, prompt_ids: &[u32], max_new_tokens: usize) -> Result<Vec<u32>> {
        match self {
            AnyGenerator::Gpu(g) => g.generate(prompt_ids, max_new_tokens),
            AnyGenerator::Cpu(c) => c.generate(prompt_ids, max_new_tokens),
        }
    }
}

/// Tries the GPU engine first (an adapter request failing, or the model's
/// `head_dim` being unsupported by the GPU attention kernels, are both
/// capability facts, not performance ones); falls back to the CPU engine
/// on either failure. Never benchmarks the two to choose.
pub fn best_generator(gguf_path: &str) -> Result<AnyGenerator> {
    match GpuGenerator::load(gguf_path) {
        Ok(g) => Ok(AnyGenerator::Gpu(Box::new(g))),
        Err(gpu_err) => {
            eprintln!("GPU engine unavailable ({gpu_err:#}), falling back to the CPU engine");
            Ok(AnyGenerator::Cpu(Box::new(CpuGenerator::load(gguf_path)?)))
        }
    }
}
