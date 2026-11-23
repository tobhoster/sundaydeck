use std::time::Duration;

use serde::{Deserialize, Serialize};
use ureq::Agent;

use crate::error::PlannerError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalLlmPreset {
    pub id: &'static str,
    pub label: &'static str,
    pub model_name: &'static str,
    pub ram_requirement: &'static str,
    pub notes: &'static str,
}

impl LocalLlmPreset {
    /// Curated local LLM recommendations for church presentation laptops and studio Macs.
    pub const ALL: &'static [Self] = &[
        Self {
            id: "qwen3-4b",
            label: "Qwen3 4B",
            model_name: "qwen3:4b",
            ram_requirement: "8-16 GB RAM",
            notes: "Recommended default for church laptops; strong JSON schema adherence.",
        },
        Self {
            id: "qwen25-3b",
            label: "Qwen2.5 3B",
            model_name: "qwen2.5-3b-instruct",
            ram_requirement: "8 GB RAM",
            notes: "Fast 2.0 GB Q4_K_M GGUF; 100% messy-doc accuracy with Hybrid Post-Pass.",
        },
        Self {
            id: "phi4-mini",
            label: "Phi-4 Mini",
            model_name: "phi4-mini",
            ram_requirement: "8 GB RAM",
            notes: "Microsoft 3.8B compact reasoning model for strict structured output.",
        },
        Self {
            id: "gemma3-4b",
            label: "Gemma 3 4B",
            model_name: "gemma3:4b",
            ram_requirement: "8-16 GB RAM",
            notes: "Google 4B instruction model; clean summarization of long pastor paragraphs.",
        },
        Self {
            id: "inkling-2-mini",
            label: "Inkling 2 Mini",
            model_name: "inkling-2-mini",
            ram_requirement: "32 GB+ RAM",
            notes: "Thinking Machines 26.4B MoE (~20 GB weights); great on 32 GB+ Apple Silicon Macs.",
        },
    ];
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalLlmConfig {
    /// OpenAI-compatible chat completions endpoint (llama-server, Ollama `/v1`, LM Studio).
    pub endpoint_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_tokens: u32,
    pub api_key: Option<String>,
}

impl Default for LocalLlmConfig {
    fn default() -> Self {
        Self {
            endpoint_url: "http://127.0.0.1:8080/v1/chat/completions".to_owned(),
            model: "qwen3:4b".to_owned(),
            timeout_secs: 120,
            max_tokens: 1400,
            api_key: None,
        }
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    temperature: f32,
    max_tokens: u32,
    response_format: ResponseFormat<'a>,
    messages: [ChatMessage<'a>; 2],
}

#[derive(Serialize)]
struct ResponseFormat<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageOut,
}

#[derive(Deserialize)]
struct ChatMessageOut {
    content: String,
}

/// Calls a local OpenAI-compatible inference server (`llama-server`, Ollama `/v1`, LM Studio)
/// and returns the assistant message content string.
pub fn call_local_llm(
    config: &LocalLlmConfig,
    system_prompt: &str,
    user_prompt: &str,
) -> Result<String, PlannerError> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(config.timeout_secs)))
        .build()
        .into();

    let payload = ChatRequest {
        model: &config.model,
        temperature: 0.1,
        max_tokens: config.max_tokens,
        response_format: ResponseFormat {
            kind: "json_object",
        },
        messages: [
            ChatMessage {
                role: "system",
                content: system_prompt,
            },
            ChatMessage {
                role: "user",
                content: user_prompt,
            },
        ],
    };

    let body_json = serde_json::to_string(&payload).map_err(|err| PlannerError::Http {
        detail: err.to_string(),
    })?;

    let mut req = agent
        .post(&config.endpoint_url)
        .header("Content-Type", "application/json");
    if let Some(key) = config.api_key.as_deref().filter(|k| !k.trim().is_empty()) {
        req = req.header("Authorization", &format!("Bearer {key}"));
    }

    let mut resp = req
        .send(body_json.as_str())
        .map_err(|err| PlannerError::Http {
            detail: err.to_string(),
        })?;
    let resp_text = resp
        .body_mut()
        .read_to_string()
        .map_err(|err| PlannerError::Http {
            detail: err.to_string(),
        })?;

    let parsed: ChatResponse =
        serde_json::from_str(&resp_text).map_err(|err| PlannerError::InvalidResponse {
            detail: format!("could not decode OpenAI chat response: {err}"),
        })?;

    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message.content)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| PlannerError::InvalidResponse {
            detail: "local LLM returned empty choices array".to_owned(),
        })
}
