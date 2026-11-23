use std::path::PathBuf;

use crate::error::PlannerError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedLlmConfig {
    pub model_path: PathBuf,
    pub n_ctx: u32,
    pub n_threads: i32,
    pub max_tokens: usize,
}

impl Default for EmbeddedLlmConfig {
    fn default() -> Self {
        let threads =
            std::thread::available_parallelism().map_or(4, |n| n.get().clamp(2, 12)) as i32;
        Self {
            model_path: default_embedded_model_path(),
            n_ctx: 4096,
            n_threads: threads,
            max_tokens: 1400,
        }
    }
}

pub fn default_embedded_model_path() -> PathBuf {
    if let Ok(custom) = std::env::var("SUNDAYDECK_EMBEDDED_MODEL")
        && !custom.trim().is_empty()
    {
        return PathBuf::from(custom);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(home).join(".cache/sundaydeck/models/qwen2.5-1.5b-instruct-q4_k_m.gguf")
}

pub fn is_embedded_runtime_compiled() -> bool {
    cfg!(feature = "embedded-llm")
}

#[derive(Debug, Clone)]
pub struct EmbeddedCallOutput {
    pub raw_text: String,
    pub cold_load_ms: u64,
    pub inference_ms: u64,
}

#[cfg(feature = "embedded-llm")]
pub fn call_embedded_llm(
    config: &EmbeddedLlmConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<EmbeddedCallOutput, PlannerError> {
    imp::run(config, system_prompt, user_prompt)
}

#[cfg(not(feature = "embedded-llm"))]
pub fn call_embedded_llm(
    _config: &EmbeddedLlmConfig,
    _system_prompt: &str,
    _user_prompt: &str,
) -> Result<EmbeddedCallOutput, PlannerError> {
    Err(PlannerError::EmbeddedDisabled)
}

#[cfg(feature = "embedded-llm")]
mod imp {
    use std::num::NonZeroU32;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::Instant;

    use llama_cpp_2::context::params::LlamaContextParams;
    use llama_cpp_2::llama_backend::LlamaBackend;
    use llama_cpp_2::llama_batch::LlamaBatch;
    use llama_cpp_2::model::params::LlamaModelParams;
    use llama_cpp_2::model::{LlamaChatMessage, LlamaChatTemplate, LlamaModel};
    use llama_cpp_2::sampling::LlamaSampler;

    use crate::embedded_llm::{EmbeddedCallOutput, EmbeddedLlmConfig};
    use crate::error::PlannerError;
    use crate::prompt::PLAN_JSON_GBNF;

    struct CachedModel {
        path: PathBuf,
        model: LlamaModel,
    }

    static BACKEND: OnceLock<Result<LlamaBackend, String>> = OnceLock::new();
    static MODEL_CACHE: OnceLock<Mutex<Option<CachedModel>>> = OnceLock::new();

    fn backend() -> Result<&'static LlamaBackend, PlannerError> {
        let res = BACKEND.get_or_init(|| {
            let mut b = LlamaBackend::init().map_err(|e| e.to_string())?;
            b.void_logs();
            Ok(b)
        });
        res.as_ref()
            .map_err(|detail| PlannerError::EmbeddedRuntime {
                detail: detail.clone(),
            })
    }

    pub fn run(
        config: &EmbeddedLlmConfig,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<EmbeddedCallOutput, PlannerError> {
        if !config.model_path.exists() {
            return Err(PlannerError::ModelNotFound {
                path: config.model_path.clone(),
            });
        }
        let backend = backend()?;
        let cache = MODEL_CACHE.get_or_init(|| Mutex::new(None));
        let mut guard = cache.lock().map_err(|_| PlannerError::EmbeddedRuntime {
            detail: "embedded model cache mutex poisoned".to_owned(),
        })?;

        let needs_load = guard
            .as_ref()
            .is_none_or(|cached| cached.path != config.model_path);
        let cold_load_ms = if needs_load {
            let t0 = Instant::now();
            let model_params = LlamaModelParams::default();
            let loaded = LlamaModel::load_from_file(backend, &config.model_path, &model_params)
                .map_err(|err| PlannerError::EmbeddedRuntime {
                    detail: format!("failed loading GGUF model: {err}"),
                })?;
            let ms = t0.elapsed().as_millis() as u64;
            *guard = Some(CachedModel {
                path: config.model_path.clone(),
                model: loaded,
            });
            ms
        } else {
            0
        };

        let model = &guard
            .as_ref()
            .ok_or_else(|| PlannerError::EmbeddedRuntime {
                detail: "model cache unexpectedly empty".to_owned(),
            })?
            .model;

        let infer_start = Instant::now();
        let prompt = format_chat_prompt(model, system_prompt, user_prompt)?;
        let vocab = model.vocab();
        let tokens = vocab.tokenize(prompt.as_bytes(), true, true);
        if tokens.is_empty() {
            return Err(PlannerError::EmbeddedRuntime {
                detail: "prompt tokenized to zero tokens".to_owned(),
            });
        }

        let n_ctx = config
            .n_ctx
            .max((tokens.len() + config.max_tokens + 64) as u32);
        let n_batch = 512u32;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(n_batch)
            .with_n_threads(config.n_threads)
            .with_n_threads_batch(config.n_threads);

        let mut ctx = model.new_context(backend, ctx_params).map_err(|err| {
            PlannerError::EmbeddedRuntime {
                detail: format!("failed creating llama context: {err}"),
            }
        })?;

        let mut batch = LlamaBatch::new(n_batch as usize, 1);
        let mut pos: i32 = 0;
        let total_prompt = tokens.len();
        for chunk in tokens.chunks(n_batch as usize) {
            batch.clear();
            for &tok in chunk {
                let is_last_overall = (pos as usize) + 1 == total_prompt;
                batch.add(tok, pos, &[0], is_last_overall).map_err(|err| {
                    PlannerError::EmbeddedRuntime {
                        detail: format!("batch add prompt token failed: {err}"),
                    }
                })?;
                pos += 1;
            }
            ctx.decode(&mut batch)
                .map_err(|err| PlannerError::EmbeddedRuntime {
                    detail: format!("prompt decode failed: {err}"),
                })?;
        }

        let mut sampler = build_json_sampler(model);
        let mut out_bytes: Vec<u8> = Vec::with_capacity(2048);

        for _ in 0..config.max_tokens {
            // Note: `LlamaSampler::sample` in llama-cpp-2 already accepts the sampled token
            // internally — calling `sampler.accept(token)` again would double-advance GBNF state.
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            if vocab.is_eog(token) {
                break;
            }
            vocab.token_to_piece_into(token, &mut out_bytes, false, None);

            batch.clear();
            batch
                .add(token, pos, &[0], true)
                .map_err(|err| PlannerError::EmbeddedRuntime {
                    detail: format!("batch add generated token failed: {err}"),
                })?;
            pos += 1;
            ctx.decode(&mut batch)
                .map_err(|err| PlannerError::EmbeddedRuntime {
                    detail: format!("token decode failed: {err}"),
                })?;
        }

        let inference_ms = infer_start.elapsed().as_millis() as u64;
        let raw_text = String::from_utf8_lossy(&out_bytes).into_owned();
        Ok(EmbeddedCallOutput {
            raw_text,
            cold_load_ms,
            inference_ms,
        })
    }

    fn format_chat_prompt(
        model: &LlamaModel,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, PlannerError> {
        let msgs = [
            LlamaChatMessage::new("system".to_owned(), system_prompt.to_owned()).map_err(
                |err| PlannerError::EmbeddedRuntime {
                    detail: err.to_string(),
                },
            )?,
            LlamaChatMessage::new("user".to_owned(), user_prompt.to_owned()).map_err(|err| {
                PlannerError::EmbeddedRuntime {
                    detail: err.to_string(),
                }
            })?,
        ];

        let tmpl = model
            .chat_template(None)
            .or_else(|_| LlamaChatTemplate::new("chatml").map_err(|_| ()));
        if let Ok(tmpl) = tmpl
            && let Ok(rendered) = model.apply_chat_template(&tmpl, &msgs, true)
        {
            return Ok(rendered);
        }

        Ok(format!(
            "<|im_start|>system\n{system_prompt}<|im_end|>\n<|im_start|>user\n{user_prompt}<|im_end|>\n<|im_start|>assistant\n"
        ))
    }

    fn build_json_sampler(model: &LlamaModel) -> LlamaSampler {
        if let Ok(grammar) = LlamaSampler::grammar(model, PLAN_JSON_GBNF, "root") {
            LlamaSampler::chain_simple([grammar, LlamaSampler::temp(0.1), LlamaSampler::greedy()])
        } else {
            LlamaSampler::chain_simple([LlamaSampler::temp(0.1), LlamaSampler::greedy()])
        }
    }
}
